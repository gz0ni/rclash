mod app;
mod ui;

use app::RClashApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "RClash",
        options,
        Box::new(|cc| Ok(Box::new(RClashApp::new(cc)))),
    )
}
