pub fn show(ui: &mut egui::Ui) {
    ui.heading("Прокси");
    ui.separator();
    ui.label("Список прокси и групп — данные из GET /proxies.");
    ui.add_space(8.0);
    let _ = ui.button("Проверить задержки").clicked();
    ui.separator();
    ui.label(egui::RichText::new("Нет данных — ядро не запущено").weak());
}
