pub fn show(ui: &mut egui::Ui) {
    ui.heading("Настройки");
    ui.separator();
    egui::Grid::new("settings_grid")
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
            ui.label("Адрес контроллера:");
            ui.text_edit_singleline(&mut String::from("127.0.0.1:9090"));
            ui.end_row();
            ui.label("Системный прокси:");
            ui.checkbox(&mut false, "Включить при запуске");
            ui.end_row();
            ui.label("Автозапуск:");
            ui.checkbox(&mut false, "Запускать вместе с системой");
            ui.end_row();
        });
}
