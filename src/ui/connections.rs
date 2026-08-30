pub fn show(ui: &mut egui::Ui) {
    ui.heading("Соединения");
    ui.separator();
    ui.label("Активные соединения — GET /connections, закрытие по клику (F1).");
    ui.add_space(8.0);
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label(egui::RichText::new("Нет соединений").weak());
    });
}
