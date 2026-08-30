use crate::app::RClashApp;
use rclash_config::UpdateInterval;

pub fn show(ui: &mut egui::Ui, app: &mut RClashApp, ctx: &egui::Context) {
    super::widgets::section_header(
        ui,
        "Профили",
        Some("Импорт, подписки и сырые ссылки — единый custom.yaml"),
    );

    if let Some(err) = app.profiles_error.clone() {
        super::widgets::inline_error(ui, &err);
        ui.add_space(4.0);
    }
    if let Some(err) = app.raw_error.clone() {
        super::widgets::inline_error(ui, &err);
        ui.add_space(4.0);
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        super::widgets::card(ui, |ui| {
            ui.label(egui::RichText::new("Импорт профиля").strong());
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("📁 Импортировать файл…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("YAML", &["yaml", "yml"])
                        .pick_file()
                    {
                        let name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("profile")
                            .to_owned();
                        match rclash_config::profile::import_profile_file(&path, &name) {
                            Ok(profile) => {
                                let mut store = app.profile_store.clone();
                                store.add_or_replace(profile.clone());
                                if store.active.is_none() {
                                    store.active = Some(profile.name.clone());
                                }
                                if rclash_config::profile::save_profile_store(&store).is_ok() {
                                    app.profile_store = store;
                                    app.profiles_error = None;
                                    reload_core(ctx);
                                } else {
                                    app.profiles_error = Some("Не удалось сохранить список профилей".into());
                                }
                            }
                            Err(e) => {
                                app.profiles_error = Some(format!("Ошибка импорта: {e}"));
                            }
                        }
                    }
                }
                if ui.button("↻ Обновить список").clicked() {
                    app.refresh_profiles();
                }
            });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Импорт по URL (подписка)").small().weak());
            ui.horizontal(|ui| {
                ui.label("URL:");
                ui.add(
                    egui::TextEdit::singleline(&mut app.import_url)
                        .hint_text("https://example.com/sub.yaml")
                        .desired_width(320.0),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Интервал:");
                egui::ComboBox::from_id_salt("import_interval")
                    .selected_text(app.import_interval.label_ru())
                    .show_ui(ui, |ui| {
                        for iv in UpdateInterval::all() {
                            ui.selectable_value(&mut app.import_interval, *iv, iv.label_ru());
                        }
                    });
                ui.label(
                    egui::RichText::new("(из подписки — иначе 24ч)")
                        .weak()
                        .small(),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Имя:");
                ui.add(
                    egui::TextEdit::singleline(&mut app.import_name)
                        .hint_text("необязательно — из URL")
                        .desired_width(200.0),
                );
                if ui.button("⬇ Загрузить").clicked() {
                    let url = app.import_url.trim().to_owned();
                    let interval = app.import_interval;
                    let name_hint = app.import_name.trim().to_owned();
                    if url.is_empty() {
                        app.profiles_error = Some("URL пустой".into());
                    } else {
                        let name = if name_hint.is_empty() {
                            url.split('/').next_back().unwrap_or("subscription").split('?').next().unwrap_or("subscription").to_owned()
                        } else {
                            name_hint
                        };
                        match fetch_and_save_subscription(&url, &name, interval) {
                            Ok(profile) => {
                                let mut store = app.profile_store.clone();
                                store.add_or_replace(profile);
                                let _ = rclash_config::profile::save_profile_store(&store);
                                app.profile_store = store;
                                app.profiles_error = None;
                                app.import_url.clear();
                                app.import_name.clear();
                                reload_core(ctx);
                            }
                            Err(e) => {
                                app.profiles_error = Some(format!("Ошибка загрузки: {e}"));
                            }
                        }
                    }
                }
            });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Сырые ссылки (hysteria2 / trojan / vless / vmess / ss)").small().weak());
            ui.label(
                egui::RichText::new("Вставь ссылки — каждая с новой строки, пойдут в profiles/custom.yaml (PROXY)").weak().small(),
            );
            ui.add(
                egui::TextEdit::multiline(&mut app.import_raw_text)
                    .hint_text("hysteria2://...\ntrojan://...\nvless://...#name")
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            );
            ui.horizontal(|ui| {
                if ui.button("➕ Добавить в Сырые ссылки").clicked() {
                    let text = app.import_raw_text.trim().to_owned();
                    if text.is_empty() {
                        app.raw_error = Some("Вставь хотя бы одну ссылку".into());
                    } else {
                        match rclash_subscription::parse_text_links(&text) {
                            Ok(proxies) if proxies.is_empty() => {
                                app.raw_error = Some("Не распознано ни одной ссылки".into());
                            }
                            Ok(proxies) => {
                                match rclash_config::custom::add_raw_proxies(proxies) {
                                    Ok(added) => {
                                        app.raw_error = None;
                                        app.reload_raw_keys();
                                        app.import_raw_text.clear();
                                        if added == 0 {
                                            app.raw_error = Some("Все прокси уже есть (dedup)".into());
                                        } else {
                                            reload_core(ctx);
                                        }
                                    }
                                    Err(e) => {
                                        app.raw_error = Some(format!("Ошибка записи custom.yaml: {e}"));
                                    }
                                }
                            }
                            Err(e) => {
                                app.raw_error = Some(format!("Ошибка парсинга: {e}"));
                            }
                        }
                    }
                }
                if ui.button("🗑 Очистить поле").clicked() {
                    app.import_raw_text.clear();
                }
            });
        });

        ui.add_space(8.0);

        if app.profile_store.profiles.is_empty() {
            super::widgets::card(ui, |ui| {
                let clicked = super::widgets::empty_state(
                    ui,
                    "📂",
                    "Нет профилей — импортируй файл или подписку",
                    Some("Импортировать файл…"),
                );
                if clicked {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("YAML", &["yaml", "yml"])
                        .pick_file()
                    {
                        let name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("profile")
                            .to_owned();
                        if let Ok(profile) = rclash_config::profile::import_profile_file(&path, &name) {
                            let mut store = app.profile_store.clone();
                            store.add_or_replace(profile);
                            let _ = rclash_config::profile::save_profile_store(&store);
                            app.profile_store = store;
                            reload_core(ctx);
                        }
                    }
                }
            });
        } else {
            super::widgets::card(ui, |ui| {
                ui.label(egui::RichText::new(format!("Профили — {} шт.", app.profile_store.profiles.len())).strong());
                ui.add_space(4.0);
                egui::Grid::new("profiles_grid")
                    .num_columns(1)
                    .spacing([8.0, 8.0])
                    .show(ui, |ui| {
                        let profiles = app.profile_store.profiles.clone();
                        let active = app.profile_store.active.clone();
                        for profile in profiles {
                            let is_active = active.as_deref() == Some(&profile.name);
                            ui.horizontal(|ui| {
                                if is_active {
                                    super::widgets::badge(ui, "● Активен", egui::Color32::from_rgb(80, 200, 120));
                                } else {
                                    super::widgets::badge(ui, "○", egui::Color32::from_gray(120));
                                }
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new(&profile.name).strong());
                                    ui.label(
                                        egui::RichText::new(&profile.path)
                                            .weak()
                                            .small()
                                            .monospace(),
                                    );
                                    if let Some(url) = &profile.url {
                                        ui.label(
                                            egui::RichText::new(url).weak().small(),
                                        );
                                        if let Some(iv) = profile.update_interval {
                                            ui.label(
                                                egui::RichText::new(format!("↻ {}", iv.label_ru()))
                                                    .weak()
                                                    .small(),
                                            );
                                        }
                                    }
                                    if profile.is_raw {
                                        ui.label(
                                            egui::RichText::new("RAW · custom.yaml")
                                                .weak()
                                                .small(),
                                        );
                                    }
                                });
                            });
                            ui.horizontal(|ui| {
                                if !is_active && ui.small_button("✓ Активировать").clicked() {
                                    let mut store = app.profile_store.clone();
                                    store.active = Some(profile.name.clone());
                                    if rclash_config::profile::save_profile_store(&store).is_ok() {
                                        app.profile_store = store;
                                        reload_core(ctx);
                                    }
                                }
                                if ui.small_button("🗑 Удалить").clicked() {
                                    let mut store = app.profile_store.clone();
                                    store.remove(&profile.name);
                                    let path = std::path::Path::new(&profile.path);
                                    let _ = std::fs::remove_file(path);
                                    let _ = rclash_config::profile::save_profile_store(&store);
                                    app.profile_store = store;
                                    app.profiles_error = None;
                                    reload_core(ctx);
                                }
                                if profile.url.is_some() && ui.small_button("↻ Обновить").clicked() {
                                    let url = profile.url.clone().unwrap();
                                    let name = profile.name.clone();
                                    let interval = profile.update_interval.unwrap_or(UpdateInterval::H24);
                                    match fetch_and_save_subscription(&url, &name, interval) {
                                        Ok(_) => {
                                            app.profiles_error = None;
                                            reload_core(ctx);
                                        }
                                        Err(e) => {
                                            app.profiles_error = Some(format!("Ошибка обновления {name}: {e}"));
                                        }
                                    }
                                }
                            });
                            ui.end_row();
                            ui.separator();
                            ui.end_row();
                        }
                    });
            });
        }

        ui.add_space(8.0);

        super::widgets::card(ui, |ui| {
            let header = format!("Сырые ссылки — {} прокси (profiles/custom.yaml)", app.raw_keys.len());
            egui::CollapsingHeader::new(header)
                .default_open(false)
                .show(ui, |ui| {
                    if app.raw_keys.is_empty() {
                        ui.label(egui::RichText::new("Пока пусто — добавь ссылки выше").weak().italics());
                    } else {
                        ui.add_space(4.0);
                        let keys = app.raw_keys.clone();
                        for key in keys {
                            ui.horizontal(|ui| {
                                ui.monospace(&key);
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button("🗑 Удалить").clicked() {
                                        match rclash_config::custom::remove_raw_proxy(&key) {
                                            Ok(true) => {
                                                app.reload_raw_keys();
                                                app.raw_error = None;
                                                reload_core(ctx);
                                            }
                                            Ok(false) => {
                                                app.raw_error = Some(format!("Не найдено: {key}"));
                                            }
                                            Err(e) => {
                                                app.raw_error = Some(format!("Ошибка удаления: {e}"));
                                            }
                                        }
                                    }
                                });
                            });
                            ui.separator();
                        }
                        ui.add_space(4.0);
                        if ui.button("🗑 Очистить все сырые ссылки").clicked() {
                            let _ = rclash_config::custom::clear_raw_proxies();
                            app.reload_raw_keys();
                            reload_core(ctx);
                        }
                    }
                });
        });

        ui.add_space(8.0);
        ui.small("Подсказка: подписки детектят yaml/base64/text → отдельные профили; сырые ссылки всегда в один custom.yaml с dedup по name/server:port+type");
    });
}

