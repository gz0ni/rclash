#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]
use poll_promise::Promise;
use rclash_config::{AppConfig, LogLevel, Theme, UpdateInterval};
use rclash_core_manager::api::{ConnectionsSort, LogEntry, Snapshot, TrafficInfo};
use rclash_core_manager::ProxyMode;
use rclash_updater::{manifest_url, UpdateInfo};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Profiles,
    Proxies,
    Connections,
    Logs,
    Settings,
}

impl Tab {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Dashboard => "Дашборд",
            Self::Profiles => "Профили",
            Self::Proxies => "Прокси",
            Self::Connections => "Соединения",
            Self::Logs => "Логи",
            Self::Settings => "Настройки",
        }
    }

    pub fn all() -> &'static [Tab] {
        &[
            Self::Dashboard,
            Self::Profiles,
            Self::Proxies,
            Self::Connections,
            Self::Logs,
            Self::Settings,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogsTab {
    Core,
    App,
}

impl LogsTab {
    pub fn label_ru(&self) -> &'static str {
        match self {
            Self::Core => "Ядро",
            Self::App => "Приложение",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    None,
    Config,
    Settings,
    Logs,
    RawKeys,
    Editor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocFilter {
    All,
    Fav,
    Fast,
}

pub struct RClashApp {
    pub current_tab: Tab,
    pub app_config: AppConfig,
    pub core_version: Option<String>,
    pub core_alive: bool,
    last_poll: f64,
    tray: Option<crate::tray::TrayHandle>,
    version_promise: Option<Promise<(bool, Option<String>)>>,
    pub profile_store: rclash_config::profile::ProfileStore,
    pub profiles_error: Option<String>,
    pub raw_keys: Vec<String>,
    pub raw_error: Option<String>,
    pub import_url: String,
    pub import_interval: UpdateInterval,
    pub import_raw_text: String,
    pub import_name: String,
    pub proxies_promise: Option<Promise<anyhow::Result<serde_json::Value>>>,
    pub proxies_data: Option<serde_json::Value>,
    pub proxies_error: Option<String>,
    pub proxies_mode: ProxyMode,
    mode_promise: Option<Promise<anyhow::Result<ProxyMode>>>,
    pub proxy_delays: HashMap<String, Option<u64>>,
    #[allow(clippy::type_complexity)]
    delays_promise: Option<Promise<Vec<(String, Option<u64>)>>>,
    pub delays_running: bool,
    pub connections_data: Option<Snapshot>,
    pub connections_error: Option<String>,
    pub connections_filter: String,
    pub connections_sort: ConnectionsSort,
    connections_rx: Option<Receiver<Snapshot>>,
    connections_err: Option<String>,
    connections_close_promise: Option<Promise<anyhow::Result<()>>>,
    pub traffic_up_buf: VecDeque<f64>,
    pub traffic_down_buf: VecDeque<f64>,
    pub traffic_total_up: u64,
    pub traffic_total_down: u64,
    traffic_rx: Option<Receiver<TrafficInfo>>,
    pub logs_buf: VecDeque<LogEntry>,
    pub logs_level: String,
    pub logs_filter: String,
    pub logs_search: String,
    pub logs_autoscroll: bool,
    pub logs_tab: LogsTab,
    logs_rx: Option<Receiver<LogEntry>>,
    logs_sender: Option<Sender<LogEntry>>,
    traffic_started: bool,
    connections_started: bool,
    logs_started: bool,
    logs_restart_needed: bool,
    updater_promise: Option<Promise<anyhow::Result<Option<UpdateInfo>>>>,
    pub updater_available: Option<UpdateInfo>,
    pub updater_error: Option<String>,
    pub updater_checking: bool,
    last_updater_check: f64,
    pub updater_downloading: bool,
    updater_download_promise: Option<Promise<anyhow::Result<()>>>,
    pub overlay: Overlay,
    pub loc_search: String,
    pub loc_filter: LocFilter,
    pub loc_fav: HashSet<String>,
    pub selected_proxy: Option<String>,
    pub selected_group: String,
    pub editor_name: Option<String>,
    pub editor_content: String,
    pub editor_error: Option<String>,
    pub settings_tab: usize,
    pub input_popup_open: bool,
    pub input_popup_text: String,
    pub show_add_menu_requested: bool,
}

fn theme_visuals(theme: Theme) -> egui::Visuals {
    let mut v = match theme {
        Theme::Light => egui::Visuals::light(),
        Theme::Dark => egui::Visuals::dark(),
        Theme::Oled => {
            let mut o = egui::Visuals::dark();
            o.panel_fill = egui::Color32::BLACK;
            o.window_fill = egui::Color32::BLACK;
            o.extreme_bg_color = egui::Color32::from_rgb(0x1A, 0x1A, 0x1A);
            o.faint_bg_color = egui::Color32::from_rgb(0x1A, 0x1A, 0x1A);
            o.code_bg_color = egui::Color32::from_rgb(0x1A, 0x1A, 0x1A);
            o
        }
    };
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = egui::CornerRadius::ZERO;
        let border = match theme {
            Theme::Light => egui::Color32::BLACK,
            Theme::Dark | Theme::Oled => egui::Color32::WHITE,
        };
        w.bg_stroke = egui::Stroke::new(1.0_f32, border);
        w.fg_stroke = egui::Stroke::new(1.0_f32, border);
        if theme == Theme::Oled {
            w.bg_fill = egui::Color32::from_rgb(0x1A, 0x1A, 0x1A);
        }
    }
    v.widgets.noninteractive.corner_radius = egui::CornerRadius::ZERO;
    v.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
    v.widgets.hovered.corner_radius = egui::CornerRadius::ZERO;
    v.widgets.active.corner_radius = egui::CornerRadius::ZERO;
    v.widgets.open.corner_radius = egui::CornerRadius::ZERO;
    v
}

pub fn border_color_for(theme: Theme) -> egui::Color32 {
    match theme {
        Theme::Light => egui::Color32::BLACK,
        Theme::Dark | Theme::Oled => egui::Color32::WHITE,
    }
}

impl RClashApp {
    pub fn new(cc: &eframe::CreationContext<'_>, tray: Option<crate::tray::TrayHandle>) -> Self {
        let app_config = rclash_config::load_app_config();
        cc.egui_ctx.set_visuals(theme_visuals(app_config.theme));
        let profile_store = rclash_config::profile::load_profile_store();
        let raw_keys = rclash_config::custom::list_raw_keys().unwrap_or_default();
        let _ = rclash_db::open().map(|c| {
            let _ = rclash_db::migrate_from_files(&c);
        });
        let loc_fav = rclash_db::open()
            .and_then(|c| rclash_db::list_favorites(&c))
            .unwrap_or_default();
        let logs_level = "info".to_owned();
        Self {
            current_tab: Tab::Dashboard,
            app_config,
            core_version: None,
            core_alive: false,
            last_poll: 0.0,
            tray,
            version_promise: None,
            profile_store,
            profiles_error: None,
            raw_keys,
            raw_error: None,
            import_url: String::new(),
            import_interval: UpdateInterval::H24,
            import_raw_text: String::new(),
            import_name: String::new(),
            proxies_promise: None,
            proxies_data: None,
            proxies_error: None,
            proxies_mode: ProxyMode::Rule,
            mode_promise: None,
            proxy_delays: HashMap::new(),
            delays_promise: None,
            delays_running: false,
            connections_data: None,
            connections_error: None,
            connections_filter: String::new(),
            connections_sort: ConnectionsSort::StartDesc,
            connections_rx: None,
            connections_err: None,
            connections_close_promise: None,
            traffic_up_buf: VecDeque::with_capacity(62),
            traffic_down_buf: VecDeque::with_capacity(62),
            traffic_total_up: 0,
            traffic_total_down: 0,
            traffic_rx: None,
            logs_buf: VecDeque::with_capacity(502),
            logs_level,
            logs_filter: String::new(),
            logs_search: String::new(),
            logs_autoscroll: true,
            logs_tab: LogsTab::Core,
            logs_rx: None,
            logs_sender: None,
            traffic_started: false,
            connections_started: false,
            logs_started: false,
            logs_restart_needed: false,
            updater_promise: None,
            updater_available: None,
            updater_error: None,
            updater_checking: false,
            last_updater_check: 0.0,
            updater_downloading: false,
            updater_download_promise: None,
            overlay: Overlay::None,
            loc_search: String::new(),
            loc_filter: LocFilter::All,
            loc_fav,
            selected_proxy: None,
            selected_group: "all".to_owned(),
            editor_name: None,
            editor_content: String::new(),
            editor_error: None,
            settings_tab: 0,
            input_popup_open: false,
            input_popup_text: String::new(),
            show_add_menu_requested: false,
        }
    }

    pub fn set_theme(&mut self, theme: Theme, ctx: &egui::Context) {
        self.app_config.theme = theme;
        ctx.set_visuals(theme_visuals(theme));
        let _ = rclash_config::save_app_config(&self.app_config);
        log::info!("theme changed to {:?}", theme);
    }

    pub fn set_log_level(&mut self, level: LogLevel, _ctx: &egui::Context) {
        self.app_config.log_level = level;
        crate::logger::set_level(level.to_log_filter());
        let _ = rclash_config::save_app_config(&self.app_config);
        log::info!("app log level changed to {}", level.as_str());
    }

    pub fn refresh_profiles(&mut self) {
        self.profile_store = rclash_config::profile::load_profile_store();
        self.raw_keys = rclash_config::custom::list_raw_keys().unwrap_or_default();
        log::info!(
            "profiles refreshed count={}",
            self.profile_store.profiles.len()
        );
    }

    pub fn reload_raw_keys(&mut self) {
        self.raw_keys = rclash_config::custom::list_raw_keys().unwrap_or_default();
    }

    fn poll_core(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        if now - self.last_poll < 2.0 && self.version_promise.is_some() {
            if let Some(p) = &self.version_promise {
                if let Some(res) = p.ready() {
                    self.core_alive = res.0;
                    self.core_version = res.1.clone();
                    self.version_promise = None;
                    if self.core_alive && !self.traffic_started {
                        self.start_traffic_stream();
                    }
                    if self.core_alive && !self.logs_started {
                        self.start_logs_stream();
                    }
                    if self.core_alive && !self.connections_started {
                        self.start_connections_stream();
                    }
                } else {
                    ctx.request_repaint_after(std::time::Duration::from_millis(200));
                }
            }
            return;
        }
        if now - self.last_poll < 2.0 {
            return;
        }
        self.last_poll = now;
        let promise = Promise::spawn_thread("core_version", || {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(2))
                .build();
            let Ok(client) = client else {
                return (false, None);
            };
            let Ok(resp) = client.get("http://127.0.0.1:9090/version").send() else {
                return (false, None);
            };
            if !resp.status().is_success() {
                return (false, None);
            }
            let Ok(v) = resp.json::<serde_json::Value>() else {
                return (false, None);
            };
            let ver = v
                .get("version")
                .and_then(|x| x.as_str())
                .map(|s| s.to_owned());
            (true, ver)
        });
        self.version_promise = Some(promise);
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
        if let Some(p) = &self.version_promise {
            if let Some(res) = p.ready() {
                let was_alive = self.core_alive;
                self.core_alive = res.0;
                self.core_version = res.1.clone();
                if self.core_alive && !was_alive {
                    if !self.traffic_started {
                        self.start_traffic_stream();
                    }
                    if !self.logs_started {
                        self.start_logs_stream();
                    }
                    if !self.connections_started {
                        self.start_connections_stream();
                    }
                }
            }
        }
    }

    pub fn fetch_proxies(&mut self, ctx: &egui::Context) {
        if self.proxies_promise.is_some() {
            return;
        }
        let promise = Promise::spawn_thread("fetch_proxies", || {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let resp = client
                .get("http://127.0.0.1:9090/proxies")
                .send()
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            if !resp.status().is_success() {
                anyhow::bail!("GET /proxies {}", resp.status());
            }
            let v: serde_json::Value = resp.json().map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(v)
        });
        self.proxies_promise = Some(promise);
        ctx.request_repaint();
    }

    pub fn poll_proxies(&mut self, ctx: &egui::Context) {
        if let Some(p) = &self.proxies_promise {
            if let Some(res) = p.ready() {
                match res {
                    Ok(v) => {
                        self.proxies_data = Some(v.clone());
                        self.proxies_error = None;
                    }
                    Err(e) => {
                        self.proxies_error = Some(format!("{e}"));
                    }
                }
                self.proxies_promise = None;
                ctx.request_repaint();
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
            }
        }
        if let Some(p) = &self.mode_promise {
            if let Some(res) = p.ready() {
                if let Ok(m) = res {
                    self.proxies_mode = *m;
                }
                self.mode_promise = None;
                ctx.request_repaint();
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
            }
        }
        if let Some(p) = &self.delays_promise {
            if let Some(res) = p.ready() {
                for (name, delay) in res {
                    self.proxy_delays.insert(name.clone(), *delay);
                }
                self.delays_promise = None;
                self.delays_running = false;
                ctx.request_repaint();
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
            }
        }
        if let Some(p) = &self.connections_close_promise {
            if let Some(res) = p.ready() {
                if let Err(e) = res {
                    self.connections_error = Some(format!("{e}"));
                }
                self.connections_close_promise = None;
                ctx.request_repaint();
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
            }
        }
    }

    pub fn fetch_mode(&mut self, _ctx: &egui::Context) {
        if self.mode_promise.is_some() {
            return;
        }
        let promise = Promise::spawn_thread("fetch_mode", || {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(3))
                .build()
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let resp = client
                .get("http://127.0.0.1:9090/configs")
                .send()
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let v: serde_json::Value = resp.json().map_err(|e| anyhow::anyhow!("{e}"))?;
            let mode_str = v.get("mode").and_then(|x| x.as_str()).unwrap_or("rule");
            Ok(ProxyMode::from_str(mode_str).unwrap_or(ProxyMode::Rule))
        });
        self.mode_promise = Some(promise);
    }

    pub fn set_mode_async(&mut self, mode: ProxyMode, ctx: &egui::Context) {
        self.proxies_mode = mode;
        let mode_str = mode.as_str().to_owned();
        log::info!("set proxy mode to {}", mode_str);
        let _ = Promise::spawn_thread("set_mode", move || {
            let client = reqwest::blocking::Client::new();
            let res = client
                .patch("http://127.0.0.1:9090/configs")
                .json(&serde_json::json!({"mode": mode_str}))
                .send();
            if let Err(e) = res {
                log::warn!("set_mode failed: {e}");
            }
        });
        ctx.request_repaint();
    }

    pub fn test_delays_async(&mut self, proxies: Vec<String>, ctx: &egui::Context) {
        if self.delays_running {
            return;
        }
        self.delays_running = true;
        log::info!("testing delays for {} proxies", proxies.len());
        let promise = Promise::spawn_thread("test_delays", move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(8))
                .build()
                .unwrap();
            let mut out = Vec::new();
            for name in proxies {
                let url = format!(
                    "http://127.0.0.1:9090/proxies/{}/delay?url={}&timeout=5000",
                    percent_encode(&name),
                    percent_encode("http://www.gstatic.com/generate_204")
                );
                let delay = client
                    .get(&url)
                    .send()
                    .ok()
                    .and_then(|r| r.json::<serde_json::Value>().ok())
                    .and_then(|v| v.get("delay").and_then(|d| d.as_u64()));
                out.push((name, delay));
            }
            out
        });
        self.delays_promise = Some(promise);
        ctx.request_repaint();
    }

    fn start_traffic_stream(&mut self) {
        if self.traffic_started {
            return;
        }
        self.traffic_started = true;
        log::info!("starting traffic WS stream");
        let (tx, rx) = channel::<TrafficInfo>();
        self.traffic_rx = Some(rx);
        std::thread::spawn(move || loop {
            match ws_traffic_once(&tx) {
                Ok(()) => log::warn!("traffic WS closed, reconnect in 3s"),
                Err(e) => log::warn!("traffic WS error: {e}, reconnect in 3s"),
            }
            std::thread::sleep(std::time::Duration::from_secs(3));
        });
    }

    fn poll_traffic(&mut self, ctx: &egui::Context) {
        let mut updated = false;
        if let Some(rx) = &self.traffic_rx {
            while let Ok(info) = rx.try_recv() {
                self.traffic_total_up = info.up_total;
                self.traffic_total_down = info.down_total;
                self.traffic_up_buf.push_back(info.up as f64);
                self.traffic_down_buf.push_back(info.down as f64);
                if self.traffic_up_buf.len() > 60 {
                    self.traffic_up_buf.pop_front();
                }
                if self.traffic_down_buf.len() > 60 {
                    self.traffic_down_buf.pop_front();
                }
                updated = true;
            }
        }
        if updated {
            ctx.request_repaint();
        } else if self.traffic_started {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }

    fn start_connections_stream(&mut self) {
        if self.connections_started {
            return;
        }
        self.connections_started = true;
        log::info!("starting connections WS stream interval=1000");
        let (tx, rx) = channel::<Snapshot>();
        self.connections_rx = Some(rx);
        std::thread::spawn(move || loop {
            match ws_connections_once(&tx) {
                Ok(()) => log::warn!("connections WS closed, reconnect in 3s"),
                Err(e) => log::warn!("connections WS error: {e}, reconnect in 3s"),
            }
            std::thread::sleep(std::time::Duration::from_secs(3));
        });
    }

    fn poll_connections_stream(&mut self, ctx: &egui::Context) {
        let mut updated = false;
        if let Some(rx) = &self.connections_rx {
            while let Ok(snap) = rx.try_recv() {
                self.connections_data = Some(snap);
                self.connections_error = None;
                updated = true;
            }
        }
        if self.logs_restart_needed {
            self.logs_restart_needed = false;
        }
        if updated {
            ctx.request_repaint();
        } else if self.connections_started {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
        if let Some(err) = &self.connections_err {
            self.connections_error = Some(err.clone());
        }
    }

    pub fn close_connection_async(&mut self, id: String, ctx: &egui::Context) {
        log::info!("close connection {}", id);
        let promise = Promise::spawn_thread("close_connection", move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let url = format!("http://127.0.0.1:9090/connections/{}", percent_encode(&id));
            client
                .delete(&url)
                .send()
                .map_err(|e| anyhow::anyhow!("{e}"))?
                .error_for_status()
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(())
        });
        self.connections_close_promise = Some(promise);
        ctx.request_repaint();
    }

    pub fn close_all_connections_async(&mut self, ctx: &egui::Context) {
        log::info!("close all connections");
        let promise = Promise::spawn_thread("close_all_connections", move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            client
                .delete("http://127.0.0.1:9090/connections")
                .send()
                .map_err(|e| anyhow::anyhow!("{e}"))?
                .error_for_status()
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(())
        });
        self.connections_close_promise = Some(promise);
        ctx.request_repaint();
    }

    fn start_logs_stream(&mut self) {
        if self.logs_started && !self.logs_restart_needed {
            return;
        }
        let level = self.logs_level.clone();
        log::info!("starting logs WS stream level={}", level);
        let (tx, rx) = channel::<LogEntry>();
        self.logs_rx = Some(rx);
        self.logs_sender = Some(tx.clone());
        self.logs_started = true;
        self.logs_restart_needed = false;
        std::thread::spawn(move || loop {
            match ws_logs_once(&level, &tx) {
                Ok(()) => log::warn!("logs WS closed, reconnect in 3s"),
                Err(e) => log::warn!("logs WS error: {e}, reconnect in 3s"),
            }
            std::thread::sleep(std::time::Duration::from_secs(3));
        });
    }

    pub fn restart_logs_stream(&mut self) {
        log::info!("restart logs stream level={}", self.logs_level);
        self.logs_started = false;
        self.logs_restart_needed = true;
        self.start_logs_stream();
    }

    fn poll_logs_stream(&mut self, ctx: &egui::Context) {
        let mut updated = false;
        if let Some(rx) = &self.logs_rx {
            while let Ok(entry) = rx.try_recv() {
                if self.logs_buf.len() >= 500 {
                    self.logs_buf.pop_front();
                }
                self.logs_buf.push_back(entry);
                updated = true;
            }
        }
        if updated {
            ctx.request_repaint();
        } else if self.logs_started {
            ctx.request_repaint_after(std::time::Duration::from_millis(300));
        }
    }

    pub fn check_update_async(&mut self, ctx: &egui::Context, force: bool) {
        if self.updater_checking || self.updater_downloading {
            return;
        }
        let now = ctx.input(|i| i.time);
        if !force && now - self.last_updater_check < 60.0 {
            return;
        }
        if !force
            && !rclash_updater::should_check(
                self.app_config.last_check.as_deref(),
                self.app_config.update_interval,
            )
        {
            return;
        }
        if let Some(ver) = &self.app_config.skipped_version {
            if self.updater_available.as_ref().map(|u| &u.version) == Some(ver) {
                return;
            }
        }
        self.updater_checking = true;
        self.last_updater_check = now;
        let url = manifest_url();
        log::info!("check update manifest {}", url);
        let promise = Promise::spawn_thread("check_update", move || {
            let res = rclash_updater::check_for_update(&url);
            match res {
                Ok(Some(info)) => Ok(Some(info)),
                Ok(None) => Ok(None),
                Err(e) => Err(anyhow::anyhow!("{e}")),
            }
        });
        self.updater_promise = Some(promise);
        ctx.request_repaint();
    }

    fn poll_updater(&mut self, ctx: &egui::Context) {
        if let Some(p) = &self.updater_promise {
            if let Some(res) = p.ready() {
                match res {
                    Ok(Some(info)) => {
                        let skipped = self
                            .app_config
                            .skipped_version
                            .as_ref()
                            .map(|s| s == &info.version)
                            .unwrap_or(false);
                        if !skipped {
                            log::info!("update available {}", info.version);
                            self.updater_available = Some(info.clone());
                            self.updater_error = None;
                        }
                        self.app_config.last_check = Some(chrono::Utc::now().to_rfc3339());
                        let _ = rclash_config::save_app_config(&self.app_config);
                    }
                    Ok(None) => {
                        log::info!("no update available");
                        self.updater_available = None;
                        self.updater_error = None;
                        self.app_config.last_check = Some(chrono::Utc::now().to_rfc3339());
                        let _ = rclash_config::save_app_config(&self.app_config);
                    }
                    Err(e) => {
                        log::warn!("update check failed: {e}");
                        self.updater_error = Some(format!("{e}"));
                    }
                }
                self.updater_promise = None;
                self.updater_checking = false;
                ctx.request_repaint();
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
            }
        }
        if let Some(p) = &self.updater_download_promise {
            if let Some(res) = p.ready() {
                match res {
                    Ok(()) => {
                        log::info!("update downloaded, restart core?");
                        self.updater_downloading = false;
                        self.updater_available = None;
                        self.updater_error = None;
                    }
                    Err(e) => {
                        log::warn!("download failed: {e}");
                        self.updater_error = Some(format!("{e}"));
                        self.updater_downloading = false;
                    }
                }
                self.updater_download_promise = None;
                ctx.request_repaint();
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
            }
        }
    }

    pub fn download_update_async(&mut self, ctx: &egui::Context) {
        let Some(info) = self.updater_available.clone() else {
            return;
        };
        if self.updater_downloading {
            return;
        }
        self.updater_downloading = true;
        log::info!("download update {} {}", info.version, info.url);
        let promise = Promise::spawn_thread("download_update", move || {
            let dest =
                rclash_updater::core_bin_path().ok_or_else(|| anyhow::anyhow!("no core path"))?;
            rclash_updater::download_and_verify(&info.url, &info.sha256, &dest)?;
            Ok(())
        });
        self.updater_download_promise = Some(promise);
        ctx.request_repaint();
    }

    pub fn skip_update_version(&mut self) {
        if let Some(info) = &self.updater_available {
            self.app_config.skipped_version = Some(info.version.clone());
            let _ = rclash_config::save_app_config(&self.app_config);
            log::info!("skip version {}", info.version);
        }
        self.updater_available = None;
    }

    pub fn dismiss_update(&mut self) {
        log::info!("dismiss update until next interval");
        self.updater_available = None;
    }
}

