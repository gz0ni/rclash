mod app;
mod icon;
mod logger;
mod tray;

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
    let icon_data = (|| {
        let bytes = include_bytes!("../assets/icons/app/app-256.png");
        let (rgba, width, height) = icon::decode_png_rgba(bytes).ok()?;
        Some(std::sync::Arc::new(egui::IconData {
            rgba,
            width,
            height,
        }))
    })();
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([840.0, 560.0])
        .with_min_inner_size([840.0, 560.0])
        .with_max_inner_size([840.0, 560.0])
        .with_resizable(false)
        .with_visible(!minimized);
    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "RClash",
        options,
        Box::new(|cc| Ok(Box::new(RClashApp::new(cc, tray_handle)))),
    )
}
