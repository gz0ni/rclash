use super::widgets;
use std::collections::VecDeque;

pub fn show(
    ui: &mut egui::Ui,
    alive: bool,
    version: Option<&str>,
    up_buf: &VecDeque<f64>,
    down_buf: &VecDeque<f64>,
    total_up: u64,
    total_down: u64,
) {
    widgets::section_header(ui, "Дашборд", Some("Обзор ядра и быстрые действия"));

    ui.columns(3, |cols| {
        widgets::card(&mut cols[0], |ui| {
            ui.label(egui::RichText::new("Ядро").weak().small());
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(version.unwrap_or("rclash-core"))
                    .strong()
                    .size(14.0),
            );
            ui.add_space(6.0);
            widgets::status_badge(ui, alive);
            ui.add_space(6.0);
            ui.small(if alive {
                "GET /version каждые 2с"
            } else {
                "Запусти rclash-core"
            });
        });
        widgets::card(&mut cols[1], |ui| {
            ui.label(egui::RichText::new("Трафик").weak().small());
            ui.add_space(4.0);
            if alive {
                let cur_up = up_buf.back().copied().unwrap_or(0.0) as u64;
                let cur_down = down_buf.back().copied().unwrap_or(0.0) as u64;
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("↑").color(egui::Color32::from_rgb(120, 180, 255)),
                    );
                    ui.label(rclash_core_manager::api::format_bytes(cur_up) + "/с");
                    ui.label(
                        egui::RichText::new("↓").color(egui::Color32::from_rgb(120, 220, 140)),
                    );
                    ui.label(rclash_core_manager::api::format_bytes(cur_down) + "/с");
                });
                ui.add_space(4.0);
                ui.small(format!(
                    "Всего ↑ {} ↓ {}",
                    rclash_core_manager::api::format_bytes(total_up),
                    rclash_core_manager::api::format_bytes(total_down)
                ));
                let max_val = up_buf
                    .iter()
                    .chain(down_buf.iter())
                    .copied()
                    .fold(0.0_f64, f64::max)
                    .max(1024.0);
                ui.small(format!("пик {:.1} KB/с", max_val / 1024.0));
            } else {
                ui.label(egui::RichText::new("—").weak().size(16.0));
                ui.small("Нет данных");
            }
        });
        widgets::card(&mut cols[2], |ui| {
            ui.label(egui::RichText::new("Быстрые действия").weak().small());
            ui.add_space(4.0);
            let proxy_on = rclash_sys_proxy::current()
                .get()
                .map(|s| s == rclash_sys_proxy::ProxyState::Enabled)
                .unwrap_or(false);
            if ui
                .add_enabled(
                    !proxy_on || alive,
                    egui::Button::new(if proxy_on {
                        "Прокси включён ✓"
                    } else {
                        "Включить прокси"
                    }),
                )
                .on_hover_text("Переключить системный прокси 127.0.0.1:7890")
                .clicked()
            {
                let _ = rclash_sys_proxy::current()
                    .set(rclash_sys_proxy::ProxyState::Enabled, "127.0.0.1:7890");
                log::info!("sys proxy toggled on");
            }
            if ui
                .add_enabled(alive, egui::Button::new("↻ Перезагрузить ядро"))
                .on_hover_text("PUT /configs?force=true")
                .clicked()
            {
                let api = rclash_core_manager::CoreApi::with_default();
                let ctx = ui.ctx().clone();
                log::info!("reload core requested");
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.spawn(async move {
                        let res = api.reload().await;
                        if let Err(e) = res {
                            log::warn!("reload failed: {e}");
                        } else {
                            log::info!("reload ok");
                        }
                        ctx.request_repaint();
                    });
                } else {
                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build();
                        if let Ok(rt) = rt {
                            let res = rt.block_on(api.reload());
                            if let Err(e) = res {
                                log::warn!("reload failed: {e}");
                            }
                        }
                        ctx.request_repaint();
                    });
                }
            }
            ui.small("Подсказка: используй трей для скрытия окна");
        });
    });

    ui.add_space(12.0);

    widgets::card(ui, |ui| {
        ui.label(egui::RichText::new("Трафик (60с)").strong());
        ui.add_space(4.0);
        if !alive {
            ui.label(egui::RichText::new("Ядро не запущено — график недоступен").weak());
            return;
        }
        if up_buf.is_empty() && down_buf.is_empty() {
            ui.label(egui::RichText::new("Ожидание данных WS /traffic …").weak());
            ui.add(egui::ProgressBar::new(0.0).animate(true));
            return;
        }
        let max_val = up_buf
            .iter()
            .chain(down_buf.iter())
            .copied()
            .fold(0.0_f64, f64::max)
            .max(1024.0);
        let desired_h = 140.0;
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), desired_h),
            egui::Sense::hover(),
        );
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 6.0, ui.visuals().extreme_bg_color);
        painter.rect_stroke(
            rect,
            6.0,
            ui.visuals().widgets.noninteractive.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let inner = rect.shrink2(egui::vec2(8.0, 8.0));
        let grid_color = egui::Color32::from_gray(60);
        for i in 1..3 {
            let y = inner.top() + inner.height() * (i as f32 / 3.0);
            painter.hline(inner.x_range(), y, (1.0, grid_color));
        }
        let plot_line = |buf: &VecDeque<f64>, color: egui::Color32| {
            if buf.len() < 2 {
                return;
            }
            let n = 60.0;
            let mut points = Vec::with_capacity(buf.len());
            for (i, v) in buf.iter().enumerate() {
                let x = inner.left() + (i as f32 / n) * inner.width();
                let norm = (*v as f32 / max_val as f32).clamp(0.0, 1.0);
                let y = inner.bottom() - norm * inner.height();
                points.push(egui::pos2(x, y));
            }
            painter.add(egui::Shape::line(points, egui::Stroke::new(1.5_f32, color)));
        };
        plot_line(up_buf, egui::Color32::from_rgb(120, 180, 255));
        plot_line(down_buf, egui::Color32::from_rgb(120, 220, 140));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let cur_up = up_buf.back().copied().unwrap_or(0.0) as u64;
            let cur_down = down_buf.back().copied().unwrap_or(0.0) as u64;
            ui.label(
                egui::RichText::new(format!(
                    "↑ {} /с",
                    rclash_core_manager::api::format_bytes(cur_up)
                ))
                .color(egui::Color32::from_rgb(120, 180, 255))
                .small(),
            );
            ui.label(
                egui::RichText::new(format!(
                    "↓ {} /с",
                    rclash_core_manager::api::format_bytes(cur_down)
                ))
                .color(egui::Color32::from_rgb(120, 220, 140))
                .small(),
            );
            ui.label(
                egui::RichText::new(format!(
                    "Всего ↑ {} ↓ {}",
                    rclash_core_manager::api::format_bytes(total_up),
                    rclash_core_manager::api::format_bytes(total_down)
                ))
                .weak()
                .small(),
            );
            ui.label(
                egui::RichText::new(format!("макс {:.1} KB/с", max_val / 1024.0))
                    .weak()
                    .small(),
            );
        });
    });

    ui.add_space(8.0);
    if !alive {
        widgets::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("⚠").size(16.0));
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Ядро не запущено").strong());
                    ui.small("Проверь, что rclash-core в PATH или рядом с приложением, и что порт 9090 свободен.");
                });
            });
        });
    }
}
