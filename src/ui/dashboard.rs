pub fn show(ui: &mut egui::Ui) {
    ui.heading("Дашборд");
    ui.separator();
    ui.label("Трафик, версия ядра и статус — скоро здесь.");
    ui.add_space(12.0);
    egui::Grid::new("dashboard_grid")
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
            ui.label("Ядро:");
            ui.label("rclash-core (не запущено)");
            ui.end_row();
            ui.label("Трафик ↑/↓:");
            ui.label("—");
            ui.end_row();
            ui.label("Аптайм:");
            ui.label("—");
            ui.end_row();
        });
}
