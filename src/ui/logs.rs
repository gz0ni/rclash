pub fn show(ui: &mut egui::Ui) {
    ui.heading("Логи");
    ui.separator();
    ui.label("Вывод rclash-core stdout/stderr и API /logs.");
    ui.add_space(8.0);
    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .show(ui, |ui| {
            ui.monospace("[12:00:00] ядро не запущено");
        });
}
