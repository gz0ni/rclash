use crate::ui;
use poll_promise::Promise;
use rclash_config::{AppConfig, LogLevel, Theme, UpdateInterval};
use rclash_core_manager::api::{ConnectionsSort, LogEntry, Snapshot, TrafficInfo};
use rclash_core_manager::ProxyMode;
use rclash_updater::{manifest_url, UpdateInfo};
use std::collections::{HashMap, VecDeque};
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
}

impl RClashApp {
    pub fn new(cc: &eframe::CreationContext<'_>, tray: Option<crate::tray::TrayHandle>) -> Self {
        let app_config = rclash_config::load_app_config();
        cc.egui_ctx.set_visuals(match app_config.theme {
            Theme::Dark => egui::Visuals::dark(),
            Theme::Light => egui::Visuals::light(),
        });
        let profile_store = rclash_config::profile::load_profile_store();
        let raw_keys = rclash_config::custom::list_raw_keys().unwrap_or_default();
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
        }
    }

    pub fn set_theme(&mut self, theme: Theme, ctx: &egui::Context) {
        self.app_config.theme = theme;
        ctx.set_visuals(match theme {
            Theme::Dark => egui::Visuals::dark(),
            Theme::Light => egui::Visuals::light(),
        });
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
        self.poll_updater(ctx);
        self.check_update_async(ctx, false);

        egui::SidePanel::left("side_panel")
            .resizable(false)
            .default_width(180.0)
            .show(ctx, |ui| {
                ui.heading("RClash");
                ui.separator();
                for tab in Tab::all() {
                    let selected = *tab == self.current_tab;
                    if ui.selectable_label(selected, tab.label()).clicked() {
                        self.current_tab = *tab;
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    let status = if self.core_alive {
                        "● online"
                    } else {
                        "○ offline"
                    };
                    ui.label(egui::RichText::new(status).weak().small());
                    ui.label(
                        egui::RichText::new(
                            self.core_version.as_deref().unwrap_or("v0.1.0-rclash"),
                        )
                        .weak()
                        .small(),
                    );
                    let is_dark = self.app_config.theme == Theme::Dark;
                    if ui
                        .small_button(if is_dark {
                            "☀ Светлая"
                        } else {
                            "🌙 Тёмная"
                        })
                        .clicked()
                    {
                        let next = if is_dark { Theme::Light } else { Theme::Dark };
                        self.set_theme(next, ctx);
                    }
                });
            });

        let mut do_update = false;
        let mut do_later = false;
        let mut do_skip = false;
        let mut update_version: Option<String> = None;
        if let Some(info) = self.updater_available.clone() {
            update_version = Some(info.version.clone());
            egui::Window::new("Обновление доступно")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("Доступно {} — установить?", info.version));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(!self.updater_downloading, egui::Button::new("Обновить"))
                            .clicked()
                        {
                            do_update = true;
                        }
                        if ui.button("Позже").clicked() {
                            do_later = true;
                        }
                        if ui.button("Пропустить версию").clicked() {
                            do_skip = true;
                        }
                    });
                    if self.updater_downloading {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Загрузка…");
                        });
                    }
                    if let Some(e) = &self.updater_error {
                        ui.label(
                            egui::RichText::new(format!("Ошибка: {e}"))
                                .color(egui::Color32::from_rgb(220, 80, 80))
                                .small(),
                        );
                    }
                });
        }
        if do_update {
            self.download_update_async(ctx);
        }
        if do_later {
            self.dismiss_update();
        }
        if do_skip {
            self.skip_update_version();
        }
        if self.updater_error.is_some() && update_version.is_none() {
            egui::Window::new("Ошибка обновления")
                .collapsible(true)
                .show(ctx, |ui| {
                    if let Some(e) = &self.updater_error {
                        ui.label(e);
                    }
                    if ui.button("Закрыть").clicked() {
                        self.updater_error = None;
                    }
                });
        }

        let has_tray = self.tray.is_some();
        egui::CentralPanel::default().show(ctx, |ui| match self.current_tab {
            Tab::Dashboard => ui::dashboard::show(
                ui,
                self.core_alive,
                self.core_version.as_deref(),
                &self.traffic_up_buf,
                &self.traffic_down_buf,
                self.traffic_total_up,
                self.traffic_total_down,
            ),
            Tab::Profiles => ui::profiles::show(ui, self, ctx),
            Tab::Proxies => ui::proxies::show(ui, self, ctx),
            Tab::Connections => ui::connections::show(ui, self, ctx),
            Tab::Logs => ui::logs::show(ui, self, ctx),
            Tab::Settings => ui::settings::show(ui, self, ctx, has_tray),
        });
    }
}
