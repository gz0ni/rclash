use crate::app::RClashApp;
use egui::RichText;
use rclash_core_manager::api::{format_bytes, ConnectionsSort};

pub fn show(ui: &mut egui::Ui, app: &mut RClashApp, ctx: &egui::Context) {
    super::widgets::section_header(
        ui,
        "Соединения",
        Some("WS /connections?interval=1000 — таблица, фильтр, закрытие"),
    );
    let mut need_close_all = false;
    let mut close_id: Option<String> = None;

    ui.horizontal(|ui| {
        ui.label("Фильтр:");
        let resp = ui.text_edit_singleline(&mut app.connections_filter);
        if resp.changed() {
            log::info!("connections filter: {}", app.connections_filter);
        }
        egui::ComboBox::from_id_salt("conn_sort")
            .selected_text(app.connections_sort.label_ru())
            .show_ui(ui, |ui| {
                for s in ConnectionsSort::all() {
                    if ui
                        .selectable_value(&mut app.connections_sort, *s, s.label_ru())
                        .clicked()
                    {
                        log::info!("connections sort: {}", s.label_ru());
                    }
                }
            });
        if ui
            .button("⟳ Переподкл.")
            .on_hover_text("Перезапустить WS")
            .clicked()
        {
            app.connections_data = None;
            log::info!("connections reconnect requested");
        }
        let cnt = app
            .connections_data
            .as_ref()
            .map(|s| s.connections.len())
            .unwrap_or(0);
        ui.label(RichText::new(format!("Всего: {cnt}")).weak().small());
        if ui
            .add_enabled(cnt > 0, egui::Button::new("✕ Закрыть все"))
            .on_hover_text("DELETE /connections")
            .clicked()
        {
            need_close_all = true;
        }
    });

    if let Some(err) = &app.connections_error {
        super::widgets::inline_error(ui, err);
    }

    if let Some(snap) = app.connections_data.clone() {
        let filter = app.connections_filter.to_lowercase();
        let mut rows = snap.connections;
        if !filter.is_empty() {
            rows.retain(|c| {
                let meta_str = c.metadata.to_string().to_lowercase();
                let chains_str = c.chains.join(" ").to_lowercase();
                meta_str.contains(&filter)
                    || chains_str.contains(&filter)
                    || c.rule.to_lowercase().contains(&filter)
                    || c.rule_payload.to_lowercase().contains(&filter)
            });
        }
        match app.connections_sort {
            ConnectionsSort::StartDesc => rows.sort_by(|a, b| b.start.cmp(&a.start)),
            ConnectionsSort::UploadDesc => rows.sort_by_key(|b| std::cmp::Reverse(b.upload)),
            ConnectionsSort::DownloadDesc => rows.sort_by_key(|b| std::cmp::Reverse(b.download)),
            ConnectionsSort::HostAsc => rows.sort_by(|a, b| {
                let ha = extract_host(&a.metadata);
                let hb = extract_host(&b.metadata);
                ha.cmp(&hb)
            }),
        }
        let filtered_cnt = rows.len();
        if filtered_cnt == 0 {
            super::widgets::empty_state(ui, "🔍", "Ничего не найдено", None);
            return;
        }
        ui.label(
            RichText::new(format!("Показано: {filtered_cnt}"))
                .weak()
                .small(),
        );
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("conn_grid")
                    .num_columns(7)
                    .spacing([8.0, 6.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Хост").strong().small());
                        ui.label(RichText::new("Сеть").strong().small());
                        ui.label(RichText::new("Правило").strong().small());
                        ui.label(RichText::new("Цепочка").strong().small());
                        ui.label(RichText::new("↑").strong().small());
                        ui.label(RichText::new("↓").strong().small());
                        ui.label(RichText::new("").strong().small());
                        ui.end_row();
                        for c in rows.iter().take(300) {
                            let host = extract_host(&c.metadata);
                            let network = c
                                .metadata
                                .get("network")
                                .and_then(|v| v.as_str())
                                .unwrap_or("—");
                            let rule = if c.rule.is_empty() {
                                "—".to_owned()
                            } else {
                                format!("{}:{}", c.rule, c.rule_payload)
                            };
                            let chains = if c.chains.is_empty() {
                                "—".to_owned()
                            } else {
                                c.chains.join(" → ")
                            };
                            ui.label(egui::RichText::new(truncate(&host, 28)).small());
                            ui.label(egui::RichText::new(network).small().weak());
                            ui.label(egui::RichText::new(truncate(&rule, 22)).small());
                            ui.label(egui::RichText::new(truncate(&chains, 26)).small().weak());
                            ui.label(egui::RichText::new(format_bytes(c.upload as u64)).small());
                            ui.label(egui::RichText::new(format_bytes(c.download as u64)).small());
                            if ui
                                .small_button("❌")
                                .on_hover_text(format!("Закрыть {}", c.id))
                                .clicked()
                            {
                                close_id = Some(c.id.clone());
                            }
                            ui.end_row();
                        }
                    });
            });
        ui.add_space(4.0);
        ui.small(format!(
            "Память ядра: {} · Всего ↑ {} ↓ {}",
            format_bytes(snap.memory),
            format_bytes(snap.upload_total as u64),
            format_bytes(snap.download_total as u64)
        ));
    } else {
        ui.add_space(12.0);
        if app.connections_error.is_some() {
            super::widgets::empty_state(ui, "⚠", "Ошибка загрузки — проверь ядро", None);
        } else {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Ожидание WS /connections …").weak());
            });
            ui.small("Подключается к ws://127.0.0.1:9090/connections?interval=1000");
        }
    }

    if let Some(id) = close_id {
        app.close_connection_async(id, ctx);
    }
    if need_close_all {
        app.close_all_connections_async(ctx);
    }
}

fn extract_host(meta: &serde_json::Value) -> String {
    if let Some(h) = meta.get("host").and_then(|v| v.as_str()) {
        if !h.is_empty() {
            return h.to_owned();
        }
    }
    if let Some(h) = meta.get("destinationIP").and_then(|v| v.as_str()) {
        if !h.is_empty() {
            let port = meta
                .get("destinationPort")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !port.is_empty() {
                return format!("{h}:{port}");
            }
            return h.to_owned();
        }
    }
    if let Some(h) = meta.get("remoteDestination").and_then(|v| v.as_str()) {
        if !h.is_empty() {
            return h.to_owned();
        }
    }
    meta.get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("—")
        .to_owned()
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_owned()
    } else {
        let t: String = s.chars().take(n - 1).collect();
        format!("{t}…")
    }
}
