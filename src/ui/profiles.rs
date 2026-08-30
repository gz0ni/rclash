pub fn show(ui: &mut egui::Ui) {
    ui.heading("Профили");
    ui.separator();
    ui.label("Импорт и переключение config.yaml (mihomo-совместимый).");
    ui.add_space(8.0);
    let _ = ui.button("Импортировать профиль…").clicked();
    ui.separator();
    ui.label(egui::RichText::new("Нет профилей").weak());
}
