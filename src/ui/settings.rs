use crate::app::RClashApp;

pub fn show(ui: &mut egui::Ui, app: &mut RClashApp, ctx: &egui::Context, has_tray: bool) {
    use super::widgets;
    widgets::section_header(ui, "Настройки", Some("Персонализация, сеть и ядро"));

    let proxy_on = rclash_sys_proxy::current()
        .get()
        .map(|s| s == rclash_sys_proxy::ProxyState::Enabled)
        .unwrap_or(false);
    let autostart_on = rclash_autostart::current().is_enabled().unwrap_or(false);

    egui::ScrollArea::vertical().show(ui, |ui| {
        widgets::card(ui, |ui| {
            ui.label(egui::RichText::new("Общие").strong());
            ui.add_space(4.0);
            egui::Grid::new("settings_general")
                .num_columns(2)
                .spacing([16.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Тема:");
                    let mut is_dark = app.app_config.theme == rclash_config::Theme::Dark;
                    let theme_label = if is_dark {
                        "Тёмная"
                    } else {
                        "Светлая"
                    };
                    if ui.checkbox(&mut is_dark, theme_label).changed() {
                        let next = if is_dark {
                            rclash_config::Theme::Dark
                        } else {
                            rclash_config::Theme::Light
                        };
                        app.app_config.theme = next;
                        ctx.set_visuals(match next {
                            rclash_config::Theme::Dark => egui::Visuals::dark(),
                            rclash_config::Theme::Light => egui::Visuals::light(),
                        });
                        let _ = rclash_config::save_app_config(&app.app_config);
                    }
                    ui.end_row();
                    ui.label("Сворачивать в трей:");
                    let mut tray_checked = app.app_config.minimize_to_tray;
                    let tray_label = if tray_checked { "Да" } else { "Нет" };
                    ui.add_enabled_ui(has_tray, |ui| {
                        if ui.checkbox(&mut tray_checked, tray_label).changed() {
                            app.app_config.minimize_to_tray = tray_checked;
                            let _ = rclash_config::save_app_config(&app.app_config);
                        }
                    });
                    if !has_tray {
                        ui.label(egui::RichText::new("(трей недоступен)").weak().small());
                    }
                    ui.end_row();
                    ui.label("Автозапуск:");
                    let mut auto_checked = autostart_on;
                    if ui
                        .checkbox(&mut auto_checked, "Вместе с системой")
                        .changed()
                    {
                        let res = if auto_checked {
                            rclash_autostart::current().enable()
                        } else {
                            rclash_autostart::current().disable()
                        };
                        if let Err(e) = res {
                            ui.label(
                                egui::RichText::new(format!("Ошибка: {e}"))
                                    .color(egui::Color32::from_rgb(220, 80, 80)),
                            );
                        }
                    }
                    ui.end_row();
                });
            if autostart_on {
                ui.add_space(4.0);
                ui.small("Автозапуск включён — стартует с --minimized");
            }
        });

        ui.add_space(8.0);
        widgets::card(ui, |ui| {
            ui.label(egui::RichText::new("Ядро").strong());
            ui.small("Параметры подключения к rclash-core");
            ui.add_space(8.0);
            egui::Grid::new("settings_core")
                .num_columns(2)
                .spacing([16.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Контроллер:");
                    let mut addr = String::from("127.0.0.1:9090");
                    ui.add_enabled(false, egui::TextEdit::singleline(&mut addr))
                        .on_disabled_hover_text(
                            "Настройка в следующем этапе (пока 127.0.0.1:9090)",
                        );
                    ui.end_row();
                    ui.label("Secret:");
                    let mut secret = String::new();
                    ui.add(
                        egui::TextEdit::singleline(&mut secret)
                            .hint_text("опционально")
                            .password(true),
                    )
                    .on_hover_text("Bearer для Authorization");
                    ui.end_row();
                    ui.label("Лог уровень (приложение):");
                    egui::ComboBox::from_id_salt("log_level")
                        .selected_text(app.app_config.log_level.label_ru())
                        .show_ui(ui, |ui| {
                            for lvl in rclash_config::LogLevel::all() {
                                if ui
                                    .selectable_value(
                                        &mut app.app_config.log_level,
                                        *lvl,
                                        lvl.label_ru(),
                                    )
                                    .clicked()
                                {
                                    crate::logger::set_level(lvl.to_log_filter());
                                    let _ = rclash_config::save_app_config(&app.app_config);
                                    log::info!("log level changed to {}", lvl.as_str());
                                }
                            }
                        });
                    ui.end_row();
                    ui.label("Логи ядра (WS):");
                    ui.label(
                        egui::RichText::new("настраивается во вкладке Логи → уровень")
                            .weak()
                            .small(),
                    );
                    ui.end_row();
                    ui.label("Файл логов:");
                    let log_path = crate::logger::log_file_path()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "—".to_owned());
                    ui.horizontal(|ui| {
                        ui.monospace(&log_path);
                        if ui.small_button("📂 Открыть").clicked() {
                            if let Some(dir) = crate::logger::logs_dir() {
                                let _ = std::fs::create_dir_all(&dir);
                                #[cfg(target_os = "windows")]
                                {
                                    let _ =
                                        std::process::Command::new("explorer").arg(&dir).spawn();
                                }
                                #[cfg(target_os = "linux")]
                                {
                                    let _ =
                                        std::process::Command::new("xdg-open").arg(&dir).spawn();
                                }
                                #[cfg(target_os = "macos")]
                                {
                                    let _ = std::process::Command::new("open").arg(&dir).spawn();
                                }
                            }
                        }
                    });
                    ui.end_row();
                });
        });

        ui.add_space(8.0);
        widgets::card(ui, |ui| {
            ui.label(egui::RichText::new("Сеть").strong());
            ui.small("Прокси и TUN");
            ui.add_space(8.0);
            egui::Grid::new("settings_network")
                .num_columns(2)
                .spacing([16.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Системный прокси:");
                    let mut checked = proxy_on;
                    let label = if checked {
                        "Включён (127.0.0.1:7890)"
                    } else {
                        "Выключен"
                    };
                    if ui.checkbox(&mut checked, label).changed() {
                        let _ = rclash_sys_proxy::current().set(
                            if checked {
                                rclash_sys_proxy::ProxyState::Enabled
                            } else {
                                rclash_sys_proxy::ProxyState::Disabled
                            },
                            "127.0.0.1:7890",
                        );
                    }
                    ui.end_row();
                    ui.label("TUN режим:");
                    let tun_status = rclash_tun::status();
                    let tun_on = matches!(tun_status, rclash_tun::TunStatus::Enabled { .. });
                    let mut tun_checked = app.app_config.tun_enabled || tun_on;
                    let tun_label = if tun_on {
                        "Включён ✓"
                    } else if tun_checked {
                        "Включается…"
                    } else {
                        "Выключен"
                    };
                    if ui.checkbox(&mut tun_checked, tun_label).changed() {
                        if tun_checked {
                            match rclash_tun::enable() {
                                Ok(()) => {
                                    app.app_config.tun_enabled = true;
                                    let _ = rclash_config::save_app_config(&app.app_config);
                                    log::info!("TUN enabled");
                                }
                                Err(e) => {
                                    log::warn!("TUN enable failed: {e}");
                                    ui.label(
                                        egui::RichText::new(format!("Ошибка: {e}"))
                                            .color(egui::Color32::from_rgb(220, 80, 80))
                                            .small(),
                                    );
                                }
                            }
                        } else {
                            match rclash_tun::disable() {
                                Ok(()) => {
                                    app.app_config.tun_enabled = false;
                                    let _ = rclash_config::save_app_config(&app.app_config);
                                    log::info!("TUN disabled");
                                }
                                Err(e) => {
                                    log::warn!("TUN disable failed: {e}");
                                }
                            }
                        }
                    }
                    match tun_status {
                        rclash_tun::TunStatus::Enabled { name } => {
                            ui.label(egui::RichText::new(name).weak().small());
                        }
                        rclash_tun::TunStatus::Disabled => {
                            ui.label(
                                egui::RichText::new("Linux pkexec · Win Service · macOS osascript")
                                    .weak()
                                    .small(),
                            );
                        }
                        rclash_tun::TunStatus::Error(e) => {
                            ui.label(
                                egui::RichText::new(e)
                                    .color(egui::Color32::from_rgb(220, 180, 60))
                                    .small(),
                            );
                        }
                    }
                    ui.end_row();
                });
            ui.add_space(4.0);
            ui.small("Подсказка: системный прокси покрывает браузеры; TUN — весь трафик (F1).");
        });

        ui.add_space(8.0);
        widgets::card(ui, |ui| {
            ui.label(egui::RichText::new("О программе").strong());
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Версия:");
                ui.monospace("v0.1.0-rclash");
                let btn_label = if app.updater_checking {
                    "Проверка…"
                } else {
                    "Проверить обновления"
                };
                if ui
                    .add_enabled(!app.updater_checking, egui::Button::new(btn_label))
                    .clicked()
                {
                    app.check_update_async(ctx, true);
                }
                if app.updater_downloading {
                    ui.spinner();
                    ui.label("Загрузка…");
                }
            });
            let upd_ver = app.updater_available.as_ref().map(|i| i.version.clone());
            let mut do_up = false;
            let mut do_later = false;
            if let Some(ver) = upd_ver {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Доступно {ver} — установить?"))
                            .color(egui::Color32::from_rgb(120, 180, 255))
                            .strong(),
                    );
                    if ui.button("Обновить").clicked() {
                        do_up = true;
                    }
                    if ui.button("Позже").clicked() {
                        do_later = true;
                    }
                });
            }
            if do_up {
                app.download_update_async(ctx);
            }
            if do_later {
                app.dismiss_update();
            }
            if let Some(e) = &app.updater_error {
                ui.label(
                    egui::RichText::new(format!("Ошибка обновления: {e}"))
                        .color(egui::Color32::from_rgb(220, 80, 80))
                        .small(),
                );
            } else {
                let interval_label = app.app_config.update_interval.label_ru();
                let skipped = app.app_config.skipped_version.as_deref().unwrap_or("—");
                ui.small(format!(
                    "Интервал: {} · Пропущена: {} · Последняя проверка: {}",
                    interval_label,
                    skipped,
                    app.app_config.last_check.as_deref().unwrap_or("никогда")
                ));
            }
            ui.add_space(4.0);
            let cfg_path = rclash_config::config_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "—".to_owned());
            ui.horizontal(|ui| {
                ui.label("Конфиги:");
                ui.monospace(&cfg_path);
            });
            ui.small("Manifest источник — RClash/rclash core-nightly");
        });
    });
}
