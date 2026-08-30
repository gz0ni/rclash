mod app;
mod logger;
mod tray;
mod ui;

use app::RClashApp;

fn main() -> eframe::Result<()> {
    let cfg = rclash_config::load_app_config();
    let _ = logger::init(cfg.log_level.to_log_filter());
    log::info!(
        "RClash starting minimized={}",
        std::env::args().any(|a| a == "--minimized")
    );
    let minimized = std::env::args().any(|a| a == "--minimized");
    let tray_handle = tray::init_tray();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([900.0, 600.0])
            .with_visible(!minimized),
        ..Default::default()
    };
    eframe::run_native(
        "RClash",
        options,
        Box::new(|cc| Ok(Box::new(RClashApp::new(cc, tray_handle)))),
    )
}
