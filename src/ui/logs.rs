use crate::app::{LogsTab, RClashApp};
use egui::{Color32, RichText};
use rclash_core_manager::api::log_level_color;

pub fn show(ui: &mut egui::Ui, app: &mut RClashApp, _ctx: &egui::Context) {
    super::widgets::section_header(
        ui,
        "Логи",
        Some("WS /logs + логи приложения — уровни, поиск, автоскролл"),
    );

    ui.horizontal(|ui| {
        for tab in [LogsTab::Core, LogsTab::App] {
            let sel = app.logs_tab == tab;
            if ui.selectable_label(sel, tab.label_ru()).clicked() {
                app.logs_tab = tab;
                log::info!("logs tab switched to {:?}", tab);
            }
        }
        ui.separator();
        if app.logs_tab == LogsTab::Core {
            ui.label("Уровень:");
            egui::ComboBox::from_id_salt("log_level")
                .selected_text(&app.logs_level)
                .show_ui(ui, |ui| {
                    for lvl in ["debug", "info", "warning", "error", "silent"] {
                        if ui
                            .selectable_value(&mut app.logs_level, lvl.to_owned(), lvl)
                            .clicked()
                        {
                            app.restart_logs_stream();
                            log::info!("core log level changed to {}", lvl);
                        }
                    }
                });
        } else {
            ui.label("Уровень:");
            let current = app.app_config.log_level.as_str().to_owned();
            egui::ComboBox::from_id_salt("app_log_level")
                .selected_text(&current)
                .show_ui(ui, |ui| {
                    for lvl in rclash_config::LogLevel::all() {
                        let mut is_sel = app.app_config.log_level == *lvl;
                        if ui.checkbox(&mut is_sel, lvl.label_ru()).clicked() {
                            app.set_log_level(*lvl, ui.ctx());
                        }
                    }
                });
        }
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("Фильтр:");
        ui.add(
            egui::TextEdit::singleline(&mut app.logs_filter)
                .hint_text("подстрока payload")
                .desired_width(160.0),
        );
        ui.label("Поиск:");
        ui.add(
            egui::TextEdit::singleline(&mut app.logs_search)
                .hint_text("выделение")
                .desired_width(160.0),
        );
        ui.checkbox(&mut app.logs_autoscroll, "Автоскролл");
        if ui.button("Очистить").clicked() {
            if app.logs_tab == LogsTab::Core {
                app.logs_buf.clear();
                log::info!("core logs cleared");
            } else {
                crate::logger::clear();
            }
        }
        if ui.button("Копировать").clicked() {
            let text = if app.logs_tab == LogsTab::Core {
                filtered_core(app)
                    .into_iter()
                    .map(|e| format!("[{}] {}", e.level, e.payload))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                filtered_app(app)
                    .into_iter()
                    .map(|e| format!("[{}][{}] {}", e.level, e.target, e.message))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            ui.ctx().copy_text(text);
        }
        if app.logs_tab == LogsTab::App {
            if ui
                .button("📂 Папка логов")
                .on_hover_text("Открыть папку логов")
                .clicked()
            {
                if let Some(dir) = crate::logger::logs_dir() {
                    let _ = std::fs::create_dir_all(&dir);
                    #[cfg(target_os = "windows")]
                    {
                        let _ = std::process::Command::new("explorer").arg(&dir).spawn();
                    }
                    #[cfg(target_os = "linux")]
                    {
                        let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
                    }
                    #[cfg(target_os = "macos")]
                    {
                        let _ = std::process::Command::new("open").arg(&dir).spawn();
                    }
                    log::info!("open logs dir: {}", dir.display());
                }
            }
            if let Some(path) = crate::logger::log_file_path() {
                ui.label(RichText::new(path.display().to_string()).weak().small());
            }
        }
    });

    ui.separator();

    match app.logs_tab {
        LogsTab::Core => show_core(ui, app),
        LogsTab::App => show_app(ui, app),
    }
}

fn show_core(ui: &mut egui::Ui, app: &RClashApp) {
    let filtered = filtered_core(app);
    if filtered.is_empty() {
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(RichText::new("Ожидание логов WS /logs …").weak());
        });
        ui.small(format!(
            "Уровень: {} · фильтр: '{}' · всего в буфере: {}",
            app.logs_level,
            app.logs_filter,
            app.logs_buf.len()
        ));
        return;
    }
    ui.label(
        RichText::new(format!(
            "Показано {}/{} · уровень {} · фильтр '{}'",
            filtered.len(),
            app.logs_buf.len(),
            app.logs_level,
            app.logs_filter
        ))
        .weak()
        .small(),
    );
    egui::ScrollArea::vertical()
        .stick_to_bottom(app.logs_autoscroll)
        .auto_shrink([false, false])
        .max_height(f32::INFINITY)
        .show(ui, |ui| {
            for e in filtered {
                let color = log_level_color(&e.level);
                let needs_highlight =
                    !app.logs_search.is_empty() && e.payload.contains(&app.logs_search);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&e.level).color(color).small().strong());
                    if needs_highlight {
                        ui.label(
                            RichText::new(&e.payload)
                                .background_color(Color32::from_rgb(60, 60, 20))
                                .monospace()
                                .small(),
                        );
                    } else {
                        ui.monospace(RichText::new(&e.payload).small());
                    }
                });
            }
        });
}