fn ws_traffic_once(tx: &Sender<TrafficInfo>) -> anyhow::Result<()> {
    let (mut socket, _) = tungstenite::connect("ws://127.0.0.1:9090/traffic")?;
    loop {
        let msg = socket.read()?;
        if msg.is_text() || msg.is_binary() {
            let text = msg.to_text()?;
            if let Ok(info) = serde_json::from_str::<TrafficInfo>(text) {
                let _ = tx.send(info);
            }
        }
    }
}

fn ws_connections_once(tx: &Sender<Snapshot>) -> anyhow::Result<()> {
    let (mut socket, _) = tungstenite::connect("ws://127.0.0.1:9090/connections?interval=1000")?;
    loop {
        let msg = socket.read()?;
        if msg.is_text() || msg.is_binary() {
            let text = msg.to_text()?;
            if let Ok(snap) = serde_json::from_str::<Snapshot>(text) {
                let _ = tx.send(snap);
            }
        }
    }
}

fn ws_logs_once(level: &str, tx: &Sender<LogEntry>) -> anyhow::Result<()> {
    let url = format!("ws://127.0.0.1:9090/logs?level={}", percent_encode(level));
    let (mut socket, _) = tungstenite::connect(url)?;
    loop {
        let msg = socket.read()?;
        if msg.is_text() || msg.is_binary() {
            let text = msg.to_text()?;
            if let Ok(entry) = serde_json::from_str::<LogEntry>(text) {
                let _ = tx.send(entry);
            } else if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
                let level = v
                    .get("type")
                    .or_else(|| v.get("level"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("info")
                    .to_owned();
                let payload = v
                    .get("payload")
                    .or_else(|| v.get("message"))
                    .and_then(|x| x.as_str())
                    .unwrap_or(text)
                    .to_owned();
                let _ = tx.send(LogEntry { level, payload });
            }
        }
    }
}

fn percent_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

impl eframe::App for RClashApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(handle) = &self.tray {
            crate::tray::poll_tray(handle, ctx);
        }
        if ctx.input(|i| i.viewport().close_requested())
            && self.app_config.minimize_to_tray
            && self.tray.is_some()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        self.poll_core(ctx);
        self.poll_proxies(ctx);
        self.poll_traffic(ctx);
        self.poll_connections_stream(ctx);
        self.poll_logs_stream(ctx);
        if self.proxies_data.is_none() && self.proxies_promise.is_none() && self.core_alive {
            self.fetch_proxies(ctx);
            self.fetch_mode(ctx);
        }
        match self.overlay {
            Overlay::None => self.show_main(ctx),
            Overlay::Config => self.show_config(ctx),
            Overlay::Logs => self.show_logs(ctx),
            Overlay::Settings => self.show_settings(ctx),
            Overlay::RawKeys => self.show_raw_keys(ctx),
            Overlay::Editor => self.show_editor(ctx),
        }
        self.show_input_popup(ctx);
        self.show_add_menu(ctx);
    }
}

