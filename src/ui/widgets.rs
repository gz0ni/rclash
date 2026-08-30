use egui::{Color32, CornerRadius, Frame, Margin, Ui};

pub fn card<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let visuals = ui.visuals().clone();
    let fill = visuals.extreme_bg_color;
    let stroke = visuals.widgets.noninteractive.bg_stroke;
    Frame {
        fill,
        corner_radius: CornerRadius::same(8),
        stroke,
        inner_margin: Margin::same(12),
        outer_margin: Margin::symmetric(4, 4),
        ..Default::default()
    }
    .show(ui, add_contents)
    .inner
}

pub fn section_header(ui: &mut Ui, title: &str, subtitle: Option<&str>) {
    ui.heading(title);
    if let Some(sub) = subtitle {
        ui.label(egui::RichText::new(sub).weak().small());
    }
    ui.separator();
    ui.add_space(4.0);
}

pub fn badge(ui: &mut Ui, text: &str, color: Color32) {
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        egui::FontId::proportional(11.0),
        Color32::WHITE,
    );
    let pad_x = 6.0;
    let pad_y = 2.0;
    let desired = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    ui.painter().rect_filled(rect, 4.0, color);
    ui.painter()
        .galley(rect.min + egui::vec2(pad_x, pad_y), galley, Color32::WHITE);
}

pub fn status_badge(ui: &mut Ui, alive: bool) {
    if alive {
        badge(ui, "● Online", Color32::from_rgb(80, 200, 120));
    } else {
        badge(ui, "○ Offline", Color32::from_gray(120));
    }
}

#[allow(dead_code)]
pub fn empty_state(ui: &mut Ui, icon: &str, text: &str, action_label: Option<&str>) -> bool {
    let mut clicked = false;
    ui.vertical_centered(|ui| {
        ui.add_space(24.0);
        ui.label(egui::RichText::new(icon).size(32.0).weak());
        ui.add_space(8.0);
        ui.label(egui::RichText::new(text).weak().italics());
        if let Some(label) = action_label {
            ui.add_space(12.0);
            if ui.button(label).clicked() {
                clicked = true;
            }
        }
        ui.add_space(16.0);
    });
    clicked
}

#[allow(dead_code)]
pub fn kv_row(ui: &mut Ui, key: &str, value_widget: impl FnOnce(&mut Ui)) {
    ui.label(egui::RichText::new(key).weak().small());
    value_widget(ui);
}

#[allow(dead_code)]
pub fn inline_error(ui: &mut Ui, msg: &str) {
    ui.label(
        egui::RichText::new(msg)
            .color(Color32::from_rgb(220, 80, 80))
            .small(),
    );
}