fn fetch_and_save_subscription(
    url: &str,
    name: &str,
    interval: UpdateInterval,
) -> anyhow::Result<rclash_config::profile::Profile> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let resp = client.get(url).send()?;
    if !resp.status().is_success() {
        anyhow::bail!("HTTP {}", resp.status());
    }
    let headers = resp.headers().clone();
    let text = resp.text()?;
    let proxies = rclash_subscription::parse_subscription_content(&text)?;

    if proxies.is_empty() {
        anyhow::bail!("пустой контент — не найдено прокси");
    }

    let effective_interval =
        if let Some(hv) = headers.get("update-interval").and_then(|v| v.to_str().ok()) {
            UpdateInterval::from_str(hv).unwrap_or(interval)
        } else if text.contains("update-interval:") {
            if let Ok(v) = serde_yaml::from_str::<serde_yaml::Value>(&text) {
                if let Some(iv) = v.get("update-interval").and_then(|x| x.as_str()) {
                    UpdateInterval::from_str(iv).unwrap_or(interval)
                } else {
                    interval
                }
            } else {
                interval
            }
        } else {
            interval
        };

    let yaml_value = if text.trim_start().starts_with("proxies:") || text.contains("proxy-groups:")
    {
        serde_yaml::from_str::<serde_yaml::Value>(&text).unwrap_or_else(|_| {
            let mut m = serde_yaml::Mapping::new();
            m.insert(
                serde_yaml::Value::String("proxies".into()),
                serde_yaml::Value::Sequence(proxies.clone()),
            );
            serde_yaml::Value::Mapping(m)
        })
    } else {
        let mut m = serde_yaml::Mapping::new();
        m.insert(
            serde_yaml::Value::String("proxies".into()),
            serde_yaml::Value::Sequence(proxies.clone()),
        );
        m.insert(
            serde_yaml::Value::String("proxy-groups".into()),
            serde_yaml::Value::Sequence(vec![{
                let mut g = serde_yaml::Mapping::new();
                g.insert(
                    serde_yaml::Value::String("name".into()),
                    serde_yaml::Value::String("PROXY".into()),
                );
                g.insert(
                    serde_yaml::Value::String("type".into()),
                    serde_yaml::Value::String("select".into()),
                );
                let names: Vec<serde_yaml::Value> = proxies
                    .iter()
                    .filter_map(|p| {
                        p.as_mapping()
                            .and_then(|m| m.get(serde_yaml::Value::String("name".into())))
                            .cloned()
                    })
                    .collect();
                g.insert(
                    serde_yaml::Value::String("proxies".into()),
                    serde_yaml::Value::Sequence(names),
                );
                serde_yaml::Value::Mapping(g)
            }]),
        );
        m.insert(
            serde_yaml::Value::String("rules".into()),
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::String("MATCH,PROXY".into())]),
        );
        serde_yaml::Value::Mapping(m)
    };

    let content = serde_yaml::to_string(&yaml_value)?;
    let profile = rclash_config::profile::import_profile_content(&content, name)?;
    let mut profile = profile;
    profile.url = Some(url.to_owned());
    profile.update_interval = Some(effective_interval);
    Ok(profile)
}

fn reload_core(ctx: &egui::Context) {
    let ctx2 = ctx.clone();
    let _ = poll_promise::Promise::spawn_thread("reload_core", move || {
        let client = reqwest::blocking::Client::new();
        let _ = client
            .put("http://127.0.0.1:9090/configs?force=true")
            .send();
        ctx2.request_repaint();
    });
}