impl RClashApp {
    fn border(&self) -> egui::Color32 {
        border_color_for(self.app_config.theme)
    }
    fn show_main(&mut self, ctx: &egui::Context) {
        let border = self.border();
        let proxy_on = rclash_sys_proxy::current()
            .get()
            .map(|s| s == rclash_sys_proxy::ProxyState::Enabled)
            .unwrap_or(false);
        let core_path = rclash_core_manager::resolve_core_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "рядом с приложением".to_owned());
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(6.0);
                egui::Frame::new().stroke(egui::Stroke::new(1.0_f32, border)).inner_margin(egui::Margin::same(8)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Профили").strong().size(11.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(egui::RichText::new("\u{2699}").size(13.0)).on_hover_text("Настройки").clicked() { self.overlay = Overlay::Settings; }
                            if ui.button(egui::RichText::new("\u{2630}").size(13.0)).on_hover_text("Логи").clicked() { self.overlay = Overlay::Logs; }
                            if ui.button(egui::RichText::new("\u{270E}").size(13.0)).on_hover_text("Редактировать конфиг").clicked() { self.overlay = Overlay::Editor; }
                            if ui.button(egui::RichText::new("+").size(14.0)).on_hover_text("Добавить профиль").clicked() { self.show_add_menu_requested = true; }
                            if ui.button(egui::RichText::new("\u{21BB}").size(13.0)).on_hover_text("Обновить").clicked() {
                                let _ = poll_promise::Promise::spawn_thread("reload_core", move || { let c = reqwest::blocking::Client::new(); let _ = c.put("http://127.0.0.1:9090/configs?force=true").send(); });
                            }
                        });
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let active = self.profile_store.active.clone().unwrap_or_else(|| "—".to_owned());
                        egui::ComboBox::from_id_salt("profiles_combo").selected_text(active.clone()).width(220.0).show_ui(ui, |ui| {
                            for pr in &self.profile_store.profiles.clone() {
                                let is_active = self.profile_store.active.as_deref() == Some(&pr.name);
                                if ui.selectable_label(is_active, &pr.name).clicked() {
                                    let mut store = self.profile_store.clone();
                                    store.active = Some(pr.name.clone());
                                    let _ = rclash_config::profile::save_profile_store(&store);
                                    self.profile_store = store;
                                    let _ = poll_promise::Promise::spawn_thread("reload_core", move || { let c = reqwest::blocking::Client::new(); let _ = c.put("http://127.0.0.1:9090/configs?force=true").send(); });
                                }
                            }
                            if !self.raw_keys.is_empty() {
                                ui.separator();
                                for k in self.raw_keys.clone() {
                                    let lab = format!("сырой: {}", k);
                                    if ui.selectable_label(false, lab).clicked() { self.overlay = Overlay::RawKeys; }
                                }
                            }
                        });
                        ui.label(egui::RichText::new(format!("{} прокси", self.raw_keys.len())).weak().small());
                    });
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("профили: {} | {}", self.profile_store.profiles.len(), core_path)).weak().small().monospace());
                    });
                });
                ui.add_space(6.0);
                egui::Frame::new().stroke(egui::Stroke::new(1.0_f32, border)).inner_margin(egui::Margin::same(8)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Прокси").strong().size(11.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let testing = self.delays_running;
                            if ui.add_enabled(!testing, egui::Button::new(egui::RichText::new("\u{25CE}").size(12.0))).on_hover_text("Пинг").clicked() {
                                if let Some(data) = self.proxies_data.clone() { let names = extract_proxy_names(&data); self.test_delays_async(names, ctx); } else { self.fetch_proxies(ctx); }
                            }
                            if testing { ui.spinner(); }
                        });
                    });
                    ui.horizontal(|ui| {
                        let groups = extract_groups(&self.proxies_data);
                        let mut sel = self.selected_group.clone();
                        egui::ComboBox::from_id_salt("proxy_group").selected_text(if sel=="all" {"Все группы".to_owned()} else {sel.clone()}).width(180.0).show_ui(ui, |ui| {
                            if ui.selectable_label(sel=="all", "Все группы").clicked() { sel="all".to_owned(); }
                            for (g, _) in &groups { if ui.selectable_label(sel==*g, g).clicked() { sel=g.clone(); } }
                            if ui.selectable_label(sel=="fav", "★ Избранное").clicked() { sel="fav".to_owned(); }
                        });
                        self.selected_group = sel;
                    });
                });
                ui.add_space(6.0);
                egui::Frame::new().stroke(egui::Stroke::new(1.0_f32, border)).inner_margin(egui::Margin::same(6)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Локации").strong().size(11.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(format!("{} избранных", self.loc_fav.len())).weak().small());
                        });
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.loc_search).hint_text("Поиск...").desired_width(160.0));
                        ui.separator();
                        for (f,l) in [(LocFilter::All,"Все"),(LocFilter::Fav,"★"),(LocFilter::Fast,"●")] {
                            let sel = self.loc_filter==f;
                            if ui.selectable_label(sel,l).on_hover_text(l).clicked(){ self.loc_filter=f; }
                        }
                    });
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                        egui::Grid::new("locs_grid").num_columns(4).spacing([8.0,2.0]).striped(true).show(ui, |ui| {
                            ui.label(egui::RichText::new("Страна").weak().small()); ui.label(egui::RichText::new("Тип").weak().small()); ui.label(egui::RichText::new("Пинг").weak().small()); ui.label(egui::RichText::new("").weak().small()); ui.end_row();
                            let proxies = extract_singles(&self.proxies_data);
                            let groups2 = extract_groups(&self.proxies_data);
                            let filtered: Vec<_> = proxies.into_iter().filter(|(name,val)| {
                                let q=self.loc_search.to_lowercase();
                                if !q.is_empty() && !name.to_lowercase().contains(&q) && !val.get("type").and_then(|v| v.as_str()).unwrap_or("").to_lowercase().contains(&q) { return false; }
                                if self.loc_filter==LocFilter::Fav && !self.loc_fav.contains(name) { return false; }
                                if self.loc_filter==LocFilter::Fast { let d=self.proxy_delays.get(name).copied().flatten().or_else(|| val.get("history").and_then(|v| v.as_array()).and_then(|a| a.last()).and_then(|e| e.get("delay")).and_then(|d| d.as_u64())); if d.map_or(true, |v| v>=100) { return false; } }
                                if self.selected_group!="all" {
                                    if self.selected_group=="fav" { if !self.loc_fav.contains(name) { return false; } } else {
                                        let mut found=false;
                                        for (gname,gval) in &groups2 { if gname==&self.selected_group { if let Some(arr)=gval.get("all").and_then(|v| v.as_array()) { for v in arr { if v.as_str()==Some(name.as_str()) { found=true; break; } } } break; } }
                                        if !found { return false; }
                                    }
                                }
                                true
                            }).collect();
                            if filtered.is_empty() {
                                ui.label(egui::RichText::new("Нет локаций").weak()); ui.end_row();
                            } else {
                                for (name,val) in filtered {
                                    let tp = val.get("type").and_then(|v| v.as_str()).unwrap_or("?").to_owned();
                                    let delay = self.proxy_delays.get(&name).copied().flatten().or_else(|| val.get("history").and_then(|v| v.as_array()).and_then(|a| a.last()).and_then(|e| e.get("delay")).and_then(|d| d.as_u64()));
                                    let delay_str = delay_text(delay);
                                    let is_fav = self.loc_fav.contains(&name);
                                    let is_sel = self.selected_proxy.as_deref()==Some(&name);
                                    let name_rich = if is_sel { egui::RichText::new(&name).strong() } else { egui::RichText::new(&name) };
                                    ui.label(name_rich);
                                    ui.label(egui::RichText::new(tp).weak().small().monospace());
                                    ui.label(egui::RichText::new(delay_str).small().monospace());
                                    ui.horizontal(|ui| {
                                        if ui.add(egui::RadioButton::new(is_sel, "")).on_hover_text("Выбрать").clicked() {
                                            self.selected_proxy = Some(name.clone());
                                            let target=name.clone();
                                            let group=if self.selected_group!="all" && self.selected_group!="fav" { self.selected_group.clone() } else { "PROXY".to_owned() };
                                            let _ = poll_promise::Promise::spawn_thread("select_proxy", move || { let c=reqwest::blocking::Client::new(); let _=c.put(format!("http://127.0.0.1:9090/proxies/{}", group)).json(&serde_json::json!({"name": target})).send(); });
                                        }
                                        let fav_icon = if is_fav {"★"} else {"☆"};
                                        if ui.button(egui::RichText::new(fav_icon).size(11.0)).on_hover_text(if is_fav {"Убрать из избранного"} else {"В избранное"}).clicked() {
                                            if is_fav { self.loc_fav.remove(&name); let _=rclash_db::open().map(|c| {let _=rclash_db::remove_favorite(&c,&name);}); } else { self.loc_fav.insert(name.clone()); let g=if self.selected_group!="all" && self.selected_group!="fav" {Some(self.selected_group.as_str())} else {None}; let _=rclash_db::open().map(|c| {let _=rclash_db::add_favorite(&c,&name,g);}); }
                                        }
                                    });
                                    ui.end_row();
                                }
                            }
                        });
                    });
                });
                ui.add_space(6.0);
                if self.app_config.show_traffic_graph {
                    egui::Frame::new().stroke(egui::Stroke::new(1.0_f32, border)).inner_margin(egui::Margin::same(8)).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("График").strong().size(11.0));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(format!("↑ {} ↓ {}", format_bytes(self.traffic_total_up), format_bytes(self.traffic_total_down))).weak().small().monospace());
                            });
                        });
                        ui.add_space(4.0);
                        if !self.core_alive { ui.label(egui::RichText::new("Ядро не запущено").weak()); }
                        else if self.traffic_up_buf.is_empty() && self.traffic_down_buf.is_empty() { ui.label(egui::RichText::new("Ожидание данных...").weak()); ui.add(egui::ProgressBar::new(0.0).animate(true)); }
                        else {
                            let max_val = self.traffic_up_buf.iter().chain(self.traffic_down_buf.iter()).copied().fold(0.0_f64, f64::max).max(1024.0);
                            let (rect,_) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 80.0), egui::Sense::hover());
                            let painter = ui.painter_at(rect);
                            painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0_f32, border), egui::StrokeKind::Inside);
                            let inner = rect.shrink2(egui::vec2(4.0,4.0));
                            for i in 1..3 { let y=inner.top()+inner.height()*(i as f32/3.0); painter.hline(inner.x_range(), y, (1.0, egui::Color32::from_gray(80))); }
                            let plot = |buf:&VecDeque<f64>, col:egui::Color32| {
                                if buf.len()<2 { return; }
                                let mut pts=Vec::with_capacity(buf.len());
                                for (i,v) in buf.iter().enumerate() {
                                    let x=inner.left()+(i as f32/60.0)*inner.width();
                                    let y=inner.bottom()-(*v as f32/max_val as f32).clamp(0.0,1.0)*inner.height();
                                    pts.push(egui::pos2(x,y));
                                }
                                painter.add(egui::Shape::line(pts, egui::Stroke::new(1.2_f32, col)));
                            };
                            plot(&self.traffic_up_buf, egui::Color32::from_gray(140));
                            plot(&self.traffic_down_buf, egui::Color32::WHITE);
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                let cur_up=self.traffic_up_buf.back().copied().unwrap_or(0.0) as u64;
                                let cur_down=self.traffic_down_buf.back().copied().unwrap_or(0.0) as u64;
                                ui.label(egui::RichText::new(format!("↑ {}/с", rclash_core_manager::api::format_bytes(cur_up))).weak().small());
                                ui.label(egui::RichText::new(format!("↓ {}/с", rclash_core_manager::api::format_bytes(cur_down))).weak().small());
                                ui.label(egui::RichText::new(format!("макс {:.1} KB/с", max_val/1024.0)).weak().small());
                            });
                        }
                    });
                    ui.add_space(6.0);
                }
                egui::Frame::new().stroke(egui::Stroke::new(1.0_f32, border)).inner_margin(egui::Margin::same(8)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Приём").weak().small());
                            ui.label(egui::RichText::new(format_bytes(self.traffic_total_down)).strong().monospace());
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Отдача").weak().small());
                            ui.label(egui::RichText::new(format_bytes(self.traffic_total_up)).strong().monospace());
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("IP").weak().small());
                            ui.label(egui::RichText::new(self.core_version.clone().unwrap_or_else(|| "-".to_owned())).small().monospace());
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let status = if self.core_alive {"● Online"} else {"○ Offline"};
                            ui.label(egui::RichText::new(status).small());
                        });
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    egui::Frame::new().stroke(egui::Stroke::new(1.0_f32, border)).inner_margin(egui::Margin::same(8)).show(ui, |ui| {
                        ui.label(egui::RichText::new("Режим").strong().size(11.0));
                        ui.horizontal(|ui| {
                            for (mode,label,help) in [(ProxyMode::Rule,"rule","Правила — по конфигу"),(ProxyMode::Global,"global","Весь трафик через прокси"),(ProxyMode::Direct,"direct","Без прокси")] {
                                let sel=self.proxies_mode==mode;
                                if ui.selectable_label(sel,label).on_hover_text(help).clicked() { self.set_mode_async(mode, ctx); }
                            }
                        });
                    });
                    ui.add_space(6.0);
                    egui::Frame::new().stroke(egui::Stroke::new(1.0_f32, border)).inner_margin(egui::Margin::same(8)).show(ui, |ui| {
                        ui.label(egui::RichText::new("Система").strong().size(11.0));
                        let mut proxy_on2 = proxy_on;
                        if ui.checkbox(&mut proxy_on2, "proxy").on_hover_text("Системный прокси 127.0.0.1:7890").changed() {
                            let target=if proxy_on2 {rclash_sys_proxy::ProxyState::Enabled} else {rclash_sys_proxy::ProxyState::Disabled};
                            let _=rclash_sys_proxy::current().set(target, "127.0.0.1:7890");
                        }
                        let mut tun_on = self.app_config.tun_enabled;
                        if ui.checkbox(&mut tun_on, "tun").on_hover_text("TUN-режим (требует прав администратора)").changed() {
                            self.app_config.tun_enabled = tun_on;
                            let _=rclash_config::save_app_config(&self.app_config);
                        }
                        if tun_on || proxy_on {
                            ui.horizontal(|ui| {
                                ui.label("Автообновление");
                                let mut iv = self.app_config.update_interval;
                                egui::ComboBox::from_id_salt("auto_iv_main").selected_text(iv.label_ru()).width(60.0).show_ui(ui, |ui| {
                                    for v in UpdateInterval::all() { if ui.selectable_label(iv==*v, v.label_ru()).clicked() { iv = *v; } }
                                });
                                if iv!=self.app_config.update_interval { self.app_config.update_interval=iv; let _=rclash_config::save_app_config(&self.app_config); }
                                let _ = ui.small_button(egui::RichText::new("?").size(9.0)).on_hover_text("Интервал обновления подписок; берётся из подписки если указан, иначе из приложения");
                            });
                        }
                    });
                });
                ui.add_space(8.0);
            });
        });
    }
    fn show_config(&mut self, ctx: &egui::Context) {
        let border = self.border();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(egui::RichText::new("< Назад").size(11.0))
                    .on_hover_text("Назад")
                    .clicked()
                {
                    self.overlay = Overlay::None;
                }
                ui.label(egui::RichText::new("Выбор конфига").strong().size(13.0));
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some(e) = self.profiles_error.clone() {
                    ui.colored_label(egui::Color32::from_rgb(220, 80, 80), e);
                }
                if let Some(e) = self.raw_error.clone() {
                    ui.colored_label(egui::Color32::from_rgb(220, 180, 60), e);
                }
                egui::Frame::new()
                    .stroke(egui::Stroke::new(1.0_f32, border))
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Импорт по URL").strong().size(11.0));
                        ui.horizontal(|ui| {
                            ui.label("URL:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.import_url)
                                    .hint_text("https://example.com/sub.yaml")
                                    .desired_width(300.0),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Имя:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.import_name)
                                    .hint_text("необязательно")
                                    .desired_width(150.0),
                            );
                            egui::ComboBox::from_id_salt("cfg_iv")
                                .selected_text(self.import_interval.label_ru())
                                .width(70.0)
                                .show_ui(ui, |ui| {
                                    for iv in UpdateInterval::all() {
                                        ui.selectable_value(
                                            &mut self.import_interval,
                                            *iv,
                                            iv.label_ru(),
                                        );
                                    }
                                });
                            if ui
                                .button(egui::RichText::new("⬇ Загрузить").size(11.0))
                                .on_hover_text("Загрузить подписку")
                                .clicked()
                            {
                                let url = self.import_url.trim().to_owned();
                                let name_hint = self.import_name.trim().to_owned();
                                let interval = self.import_interval;
                                if url.is_empty() {
                                    self.profiles_error = Some("URL пустой".into());
                                } else {
                                    let name = if name_hint.is_empty() {
                                        url.split('/')
                                            .next_back()
                                            .unwrap_or("subscription")
                                            .split('?')
                                            .next()
                                            .unwrap_or("subscription")
                                            .to_owned()
                                    } else {
                                        name_hint
                                    };
                                    match fetch_and_save_subscription(&url, &name, interval) {
                                        Ok(profile) => {
                                            let mut store = self.profile_store.clone();
                                            store.add_or_replace(profile);
                                            let _ =
                                                rclash_config::profile::save_profile_store(&store);
                                            self.profile_store = store;
                                            self.profiles_error = None;
                                            self.import_url.clear();
                                            self.import_name.clear();
                                            reload_core(ctx);
                                        }
                                        Err(e) => {
                                            self.profiles_error = Some(format!("Ошибка: {e}"));
                                        }
                                    }
                                }
                            }
                        });
                    });
                ui.add_space(6.0);
                egui::Frame::new()
                    .stroke(egui::Stroke::new(1.0_f32, border))
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Сырые ссылки").strong().size(11.0));
                        ui.small("Каждая с новой строки → custom.yaml");
                        ui.add(
                            egui::TextEdit::multiline(&mut self.import_raw_text)
                                .hint_text("hysteria2://...\ntrojan://...")
                                .desired_rows(3)
                                .desired_width(f32::INFINITY),
                        );
                        ui.horizontal(|ui| {
                            if ui
                                .button(egui::RichText::new("+ Добавить").size(11.0))
                                .on_hover_text("Добавить ссылки")
                                .clicked()
                            {
                                let text = self.import_raw_text.trim().to_owned();
                                if text.is_empty() {
                                    self.raw_error = Some("Вставь ссылку".into());
                                } else {
                                    match rclash_subscription::parse_text_links(&text) {
                                        Ok(proxies) if proxies.is_empty() => {
                                            self.raw_error = Some("Не распознано".into());
                                        }
                                        Ok(proxies) => {
                                            match rclash_config::custom::add_raw_proxies(proxies) {
                                                Ok(added) => {
                                                    self.raw_error = None;
                                                    self.reload_raw_keys();
                                                    self.import_raw_text.clear();
                                                    if added == 0 {
                                                        self.raw_error = Some("Уже есть".into());
                                                    } else {
                                                        reload_core(ctx);
                                                    }
                                                }
                                                Err(e) => {
                                                    self.raw_error = Some(format!("{e}"));
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            self.raw_error = Some(format!("{e}"));
                                        }
                                    }
                                }
                            }
                            if ui
                                .button(egui::RichText::new("Очистить").size(11.0))
                                .on_hover_text("Очистить поле")
                                .clicked()
                            {
                                self.import_raw_text.clear();
                            }
                        });
                    });
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Профили — {} шт.",
                        self.profile_store.profiles.len()
                    ))
                    .strong(),
                );
                let profiles = self.profile_store.profiles.clone();
                let active = self.profile_store.active.clone();
                for p in profiles {
                    let is_active = active.as_deref() == Some(&p.name);
                    egui::Frame::new()
                        .stroke(egui::Stroke::new(1.0_f32, border))
                        .inner_margin(egui::Margin::same(6))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(if is_active { "●" } else { "○" });
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new(&p.name).strong());
                                    ui.label(
                                        egui::RichText::new(&p.path).weak().small().monospace(),
                                    );
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .button(egui::RichText::new("×").size(12.0))
                                            .on_hover_text("Удалить")
                                            .clicked()
                                        {
                                            let mut store = self.profile_store.clone();
                                            store.profiles.retain(|x| x.name != p.name);
                                            if store.active.as_deref() == Some(&p.name) {
                                                store.active = None;
                                            }
                                            let _ =
                                                rclash_config::profile::save_profile_store(&store);
                                            self.profile_store = store;
                                            reload_core(ctx);
                                        }
                                        if !is_active
                                            && ui
                                                .button(egui::RichText::new("Выбрать").size(11.0))
                                                .on_hover_text("Активировать")
                                                .clicked()
                                        {
                                            let mut store = self.profile_store.clone();
                                            store.active = Some(p.name.clone());
                                            let _ =
                                                rclash_config::profile::save_profile_store(&store);
                                            self.profile_store = store;
                                            reload_core(ctx);
                                        }
                                    },
                                );
                            });
                        });
                }
                ui.add_space(6.0);
                egui::Frame::new()
                    .stroke(egui::Stroke::new(1.0_f32, border))
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Сырые ключи").strong().size(11.0));
                            if ui
                                .button(egui::RichText::new("+").size(12.0))
                                .on_hover_text("Добавить сырой ключ")
                                .clicked()
                            {
                                self.show_add_menu_requested = true;
                            }
                            if ui
                                .button(egui::RichText::new("Открыть").size(11.0))
                                .on_hover_text("Открыть все сырые")
                                .clicked()
                            {
                                self.overlay = Overlay::RawKeys;
                            }
                        });
                        if self.raw_keys.is_empty() {
                            ui.label(egui::RichText::new("Пока пусто").weak());
                        } else {
                            for k in self.raw_keys.clone() {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(&k).small().monospace());
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .button(egui::RichText::new("×").size(10.0))
                                                .on_hover_text("Удалить")
                                                .clicked()
                                            {
                                                let _ = rclash_config::custom::remove_raw_proxy(&k);
                                                self.reload_raw_keys();
                                                reload_core(ctx);
                                            }
                                        },
                                    );
                                });
                            }
                        }
                    });
            });
        });
    }
    fn show_raw_keys(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(egui::RichText::new("< Назад").size(11.0))
                    .on_hover_text("Назад")
                    .clicked()
                {
                    self.overlay = Overlay::Config;
                }
                ui.label(egui::RichText::new("Сырые ключи").strong().size(13.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(egui::RichText::new("+").size(13.0))
                        .on_hover_text("Добавить")
                        .clicked()
                    {
                        self.show_add_menu_requested = true;
                    }
                });
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.raw_keys.is_empty() {
                    ui.label(egui::RichText::new("Нет ключей").weak());
                    return;
                }
                egui::Grid::new("raw_grid")
                    .num_columns(3)
                    .spacing([8.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("#").weak().small());
                        ui.label(egui::RichText::new("Ссылка").weak().small());
                        ui.label(egui::RichText::new("").weak().small());
                        ui.end_row();
                        for (i, key) in self.raw_keys.clone().into_iter().enumerate() {
                            let raw = rclash_config::custom::read_custom_yaml()
                                .ok()
                                .and_then(|v| {
                                    v.get("proxies")
                                        .and_then(|p| p.as_sequence())
                                        .and_then(|seq| {
                                            seq.iter()
                                                .find(|item| {
                                                    item.as_mapping()
                                                        .and_then(|m| {
                                                            m.get(serde_yaml::Value::String(
                                                                "name".into(),
                                                            ))
                                                        })
                                                        .and_then(|n| n.as_str())
                                                        == Some(key.as_str())
                                                })
                                                .map(|val| {
                                                    serde_yaml::to_string(val)
                                                        .unwrap_or_else(|_| key.clone())
                                                })
                                        })
                                })
                                .unwrap_or_else(|| key.clone());
                            ui.label(format!("{}", i + 1));
                            ui.label(egui::RichText::new(truncate(&raw, 48)).small().monospace());
                            if ui
                                .button(egui::RichText::new("×").size(11.0))
                                .on_hover_text("Удалить")
                                .clicked()
                            {
                                let _ = rclash_config::custom::remove_raw_proxy(&key);
                                self.reload_raw_keys();
                                reload_core(ctx);
                            }
                            ui.end_row();
                        }
                    });
            });
        });
    }
    fn show_editor(&mut self, ctx: &egui::Context) {
        let border = self.border();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(egui::RichText::new("< Назад").size(11.0))
                    .on_hover_text("Назад")
                    .clicked()
                {
                    self.overlay = Overlay::None;
                }
                ui.label(egui::RichText::new("Редактор конфига").strong().size(13.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(egui::RichText::new("Сохранить").size(11.0))
                        .on_hover_text("Сохранить")
                        .clicked()
                    {
                        if let Some(name) = self.editor_name.clone() {
                            match rclash_db::open().and_then(|c| {
                                rclash_db::update_content(&c, &name, &self.editor_content)
                                    .map_err(|e| anyhow::anyhow!("{}", e))
                            }) {
                                Ok(_) => {
                                    self.editor_error = None;
                                    reload_core(ctx);
                                    self.overlay = Overlay::None;
                                }
                                Err(e) => {
                                    self.editor_error = Some(format!("{}", e));
                                }
                            }
                        } else {
                            match (|| -> anyhow::Result<()> {
                                let path = rclash_config::custom::custom_yaml_path()
                                    .ok_or_else(|| anyhow::anyhow!("no path"))?;
                                std::fs::write(&path, self.editor_content.as_bytes())?;
                                Ok(())
                            })() {
                                Ok(_) => {
                                    self.editor_error = None;
                                    reload_core(ctx);
                                    self.overlay = Overlay::None;
                                }
                                Err(e) => {
                                    self.editor_error = Some(format!("{}", e));
                                }
                            }
                        }
                    }
                    if ui
                        .button(egui::RichText::new("Сбросить").size(11.0))
                        .on_hover_text("Сбросить")
                        .clicked()
                    {
                        self.editor_content.clear();
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Автообновление");
                let mut iv = self.app_config.update_interval;
                egui::ComboBox::from_id_salt("editor_iv")
                    .selected_text(iv.label_ru())
                    .width(60.0)
                    .show_ui(ui, |ui| {
                        for v in UpdateInterval::all() {
                            if ui.selectable_label(iv == *v, v.label_ru()).clicked() {
                                iv = *v;
                            }
                        }
                    });
                if iv != self.app_config.update_interval {
                    self.app_config.update_interval = iv;
                    let _ = rclash_config::save_app_config(&self.app_config);
                }
                let _ = ui
                    .small_button(egui::RichText::new("?").size(9.0))
                    .on_hover_text("Интервал автообновления подписок");
            });
            if let Some(e) = self.editor_error.clone() {
                ui.colored_label(egui::Color32::from_rgb(220, 80, 80), e);
            }
            egui::Frame::new()
                .stroke(egui::Stroke::new(1.0_f32, border))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.editor_content)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .desired_rows(18),
                        );
                    });
                });
        });
    }
    fn show_logs(&mut self, ctx: &egui::Context) {
        let border = self.border();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(egui::RichText::new("< Назад").size(11.0))
                    .on_hover_text("Назад")
                    .clicked()
                {
                    self.overlay = Overlay::None;
                }
                ui.label(egui::RichText::new("Логи").strong().size(13.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(egui::RichText::new("⌫").size(11.0))
                        .on_hover_text("Очистить")
                        .clicked()
                    {
                        self.logs_buf.clear();
                    }
                    let mut auto = self.logs_autoscroll;
                    if ui
                        .checkbox(&mut auto, "Автоскролл")
                        .on_hover_text("Автопрокрутка")
                        .changed()
                    {
                        self.logs_autoscroll = auto;
                    }
                    if ui
                        .button(egui::RichText::new("⎘").size(11.0))
                        .on_hover_text("Копировать")
                        .clicked()
                    {
                        let txt = self
                            .logs_buf
                            .iter()
                            .map(|e| format!("{} {}", e.level, e.payload))
                            .collect::<Vec<_>>()
                            .join("\n");
                        ui.ctx().copy_text(txt);
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Уровень");
                egui::ComboBox::from_id_salt("logs_level")
                    .selected_text(self.logs_level.clone())
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        for lv in ["debug", "info", "warning", "error", "silent"] {
                            if ui.selectable_label(self.logs_level == lv, lv).clicked() {
                                self.logs_level = lv.to_owned();
                                self.restart_logs_stream();
                            }
                        }
                    });
                let _ = ui
                    .small_button(egui::RichText::new("?").size(9.0))
                    .on_hover_text("Уровень логов ядра mihomo");
                ui.add(
                    egui::TextEdit::singleline(&mut self.logs_filter)
                        .hint_text("Фильтр...")
                        .desired_width(120.0),
                );
                let mut tab = self.logs_tab;
                egui::ComboBox::from_id_salt("logs_tab")
                    .selected_text(tab.label_ru())
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(tab == LogsTab::Core, "Ядро").clicked() {
                            tab = LogsTab::Core;
                        }
                        if ui
                            .selectable_label(tab == LogsTab::App, "Приложение")
                            .clicked()
                        {
                            tab = LogsTab::App;
                        }
                    });
                self.logs_tab = tab;
            });
            ui.add_space(4.0);
            egui::Frame::new()
                .stroke(egui::Stroke::new(1.0_f32, border))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(self.logs_autoscroll)
                        .max_height(420.0)
                        .show(ui, |ui| {
                            if self.logs_buf.is_empty() {
                                ui.label(egui::RichText::new("Логов пока нет").weak());
                                return;
                            }
                            for e in self.logs_buf.iter().filter(|e| {
                                self.logs_filter.is_empty()
                                    || e.payload
                                        .to_lowercase()
                                        .contains(&self.logs_filter.to_lowercase())
                            }) {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(&e.level).small().monospace().weak(),
                                    );
                                    ui.label(egui::RichText::new(&e.payload).small());
                                });
                            }
                        });
                });
        });
    }
    fn show_settings(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(egui::RichText::new("< Назад").size(11.0))
                    .on_hover_text("Назад")
                    .clicked()
                {
                    self.overlay = Overlay::None;
                }
                ui.label(egui::RichText::new("Настройки").strong().size(13.0));
            });
            ui.separator();
            ui.horizontal(|ui| {
                for (i, label) in [(0, "Приложение"), (1, "Ядро"), (2, "DNS"), (3, "Сеть")].iter()
                {
                    let sel = self.settings_tab == *i;
                    if ui.selectable_label(sel, *label).clicked() {
                        self.settings_tab = *i;
                    }
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| match self.settings_tab {
                0 => self.settings_app(ui),
                1 => self.settings_core(ui),
                2 => self.settings_dns(ui),
                3 => self.settings_network(ui),
                _ => {}
            });
        });
    }
    fn settings_row(
        ui: &mut egui::Ui,
        title: &str,
        help: Option<&str>,
        add: impl FnOnce(&mut egui::Ui),
    ) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(title).strong().size(11.0));
                    if let Some(h) = help {
                        let _ = ui
                            .small_button(egui::RichText::new("?").size(9.0))
                            .on_hover_text(h);
                    }
                });
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                add(ui);
            });
        });
        ui.add_space(4.0);
        ui.separator();
    }
    fn settings_app(&mut self, ui: &mut egui::Ui) {
        let mut t = self.app_config.theme;
        Self::settings_row(
            ui,
            "Тема",
            Some("Светлая / Тёмная / OLED (чёрный)"),
            |ui| {
                egui::ComboBox::from_id_salt("app_theme")
                    .selected_text(t.label_ru())
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        for v in Theme::all() {
                            if ui.selectable_label(t == *v, v.label_ru()).clicked() {
                                t = *v;
                            }
                        }
                    });
            },
        );
        if t != self.app_config.theme {
            self.set_theme(t, ui.ctx());
        }
        let mut v = self.app_config.show_traffic_graph;
        Self::settings_row(
            ui,
            "Показывать график",
            Some("Отключить для экономии ресурсов"),
            |ui| {
                if ui
                    .checkbox(&mut v, "")
                    .on_hover_text("График трафика")
                    .changed()
                {
                    self.app_config.show_traffic_graph = v;
                    let _ = rclash_config::save_app_config(&self.app_config);
                }
            },
        );
        let mut m = self.app_config.minimize_to_tray;
        Self::settings_row(
            ui,
            "Свернуть в трей",
            Some("Скрывать окно вместо закрытия"),
            |ui| {
                if ui.checkbox(&mut m, "").changed() {
                    self.app_config.minimize_to_tray = m;
                    let _ = rclash_config::save_app_config(&self.app_config);
                }
            },
        );
        let mut ll = self.app_config.log_level;
        Self::settings_row(
            ui,
            "Уровень логов",
            Some("debug/info/warning/error/silent ядра"),
            |ui| {
                egui::ComboBox::from_id_salt("app_ll")
                    .selected_text(ll.label_ru())
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        for v in LogLevel::all() {
                            if ui.selectable_label(ll == *v, v.label_ru()).clicked() {
                                ll = *v;
                            }
                        }
                    });
            },
        );
        if ll != self.app_config.log_level {
            self.set_log_level(ll, ui.ctx());
        }
        let mut iv = self.app_config.update_interval;
        Self::settings_row(
            ui,
            "Интервал обновления",
            Some("Берётся из подписки если указан, иначе отсюда"),
            |ui| {
                egui::ComboBox::from_id_salt("app_iv")
                    .selected_text(iv.label_ru())
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        for v in UpdateInterval::all() {
                            if ui.selectable_label(iv == *v, v.label_ru()).clicked() {
                                iv = *v;
                            }
                        }
                    });
            },
        );
        if iv != self.app_config.update_interval {
            self.app_config.update_interval = iv;
            let _ = rclash_config::save_app_config(&self.app_config);
        }
    }
    fn settings_core(&mut self, ui: &mut egui::Ui) {
        let modes = [
            (ProxyMode::Rule, "rule"),
            (ProxyMode::Global, "global"),
            (ProxyMode::Direct, "direct"),
        ];
        let mut cur = self.proxies_mode;
        Self::settings_row(
            ui,
            "Режим по умолчанию",
            Some("Правила — по конфигу; Глобальный — всё через прокси; Прямой — без прокси"),
            |ui| {
                egui::ComboBox::from_id_salt("core_mode")
                    .selected_text(cur.as_str())
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        for (m, l) in modes {
                            if ui.selectable_label(cur == m, l).clicked() {
                                cur = m;
                            }
                        }
                    });
            },
        );
        if cur != self.proxies_mode {
            self.set_mode_async(cur, ui.ctx());
        }
        let mut tun = self.app_config.tun_enabled;
        Self::settings_row(
            ui,
            "TUN",
            Some("TUN-режим требует прав администратора; перехватывает весь трафик"),
            |ui| {
                if ui.checkbox(&mut tun, "").changed() {
                    self.app_config.tun_enabled = tun;
                    let _ = rclash_config::save_app_config(&self.app_config);
                }
            },
        );
        let mut allow = self.app_config.allow_lan.unwrap_or(false);
        Self::settings_row(
            ui,
            "Allow LAN",
            Some("Разрешить подключения из локальной сети"),
            |ui| {
                if ui.checkbox(&mut allow, "").changed() {
                    self.app_config.allow_lan = Some(allow);
                    let _ = rclash_config::save_app_config(&self.app_config);
                }
            },
        );
        let mut ipv6 = self.app_config.ipv6.unwrap_or(false);
        Self::settings_row(
            ui,
            "IPv6",
            Some("Включить IPv6 трафик через прокси"),
            |ui| {
                if ui.checkbox(&mut ipv6, "").changed() {
                    self.app_config.ipv6 = Some(ipv6);
                    let _ = rclash_config::save_app_config(&self.app_config);
                }
            },
        );
        let mut uni = self.app_config.unified_delay.unwrap_or(true);
        Self::settings_row(
            ui,
            "Unified Delay",
            Some("Одна задержка для всех групп, экономит проверки"),
            |ui| {
                if ui.checkbox(&mut uni, "").changed() {
                    self.app_config.unified_delay = Some(uni);
                    let _ = rclash_config::save_app_config(&self.app_config);
                }
            },
        );
        let mut conc = self.app_config.tcp_concurrent.unwrap_or(true);
        Self::settings_row(
            ui,
            "TCP Concurrent",
            Some("Параллельные подключения, быстрее но больше нагрузка"),
            |ui| {
                if ui.checkbox(&mut conc, "").changed() {
                    self.app_config.tcp_concurrent = Some(conc);
                    let _ = rclash_config::save_app_config(&self.app_config);
                }
            },
        );
        let mut mp = self.app_config.mixed_port.unwrap_or(7890).to_string();
        Self::settings_row(
            ui,
            "Mixed Port",
            Some("Порт mixed (HTTP+SOCKS), 0=выкл"),
            |ui| {
                if ui
                    .add(egui::TextEdit::singleline(&mut mp).desired_width(60.0))
                    .changed()
                {
                    if let Ok(v) = mp.parse::<u16>() {
                        self.app_config.mixed_port = Some(v);
                        let _ = rclash_config::save_app_config(&self.app_config);
                    }
                }
            },
        );
        let mut sp = self.app_config.socks_port.unwrap_or(0).to_string();
        Self::settings_row(
            ui,
            "SOCKS Port",
            Some("Отдельный SOCKS порт, 0=выкл"),
            |ui| {
                if ui
                    .add(egui::TextEdit::singleline(&mut sp).desired_width(60.0))
                    .changed()
                {
                    if let Ok(v) = sp.parse::<u16>() {
                        self.app_config.socks_port = Some(v);
                        let _ = rclash_config::save_app_config(&self.app_config);
                    }
                }
            },
        );
        let mut ec = self
            .app_config
            .external_controller
            .clone()
            .unwrap_or_else(|| "127.0.0.1:9090".to_owned());
        Self::settings_row(
            ui,
            "External Controller",
            Some("API для управления ядром"),
            |ui| {
                if ui
                    .add(egui::TextEdit::singleline(&mut ec).desired_width(120.0))
                    .changed()
                {
                    self.app_config.external_controller = Some(ec.clone());
                    let _ = rclash_config::save_app_config(&self.app_config);
                }
            },
        );
        let mut ka = self
            .app_config
            .keep_alive_interval
            .unwrap_or(30)
            .to_string();
        Self::settings_row(
            ui,
            "Keep Alive",
            Some("Интервал keep-alive секунд"),
            |ui| {
                if ui
                    .add(egui::TextEdit::singleline(&mut ka).desired_width(50.0))
                    .changed()
                {
                    if let Ok(v) = ka.parse::<u32>() {
                        self.app_config.keep_alive_interval = Some(v);
                        let _ = rclash_config::save_app_config(&self.app_config);
                    }
                }
            },
        );
        let mut geo = self
            .app_config
            .geodata_loader
            .clone()
            .unwrap_or_else(|| "memconservative".to_owned());
        Self::settings_row(
            ui,
            "Geodata Loader",
            Some("Memory — меньше памяти, Standard — быстрее"),
            |ui| {
                egui::ComboBox::from_id_salt("geo_loader")
                    .selected_text(geo.clone())
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(geo == "memconservative", "Memory")
                            .clicked()
                        {
                            geo = "memconservative".to_owned();
                        }
                        if ui.selectable_label(geo == "standard", "Standard").clicked() {
                            geo = "standard".to_owned();
                        }
                    });
            },
        );
        if geo
            != self
                .app_config
                .geodata_loader
                .clone()
                .unwrap_or_else(|| "memconservative".to_owned())
        {
            self.app_config.geodata_loader = Some(geo.clone());
            let _ = rclash_config::save_app_config(&self.app_config);
        }
    }
    fn settings_dns(&mut self, ui: &mut egui::Ui) {
        Self::settings_row(
            ui,
            "DNS Enable",
            Some("Включить встроенный DNS"),
            |ui| {
                let mut v = true;
                ui.checkbox(&mut v, "").on_hover_text("DNS");
            },
        );
        Self::settings_row(
            ui,
            "Режим",
            Some("FakeIP — виртуальные IP; RedirHost — подмена Host"),
            |ui| {
                egui::ComboBox::from_id_salt("dns_mode")
                    .selected_text("FakeIP")
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        let _ = ui.selectable_label(true, "FakeIP");
                        let _ = ui.selectable_label(false, "RedirHost");
                    });
            },
        );
        Self::settings_row(
            ui,
            "Listen",
            Some("адрес:порт DNS сервера"),
            |ui| {
                let mut s = "0.0.0.0:1053".to_owned();
                ui.add(egui::TextEdit::singleline(&mut s).desired_width(110.0));
            },
        );
        Self::settings_row(
            ui,
            "IPv6",
            Some("Отвечать AAAA записями"),
            |ui| {
                let mut v = false;
                ui.checkbox(&mut v, "");
            },
        );
        Self::settings_row(
            ui,
            "FakeIP Range",
            Some("Диапазон фейк IP"),
            |ui| {
                let mut s = "198.18.0.1/16".to_owned();
                ui.add(egui::TextEdit::singleline(&mut s).desired_width(110.0));
            },
        );
        Self::settings_row(
            ui,
            "Nameserver",
            Some("Основные резолверы"),
            |ui| {
                if ui
                    .button(egui::RichText::new("0 ✎").size(11.0))
                    .on_hover_text("Редактировать список")
                    .clicked()
                {}
            },
        );
        Self::settings_row(
            ui,
            "Fallback",
            Some("Резервные резолверы"),
            |ui| {
                if ui
                    .button(egui::RichText::new("0 ✎").size(11.0))
                    .on_hover_text("Редактировать список")
                    .clicked()
                {}
            },
        );
    }
    fn settings_network(&mut self, ui: &mut egui::Ui) {
        Self::settings_row(
            ui,
            "Bypass Domain",
            Some("Домены мимо прокси"),
            |ui| {
                if ui
                    .button(egui::RichText::new("0 ✎").size(11.0))
                    .on_hover_text("Редактировать список")
                    .clicked()
                {}
            },
        );
        Self::settings_row(
            ui,
            "Append System DNS",
            Some("Добавлять системные DNS"),
            |ui| {
                let mut v = false;
                ui.checkbox(&mut v, "");
            },
        );
        Self::settings_row(
            ui,
            "Hosts",
            Some("Статические хосты"),
            |ui| {
                if ui
                    .button(egui::RichText::new("0 ✎").size(11.0))
                    .on_hover_text("Редактировать")
                    .clicked()
                {}
            },
        );
    }
    fn show_input_popup(&mut self, ctx: &egui::Context) {
        if !self.input_popup_open {
            return;
        }
        let mut open = self.input_popup_open;
        egui::Window::new("Введите текст ниже")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.input_popup_text)
                        .hint_text("https://... или hysteria2://...")
                        .desired_width(360.0)
                        .desired_rows(3),
                );
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("OK").size(11.0)).clicked() {
                        let t = self.input_popup_text.trim().to_owned();
                        if !t.is_empty() {
                            if t.starts_with("http") {
                                let _ =
                                    fetch_and_save_subscription(&t, "manual", UpdateInterval::H24)
                                        .map(|p| {
                                            let mut s = self.profile_store.clone();
                                            s.add_or_replace(p);
                                            let _ = rclash_config::profile::save_profile_store(&s);
                                            self.profile_store = s;
                                            reload_core(ctx);
                                        });
                            } else if let Ok(proxies) = rclash_subscription::parse_text_links(&t) {
                                let _ = rclash_config::custom::add_raw_proxies(proxies).map(|_| {
                                    self.reload_raw_keys();
                                    reload_core(ctx);
                                });
                            }
                        }
                        self.input_popup_text.clear();
                        self.input_popup_open = false;
                    }
                    if ui
                        .button(egui::RichText::new("Отмена").size(11.0))
                        .clicked()
                    {
                        self.input_popup_open = false;
                    }
                });
            });
        self.input_popup_open = open;
    }
    fn show_add_menu(&mut self, ctx: &egui::Context) {
        if !self.show_add_menu_requested {
            return;
        }
        let mut open = self.show_add_menu_requested;
        egui::Window::new("Добавить")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if ui
                    .button(egui::RichText::new("Из буфера").size(11.0))
                    .on_hover_text("Вставить из буфера обмена")
                    .clicked()
                {
                    self.input_popup_open = true;
                    self.show_add_menu_requested = false;
                }
                if ui
                    .button(egui::RichText::new("URL").size(11.0))
                    .on_hover_text("Ввести URL вручную")
                    .clicked()
                {
                    self.input_popup_open = true;
                    self.show_add_menu_requested = false;
                }
                if ui
                    .button(egui::RichText::new("Файл").size(11.0))
                    .on_hover_text("Выбрать файл")
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        if let Ok(s) = std::fs::read_to_string(&path) {
                            let _ = fetch_and_save_subscription(&s, "file", UpdateInterval::H24);
                        }
                    }
                    self.show_add_menu_requested = false;
                }
                if ui
                    .button(egui::RichText::new("Сырой ключ").size(11.0))
                    .on_hover_text("Ввести сырую ссылку")
                    .clicked()
                {
                    self.input_popup_open = true;
                    self.show_add_menu_requested = false;
                }
                if ui
                    .button(egui::RichText::new("× Закрыть").size(11.0))
                    .on_hover_text("Закрыть")
                    .clicked()
                {
                    self.show_add_menu_requested = false;
                }
            });
        self.show_add_menu_requested = open;
        let _ = ctx;
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_owned()
    } else {
        format!("{}…", &s[..n])
    }
}
fn extract_proxy_names(v: &serde_json::Value) -> Vec<String> {
    v.get("proxies")
        .and_then(|p| p.as_object())
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default()
}
fn extract_singles(data: &Option<serde_json::Value>) -> Vec<(String, serde_json::Value)> {
    let Some(v) = data else {
        return Vec::new();
    };
    let Some(proxies) = v.get("proxies").and_then(|p| p.as_object()) else {
        return Vec::new();
    };
    proxies
        .iter()
        .filter(|(_, val)| val.get("all").is_none())
        .map(|(k, val)| (k.clone(), val.clone()))
        .collect()
}
fn extract_groups(data: &Option<serde_json::Value>) -> Vec<(String, serde_json::Value)> {
    let Some(v) = data else {
        return Vec::new();
    };
    let Some(proxies) = v.get("proxies").and_then(|p| p.as_object()) else {
        return Vec::new();
    };
    proxies
        .iter()
        .filter(|(_, val)| val.get("all").is_some())
        .map(|(k, val)| (k.clone(), val.clone()))
        .collect()
}
#[allow(dead_code)]
fn filtered_len(data: &Option<serde_json::Value>) -> usize {
    extract_singles(data).len()
}
#[allow(dead_code)]
fn delay_color(delay: Option<u64>) -> egui::Color32 {
    match delay {
        Some(d) if d < 100 => egui::Color32::from_rgb(80, 180, 80),
        Some(d) if d < 300 => egui::Color32::from_rgb(200, 180, 60),
        Some(_) => egui::Color32::from_rgb(200, 80, 80),
        None => egui::Color32::from_gray(120),
    }
}
fn delay_text(delay: Option<u64>) -> String {
    delay
        .map(|d| format!("{d} ms"))
        .unwrap_or_else(|| "n/a".to_owned())
}
fn format_bytes(b: u64) -> String {
    rclash_core_manager::api::format_bytes(b)
}
fn reload_core(_ctx: &egui::Context) {
    let _ = poll_promise::Promise::spawn_thread("reload_core", move || {
        let client = reqwest::blocking::Client::new();
        let _ = client
            .put("http://127.0.0.1:9090/configs?force=true")
            .send();
    });
}
const SUBS_USER_AGENT: &str = "clash-verge/v2.10.2";
fn fetch_and_save_subscription(
    url: &str,
    name: &str,
    interval: UpdateInterval,
) -> anyhow::Result<rclash_config::profile::Profile> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let resp = client
        .get(url)
        .header("User-Agent", SUBS_USER_AGENT)
        .send()?;
    if !resp.status().is_success() {
        anyhow::bail!("HTTP {}", resp.status());
    }
    let text = resp.text()?;
    let profile = rclash_config::profile::Profile {
        name: name.to_owned(),
        path: format!("{name}.yaml"),
        url: Some(url.to_owned()),
        update_interval: Some(interval),
        last_update: Some(chrono::Utc::now().to_rfc3339()),
        is_raw: false,
    };
    let dir = rclash_config::config_dir().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(&profile.path);
    std::fs::write(&dest, text.as_bytes())?;
    let mut store = rclash_config::profile::load_profile_store();
    store.add_or_replace(profile.clone());
    let _ = rclash_config::profile::save_profile_store(&store);
    Ok(profile)
}
