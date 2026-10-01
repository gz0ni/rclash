mod logger;
mod tui;

fn startup_guard() {
    let cfg = rclash_config::load_app_config();
    let addr = format!("127.0.0.1:{}", cfg.mixed_port.unwrap_or(7890));
    let base = rclash_config::api_base(&cfg.external_controller);
    let secret = cfg.core_secret.clone().unwrap_or_default();
    if rclash_core_manager::supervisor::core_alive(&base, &secret) {
        return;
    }
    rclash_core_manager::supervisor::reap_orphans();
    let had_guard = rclash_config::guard::read_guard().is_some();
    if had_guard && rclash_sys_proxy::is_ours_current(&addr) {
        if rclash_core_manager::supervisor::port_open(&addr) {
            log::warn!("startup guard: proxy {addr} still serves traffic, left enabled");
        } else {
            match rclash_sys_proxy::disable_if_ours(&addr) {
                Ok(true) => log::warn!("startup guard: stale sys proxy {addr} disabled"),
                Ok(false) => {}
                Err(e) => log::warn!("startup guard: sys proxy check failed: {e}"),
            }
        }
    }
    if had_guard
        && cfg.tun_enabled
        && matches!(rclash_tun::status(), rclash_tun::TunStatus::Enabled { .. })
    {
        if let Err(e) = rclash_tun::disable() {
            log::warn!("startup guard: tun disable failed: {e}");
        }
    }
    if had_guard || cfg.master_enabled {
        let mut cfg = cfg;
        cfg.master_enabled = false;
        if let Err(e) = rclash_config::save_app_config(&cfg) {
            log::warn!("startup guard: save app config failed: {e}");
        }
    }
    rclash_config::guard::clear_guard();
}

fn main() -> anyhow::Result<()> {
    let cfg = rclash_config::load_app_config();
    let _ = logger::init(cfg.log_level.to_log_filter());
    startup_guard();
    log::info!("RClash TUI starting");
    tui::run()
}