fn show_app(ui: &mut egui::Ui, app: &RClashApp) {
    let entries = crate::logger::snapshot();
    let filtered = filtered_app(app);
    if filtered.is_empty() && entries.is_empty() {
        ui.add_space(12.0);
        super::widgets::empty_state(
            ui,
            "📝",
            "Логов приложения пока нет — действия появятся здесь",
            None,
        );
        return;
    }
    if filtered.is_empty() {
        ui.label(RichText::new("Ничего не найдено по фильтру").weak());
        return;
    }
    ui.label(
        RichText::new(format!(
            "Показано {}/{} · фильтр '{}'",
            filtered.len(),
            entries.len(),
            app.logs_filter
        ))
        .weak()
        .small(),
    );
    egui::ScrollArea::vertical()
        .stick_to_bottom(app.logs_autoscroll)
        .show(ui, |ui| {
            for e in filtered {
                let color = log_level_color(&e.level);
                let needs_highlight = !app.logs_search.is_empty()
                    && (e.message.contains(&app.logs_search)
                        || e.target.contains(&app.logs_search));
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&e.time).weak().small().monospace());
                    ui.label(RichText::new(&e.level).color(color).small().strong());
                    ui.label(RichText::new(&e.target).weak().small());
                    if needs_highlight {
                        ui.label(
                            RichText::new(&e.message)
                                .background_color(Color32::from_rgb(60, 60, 20))
                                .monospace()
                                .small(),
                        );
                    } else {
                        ui.monospace(RichText::new(&e.message).small());
                    }
                });
            }
        });
}

fn filtered_core(app: &RClashApp) -> Vec<rclash_core_manager::api::LogEntry> {
    let filter = app.logs_filter.to_lowercase();
    let search = app.logs_search.to_lowercase();
    app.logs_buf
        .iter()
        .filter(|e| {
            if !filter.is_empty() && !e.payload.to_lowercase().contains(&filter) {
                return false;
            }
            if !search.is_empty()
                && !(e.payload.to_lowercase().contains(&search)
                    || e.level.to_lowercase().contains(&search))
            {
                return false;
            }
            true
        })
        .cloned()
        .collect()
}

fn filtered_app(app: &RClashApp) -> Vec<crate::logger::AppLogEntry> {
    let entries = crate::logger::snapshot();
    let filter = app.logs_filter.to_lowercase();
    let search = app.logs_search.to_lowercase();
    entries
        .into_iter()
        .filter(|e| {
            if !filter.is_empty()
                && !(e.message.to_lowercase().contains(&filter)
                    || e.target.to_lowercase().contains(&filter)
                    || e.level.to_lowercase().contains(&filter))
            {
                return false;
            }
            if !search.is_empty()
                && !(e.message.to_lowercase().contains(&search)
                    || e.target.to_lowercase().contains(&search))
            {
                return false;
            }
            true
        })
        .collect()
}
