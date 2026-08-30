use crate::app::RClashApp;
use rclash_core_manager::ProxyMode;
use serde_json::Value;

pub fn show(ui: &mut egui::Ui, app: &mut RClashApp, ctx: &egui::Context) {
    super::widgets::section_header(
        ui,
        "Прокси",
        Some("Группы и узлы — данные из GET /proxies, задержки цвет"),
    );

    app.poll_proxies(ctx);

    ui.horizontal(|ui| {
        ui.label("Режим:");
        let mut mode = app.proxies_mode;
        egui::ComboBox::from_id_salt("proxy_mode")
            .selected_text(mode.label_ru())
            .show_ui(ui, |ui| {
                for m in ProxyMode::all() {
                    ui.selectable_value(&mut mode, *m, m.label_ru());
                }
            });
        if mode != app.proxies_mode {
            app.set_mode_async(mode, ctx);
        }
        ui.separator();
        if ui.button("↻ Обновить").clicked() {
            app.proxies_promise = None;
            app.fetch_proxies(ctx);
            app.fetch_mode(ctx);
        }
        let testing = app.delays_running;
        if ui
            .add_enabled(!testing, egui::Button::new("⚡ Проверить задержки"))
            .clicked()
        {
            if let Some(data) = app.proxies_data.clone() {
                let names = extract_proxy_names(&data);
                app.test_delays_async(names, ctx);
            } else {
                app.fetch_proxies(ctx);
            }
        }
        if testing {
            ui.spinner();
            ui.label(egui::RichText::new("тестируем…").weak().small());
        }
    });

    if let Some(err) = app.proxies_error.clone() {
        ui.add_space(4.0);
        super::widgets::inline_error(ui, &err);
    }

    ui.add_space(8.0);

    if app.proxies_data.is_none() && app.proxies_promise.is_none() {
        super::widgets::card(ui, |ui| {
            let clicked = super::widgets::empty_state(
                ui,
                "🌐",
                "Нет данных — ядро не запущено или прокси не загружены",
                Some("Загрузить прокси"),
            );
            if clicked {
                app.fetch_proxies(ctx);
                app.fetch_mode(ctx);
            }
            ui.add_space(8.0);
            ui.small("Подсказка: задержки <100 мс — зелёный, <300 мс — жёлтый, выше — красный");
        });
        if app.proxies_promise.is_none() {
            app.fetch_proxies(ctx);
            app.fetch_mode(ctx);
        }
        return;
    }

    if app.proxies_promise.is_some() && app.proxies_data.is_none() {
        super::widgets::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Загрузка прокси…");
            });
        });
        return;
    }

    let Some(data) = app.proxies_data.clone() else {
        return;
    };

    let proxies_map = data
        .get("proxies")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    if proxies_map.is_empty() {
        super::widgets::card(ui, |ui| {
            ui.label(egui::RichText::new("Прокси не найдены").weak());
            ui.small("Проверь profiles/custom.yaml или активный профиль");
        });
        return;
    }

    let mut groups: Vec<(String, Value)> = Vec::new();
    let mut singles: Vec<(String, Value)> = Vec::new();
    for (name, val) in proxies_map.iter() {
        let tp = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let is_group = matches!(
            tp.to_ascii_lowercase().as_str(),
            "selector" | "relay" | "fallback" | "urltest" | "loadbalance" | "select"
        );
        if is_group {
            groups.push((name.clone(), val.clone()));
        } else {
            singles.push((name.clone(), val.clone()));
        }
    }
    groups.sort_by(|a, b| a.0.cmp(&b.0));
    singles.sort_by(|a, b| a.0.cmp(&b.0));

    egui::ScrollArea::vertical().show(ui, |ui| {
        if !groups.is_empty() {
            super::widgets::card(ui, |ui| {
                ui.label(egui::RichText::new(format!("Группы — {} шт.", groups.len())).strong());
                ui.add_space(4.0);
                for (name, val) in groups {
                    group_card(ui, app, ctx, &name, &val);
                }
            });
            ui.add_space(8.0);
        }

        super::widgets::card(ui, |ui| {
            ui.label(egui::RichText::new(format!("Прокси — {} шт.", singles.len())).strong());
            if singles.is_empty() {
                ui.label(egui::RichText::new("Нет прокси-узлов").weak());
                return;
            }
            let mut to_remove: Option<String> = None;
            egui::Grid::new("proxies_grid")
                .num_columns(2)
                .spacing([12.0, 12.0])
                .show(ui, |ui| {
                    let mut idx = 0;
                    for (name, val) in &singles {
                        proxy_card(ui, app, ctx, name, val);
                        if app.raw_keys.contains(name)
                            && ui
                                .small_button("❌")
                                .on_hover_text("Удалить из Сырых ссылок (custom.yaml)")
                                .clicked()
                        {
                            to_remove = Some(name.clone());
                        }
                        idx += 1;
                        if idx % 2 == 0 {
                            ui.end_row();
                        }
                    }
                    if idx % 2 != 0 {
                        ui.end_row();
                    }
                });
            if let Some(n) = to_remove {
                match rclash_config::custom::remove_raw_proxy(&n) {
                    Ok(true) => {
                        app.reload_raw_keys();
                        app.proxy_delays.remove(&n);
                        reload_core(ctx);
                        app.fetch_proxies(ctx);
                    }
                    Ok(false) => {
                        app.proxies_error = Some(format!("Не найдено в custom.yaml: {n}"));
                    }
                    Err(e) => {
                        app.proxies_error = Some(format!("Ошибка удаления {n}: {e}"));
                    }
                }
            }
        });

        ui.add_space(8.0);
        ui.small(
            "Дедyп по name/server:port+type, PROXY select — в custom.yaml, atomic write + reload",
        );
    });
}

