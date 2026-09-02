use egui_kittest::Harness;

const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x3B, 0x82, 0xF6);
const GREEN: egui::Color32 = egui::Color32::from_rgb(0x27, 0xAE, 0x60);
const RED: egui::Color32 = egui::Color32::from_rgb(0xE7, 0x4C, 0x3C);
const R6: u8 = 6;
const R4: u8 = 4;

fn dark_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    let panel = egui::Color32::from_rgb(0x1E, 0x1E, 0x1E);
    let card = egui::Color32::from_rgb(0x2D, 0x2F, 0x33);
    let border = egui::Color32::from_rgb(0x3C, 0x3F, 0x41);
    v.panel_fill = panel;
    v.window_fill = panel;
    v.extreme_bg_color = egui::Color32::from_rgb(0x2A, 0x2C, 0x2F);
    v.faint_bg_color = panel;
    v.code_bg_color = card;
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = egui::CornerRadius::same(R6);
        w.bg_stroke = egui::Stroke::new(1.0_f32, border);
        w.fg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
        w.bg_fill = card;
    }
    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(0x3A, 0x3D, 0x45);
    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
    v.selection.bg_fill = ACCENT;
    v.override_text_color = Some(egui::Color32::WHITE);
    v.weak_text_color = Some(egui::Color32::from_rgb(0x9A, 0xA0, 0xA6));
    v
}

fn light_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::light();
    let panel = egui::Color32::from_rgb(0xF0, 0xF2, 0xF5);
    let card = egui::Color32::WHITE;
    let border = egui::Color32::from_rgb(0xD0, 0xD3, 0xD8);
    v.panel_fill = panel;
    v.window_fill = panel;
    v.extreme_bg_color = egui::Color32::from_rgb(0xE8, 0xEA, 0xED);
    v.faint_bg_color = panel;
    v.code_bg_color = card;
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = egui::CornerRadius::same(R6);
        w.bg_stroke = egui::Stroke::new(1.0_f32, border);
        w.fg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::BLACK);
        w.bg_fill = card;
    }
    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(0xE8, 0xEA, 0xED);
    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
    v.override_text_color = Some(egui::Color32::BLACK);
    v.weak_text_color = Some(egui::Color32::from_rgb(0x6B, 0x72, 0x80));
    v
}