fn group_card(
    ui: &mut egui::Ui,
    app: &mut RClashApp,
    ctx: &egui::Context,
    name: &str,
    val: &Value,
) {
    let tp = val.get("type").and_then(|v| v.as_str()).unwrap_or("?");
    let now = val.get("now").and_then(|v| v.as_str()).unwrap_or("—");
    let all = val
        .get("all")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let delay = app.proxy_delays.get(name).copied().flatten();
    let color = delay_color(delay);
    let delay_str = delay_text(delay);

    super::widgets::card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(name).strong());
                ui.horizontal(|ui| {
                    super::widgets::badge(ui, tp, egui::Color32::from_rgb(80, 140, 255));
                    super::widgets::badge(ui, &delay_str, color);
                });
                ui.add_space(4.0);
                ui.label(egui::RichText::new(format!("Текущий: {now}")).small());
                ui.label(
                    egui::RichText::new(format!("Вариантов: {all}"))
                        .weak()
                        .small(),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("⚡ Тест").clicked() {
                    app.test_delays_async(vec![name.to_owned()], ctx);
                }
                if let Some(opts) = val.get("all").and_then(|v| v.as_array()) {
                    egui::ComboBox::from_id_salt(format!("group_select_{name}"))
                        .selected_text(now)
                        .show_ui(ui, |ui| {
                            for opt in opts {
                                if let Some(s) = opt.as_str() {
                                    let selected = s == now;
                                    if ui.selectable_label(selected, s).clicked() {
                                        let group = name.to_owned();
                                        let target = s.to_owned();
                                        let ctx2 = ctx.clone();
                                        let _ = poll_promise::Promise::spawn_thread(
                                            format!("select_{group}"),
                                            move || {
                                                let client = reqwest::blocking::Client::new();
                                                let _ = client
                                                    .put(format!(
                                                        "http://127.0.0.1:9090/proxies/{group}"
                                                    ))
                                                    .json(&serde_json::json!({"name": target}))
                                                    .send();
                                                ctx2.request_repaint();
                                            },
                                        );
                                    }
                                }
                            }
                        });
                }
            });
        });
    });
}

fn proxy_card(ui: &mut egui::Ui, app: &RClashApp, _ctx: &egui::Context, name: &str, val: &Value) {
    let tp = val.get("type").and_then(|v| v.as_str()).unwrap_or("?");
    let delay = app.proxy_delays.get(name).copied().flatten();
    let history_delay = val
        .get("history")
        .and_then(|v| v.as_array())
        .and_then(|a| a.last())
        .and_then(|e| e.get("delay"))
        .and_then(|d| d.as_u64());
    let delay = delay.or(history_delay);
    let color = delay_color(delay);
    let delay_str = delay_text(delay);
    let is_raw = app.raw_keys.contains(&name.to_owned());

    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .corner_radius(8)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .inner_margin(egui::Margin::same(8))
        .outer_margin(egui::Margin::symmetric(4, 4))
        .show(ui, |ui| {
            ui.set_min_width(180.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(name).strong().size(13.0));
                if is_raw {
                    super::widgets::badge(ui, "RAW", egui::Color32::from_rgb(200, 120, 255));
                }
            });
            ui.horizontal(|ui| {
                super::widgets::badge(ui, tp, egui::Color32::from_rgb(100, 160, 255));
                super::widgets::badge(ui, &delay_str, color);
            });
            if let Some(server) = val.get("server").and_then(|v| v.as_str()) {
                ui.label(egui::RichText::new(server).weak().small().monospace());
            }
        });
}

fn extract_proxy_names(data: &Value) -> Vec<String> {
    data.get("proxies")
        .and_then(|v| v.as_object())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

fn delay_color(delay_ms: Option<u64>) -> egui::Color32 {
    match delay_ms {
        None => egui::Color32::from_gray(120),
        Some(d) if d < 100 => egui::Color32::from_rgb(80, 200, 120),
        Some(d) if d < 300 => egui::Color32::from_rgb(220, 180, 60),
        Some(_) => egui::Color32::from_rgb(220, 80, 80),
    }
}

fn delay_text(delay_ms: Option<u64>) -> String {
    match delay_ms {
        None => "—".to_owned(),
        Some(d) => format!("{d} ms"),
    }
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