fn render_mockup_fragment(ui: &mut egui::Ui, visuals: egui::Visuals) {
    ui.ctx().set_visuals(visuals);
    ui.spacing_mut().item_spacing = egui::vec2(6.0_f32, 6.0_f32);
    let card = if ui.visuals().dark_mode {
        egui::Color32::from_rgb(0x2D, 0x2F, 0x33)
    } else {
        egui::Color32::WHITE
    };
    let border = if ui.visuals().dark_mode {
        egui::Color32::from_rgb(0x3C, 0x3F, 0x41)
    } else {
        egui::Color32::from_rgb(0xD0, 0xD3, 0xD8)
    };

    ui.horizontal_top(|ui| {
        egui::Frame::new()
            .fill(card)
            .stroke(egui::Stroke::new(1.0_f32, border))
            .corner_radius(egui::CornerRadius::same(R6))
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.set_min_width(200.0_f32);
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("График").weak().size(10.0_f32));
                    let graph_h = 60.0_f32;
                    let (rect, _) = ui
                        .allocate_exact_size(egui::vec2(200.0_f32, graph_h), egui::Sense::hover());
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rect, egui::CornerRadius::same(6), card);
                    painter.rect_stroke(
                        rect,
                        egui::CornerRadius::same(6),
                        egui::Stroke::new(1.0_f32, border),
                        egui::StrokeKind::Inside,
                    );
                    let inner = rect.shrink2(egui::vec2(6.0_f32, 6.0_f32));
                    for i in 1..3 {
                        let y = inner.top() + inner.height() * (i as f32 / 3.0_f32);
                        painter.hline(inner.x_range(), y, (0.8_f32, border));
                    }
                    painter.add(egui::Shape::line(
                        vec![
                            egui::pos2(inner.left(), inner.center().y),
                            egui::pos2(inner.right(), inner.center().y - 8.0_f32),
                        ],
                        egui::Stroke::new(1.6_f32, ACCENT),
                    ));
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("●").color(ACCENT).small());
                        ui.label(egui::RichText::new("↑ 1.2 KB/s").small().monospace());
                        ui.label(egui::RichText::new("●").color(GREEN).small());
                        ui.label(egui::RichText::new("↓ 3.4 KB/s").small().monospace());
                    });
                });
            });
        egui::Frame::new()
            .fill(card)
            .stroke(egui::Stroke::new(1.0_f32, border))
            .corner_radius(egui::CornerRadius::same(R6))
            .inner_margin(egui::Margin::same(6))
            .show(ui, |ui| {
                ui.set_min_width(180.0_f32);
                ui.vertical(|ui| {
                    for (name, tp, delay) in [
                        ("Germany02-Hysteria2", "hysteria2", Some(59u64)),
                        ("Poland01-Shadowsocks", "ss", Some(88)),
                    ] {
                        let row_bg = egui::Color32::TRANSPARENT;
                        let row_frame = egui::Frame::new()
                            .fill(row_bg)
                            .corner_radius(egui::CornerRadius::same(R4))
                            .inner_margin(egui::Margin {
                                left: 4,
                                right: 4,
                                top: 0,
                                bottom: 0,
                            });
                        row_frame.show(ui, |ui| {
                            ui.set_height(28.0_f32);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("🇩🇪 {name}")).size(11.0_f32));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::vec2(12.0_f32, 12.0_f32),
                                            egui::Sense::hover(),
                                        );
                                        let ring = egui::Color32::from_rgb(0x6B, 0x6F, 0x76);
                                        ui.painter_at(rect).circle_stroke(
                                            rect.center(),
                                            5.5_f32,
                                            egui::Stroke::new(1.0_f32, ring),
                                        );
                                        let col = match delay {
                                            Some(d) if d < 100 => {
                                                egui::Color32::from_rgb(0x50, 0xB4, 0x78)
                                            }
                                            Some(d) if d < 300 => {
                                                egui::Color32::from_rgb(0xC8, 0xB4, 0x5A)
                                            }
                                            _ => egui::Color32::from_rgb(0xE0, 0x60, 0x60),
                                        };
                                        ui.label(
                                            egui::RichText::new(format!("{delay:?} ms"))
                                                .color(col)
                                                .monospace()
                                                .size(11.0_f32),
                                        );
                                        ui.label(
                                            egui::RichText::new(tp).monospace().size(10.0_f32),
                                        );
                                    },
                                );
                            });
                        });
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("приём").size(10.0_f32));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("1.2 MB").monospace().size(11.0_f32));
                        });
                    });
                    let btn = egui::Button::new(egui::RichText::new("rule").size(11.0_f32))
                        .fill(ACCENT)
                        .stroke(egui::Stroke::new(1.0_f32, ACCENT));
                    let _ = ui.add_sized([60.0_f32, 26.0_f32], btn);
                    let master = egui::Button::new(
                        egui::RichText::new("ВЫКЛ • Включить")
                            .size(12.0_f32)
                            .color(egui::Color32::WHITE),
                    )
                    .fill(RED)
                    .corner_radius(egui::CornerRadius::same(6));
                    ui.add_sized([180.0_f32, 32.0_f32], master);
                });
            });
    });
}

#[test]
fn golden_dark() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(420.0_f32, 260.0_f32))
        .wgpu()
        .build_ui(|ui| render_mockup_fragment(ui, dark_visuals()));
    h.snapshot("golden_dark");
}

#[test]
fn golden_light() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(420.0_f32, 260.0_f32))
        .wgpu()
        .build_ui(|ui| render_mockup_fragment(ui, light_visuals()));
    h.snapshot("golden_light");
}

#[test]
fn golden_proxy_row_selected() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(380.0_f32, 40.0_f32))
        .wgpu()
        .build_ui(|ui| {
            ui.ctx().set_visuals(dark_visuals());
            let sel_bg = egui::Color32::from_rgba_premultiplied(0x3B, 0x82, 0xF6, 36);
            egui::Frame::new()
                .fill(sel_bg)
                .stroke(egui::Stroke::new(1.0_f32, ACCENT))
                .corner_radius(egui::CornerRadius::same(R4))
                .inner_margin(egui::Margin {
                    left: 4,
                    right: 4,
                    top: 0,
                    bottom: 0,
                })
                .show(ui, |ui| {
                    ui.set_height(28.0_f32);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("🇩🇪 Germany02-Hysteria2")
                                .color(ACCENT)
                                .size(11.0_f32),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(12.0_f32, 12.0_f32),
                                egui::Sense::hover(),
                            );
                            let p = ui.painter_at(rect);
                            p.circle_stroke(
                                rect.center(),
                                5.5_f32,
                                egui::Stroke::new(1.0_f32, ACCENT),
                            );
                            p.circle_filled(rect.center(), 3.0_f32, ACCENT);
                            ui.label(
                                egui::RichText::new("59 ms")
                                    .color(egui::Color32::from_rgb(0x50, 0xB4, 0x78))
                                    .monospace()
                                    .size(11.0_f32),
                            );
                            ui.label(egui::RichText::new("hysteria2").monospace().size(10.0_f32));
                        });
                    });
                });
        });
    h.snapshot("golden_proxy_selected");
}
