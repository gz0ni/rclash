use rclash_config::profile::ProfileStore;
use rclash_config::{AppConfig, CoreMode, LogLevel, Theme, UpdateInterval};
use rclash_core_manager::api::log_level_color;

#[derive(Debug)]
enum CoreOpOutcome {
    Started {
        version: String,
        child: std::process::Child,
        warning: Option<String>,
    },
    Proxies {
        items: Vec<ProxyItem>,
        groups: Vec<String>,
        mode: Option<String>,
        now: Option<String>,
    },
    Delays {
        delays: Vec<(String, Option<u64>)>,
    },
    TunApplied {
        enabled: bool,
    },
    TunFailed {
        target: bool,
        message: String,
    },
    GeodataDone,
    Done,
    Failed(String),
}

#[derive(Debug, Clone, Default)]
struct TrafficSample {
    up: u64,
    down: u64,
    up_total: u64,
    down_total: u64,
}

fn secret_for_ops() -> String {
    rclash_config::load_app_config()
        .core_secret
        .unwrap_or_default()
}

fn base_for_ops() -> String {
    rclash_config::api_base(&rclash_config::load_app_config().external_controller)
}

fn sys_proxy_addr() -> String {
    let cfg = rclash_config::load_app_config();
    format!("127.0.0.1:{}", cfg.mixed_port.unwrap_or(7890))
}

fn apply_sys_proxy_state(enabled: bool) {
    use rclash_sys_proxy::ProxyState;
    let proxy = rclash_sys_proxy::current();
    let addr = sys_proxy_addr();
    let res = if enabled {
        proxy.set(ProxyState::Enabled, &addr)
    } else {
        proxy.set(ProxyState::Disabled, &addr)
    };
    match res {
        Ok(()) => log::info!("sys proxy enabled={enabled} {addr}"),
        Err(e) => log::warn!("sys proxy failed: {e}"),
    }
}

fn load_raw_proxies() -> Vec<ProxyItem> {
    let mut items = Vec::new();
    let Ok(conn) = rclash_db::open() else {
        return items;
    };
    let Ok(keys) = rclash_db::list_raw_keys(&conn) else {
        return items;
    };
    for k in keys {
        let proxy_type = k.scheme.unwrap_or_else(|| "unknown".to_owned());
        if !items.iter().any(|p: &ProxyItem| p.name == k.name) {
            items.push(ProxyItem {
                name: k.name,
                proxy_type,
                group: "PROXY".to_owned(),
                delay: None,
            });
        }
    }
    items
}

fn active_db_content(active: &Option<String>) -> Option<(String, String)> {
    let conn = rclash_db::open().ok()?;
    if let Some(name) = active {
        if let Ok(list) = rclash_db::list_configs(&conn) {
            if let Some(c) = list.iter().find(|c| &c.name == name) {
                return Some((c.name.clone(), c.content.clone()));
            }
        }
    }
    None
}

fn raw_keys_yaml_values() -> Vec<serde_yaml::Value> {
    let Ok(conn) = rclash_db::open() else {
        return Vec::new();
    };
    let Ok(keys) = rclash_db::list_raw_keys(&conn) else {
        return Vec::new();
    };
    keys.iter()
        .filter_map(|k| serde_yaml::from_str(&k.parsed_yaml).ok())
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Config,
    Logs,
    Settings,
    Profiles,
    Dashboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Application,
    Core,
    Dns,
    Network,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogSource {
    Core,
    App,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfilesSection {
    Profiles,
    RawKeys,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputContext {
    Clipboard,
    Url,
    Raw,
}

pub struct RClashApp {
    pub app_config: AppConfig,
    tray: Option<crate::tray::TrayHandle>,
    profile_store: ProfileStore,
    active_profile: Option<String>,
    add_menu_open: bool,
    groups: Vec<String>,
    selected_group: String,
    proxies: Vec<ProxyItem>,
    selected_proxy: Option<String>,
    proxy_enabled: bool,
    tun_enabled: bool,
    tun_target: Option<bool>,
    core_enabled: bool,
    active_tab: Tab,
    config_content: String,
    config_interval: UpdateInterval,
    log_level: LogLevel,
    log_source: LogSource,
    log_autoscroll: bool,
    settings_tab: SettingsTab,
    profiles_section: ProfilesSection,
    input_open: bool,
    input_ctx: InputContext,
    input_text: String,
    input_error: String,
    sub_fetch: Option<poll_promise::Promise<Result<(String, String), String>>>,
    sub_url: Option<String>,
    settings_error: Option<String>,
    core_mixed_port: String,
    core_socks_port: String,
    core_external_controller: String,
    core_keep_alive: String,
    core_geodata_loader: String,
    dns_listen: String,
    dns_fakeip_range: String,
    dns_nameservers_csv: String,
    dns_fallback_csv: String,
    hosts_text: String,
    core_child: Option<std::process::Child>,
    core_version: Option<String>,
    core_error: Option<String>,
    core_warning: Option<String>,
    core_op: Option<poll_promise::Promise<CoreOpOutcome>>,
    last_reconcile: std::time::Instant,
    traffic_rx: Option<std::sync::mpsc::Receiver<TrafficSample>>,
    traffic_stop: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    traffic_up: u64,
    traffic_down: u64,
    traffic_max: u64,
    traffic_up_total: u64,
    traffic_down_total: u64,
}

#[derive(Clone, Debug)]
struct ProxyItem {
    name: String,
    proxy_type: String,
    group: String,
    delay: Option<u64>,
}

const R6: u8 = 6;
const R4: u8 = 4;

const NEKO_ACCENT: egui::Color32 = egui::Color32::from_rgb(0x3B, 0x82, 0xF6);
#[allow(dead_code)]
const NEKO_ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(0x4A, 0x90, 0xFF);
#[allow(dead_code)]
const NEKO_GREEN: egui::Color32 = egui::Color32::from_rgb(0x27, 0xAE, 0x60);
#[allow(dead_code)]
const NEKO_RED: egui::Color32 = egui::Color32::from_rgb(0xE7, 0x4C, 0x3C);
const NEKO_DELAY_FAST: egui::Color32 = egui::Color32::from_rgb(0x50, 0xB4, 0x78);
const NEKO_DELAY_MID: egui::Color32 = egui::Color32::from_rgb(0xC8, 0xB4, 0x5A);
const NEKO_DELAY_SLOW: egui::Color32 = egui::Color32::from_rgb(0xE0, 0x60, 0x60);

fn country_code(name: &str) -> &'static str {
    let l = name.to_lowercase();
    if l.contains("germany") || l.contains("de-") || l.contains("germani") {
        "DE"
    } else if l.contains("poland") || l.contains("pl-") {
        "PL"
    } else if l.contains("singapore") {
        "SG"
    } else if l.contains("netherlands") || l.contains("nl-") {
        "NL"
    } else {
        "•"
    }
}

fn delay_color_for(delay: Option<u64>, theme: Theme) -> egui::Color32 {
    match delay {
        None => NEKO_RED,
        Some(d) => match theme {
            Theme::Light => {
                if d < 100 {
                    egui::Color32::from_rgb(0x0A, 0x7A, 0x3A)
                } else if d < 300 {
                    egui::Color32::from_rgb(0x8A, 0x6D, 0x00)
                } else {
                    egui::Color32::from_rgb(0xC0, 0x39, 0x2B)
                }
            }
            Theme::Dark => {
                if d < 100 {
                    NEKO_DELAY_FAST
                } else if d < 300 {
                    NEKO_DELAY_MID
                } else {
                    NEKO_DELAY_SLOW
                }
            }
        },
    }
}

fn delay_text(delay: Option<u64>) -> String {
    match delay {
        None => "n/a".to_owned(),
        Some(d) => format!("{d} ms"),
    }
}

fn log_severity(level: &str) -> u8 {
    match level.to_ascii_lowercase().as_str() {
        "error" => 3,
        "warning" | "warn" => 2,
        "info" => 1,
        _ => 0,
    }
}

fn log_threshold(level: LogLevel) -> u8 {
    match level {
        LogLevel::Silent => 4,
        LogLevel::Error => 3,
        LogLevel::Warning => 2,
        LogLevel::Info => 1,
        LogLevel::Debug => 0,
    }
}

fn log_source_matches(source: LogSource, target: &str) -> bool {
    match source {
        LogSource::All => true,
        LogSource::Core => target.to_ascii_lowercase().contains("core"),
        LogSource::App => !target.to_ascii_lowercase().contains("core"),
    }
}

fn log_entry_visible(source: LogSource, threshold: u8, entry: &crate::logger::AppLogEntry) -> bool {
    if !log_source_matches(source, &entry.target) {
        return false;
    }
    log_severity(&entry.level) >= threshold
}

fn group_of(name: &str, groups_all: &std::collections::HashMap<String, Vec<String>>) -> String {
    for (g, members) in groups_all {
        if members.iter().any(|m| m == name) {
            return g.clone();
        }
    }
    "PROXY".to_owned()
}

fn parse_proxies_value(v: &serde_json::Value) -> (Vec<ProxyItem>, Vec<String>, Option<String>) {
    let mut items = Vec::new();
    let mut groups = Vec::new();
    let mut now: Option<String> = None;
    let mut groups_all: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let Some(map) = v.get("proxies").and_then(|p| p.as_object()) else {
        return (items, groups, now);
    };
    for (name, node) in map {
        if name == "GLOBAL" {
            continue;
        }
        if let Some(all) = node.get("all").and_then(|a| a.as_array()) {
            let members: Vec<String> = all
                .iter()
                .filter_map(|m| m.as_str().map(str::to_owned))
                .collect();
            groups_all.insert(name.clone(), members);
            if name == "PROXY" {
                now = node.get("now").and_then(|n| n.as_str()).map(str::to_owned);
            }
            if !groups.contains(name) {
                groups.push(name.clone());
            }
        }
    }
    if !groups.contains(&"PROXY".to_owned()) {
        groups.insert(0, "PROXY".to_owned());
    }
    for (name, node) in map {
        if name == "GLOBAL" || groups_all.contains_key(name) {
            continue;
        }
        let node_type = node
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_owned();
        match node_type.as_str() {
            "" | "Direct" | "Reject" | "Compatible" | "Selector" | "URLTest" | "LoadBalance"
            | "Relay" | "Fallback" => continue,
            _ => {}
        }
        let delay = node
            .get("history")
            .and_then(|h| h.as_array())
            .and_then(|h| h.last())
            .and_then(|e| e.get("delay"))
            .and_then(|d| d.as_u64())
            .filter(|d| *d > 0);
        items.push(ProxyItem {
            name: name.clone(),
            proxy_type: node_type.to_lowercase(),
            group: group_of(name, &groups_all),
            delay,
        });
    }
    (items, groups, now)
}

fn extract_link_tokens(text: &str) -> Vec<String> {
    let mut owned = text.to_owned();
    if !owned.contains("://") {
        if let Ok(decoded) = rclash_subscription::decode_base64_url_safe(text) {
            if let Ok(s) = String::from_utf8(decoded) {
                owned = s;
            }
        }
    }
    let mut toks = Vec::new();
    for line in owned.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        for part in line.split_whitespace() {
            if part.contains("://") {
                toks.push(part.to_owned());
            }
        }
    }
    toks
}

fn proxy_item_from_value(v: &serde_yaml::Value) -> Option<ProxyItem> {
    let m = v.as_mapping()?;
    let name = m
        .get(serde_yaml::Value::String("name".into()))?
        .as_str()?
        .to_owned();
    let proxy_type = m
        .get(serde_yaml::Value::String("type".into()))?
        .as_str()?
        .to_owned();
    Some(ProxyItem {
        name,
        proxy_type,
        group: "PROXY".to_owned(),
        delay: None,
    })
}

fn profile_name_from_url(url: &str, fallback: &str) -> String {
    let name = url
        .split('?')
        .next()
        .unwrap_or("")
        .rsplit('/')
        .next()
        .unwrap_or("")
        .trim();
    let name = if name.is_empty() { fallback } else { name };
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe = safe.trim_matches('.').trim();
    if safe.is_empty() {
        fallback.to_owned()
    } else {
        safe.to_owned()
    }
}

fn fetch_subscription(url: String, name: String) -> Result<(String, String), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("clash-verge/v2.10.2")
        .build()
        .map_err(|e| format!("client: {e}"))?;
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = resp.text().map_err(|e| format!("body: {e}"))?;
    Ok((name, text))
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Inter".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Inter.ttf")).into(),
    );
    fonts.font_data.insert(
        "JetBrainsMono".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/JetBrainsMono.ttf")).into(),
    );
    fonts.font_data.insert(
        "NotoSansSymbols2".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../assets/fonts/NotoSansSymbols2-Regular.ttf"
        ))
        .into(),
    );
    fonts.families.insert(
        egui::FontFamily::Proportional,
        vec!["Inter".to_owned(), "NotoSansSymbols2".to_owned()],
    );
    fonts.families.insert(
        egui::FontFamily::Monospace,
        vec!["JetBrainsMono".to_owned()],
    );
    ctx.set_fonts(fonts);
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(11.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(11.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::new(10.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(10.0, egui::FontFamily::Monospace),
    );
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.interact_size.y = 22.0;
    ctx.set_style(style);
}

fn card_fill_for(theme: Theme) -> egui::Color32 {
    match theme {
        Theme::Light => egui::Color32::WHITE,
        Theme::Dark => egui::Color32::from_rgb(0x2D, 0x2F, 0x33),
    }
}
fn panel_fill_for(theme: Theme) -> egui::Color32 {
    match theme {
        Theme::Light => egui::Color32::from_rgb(0xF0, 0xF2, 0xF5),
        Theme::Dark => egui::Color32::from_rgb(0x1E, 0x1E, 0x1E),
    }
}

fn theme_visuals(theme: Theme) -> egui::Visuals {
    let mut v = match theme {
        Theme::Light => {
            let mut x = egui::Visuals::light();
            x.panel_fill = panel_fill_for(theme);
            x.window_fill = card_fill_for(theme);
            x.extreme_bg_color = egui::Color32::from_rgb(0xE8, 0xEA, 0xED);
            x.faint_bg_color = panel_fill_for(theme);
            x.code_bg_color = card_fill_for(theme);
            x
        }
        Theme::Dark => {
            let mut x = egui::Visuals::dark();
            x.panel_fill = panel_fill_for(theme);
            x.window_fill = card_fill_for(theme);
            x.extreme_bg_color = egui::Color32::from_rgb(0x2A, 0x2C, 0x2F);
            x.faint_bg_color = panel_fill_for(theme);
            x.code_bg_color = card_fill_for(theme);
            x
        }
    };
    let border = border_color_for(theme);
    let card = card_fill_for(theme);
    let text = match theme {
        Theme::Light => egui::Color32::BLACK,
        Theme::Dark => egui::Color32::WHITE,
    };
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = egui::CornerRadius::same(R6);
        w.bg_stroke = egui::Stroke::new(1.0_f32, border);
        w.fg_stroke = egui::Stroke::new(1.0_f32, text);
        w.bg_fill = card;
    }
    v.widgets.hovered.bg_fill = match theme {
        Theme::Light => egui::Color32::from_rgb(0xE8, 0xEA, 0xED),
        Theme::Dark => egui::Color32::from_rgb(0x3A, 0x3D, 0x45),
    };
    v.selection.bg_fill = egui::Color32::from_rgb(0x3B, 0x82, 0xF6);
    v.selection.stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x4A, 0x90, 0xFF));
    v.override_text_color = Some(text);
    v.weak_text_color = Some(match theme {
        Theme::Light => egui::Color32::from_rgb(0x6B, 0x72, 0x80),
        Theme::Dark => egui::Color32::from_rgb(0x9A, 0xA0, 0xA6),
    });
    v
}

pub fn border_color_for(theme: Theme) -> egui::Color32 {
    match theme {
        Theme::Light => egui::Color32::from_rgb(0xD0, 0xD3, 0xD8),
        Theme::Dark => egui::Color32::from_rgb(0x3C, 0x3F, 0x41),
    }
}

fn settings_row(
    ui: &mut egui::Ui,
    border: egui::Color32,
    label: &str,
    desc: Option<&str>,
    add_control: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
        ui.label(egui::RichText::new(label).size(11.0));
        if let Some(d) = desc {
            ui.label(egui::RichText::new("?").size(10.0).weak())
                .on_hover_text(d);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(12.0);
            add_control(ui);
        });
    });
    if let Some(d) = desc {
        ui.label(egui::RichText::new(d).size(10.0).weak());
    }
    ui.add_space(6.0);
    let sep_w = ui.available_width();
    let (sep_rect, _) = ui.allocate_exact_size(egui::vec2(sep_w, 1.0), egui::Sense::hover());
    ui.painter().hline(
        sep_rect.x_range(),
        sep_rect.center().y,
        egui::Stroke::new(1.0_f32, border),
    );
    ui.add_space(6.0);
}

impl RClashApp {
    fn render_settings_screen(&mut self, ui: &mut egui::Ui) {
        let theme = self.app_config.theme;
        let card_fill = card_fill_for(theme);
        let border = border_color_for(theme);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
            if ui
                .add_sized(
                    [64.0, 22.0],
                    egui::Button::new(egui::RichText::new("< Назад").size(11.0)),
                )
                .clicked()
            {
                self.active_tab = Tab::Dashboard;
            }
            ui.label(egui::RichText::new("Настройки").size(11.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
                for (tab, label) in [
                    (SettingsTab::Network, "сеть"),
                    (SettingsTab::Dns, "dns"),
                    (SettingsTab::Core, "ядро"),
                    (SettingsTab::Application, "приложение"),
                ] {
                    let active = self.settings_tab == tab;
                    let fill = if active { NEKO_ACCENT } else { card_fill };
                    let txt_color = if active {
                        egui::Color32::WHITE
                    } else {
                        match theme {
                            Theme::Light => egui::Color32::BLACK,
                            Theme::Dark => egui::Color32::WHITE,
                        }
                    };
                    let btn =
                        egui::Button::new(egui::RichText::new(label).size(11.0).color(txt_color))
                            .fill(fill)
                            .stroke(egui::Stroke::new(
                                1.0_f32,
                                if active { NEKO_ACCENT } else { border },
                            ))
                            .corner_radius(egui::CornerRadius::same(R6));
                    if ui.add(btn).clicked() {
                        self.settings_tab = tab;
                    }
                }
            });
        });
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match self.settings_tab {
                SettingsTab::Application => self.render_settings_application(ui),
                SettingsTab::Core => self.render_settings_core(ui),
                SettingsTab::Dns => self.render_settings_dns(ui),
                SettingsTab::Network => self.render_settings_network(ui),
            });
    }

    fn render_settings_application(&mut self, ui: &mut egui::Ui) {
        let theme = self.app_config.theme;
        let card_fill = card_fill_for(theme);
        let border = border_color_for(theme);
        settings_row(ui, border, "Тема", None, |ui| {
            for th in [Theme::Dark, Theme::Light] {
                let active = theme == th;
                let fill = if active { NEKO_ACCENT } else { card_fill };
                let txt_color = if active {
                    egui::Color32::WHITE
                } else {
                    match theme {
                        Theme::Light => egui::Color32::BLACK,
                        Theme::Dark => egui::Color32::WHITE,
                    }
                };
                let btn = egui::Button::new(
                    egui::RichText::new(th.label_ru())
                        .size(11.0)
                        .color(txt_color),
                )
                .fill(fill)
                .stroke(egui::Stroke::new(
                    1.0_f32,
                    if active { NEKO_ACCENT } else { border },
                ))
                .corner_radius(egui::CornerRadius::same(R6));
                if ui.add(btn).clicked() {
                    let ctx = ui.ctx().clone();
                    self.set_theme(th, &ctx);
                }
            }
        });
        settings_row(ui, border, "Показывать график", None, |ui| {
            if ui
                .checkbox(&mut self.app_config.show_traffic_graph, "")
                .changed()
            {
                let _ = rclash_config::save_app_config(&self.app_config);
                log::info!("show_traffic_graph {}", self.app_config.show_traffic_graph);
            }
        });
        settings_row(ui, border, "Свернуть в трей", None, |ui| {
            if ui
                .checkbox(&mut self.app_config.minimize_to_tray, "")
                .changed()
            {
                let _ = rclash_config::save_app_config(&self.app_config);
                log::info!("minimize_to_tray {}", self.app_config.minimize_to_tray);
            }
        });
        settings_row(ui, border, "Уровень логов", None, |ui| {
            let before = self.app_config.log_level;
            egui::ComboBox::from_id_salt("settings_log_level")
                .selected_text(egui::RichText::new(before.as_str()).size(11.0))
                .width(90.0)
                .show_ui(ui, |ui| {
                    for lv in [
                        LogLevel::Debug,
                        LogLevel::Info,
                        LogLevel::Warning,
                        LogLevel::Error,
                        LogLevel::Silent,
                    ] {
                        ui.selectable_value(
                            &mut self.app_config.log_level,
                            lv,
                            egui::RichText::new(lv.as_str()).size(11.0),
                        );
                    }
                });
            if self.app_config.log_level != before {
                crate::logger::set_level(self.app_config.log_level.to_log_filter());
                let _ = rclash_config::save_app_config(&self.app_config);
            }
        });
    }

    fn render_settings_core(&mut self, ui: &mut egui::Ui) {
        let border = border_color_for(self.app_config.theme);
        settings_row(
            ui,
            border,
            "Режим по умолчанию",
            None,
            |ui| {
                let before = self.app_config.mode;
                egui::ComboBox::from_id_salt("core_default_mode")
                    .selected_text(egui::RichText::new(before.as_str()).size(11.0))
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for m in CoreMode::all() {
                            ui.selectable_value(
                                &mut self.app_config.mode,
                                *m,
                                egui::RichText::new(m.as_str()).size(11.0),
                            );
                        }
                    });
                if self.app_config.mode != before {
                    let mode = self.app_config.mode;
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("default mode {}", mode.as_str());
                    if self.core_child.is_some() && self.core_op.is_none() {
                        let base = base_for_ops();
                        let secret = secret_for_ops();
                        let m = mode.as_str().to_owned();
                        self.core_op = Some(poll_promise::Promise::spawn_thread(
                            "core-mode",
                            move || match rclash_core_manager::supervisor::set_mode_blocking(
                                &base, &secret, &m,
                            ) {
                                Ok(()) => CoreOpOutcome::Done,
                                Err(e) => CoreOpOutcome::Failed(format!("mode: {e}")),
                            },
                        ));
                    }
                }
            },
        );
        settings_row(
            ui,
            border,
            "TUN",
            Some("Перехватывает весь трафик, требует прав администратора. Без него работает только системный прокси."),
            |ui| {
                let mut v = self.tun_enabled;
                if ui.checkbox(&mut v, "").changed() {
                    self.set_tun_enabled(v);
                }
            },
        );
        settings_row(
            ui,
            border,
            "Allow LAN",
            Some("Разрешить подключения из локальной сети (другие устройства смогут использовать прокси)."),
            |ui| {
                let mut v = self.app_config.allow_lan.unwrap_or(false);
                if ui.checkbox(&mut v, "").changed() {
                    self.app_config.allow_lan = Some(v);
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("allow_lan {v}");
                    self.patch_core_live(serde_json::json!({"allow-lan": v}), "allow-lan");
                }
            },
        );
        settings_row(
            ui,
            border,
            "IPv6 трафик",
            Some("Пропускать IPv6 через прокси (выкл — только IPv4)."),
            |ui| {
                let mut v = self.app_config.ipv6.unwrap_or(false);
                if ui.checkbox(&mut v, "").changed() {
                    self.app_config.ipv6 = Some(v);
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("ipv6 {v}");
                    self.patch_core_live(serde_json::json!({"ipv6": v}), "ipv6");
                }
            },
        );
        settings_row(
            ui,
            border,
            "Unified Delay",
            Some("Одна проверка задержки на все группы — экономит время, но менее точно. Смена перезапускает ядро."),
            |ui| {
                let mut v = self.app_config.unified_delay.unwrap_or(true);
                if ui.checkbox(&mut v, "").changed() {
                    self.app_config.unified_delay = Some(v);
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("unified_delay {v}");
                    self.restart_core_if_running();
                }
            },
        );
        settings_row(
            ui,
            border,
            "TCP Concurrent",
            Some("Параллельные подключения к прокси, быстрее но больше нагрузка."),
            |ui| {
                let mut v = self.app_config.tcp_concurrent.unwrap_or(true);
                if ui.checkbox(&mut v, "").changed() {
                    self.app_config.tcp_concurrent = Some(v);
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("tcp_concurrent {v}");
                    self.patch_core_live(
                        serde_json::json!({"tcp-concurrent": v}),
                        "tcp-concurrent",
                    );
                }
            },
        );
        settings_row(
            ui,
            border,
            "Mixed Port",
            Some("Единый порт для HTTP и SOCKS (0 — выкл). Обычно 7890. Смена перезапускает ядро."),
            |ui| {
                let resp = ui.add_sized(
                    [80.0, 22.0],
                    egui::TextEdit::singleline(&mut self.core_mixed_port),
                );
                if resp.changed() {
                    match rclash_config::validate_port_text(&self.core_mixed_port.clone()) {
                        Some(v) => {
                            self.settings_error = None;
                            self.app_config.mixed_port = Some(v);
                            let _ = rclash_config::save_app_config(&self.app_config);
                            log::info!("mixed_port {v}");
                            if self.proxy_enabled && self.core_child.is_some() {
                                apply_sys_proxy_state(true);
                            }
                            self.restart_core_if_running();
                        }
                        None => {
                            self.settings_error = Some("Mixed Port: число 0–65535".to_owned());
                        }
                    }
                }
            },
        );
        settings_row(
            ui,
            border,
            "SOCKS Port",
            Some("Отдельный SOCKS-порт (0 — выкл, используется Mixed). Смена перезапускает ядро."),
            |ui| {
                let resp = ui.add_sized(
                    [80.0, 22.0],
                    egui::TextEdit::singleline(&mut self.core_socks_port),
                );
                if resp.changed() {
                    match rclash_config::validate_port_text(&self.core_socks_port.clone()) {
                        Some(v) => {
                            self.settings_error = None;
                            self.app_config.socks_port = Some(v);
                            let _ = rclash_config::save_app_config(&self.app_config);
                            log::info!("socks_port {v}");
                            self.restart_core_if_running();
                        }
                        None => {
                            self.settings_error = Some("SOCKS Port: число 0–65535".to_owned());
                        }
                    }
                }
            },
        );
        settings_row(
            ui,
            border,
            "External Controller",
            Some("API ядра для управления (обычно 127.0.0.1:9090). Смена перезапускает ядро."),
            |ui| {
                let resp = ui.add_sized(
                    [120.0, 22.0],
                    egui::TextEdit::singleline(&mut self.core_external_controller),
                );
                if resp.changed() && !self.core_external_controller.trim().is_empty() {
                    let v = self.core_external_controller.trim().to_owned();
                    if rclash_config::validate_listen(&v) {
                        self.settings_error = None;
                        self.app_config.external_controller = Some(v.clone());
                        let _ = rclash_config::save_app_config(&self.app_config);
                        log::info!("external_controller {v}");
                        self.restart_core_if_running();
                    } else {
                        self.settings_error =
                            Some("External Controller: формат host:port".to_owned());
                    }
                }
            },
        );
        settings_row(
            ui,
            border,
            "Keep Alive",
            Some("Интервал keep-alive в секундах для TCP. Смена перезапускает ядро."),
            |ui| {
                let resp = ui.add_sized(
                    [70.0, 22.0],
                    egui::TextEdit::singleline(&mut self.core_keep_alive),
                );
                if resp.changed() {
                    if let Ok(v) = self.core_keep_alive.trim().parse::<u32>() {
                        self.settings_error = None;
                        self.app_config.keep_alive_interval = Some(v);
                        let _ = rclash_config::save_app_config(&self.app_config);
                        log::info!("keep_alive {v}");
                        self.restart_core_if_running();
                    } else {
                        self.settings_error = Some("Keep Alive: число секунд".to_owned());
                    }
                }
            },
        );
        settings_row(
            ui,
            border,
            "Geodata Loader",
            Some("Как грузить GeoIP/Geosite: Memory — меньше RAM, Standard — быстрее. Смена перезапускает ядро."),
            |ui| {
                let before = self.core_geodata_loader.clone();
                egui::ComboBox::from_id_salt("core_geodata_loader")
                    .selected_text(egui::RichText::new(&before).size(11.0))
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for m in ["Memory", "Standard"] {
                            ui.selectable_value(
                                &mut self.core_geodata_loader,
                                m.to_owned(),
                                egui::RichText::new(m).size(11.0),
                            );
                        }
                    });
                if self.core_geodata_loader != before {
                    self.app_config.geodata_loader = Some(self.core_geodata_loader.clone());
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("geodata_loader {}", self.core_geodata_loader);
                    self.restart_core_if_running();
                }
            },
        );
        settings_row(
            ui,
            border,
            "Строгий режим геодаты",
            Some("Выкл — правила со списками, которых нет в GeoSite/GeoIP, пропускаются с варнингом. Вкл — ядро не стартует с ошибкой."),
            |ui| {
                let mut v = self.app_config.geodata_strict;
                if ui.checkbox(&mut v, "").changed() {
                    self.app_config.geodata_strict = v;
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("geodata_strict {v}");
                }
            },
        );
        settings_row(
            ui,
            border,
            "Обновить геодату",
            Some("Перекачать GeoSite.dat и GeoIP.dat с зеркала и перезапустить ядро."),
            |ui| {
                if ui
                    .add_sized(
                        [110.0, 22.0],
                        egui::Button::new(egui::RichText::new("Обновить").size(11.0)),
                    )
                    .clicked()
                {
                    self.request_geodata_refresh();
                }
            },
        );
    }

    fn render_settings_dns(&mut self, ui: &mut egui::Ui) {
        let border = border_color_for(self.app_config.theme);
        settings_row(
            ui,
            border,
            "DNS Enable",
            Some("Включать встроенный DNS ядра. Смена применяется к ядру сразу."),
            |ui| {
                let mut v = self.app_config.dns.enable;
                if ui.checkbox(&mut v, "").changed() {
                    self.app_config.dns.enable = v;
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("dns_enable {v}");
                    self.rebuild_and_reload("dns");
                }
            },
        );
        settings_row(
            ui,
            border,
            "Режим",
            Some("FakeIP — выдаёт виртуальные IP и резолвит по запросу; RedirHost — подмена Host."),
            |ui| {
                let before = self.app_config.dns.ui_mode().to_owned();
                egui::ComboBox::from_id_salt("dns_mode")
                    .selected_text(egui::RichText::new(&before).size(11.0))
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for m in ["FakeIP", "RedirHost"] {
                            let mut selected = before == m;
                            if ui
                                .selectable_label(selected, egui::RichText::new(m).size(11.0))
                                .clicked()
                            {
                                selected = true;
                            }
                            if selected && before != m {
                                self.app_config.dns.enhanced_mode = if m == "RedirHost" {
                                    "redir-host".to_owned()
                                } else {
                                    "fake-ip".to_owned()
                                };
                            }
                        }
                    });
                if self.app_config.dns.ui_mode() != before {
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("dns_mode {}", self.app_config.dns.enhanced_mode);
                    self.rebuild_and_reload("dns");
                }
            },
        );
        settings_row(
            ui,
            border,
            "Listen",
            Some("Адрес:порт DNS сервера, например 0.0.0.0:1053."),
            |ui| {
                let resp = ui.add_sized(
                    [120.0, 22.0],
                    egui::TextEdit::singleline(&mut self.dns_listen),
                );
                if resp.changed() {
                    let v = self.dns_listen.clone();
                    if rclash_config::validate_listen(&v) {
                        self.settings_error = None;
                        self.app_config.dns.listen = v.trim().to_owned();
                        let _ = rclash_config::save_app_config(&self.app_config);
                        log::info!("dns_listen {}", self.app_config.dns.listen);
                        self.rebuild_and_reload("dns");
                    } else {
                        self.settings_error = Some("DNS Listen: формат host:port".to_owned());
                    }
                }
            },
        );
        settings_row(
            ui,
            border,
            "DNS IPv6 (AAAA)",
            Some("Отвечать AAAA записями (разрешение IPv6)."),
            |ui| {
                let mut v = self.app_config.dns.ipv6;
                if ui.checkbox(&mut v, "").changed() {
                    self.app_config.dns.ipv6 = v;
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("dns_ipv6 {v}");
                    self.rebuild_and_reload("dns");
                }
            },
        );
        settings_row(
            ui,
            border,
            "FakeIP Range",
            Some("Диапазон фейк-IP, например 198.18.0.1/16."),
            |ui| {
                let resp = ui.add_sized(
                    [120.0, 22.0],
                    egui::TextEdit::singleline(&mut self.dns_fakeip_range),
                );
                if resp.changed() {
                    let v = self.dns_fakeip_range.clone();
                    if rclash_config::validate_cidr(&v) {
                        self.settings_error = None;
                        self.app_config.dns.fake_ip_range = v.trim().to_owned();
                        let _ = rclash_config::save_app_config(&self.app_config);
                        log::info!("dns_fakeip {}", self.app_config.dns.fake_ip_range);
                        self.rebuild_and_reload("dns");
                    } else {
                        self.settings_error = Some("FakeIP Range: формат ip/маска".to_owned());
                    }
                }
            },
        );
        settings_row(
            ui,
            border,
            "Nameserver",
            Some("Основные резолверы через запятую, например 223.5.5.5, 8.8.8.8."),
            |ui| {
                let resp = ui.add_sized(
                    [180.0, 22.0],
                    egui::TextEdit::singleline(&mut self.dns_nameservers_csv),
                );
                if resp.changed() {
                    let list =
                        rclash_config::parse_nameserver_list(&self.dns_nameservers_csv.clone());
                    if list.is_empty() {
                        self.settings_error =
                            Some("Nameserver: нужен хотя бы один адрес".to_owned());
                    } else {
                        self.settings_error = None;
                        self.app_config.dns.nameservers = list;
                        let _ = rclash_config::save_app_config(&self.app_config);
                        log::info!(
                            "dns_nameservers {}",
                            self.app_config.dns.nameservers.join(",")
                        );
                        self.rebuild_and_reload("dns");
                    }
                }
            },
        );
        settings_row(
            ui,
            border,
            "Fallback",
            Some("Резервные резолверы через запятую, если основные не ответили. Пусто — без fallback."),
            |ui| {
                let resp = ui.add_sized(
                    [180.0, 22.0],
                    egui::TextEdit::singleline(&mut self.dns_fallback_csv),
                );
                if resp.changed() {
                    let list =
                        rclash_config::parse_nameserver_list(&self.dns_fallback_csv.clone());
                    self.settings_error = None;
                    self.app_config.dns.fallback = list;
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("dns_fallback {}", self.app_config.dns.fallback.join(","));
                    self.rebuild_and_reload("dns");
                }
            },
        );
    }

    fn render_settings_network(&mut self, ui: &mut egui::Ui) {
        let border = border_color_for(self.app_config.theme);
        settings_row(
            ui,
            border,
            "Hosts",
            Some("Статические записи hosts — по одной на строку: домен = IP. Применяется к ядру сразу."),
            |ui| {
                let resp = ui.add_sized(
                    [220.0, 66.0],
                    egui::TextEdit::multiline(&mut self.hosts_text)
                        .hint_text("example.com = 1.2.3.4"),
                );
                if resp.changed() {
                    let hosts =
                        rclash_config::parse_hosts_text(&self.hosts_text.clone());
                    self.app_config.hosts = hosts;
                    let _ = rclash_config::save_app_config(&self.app_config);
                    log::info!("hosts {} entries", self.app_config.hosts.len());
                    self.rebuild_and_reload("hosts");
                }
            },
        );
    }
}

impl RClashApp {
    pub fn new(cc: &eframe::CreationContext<'_>, tray: Option<crate::tray::TrayHandle>) -> Self {
        let mut app_config = rclash_config::load_app_config();
        cc.egui_ctx.set_visuals(theme_visuals(app_config.theme));
        setup_fonts(&cc.egui_ctx);
        if let Ok(conn) = rclash_db::open() {
            if rclash_db::migration_needed(&conn).unwrap_or(false) {
                match rclash_db::migrate_from_files(&conn) {
                    Ok(n) => log::info!("migrated {n} legacy entries to db"),
                    Err(e) => log::warn!("db migration failed: {e}"),
                }
            }
        }
        if let Err(e) = rclash_config::ensure_core_secret(&mut app_config) {
            log::warn!("core secret init failed: {e}");
        }
        let profile_store = rclash_config::profile::load_profile_store();
        let active_profile = profile_store.active.clone();
        let proxies = load_raw_proxies();
        let mut groups = vec!["PROXY".to_owned()];
        for p in &proxies {
            if p.group != "PROXY" && !groups.contains(&p.group) {
                groups.push(p.group.clone());
            }
        }
        let config_content = active_db_content(&active_profile)
            .map(|(_, c)| c)
            .unwrap_or_else(|| "# Нет активного профиля — добавь профиль через +\n".to_owned());
        let proxy_enabled = app_config.proxy_enabled;
        let tun_enabled = app_config.tun_enabled;
        let core_mixed_port = app_config
            .mixed_port
            .map(|p| p.to_string())
            .unwrap_or_else(|| "7890".to_owned());
        let core_socks_port = app_config
            .socks_port
            .map(|p| p.to_string())
            .unwrap_or_else(|| "0".to_owned());
        let core_external_controller = app_config
            .external_controller
            .clone()
            .unwrap_or_else(|| "127.0.0.1:9090".to_owned());
        let core_keep_alive = app_config
            .keep_alive_interval
            .map(|v| v.to_string())
            .unwrap_or_else(|| "30".to_owned());
        let core_geodata_loader = app_config
            .geodata_loader
            .clone()
            .unwrap_or_else(|| "Memory".to_owned());
        let dns_listen = app_config.dns.listen.clone();
        let dns_fakeip_range = app_config.dns.fake_ip_range.clone();
        let dns_nameservers_csv = app_config.dns.nameservers.join(", ");
        let dns_fallback_csv = app_config.dns.fallback.join(", ");
        let hosts_text = rclash_config::hosts_to_text(&app_config.hosts);
        let mut app = Self {
            app_config,
            tray,
            profile_store,
            active_profile,
            add_menu_open: false,
            groups,
            selected_group: "Все группы".to_owned(),
            proxies,
            selected_proxy: None,
            proxy_enabled,
            tun_enabled,
            tun_target: None,
            core_enabled: false,
            active_tab: Tab::Dashboard,
            config_content,
            config_interval: UpdateInterval::Auto,
            log_level: LogLevel::Info,
            log_source: LogSource::All,
            log_autoscroll: true,
            settings_tab: SettingsTab::Application,
            profiles_section: ProfilesSection::Profiles,
            input_open: false,
            input_ctx: InputContext::Url,
            input_text: String::new(),
            input_error: String::new(),
            sub_fetch: None,
            sub_url: None,
            settings_error: None,
            core_mixed_port,
            core_socks_port,
            core_external_controller,
            core_keep_alive,
            core_geodata_loader,
            dns_listen,
            dns_fakeip_range,
            dns_nameservers_csv,
            dns_fallback_csv,
            hosts_text,
            core_child: None,
            core_version: None,
            core_error: None,
            core_warning: None,
            core_op: None,
            last_reconcile: std::time::Instant::now(),
            traffic_rx: None,
            traffic_stop: None,
            traffic_up: 0,
            traffic_down: 0,
            traffic_max: 0,
            traffic_up_total: 0,
            traffic_down_total: 0,
        };
        if app.app_config.master_enabled {
            log::info!("autostart core (master was on)");
            app.request_core_start();
        }
        app
    }

    fn render_config_editor(&mut self, ui: &mut egui::Ui) {
        let theme = self.app_config.theme;
        let card_fill = card_fill_for(theme);
        let border = border_color_for(theme);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
            if ui
                .add_sized(
                    [64.0, 22.0],
                    egui::Button::new(egui::RichText::new("< Назад").size(11.0)),
                )
                .clicked()
            {
                self.active_tab = Tab::Dashboard;
            }
            ui.label(egui::RichText::new("Редактор конфига").size(11.0).strong());
            let interval = self.config_interval;
            egui::ComboBox::from_id_salt("config_interval")
                .selected_text(egui::RichText::new(interval.label_ru()).size(11.0))
                .width(110.0)
                .show_ui(ui, |ui| {
                    for iv in UpdateInterval::all() {
                        if ui
                            .selectable_label(
                                interval == *iv,
                                egui::RichText::new(iv.label_ru()).size(11.0),
                            )
                            .clicked()
                        {
                            self.config_interval = *iv;
                        }
                    }
                });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let save_btn = egui::Button::new(
                    egui::RichText::new("Сохранить")
                        .size(11.0)
                        .color(egui::Color32::WHITE),
                )
                .fill(NEKO_ACCENT)
                .stroke(egui::Stroke::new(1.0_f32, NEKO_ACCENT))
                .corner_radius(egui::CornerRadius::same(R6));
                if ui.add_sized([90.0, 22.0], save_btn).clicked() {
                    match self.active_profile.clone() {
                        Some(name) => {
                            let content = self.config_content.clone();
                            match serde_yaml::from_str::<serde_yaml::Value>(&content) {
                                Ok(_) => {
                                    if let Ok(conn) = rclash_db::open() {
                                        let _ = rclash_db::update_content(
                                            &conn, &name, &content,
                                        );
                                    }
                                    log::info!("config saved {name}");
                                    if self.core_child.is_some() && self.core_op.is_none() {
                                        match self.build_runtime() {
                                            Ok(_) => {
                                                let base = base_for_ops();
                                                let secret = secret_for_ops();
                                                self.core_op = Some(
                                                    poll_promise::Promise::spawn_thread(
                                                        "core-reload",
                                                        move || {
                                                            match rclash_core_manager::supervisor::reload_blocking(
                                                                                &base, &secret,
                                                                            ) {
                                                                Ok(()) => CoreOpOutcome::Done,
                                                                Err(e) => CoreOpOutcome::Failed(
                                                                    format!("reload: {e}"),
                                                                ),
                                                            }
                                                        },
                                                    ),
                                                );
                                            }
                                            Err(e) => log::error!("runtime rebuild: {e}"),
                                        }
                                    }
                                }
                                Err(e) => log::error!("config invalid yaml: {e}"),
                            }
                        }
                        None => log::warn!("config save: no active profile"),
                    }
                }
                if ui
                    .add_sized(
                        [80.0, 22.0],
                        egui::Button::new(egui::RichText::new("Сбросить").size(11.0)),
                    )
                    .clicked()
                {
                    self.config_content = active_db_content(&self.active_profile)
                        .map(|(_, c)| c)
                        .unwrap_or_else(|| "# Нет активного профиля — добавь профиль через +\n".to_owned());
                }
                if ui
                    .add_sized(
                        [80.0, 22.0],
                        egui::Button::new(egui::RichText::new("Открыть ↗").size(11.0)),
                    )
                    .clicked()
                {
                    let path = std::env::temp_dir().join("rclash-config-edit.yaml");
                    match std::fs::write(&path, self.config_content.as_bytes()) {
                        Ok(()) => {
                            log::info!("config opened {}", path.display());
                            if let Err(e) = open::that(&path) {
                                log::error!("open editor: {e}");
                            }
                        }
                        Err(e) => log::error!("config tmp write: {e}"),
                    }
                }
            });
        });
        ui.add_space(8.0);
        let editor_frame = egui::Frame::new()
            .fill(card_fill)
            .stroke(egui::Stroke::new(1.0_f32, border))
            .corner_radius(egui::CornerRadius::same(R6))
            .inner_margin(egui::Margin::same(8));
        editor_frame.show(ui, |ui| {
            let h = ui.available_height().max(120.0);
            ui.set_min_height(h);
            ui.set_max_height(h);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.config_content)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY),
                    );
                });
        });
    }

    fn render_logs_screen(&mut self, ui: &mut egui::Ui) {
        let theme = self.app_config.theme;
        let card_fill = card_fill_for(theme);
        let border = border_color_for(theme);
        let threshold = log_threshold(self.log_level);
        let source = self.log_source;
        let entries: Vec<crate::logger::AppLogEntry> = crate::logger::snapshot()
            .into_iter()
            .filter(|e| log_entry_visible(source, threshold, e))
            .collect();

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
            if ui
                .add_sized(
                    [64.0, 22.0],
                    egui::Button::new(egui::RichText::new("< Назад").size(11.0)),
                )
                .clicked()
            {
                self.active_tab = Tab::Dashboard;
            }
            ui.label(egui::RichText::new("Логи").size(11.0).strong());
            ui.add_space(8.0);
            for (src, label) in [
                (LogSource::Core, "Ядро"),
                (LogSource::App, "Приложение"),
                (LogSource::All, "Все"),
            ] {
                let active = self.log_source == src;
                let fill = if active { NEKO_ACCENT } else { card_fill };
                let txt_color = if active {
                    egui::Color32::WHITE
                } else {
                    match theme {
                        Theme::Light => egui::Color32::BLACK,
                        Theme::Dark => egui::Color32::WHITE,
                    }
                };
                let text = egui::RichText::new(label).size(11.0).color(txt_color);
                let btn = egui::Button::new(text)
                    .fill(fill)
                    .stroke(egui::Stroke::new(
                        1.0_f32,
                        if active { NEKO_ACCENT } else { border },
                    ))
                    .corner_radius(egui::CornerRadius::same(R6));
                if ui.add(btn).clicked() {
                    self.log_source = src;
                }
            }
            ui.add_space(8.0);
            if ui
                .add_sized(
                    [70.0, 22.0],
                    egui::Button::new(egui::RichText::new("Очистить").size(11.0)),
                )
                .clicked()
            {
                crate::logger::clear();
            }
            ui.checkbox(
                &mut self.log_autoscroll,
                egui::RichText::new("Авто").size(11.0),
            );
            if ui
                .add_sized(
                    [80.0, 22.0],
                    egui::Button::new(egui::RichText::new("Копировать").size(11.0)),
                )
                .clicked()
            {
                let text = entries
                    .iter()
                    .map(|e| format!("{}  {}  [{}]  {}", e.time, e.level, e.target, e.message))
                    .collect::<Vec<_>>()
                    .join("\n");
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    let _ = cb.set_text(text);
                }
                log::info!("logs copied {} entries", entries.len());
            }
            if ui
                .add_sized(
                    [70.0, 22.0],
                    egui::Button::new(egui::RichText::new("Экспорт").size(11.0)),
                )
                .clicked()
            {
                let text = entries
                    .iter()
                    .map(|e| format!("{}  {}  [{}]  {}", e.time, e.level, e.target, e.message))
                    .collect::<Vec<_>>()
                    .join("\n");
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Text", &["txt", "log"])
                    .set_file_name("rclash-logs.txt")
                    .save_file()
                {
                    if std::fs::write(&path, text).is_ok() {
                        log::info!("logs exported to {}", path.display());
                    } else {
                        log::warn!("logs export failed");
                    }
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::ComboBox::from_id_salt("log_level")
                    .selected_text(egui::RichText::new(self.log_level.as_str()).size(11.0))
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for lv in [
                            LogLevel::Debug,
                            LogLevel::Info,
                            LogLevel::Warning,
                            LogLevel::Error,
                            LogLevel::Silent,
                        ] {
                            ui.selectable_value(
                                &mut self.log_level,
                                lv,
                                egui::RichText::new(lv.as_str()).size(11.0),
                            );
                        }
                    });
            });
        });
        ui.add_space(8.0);
        let log_frame = egui::Frame::new()
            .fill(card_fill)
            .stroke(egui::Stroke::new(1.0_f32, border))
            .corner_radius(egui::CornerRadius::same(R4))
            .inner_margin(egui::Margin::same(8));
        log_frame.show(ui, |ui| {
            let h = ui.available_height().max(120.0);
            ui.set_min_height(h);
            ui.set_max_height(h);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(self.log_autoscroll)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 2.0);
                    if entries.is_empty() {
                        ui.add_space(24.0);
                        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("Нет записей").size(11.0).weak());
                        });
                    } else {
                        for e in &entries {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                                ui.add_sized(
                                    [78.0, 16.0],
                                    egui::Label::new(
                                        egui::RichText::new(&e.time).monospace().size(10.0).weak(),
                                    ),
                                );
                                ui.add_sized(
                                    [52.0, 16.0],
                                    egui::Label::new(
                                        egui::RichText::new(format!("{:<7}", e.level))
                                            .monospace()
                                            .size(10.0)
                                            .color(log_level_color(&e.level)),
                                    ),
                                );
                                ui.label(
                                    egui::RichText::new(format!("[{}]", e.target))
                                        .monospace()
                                        .size(10.0),
                                );
                                ui.add(
                                    egui::Label::new(egui::RichText::new(&e.message).size(11.0))
                                        .truncate(),
                                );
                            });
                        }
                    }
                });
        });
    }

    fn open_input(&mut self, ctx: InputContext, text: String) {
        self.input_ctx = ctx;
        self.input_text = text;
        self.input_error.clear();
        self.input_open = true;
        self.add_menu_open = false;
    }

    fn close_input(&mut self) {
        self.input_open = false;
        self.input_text.clear();
        self.input_error.clear();
    }

    fn input_title(&self) -> &'static str {
        match self.input_ctx {
            InputContext::Clipboard => "Вставь из буфера",
            InputContext::Url => "Введи URL подписки",
            InputContext::Raw => "Введи сырую ссылку",
        }
    }

    fn refresh_groups(&mut self) {
        let mut groups = vec!["PROXY".to_owned()];
        for p in &self.proxies {
            if p.group != "PROXY" && !groups.contains(&p.group) {
                groups.push(p.group.clone());
            }
        }
        self.groups = groups;
        if self.selected_group != "Все группы" && !self.groups.contains(&self.selected_group)
        {
            self.selected_group = "Все группы".to_owned();
        }
    }

    fn add_raw_proxies(&mut self, text: &str) -> usize {
        let mut added = 0;
        let tokens = extract_link_tokens(text);
        let source: Vec<(String, serde_yaml::Value)> = if tokens.is_empty() {
            rclash_subscription::parse_text_links(text)
                .unwrap_or_default()
                .into_iter()
                .map(|v| (String::new(), v))
                .collect()
        } else {
            tokens
                .into_iter()
                .filter_map(|t| rclash_subscription::parse_raw_link(&t).ok().map(|v| (t, v)))
                .collect()
        };
        let db_ok = rclash_db::open();
        for (raw, v) in &source {
            if let Some(item) = proxy_item_from_value(v) {
                if !self.proxies.iter().any(|p| p.name == item.name) {
                    self.proxies.push(item.clone());
                    added += 1;
                }
                if let Ok(ref conn) = db_ok {
                    let dump = serde_yaml::to_string(v).unwrap_or_default();
                    let raw_text = if raw.is_empty() {
                        dump.trim()
                    } else {
                        raw.as_str()
                    };
                    let _ = rclash_db::add_raw_key(
                        conn,
                        raw_text,
                        Some(item.proxy_type.as_str()),
                        &item.name,
                        dump.trim(),
                    );
                }
            }
        }
        if added > 0 {
            self.refresh_groups();
        }
        added
    }

    fn process_text(&mut self, text: &str, suggested_name: &str) {
        let t = text.trim();
        if t.is_empty() {
            self.input_error = "Вставь текст".to_owned();
            return;
        }
        if t.starts_with("http://") || t.starts_with("https://") {
            let url = t.to_owned();
            let name = profile_name_from_url(&url, suggested_name);
            self.input_error = "Загрузка…".to_owned();
            self.sub_url = Some(url.clone());
            self.sub_fetch = Some(poll_promise::Promise::spawn_thread(
                "fetch-subscription",
                move || fetch_subscription(url, name),
            ));
            return;
        }
        if t.contains("://") {
            let added = self.add_raw_proxies(t);
            if added == 0 {
                self.input_error = "Не распознано".to_owned();
            } else {
                log::info!("raw keys added {added}");
                self.close_input();
                self.restart_core_if_running();
            }
            return;
        }
        match rclash_config::profile::import_profile_content(t, suggested_name) {
            Ok(profile) => {
                if let Ok(conn) = rclash_db::open() {
                    let _ = rclash_db::save_config(&conn, &profile.name, None, t);
                }
                self.profile_store.add_or_replace(profile.clone());
                self.active_profile = Some(profile.name.clone());
                self.profile_store.active = Some(profile.name.clone());
                let _ = rclash_config::profile::save_profile_store(&self.profile_store);
                self.config_content = t.to_owned();
                log::info!("profile imported {}", profile.name);
                self.close_input();
                self.restart_core_if_running();
            }
            Err(_) => {
                self.input_error = "Не распознано".to_owned();
            }
        }
    }

    fn submit_input(&mut self) {
        let text = self.input_text.clone();
        let fallback = match self.input_ctx {
            InputContext::Clipboard => "clipboard",
            InputContext::Url => "subscription",
            InputContext::Raw => "raw",
        };
        self.process_text(&text, fallback);
    }

    fn build_runtime(&self) -> anyhow::Result<(std::path::PathBuf, std::path::PathBuf)> {
        let cfg = rclash_config::load_app_config();
        let secret = cfg.core_secret.clone().unwrap_or_default();
        let raw_extras = raw_keys_yaml_values();
        let base = match active_db_content(&self.active_profile) {
            Some((_, content)) => {
                if raw_extras.is_empty() {
                    content
                } else {
                    rclash_config::runtime::merge_extra_proxies(&content, &raw_extras)
                        .unwrap_or(content)
                }
            }
            None => {
                if raw_extras.is_empty() {
                    anyhow::bail!("нет активного профиля и нет сырых ключей");
                }
                rclash_config::runtime::build_raw_keys_config(&raw_extras)?
            }
        };
        let patched = rclash_config::runtime::assemble_runtime_config(&base, &cfg, &secret)?;
        let dir = rclash_config::runtime::ensure_runtime_dir()?;
        let file = rclash_config::runtime::write_runtime_config(&patched)?;
        Ok((dir, file))
    }

    fn request_core_start(&mut self) {
        if self.core_op.is_some() || self.core_child.is_some() {
            return;
        }
        self.core_error = None;
        self.core_warning = None;
        let (dir, file) = match self.build_runtime() {
            Ok(v) => v,
            Err(e) => {
                self.core_error = Some(format!("{e}"));
                log::error!("core start: {e}");
                return;
            }
        };
        let binary = match rclash_core_manager::resolve_core_path() {
            Some(p) => p,
            None => {
                let e = "ядро не найдено рядом с приложением";
                self.core_error = Some(e.to_owned());
                log::error!("core start: {e}");
                return;
            }
        };
        let secret = secret_for_ops();
        let base = base_for_ops();
        let tun = rclash_config::load_app_config().tun_enabled;
        log::info!("core starting {}", binary.display());
        self.core_op = Some(poll_promise::Promise::spawn_thread(
            "core-start",
            move || {
                if let Err(e) = rclash_updater::geodata::ensure_geodata(&dir, false) {
                    log::warn!("geodata ensure: {e}");
                }
                let strict = rclash_config::load_app_config().geodata_strict;
                let mut stripped: Vec<String> = Vec::new();
                for _ in 0..60 {
                    match rclash_core_manager::supervisor::precheck(&binary, &dir, &file) {
                        Ok(()) => break,
                        Err(e) => {
                            let msg = e.to_string();
                            if strict {
                                return CoreOpOutcome::Failed(msg);
                            }
                            let missing: Vec<(String, String)> =
                                rclash_config::runtime::parse_missing_geodata(&msg)
                                    .into_iter()
                                    .filter(|(_, n)| !stripped.iter().any(|s| s == n))
                                    .collect();
                            if missing.is_empty() {
                                return CoreOpOutcome::Failed(msg);
                            }
                            let content = match std::fs::read_to_string(&file) {
                                Ok(c) => c,
                                Err(e) => return CoreOpOutcome::Failed(format!("{e}")),
                            };
                            match rclash_config::runtime::strip_missing_geodata_rules(
                                &content, &missing,
                            ) {
                                Ok((next, removed)) if removed > 0 => {
                                    if std::fs::write(&file, next.as_bytes()).is_err() {
                                        return CoreOpOutcome::Failed(msg);
                                    }
                                    for (_, n) in &missing {
                                        log::info!("geodata: skip missing list {n}");
                                        stripped.push(n.clone());
                                    }
                                }
                                _ => return CoreOpOutcome::Failed(msg),
                            }
                        }
                    }
                }
                if tun {
                    if let Err(e) = rclash_tun::enable() {
                        return CoreOpOutcome::Failed(format!("tun: {e}"));
                    }
                }
                let warning = if stripped.is_empty() {
                    None
                } else {
                    stripped.sort();
                    stripped.dedup();
                    Some(format!(
                        "геодата: пропущено правил: {} ({})",
                        stripped.len(),
                        stripped.join(", ")
                    ))
                };
                match rclash_core_manager::supervisor::spawn_and_wait(
                    &binary, &dir, &file, &base, &secret,
                ) {
                    Ok((child, version)) => CoreOpOutcome::Started {
                        version,
                        child,
                        warning,
                    },
                    Err(e) => CoreOpOutcome::Failed(format!("{e}")),
                }
            },
        ));
    }

    fn request_geodata_refresh(&mut self) {
        if self.core_op.is_some() {
            return;
        }
        let Some(home) = rclash_config::runtime::runtime_dir() else {
            self.core_error = Some("нет каталога runtime".to_owned());
            return;
        };
        self.core_error = None;
        self.core_warning = None;
        log::info!("geodata refresh requested");
        self.core_op = Some(poll_promise::Promise::spawn_thread(
            "geodata-refresh",
            move || {
                let (site, ip) = rclash_updater::geodata::geodata_paths(&home);
                let _ = std::fs::remove_file(site);
                let _ = std::fs::remove_file(ip);
                match rclash_updater::geodata::ensure_geodata(&home, true) {
                    Ok(_) => CoreOpOutcome::GeodataDone,
                    Err(e) => CoreOpOutcome::Failed(format!("геодата: {e}")),
                }
            },
        ));
    }

    fn stop_traffic(&mut self) {
        if let Some(flag) = self.traffic_stop.take() {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.traffic_rx = None;
    }

    fn stop_core_now(&mut self) {
        rclash_core_manager::supervisor::stop_child(&mut self.core_child);
        self.stop_traffic();
        self.core_version = None;
        self.traffic_up = 0;
        self.traffic_down = 0;
    }

    fn request_core_stop(&mut self) {
        let tun_was_on = self.app_config.tun_enabled;
        self.stop_core_now();
        self.core_enabled = false;
        self.app_config.master_enabled = false;
        let _ = rclash_config::save_app_config(&self.app_config);
        if self.proxy_enabled {
            apply_sys_proxy_state(false);
        }
        if tun_was_on {
            std::thread::spawn(|| {
                if let Err(e) = rclash_tun::disable() {
                    log::warn!("tun down on stop: {e}");
                }
            });
        }
        log::info!("core stopped");
    }

    fn reconcile_core_state(&mut self) {
        if !self.core_enabled || self.core_op.is_some() {
            return;
        }
        if self.last_reconcile.elapsed() < std::time::Duration::from_secs(2) {
            return;
        }
        self.last_reconcile = std::time::Instant::now();
        if rclash_core_manager::supervisor::child_alive(&mut self.core_child) {
            return;
        }
        self.stop_core_now();
        self.core_enabled = false;
        self.app_config.master_enabled = false;
        let _ = rclash_config::save_app_config(&self.app_config);
        if self.proxy_enabled {
            apply_sys_proxy_state(false);
        }
        self.core_error = Some("ядро неожиданно завершилось".to_owned());
        log::error!("core process died");
    }

    fn patch_core_live(&mut self, body: serde_json::Value, what: &str) {
        if self.core_child.is_none() || self.core_op.is_some() {
            return;
        }
        let base = base_for_ops();
        let secret = secret_for_ops();
        let what = what.to_owned();
        self.core_op = Some(poll_promise::Promise::spawn_thread(
            "core-patch",
            move || match rclash_core_manager::supervisor::patch_configs_blocking(
                &base, &secret, &body,
            ) {
                Ok(()) => CoreOpOutcome::Done,
                Err(e) => CoreOpOutcome::Failed(format!("{what}: {e}")),
            },
        ));
    }

    fn rebuild_and_reload(&mut self, what: &str) {
        if self.core_child.is_none() || self.core_op.is_some() {
            return;
        }
        match self.build_runtime() {
            Ok(_) => {
                let base = base_for_ops();
                let secret = secret_for_ops();
                let what = what.to_owned();
                self.core_op = Some(poll_promise::Promise::spawn_thread(
                    "core-reload",
                    move || match rclash_core_manager::supervisor::reload_blocking(&base, &secret) {
                        Ok(()) => CoreOpOutcome::Done,
                        Err(e) => CoreOpOutcome::Failed(format!("{what}: {e}")),
                    },
                ));
            }
            Err(e) => {
                self.core_error = Some(format!("{e}"));
                log::error!("runtime rebuild: {e}");
            }
        }
    }

    fn set_mode(&mut self, mode: CoreMode) {
        self.app_config.mode = mode;
        let _ = rclash_config::save_app_config(&self.app_config);
        log::info!("mode {}", mode.as_str());
        if self.core_child.is_some() && self.core_op.is_none() {
            let base = base_for_ops();
            let secret = secret_for_ops();
            let m = mode.as_str().to_owned();
            self.core_op = Some(poll_promise::Promise::spawn_thread(
                "core-mode",
                move || match rclash_core_manager::supervisor::set_mode_blocking(&base, &secret, &m)
                {
                    Ok(()) => CoreOpOutcome::Done,
                    Err(e) => CoreOpOutcome::Failed(format!("mode: {e}")),
                },
            ));
        }
    }

    fn set_tun_enabled(&mut self, on: bool) {
        if self.tun_enabled == on && self.app_config.tun_enabled == on {
            return;
        }
        self.tun_enabled = on;
        self.app_config.tun_enabled = on;
        let _ = rclash_config::save_app_config(&self.app_config);
        log::info!("tun {on}");
        if self.core_child.is_some() && self.core_op.is_none() {
            self.tun_target = Some(on);
            self.core_op = Some(poll_promise::Promise::spawn_thread(
                "tun-helper",
                move || {
                    let r = if on {
                        rclash_tun::enable()
                    } else {
                        rclash_tun::disable()
                    };
                    match r {
                        Ok(()) => CoreOpOutcome::TunApplied { enabled: on },
                        Err(e) => CoreOpOutcome::TunFailed {
                            target: on,
                            message: format!("tun: {e}"),
                        },
                    }
                },
            ));
        } else {
            self.tun_target = None;
        }
    }

    fn restart_core_if_running(&mut self) {
        if self.core_child.is_some() || self.core_op.is_some() {
            self.stop_core_now();
            self.request_core_start();
        }
    }

    fn start_traffic(&mut self) {
        self.stop_traffic();
        let (tx, rx) = std::sync::mpsc::channel::<TrafficSample>();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_flag = stop.clone();
        let secret = secret_for_ops();
        let base = base_for_ops();
        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder().build();
            let Ok(client) = client else { return };
            let mut req = client.get(format!("{base}/traffic"));
            if !secret.is_empty() {
                req = req.header("Authorization", format!("Bearer {secret}"));
            }
            let Ok(resp) = req.send() else { return };
            use std::io::BufRead;
            let mut reader = std::io::BufReader::new(resp);
            let mut line = String::new();
            loop {
                if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                if let Ok(info) =
                    serde_json::from_str::<rclash_core_manager::api::TrafficInfo>(line.trim())
                {
                    let sample = TrafficSample {
                        up: info.up,
                        down: info.down,
                        up_total: info.up_total,
                        down_total: info.down_total,
                    };
                    if tx.send(sample).is_err() {
                        break;
                    }
                }
            }
        });
        self.traffic_rx = Some(rx);
        self.traffic_stop = Some(stop);
    }

    fn refresh_proxies_from_core(&mut self) {
        if self.core_op.is_some() {
            return;
        }
        let base = base_for_ops();
        let secret = secret_for_ops();
        self.core_op = Some(poll_promise::Promise::spawn_thread(
            "core-proxies",
            move || match rclash_core_manager::supervisor::proxies_blocking(&base, &secret) {
                Ok(v) => {
                    let (items, groups, now) = parse_proxies_value(&v);
                    let mode =
                        rclash_core_manager::supervisor::get_configs_blocking(&base, &secret)
                            .ok()
                            .and_then(|c| {
                                c.get("mode").and_then(|m| m.as_str()).map(str::to_owned)
                            });
                    CoreOpOutcome::Proxies {
                        items,
                        groups,
                        mode,
                        now,
                    }
                }
                Err(e) => CoreOpOutcome::Failed(format!("proxies: {e}")),
            },
        ));
    }

    fn poll_core_op(&mut self) {
        let ready = self.core_op.as_ref().is_some_and(|p| p.ready().is_some());
        if !ready {
            return;
        }
        let Some(p) = self.core_op.take() else {
            return;
        };
        match p.block_and_take() {
            CoreOpOutcome::Started {
                version,
                child,
                warning,
            } => {
                self.core_child = Some(child);
                self.core_version = Some(version.clone());
                self.core_enabled = true;
                self.app_config.master_enabled = true;
                let _ = rclash_config::save_app_config(&self.app_config);
                self.core_error = None;
                self.core_warning = warning.clone();
                log::info!("core started {version}");
                if let Some(w) = &warning {
                    log::warn!("{w}");
                }
                self.start_traffic();
                if self.proxy_enabled {
                    apply_sys_proxy_state(true);
                }
                self.refresh_proxies_from_core();
            }
            CoreOpOutcome::Proxies {
                items,
                groups,
                mode,
                now,
            } => {
                log::info!("proxies from core: {}", items.len());
                self.proxies = items;
                if !groups.is_empty() {
                    self.groups = groups;
                    if self.selected_group != "Все группы"
                        && !self.groups.contains(&self.selected_group)
                    {
                        self.selected_group = "Все группы".to_owned();
                    }
                }
                if let Some(m) = mode {
                    if let Some(parsed) = CoreMode::from_str(&m) {
                        if self.app_config.mode != parsed {
                            self.app_config.mode = parsed;
                            let _ = rclash_config::save_app_config(&self.app_config);
                        }
                    }
                }
                if let Some(n) = now {
                    if self.proxies.iter().any(|p| p.name == n) {
                        self.selected_proxy = Some(n);
                    }
                }
                self.refresh_groups();
            }
            CoreOpOutcome::Delays { delays } => {
                for (name, d) in delays {
                    if let Some(p) = self.proxies.iter_mut().find(|p| p.name == name) {
                        p.delay = d;
                    }
                }
            }
            CoreOpOutcome::Done => {}
            CoreOpOutcome::GeodataDone => {
                log::info!("geodata refreshed");
                self.restart_core_if_running();
            }
            CoreOpOutcome::TunApplied { enabled } => {
                self.tun_target = None;
                log::info!("tun helper applied {enabled}");
                self.restart_core_if_running();
            }
            CoreOpOutcome::TunFailed { target, message } => {
                self.tun_target = None;
                self.tun_enabled = !target;
                self.app_config.tun_enabled = !target;
                let _ = rclash_config::save_app_config(&self.app_config);
                self.core_error = Some(message.clone());
                log::error!("tun helper failed: {message}");
            }
            CoreOpOutcome::Failed(e) => {
                self.core_error = Some(e.clone());
                self.core_enabled = false;
                log::error!("core op failed: {e}");
            }
        }
    }

    fn poll_sub_fetch(&mut self) {
        let ready = self.sub_fetch.as_ref().is_some_and(|p| p.ready().is_some());
        if !ready {
            return;
        }
        if let Some(p) = self.sub_fetch.take() {
            match p.block_and_take() {
                Ok((name, content)) => {
                    let content = content.clone();
                    let url = self.sub_url.take();
                    if content.contains("proxies:")
                        || rclash_subscription::detect_format(&content)
                            == rclash_subscription::DetectedFormat::Yaml
                    {
                        match rclash_config::profile::import_profile_content(&content, &name) {
                            Ok(profile) => {
                                if let Ok(conn) = rclash_db::open() {
                                    let _ = rclash_db::save_config(
                                        &conn,
                                        &profile.name,
                                        url.as_deref(),
                                        &content,
                                    );
                                }
                                self.profile_store.add_or_replace(profile.clone());
                                self.active_profile = Some(profile.name.clone());
                                self.profile_store.active = Some(profile.name.clone());
                                let _ =
                                    rclash_config::profile::save_profile_store(&self.profile_store);
                                self.config_content = content.clone();
                                log::info!("subscription imported {}", profile.name);
                                self.close_input();
                                self.restart_core_if_running();
                            }
                            Err(e) => {
                                self.input_error = format!("Профиль: {e}");
                            }
                        }
                    } else {
                        let added = self.add_raw_proxies(&content);
                        if added == 0 {
                            self.input_error = "Не распознано".to_owned();
                        } else {
                            log::info!("subscription raw keys added {added}");
                            self.close_input();
                        }
                    }
                }
                Err(e) => {
                    self.input_error = e;
                }
            }
        }
    }

    fn render_input_dialog(&mut self, ctx: &egui::Context) {
        if !self.input_open {
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.close_input();
            return;
        }
        let theme = self.app_config.theme;
        let card_fill = card_fill_for(theme);
        let border = border_color_for(theme);
        let title = self.input_title();
        let loading = self.sub_fetch.is_some();
        let mut submit = false;
        let mut cancel = false;
        egui::Area::new("input_dialog_v2".into())
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(card_fill)
                    .stroke(egui::Stroke::new(1.0_f32, border))
                    .corner_radius(egui::CornerRadius::same(R6))
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(394.0, 0.0));
                        ui.set_max_width(394.0);
                        ui.label(egui::RichText::new(title).size(12.0).strong());
                        ui.add_space(8.0);
                        let hint = egui::RichText::new("https://… или hysteria2://…")
                            .size(11.0)
                            .weak();
                        ui.add_sized(
                            [394.0, 58.0],
                            egui::TextEdit::multiline(&mut self.input_text)
                                .hint_text(hint)
                                .font(egui::TextStyle::Monospace),
                        );
                        if loading {
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new("Загрузка…").size(11.0).weak());
                        }
                        if !self.input_error.is_empty() && !loading {
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(&self.input_error)
                                    .size(11.0)
                                    .color(NEKO_RED),
                            );
                        }
                        ui.add_space(8.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                            let ok_btn = egui::Button::new(
                                egui::RichText::new("OK")
                                    .size(11.0)
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(NEKO_ACCENT)
                            .stroke(egui::Stroke::new(1.0_f32, NEKO_ACCENT))
                            .corner_radius(egui::CornerRadius::same(R6));
                            if ui.add_enabled(!loading, ok_btn).clicked() {
                                submit = true;
                            }
                            if ui
                                .add_sized(
                                    [80.0, 22.0],
                                    egui::Button::new(egui::RichText::new("Отмена").size(11.0)),
                                )
                                .clicked()
                            {
                                cancel = true;
                            }
                        });
                    });
            });
        if cancel {
            self.close_input();
        } else if submit {
            self.submit_input();
        }
    }

    fn render_profiles_screen(&mut self, ui: &mut egui::Ui) {
        let theme = self.app_config.theme;
        let card_fill = card_fill_for(theme);
        let border = border_color_for(theme);
        let profiles = self.profile_store.profiles.clone();
        let raw_keys = load_raw_proxies();

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
            if ui
                .add_sized(
                    [64.0, 22.0],
                    egui::Button::new(egui::RichText::new("< Назад").size(11.0)),
                )
                .clicked()
            {
                self.active_tab = Tab::Dashboard;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
                for (section, label) in [
                    (ProfilesSection::RawKeys, "Сырые ключи"),
                    (ProfilesSection::Profiles, "Профили"),
                ] {
                    let active = self.profiles_section == section;
                    let fill = if active { NEKO_ACCENT } else { card_fill };
                    let txt_color = if active {
                        egui::Color32::WHITE
                    } else {
                        match theme {
                            Theme::Light => egui::Color32::BLACK,
                            Theme::Dark => egui::Color32::WHITE,
                        }
                    };
                    let btn =
                        egui::Button::new(egui::RichText::new(label).size(11.0).color(txt_color))
                            .fill(fill)
                            .stroke(egui::Stroke::new(
                                1.0_f32,
                                if active { NEKO_ACCENT } else { border },
                            ))
                            .corner_radius(egui::CornerRadius::same(R6));
                    if ui.add(btn).clicked() {
                        self.profiles_section = section;
                    }
                }
            });
        });
        ui.add_space(8.0);
        let section = self.profiles_section;
        let mut to_delete_profile: Option<String> = None;
        let mut to_delete_raw: Option<usize> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 8.0);
                if section == ProfilesSection::Profiles {
                    let profiles_frame = egui::Frame::new()
                        .fill(card_fill)
                        .stroke(egui::Stroke::new(1.0_f32, border))
                        .corner_radius(egui::CornerRadius::same(R4))
                        .inner_margin(egui::Margin::same(8));
                    profiles_frame.show(ui, |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 6.0);
                        ui.label(
                            egui::RichText::new(format!("Профили — {} шт.", profiles.len()))
                                .size(11.0)
                                .strong(),
                        );
                        if profiles.is_empty() {
                            ui.add_space(24.0);
                            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new("Профилей нет").size(11.0).weak());
                            });
                        } else {
                            for p in &profiles {
                                let row_frame = egui::Frame::new()
                                    .fill(card_fill)
                                    .stroke(egui::Stroke::new(1.0_f32, border))
                                    .corner_radius(egui::CornerRadius::same(R6))
                                    .inner_margin(egui::Margin::symmetric(8, 6));
                                row_frame.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                                        ui.vertical(|ui| {
                                            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&p.name).size(11.0),
                                                )
                                                .truncate(),
                                            );
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&p.path)
                                                        .monospace()
                                                        .size(10.0)
                                                        .weak(),
                                                )
                                                .truncate(),
                                            );
                                        });
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let del_btn = egui::Button::new(
                                                    egui::RichText::new("×")
                                                        .size(11.0)
                                                        .color(egui::Color32::WHITE),
                                                )
                                                .fill(NEKO_RED)
                                                .stroke(egui::Stroke::new(1.0_f32, NEKO_RED))
                                                .corner_radius(egui::CornerRadius::same(R4));
                                                if ui
                                                    .add_sized([22.0, 22.0], del_btn)
                                                    .on_hover_text("Удалить")
                                                    .clicked()
                                                {
                                                    to_delete_profile = Some(p.name.clone());
                                                }
                                            },
                                        );
                                    });
                                });
                            }
                        }
                    });
                }
                let raw_frame = egui::Frame::new()
                    .fill(card_fill)
                    .stroke(egui::Stroke::new(1.0_f32, border))
                    .corner_radius(egui::CornerRadius::same(R4))
                    .inner_margin(egui::Margin::same(8));
                if section == ProfilesSection::RawKeys {
                    raw_frame.show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(format!("Сырые ключи — {} шт.", raw_keys.len()))
                                .size(11.0)
                                .strong(),
                        );
                        ui.add_space(6.0);
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0);
                        if raw_keys.is_empty() {
                            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new("Пока пусто").size(11.0).weak());
                            });
                        } else {
                            for (i, p) in raw_keys.iter().enumerate() {
                                let row_frame = egui::Frame::new()
                                    .fill(card_fill)
                                    .stroke(egui::Stroke::new(1.0_f32, border))
                                    .corner_radius(egui::CornerRadius::same(R4))
                                    .inner_margin(egui::Margin::symmetric(6, 4));
                                row_frame.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                                        ui.label(
                                            egui::RichText::new(format!("{}", i + 1))
                                                .size(10.0)
                                                .weak(),
                                        );
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(format!(
                                                    "{}#{}",
                                                    p.name, p.proxy_type
                                                ))
                                                .monospace()
                                                .size(10.0),
                                            )
                                            .truncate(),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let del_btn = egui::Button::new(
                                                    egui::RichText::new("×")
                                                        .size(11.0)
                                                        .color(egui::Color32::WHITE),
                                                )
                                                .fill(NEKO_RED)
                                                .stroke(egui::Stroke::new(1.0_f32, NEKO_RED))
                                                .corner_radius(egui::CornerRadius::same(R4));
                                                if ui
                                                    .add_sized([22.0, 22.0], del_btn)
                                                    .on_hover_text("Удалить")
                                                    .clicked()
                                                {
                                                    to_delete_raw = Some(i);
                                                }
                                            },
                                        );
                                    });
                                });
                            }
                        }
                    });
                }
            });
        if let Some(name) = to_delete_profile {
            if self.profile_store.remove(&name) {
                if let Ok(conn) = rclash_db::open() {
                    let _ = rclash_db::delete_config(&conn, &name);
                }
                if self.active_profile.as_deref() == Some(&name) {
                    self.active_profile = self.profile_store.active.clone();
                    self.config_content = active_db_content(&self.active_profile)
                        .map(|(_, c)| c)
                        .unwrap_or_else(|| {
                            "# Нет активного профиля — добавь профиль через +\n".to_owned()
                        });
                }
                let _ = rclash_config::profile::save_profile_store(&self.profile_store);
                log::info!("profile deleted {name}");
                self.restart_core_if_running();
            }
        }
        if let Some(i) = to_delete_raw {
            if let Some(target) = load_raw_proxies().get(i).cloned() {
                if let Ok(conn) = rclash_db::open() {
                    let _ = rclash_db::remove_raw_key_by_name(&conn, &target.name);
                }
                self.proxies.retain(|p| p.name != target.name);
                self.refresh_groups();
                log::info!("raw key deleted {}", target.name);
                self.restart_core_if_running();
            }
        }
    }

    #[allow(dead_code)]
    pub fn set_theme(&mut self, theme: Theme, ctx: &egui::Context) {
        self.app_config.theme = theme;
        ctx.set_visuals(theme_visuals(theme));
        let mut style = (*ctx.style()).clone();
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        style.spacing.interact_size.y = 22.0;
        ctx.set_style(style);
        let _ = rclash_config::save_app_config(&self.app_config);
        log::info!("theme changed to {:?}", theme);
    }
}

impl Drop for RClashApp {
    fn drop(&mut self) {
        rclash_core_manager::supervisor::stop_child(&mut self.core_child);
        apply_sys_proxy_state(false);
        if self.app_config.tun_enabled
            && matches!(rclash_tun::status(), rclash_tun::TunStatus::Enabled { .. })
        {
            if let Err(e) = rclash_tun::disable() {
                log::warn!("tun down on exit: {e}");
            }
        }
    }
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
        if self.active_tab == Tab::Logs {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }

        let panel_fill = panel_fill_for(self.app_config.theme);
        let card_fill = card_fill_for(self.app_config.theme);
        let border = border_color_for(self.app_config.theme);

        if self.add_menu_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.add_menu_open = false;
        }
        self.poll_sub_fetch();
        self.poll_core_op();
        self.reconcile_core_state();
        if let Some(rx) = &self.traffic_rx {
            for s in rx.try_iter() {
                self.traffic_up = s.up;
                self.traffic_down = s.down;
                self.traffic_up_total = s.up_total;
                self.traffic_down_total = s.down_total;
                self.traffic_max = self.traffic_max.max(s.up.saturating_add(s.down));
            }
        }
        if self.core_child.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(panel_fill)
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    let card_frame = egui::Frame::new()
                        .fill(card_fill)
                        .stroke(egui::Stroke::new(1.0_f32, border))
                        .corner_radius(egui::CornerRadius::same(R6))
                        .inner_margin(egui::Margin::same(8));

                    let mut add_btn_rect: Option<egui::Rect> = None;

                    let filtered: Vec<ProxyItem> = self
                        .proxies
                        .iter()
                        .filter(|p| {
                            if self.selected_group != "Все группы" && p.group != self.selected_group
                            {
                                return false;
                            }
                            true
                        })
                        .cloned()
                        .collect();
                    let count = filtered.len();
                    let proxy_selected = self.selected_proxy.clone();
                    let theme = self.app_config.theme;

                    if self.active_tab == Tab::Config {
                        self.render_config_editor(ui);
                    } else if self.active_tab == Tab::Logs {
                        self.render_logs_screen(ui);
                    } else if self.active_tab == Tab::Profiles {
                        self.render_profiles_screen(ui);
                    } else if self.active_tab == Tab::Settings {
                        self.render_settings_screen(ui);
                    } else {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;

                        ui.allocate_ui(egui::vec2(313.0, 519.0), |ui| {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(0.0, 8.0);
                                let tab_frame = egui::Frame::new()
                                    .fill(card_fill)
                                    .stroke(egui::Stroke::new(1.0_f32, border))
                                    .corner_radius(egui::CornerRadius::same(R4))
                                    .inner_margin(egui::Margin::symmetric(4, 4));
                                tab_frame.show(ui, |ui| {
                                    ui.set_min_size(egui::vec2(304.0, 24.0));
                                    ui.set_max_size(egui::vec2(304.0, 24.0));
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                                        for (tab, label) in
                                            [(Tab::Config, "конфиг"), (Tab::Logs, "логи"), (Tab::Settings, "настройки")]
                                        {
                                            let active = self.active_tab == tab;
                                            let mut rt =
                                                egui::RichText::new(label).size(11.0);
                                            if active {
                                                rt = rt.strong();
                                            }
                                            let btn = egui::Button::new(rt);
                                            if ui.add_sized(egui::vec2(100.0, 24.0), btn).clicked()
                                            {
                                                self.active_tab = tab;
                                            }
                                        }
                                    });
                                });
                                let chart_frame = egui::Frame::new()
                                    .fill(card_fill)
                                    .stroke(egui::Stroke::new(1.0_f32, border))
                                    .corner_radius(egui::CornerRadius::same(R4))
                                    .inner_margin(egui::Margin::same(8));
                                chart_frame.show(ui, |ui| {
                                    ui.set_min_size(egui::vec2(295.0, 143.0));
                                    ui.set_max_size(egui::vec2(295.0, 143.0));
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new("График").size(11.0));
                                        ui.add_space(4.0);
                                        let sep_w = ui.available_width();
                                        let (sep_rect, _) = ui.allocate_exact_size(
                                            egui::vec2(sep_w, 1.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().hline(
                                            sep_rect.x_range(),
                                            sep_rect.center().y,
                                            egui::Stroke::new(1.0_f32, border),
                                        );
                                        ui.add_space(32.0);
                                        ui.with_layout(
                                            egui::Layout::top_down(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    egui::RichText::new("Ожидание данных...")
                                                        .size(11.0)
                                                        .weak(),
                                                );
                                            },
                                        );
                                        ui.add_space(32.0);
                                        let (btm_rect, _) = ui.allocate_exact_size(
                                            egui::vec2(sep_w, 1.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().hline(
                                            btm_rect.x_range(),
                                            btm_rect.center().y,
                                            egui::Stroke::new(1.0_f32, border),
                                        );
                                        ui.add_space(1.0);
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing =
                                                egui::vec2(0.0, 0.0);
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing =
                                                    egui::vec2(4.0, 0.0);
                                                ui.label(
                                                    egui::RichText::new("●")
                                                        .size(10.0)
                                                        .color(NEKO_ACCENT),
                                                );
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "↑ {}/s",
                                                        rclash_core_manager::api::format_bytes(
                                                            self.traffic_up
                                                        )
                                                    ))
                                                    .size(10.0)
                                                    .weak(),
                                                );
                                            });
                                            let after_left = ui.available_width();
                                            let right_w = 50.0;
                                            let center_w = 55.0;
                                            let spacer = ((after_left - right_w) / 2.0
                                                - center_w / 2.0)
                                                .max(0.0);
                                            ui.add_space(spacer);
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing =
                                                    egui::vec2(4.0, 0.0);
                                                ui.label(
                                                    egui::RichText::new("●")
                                                        .size(10.0)
                                                        .color(NEKO_DELAY_FAST),
                                                );
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "↓ {}/s",
                                                        rclash_core_manager::api::format_bytes(
                                                            self.traffic_down
                                                        )
                                                    ))
                                                    .size(10.0)
                                                    .weak(),
                                                );
                                            });
                                            ui.with_layout(
                                                egui::Layout::right_to_left(
                                                    egui::Align::Center,
                                                ),
                                                |ui| {
                                                    ui.label(
                                                        egui::RichText::new(if self.traffic_max
                                                            > 0
                                                        {
                                                            format!(
                                                                "мах {}",
                                                                rclash_core_manager::api::format_bytes(
                                                                    self.traffic_max
                                                                )
                                                            )
                                                        } else {
                                                            "мах —".to_owned()
                                                        })
                                                        .size(10.0)
                                                        .weak(),
                                                    );
                                                },
                                            );
                                        });
                                    });
                                });
                                let stats_frame = egui::Frame::new()
                                    .fill(card_fill)
                                    .stroke(egui::Stroke::new(1.0_f32, border))
                                    .corner_radius(egui::CornerRadius::same(R4))
                                    .inner_margin(egui::Margin::symmetric(8, 0));
                                stats_frame.show(ui, |ui| {
                                    ui.set_min_size(egui::vec2(295.0, 104.0));
                                    ui.set_max_size(egui::vec2(295.0, 104.0));
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                                        ui.allocate_ui(egui::vec2(295.0, 34.0), |ui| {
                                            ui.with_layout(
                                                egui::Layout::left_to_right(
                                                    egui::Align::Center,
                                                ),
                                                |ui| {
                                                    ui.label(
                                                        egui::RichText::new("приём")
                                                            .size(11.0),
                                                    );
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            ui.label(
                                                                egui::RichText::new(
                                                                    rclash_core_manager::api::format_bytes(
                                                                        self.traffic_down_total,
                                                                    ),
                                                                )
                                                                .size(11.0),
                                                            );
                                                        },
                                                    );
                                                },
                                            );
                                        });
                                        let sep_w = ui.available_width();
                                        let (sep_rect, _) = ui.allocate_exact_size(
                                            egui::vec2(sep_w, 1.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().hline(
                                            sep_rect.x_range(),
                                            sep_rect.center().y,
                                            egui::Stroke::new(1.0_f32, border),
                                        );
                                        ui.allocate_ui(egui::vec2(295.0, 34.0), |ui| {
                                            ui.with_layout(
                                                egui::Layout::left_to_right(
                                                    egui::Align::Center,
                                                ),
                                                |ui| {
                                                    ui.label(
                                                        egui::RichText::new("отдача")
                                                            .size(11.0),
                                                    );
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            ui.label(
                                                                egui::RichText::new(
                                                                    rclash_core_manager::api::format_bytes(
                                                                        self.traffic_up_total,
                                                                    ),
                                                                )
                                                                .size(11.0),
                                                            );
                                                        },
                                                    );
                                                },
                                            );
                                        });
                                        let (sep_rect2, _) = ui.allocate_exact_size(
                                            egui::vec2(sep_w, 1.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().hline(
                                            sep_rect2.x_range(),
                                            sep_rect2.center().y,
                                            egui::Stroke::new(1.0_f32, border),
                                        );
                                        ui.allocate_ui(egui::vec2(295.0, 34.0), |ui| {
                                            ui.with_layout(
                                                egui::Layout::left_to_right(
                                                    egui::Align::Center,
                                                ),
                                                |ui| {
                                                    ui.label(
                                                        egui::RichText::new("ip").size(11.0),
                                                    );
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            let ip_text = egui::RichText::new(
                                                                "195.133.54.31",
                                                            )
                                                            .size(11.0);
                                                            ui.label(ip_text);
                                                            ui.label(
                                                                egui::RichText::new("RU")
                                                                    .monospace()
                                                                    .size(10.0)
                                                                    .weak(),
                                                            );
                                                        },
                                                    );
                                                },
                                            );
                                        });
                                    });
                                });
                                let controls_frame = egui::Frame::new()
                                    .fill(card_fill)
                                    .stroke(egui::Stroke::new(1.0_f32, border))
                                    .corner_radius(egui::CornerRadius::same(R4))
                                    .inner_margin(egui::Margin {
                                        left: 7,
                                        right: 7,
                                        top: 6,
                                        bottom: 6,
                                    });
                                controls_frame.show(ui, |ui| {
                                    ui.set_min_size(egui::vec2(297.0, 98.0));
                                    ui.set_max_size(egui::vec2(297.0, 98.0));
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 8.0);
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing = egui::vec2(4.5, 0.0);
                                            for m in CoreMode::all() {
                                                let label = m.as_str();
                                                let active = self.app_config.mode == *m;
                                                let fill = if active { NEKO_ACCENT } else { card_fill };
                                                let txt_color = if active {
                                                    egui::Color32::WHITE
                                                } else {
                                                    match theme {
                                                        Theme::Light => egui::Color32::BLACK,
                                                        Theme::Dark => egui::Color32::WHITE,
                                                    }
                                                };
                                                let text = egui::RichText::new(label)
                                                    .size(11.0)
                                                    .color(txt_color);
                                                let btn = egui::Button::new(text)
                                                    .fill(fill)
                                                    .stroke(egui::Stroke::new(
                                                        1.0_f32,
                                                        if active { NEKO_ACCENT } else { border },
                                                    ))
                                                    .corner_radius(egui::CornerRadius::same(R6));
                                                if ui
                                                    .add_sized(
                                                        egui::vec2(96.0, 26.0),
                                                        btn,
                                                    )
                                                    .clicked()
                                                {
                                                    self.set_mode(*m);
                                                }
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing = egui::vec2(3.0, 0.0);
                                            let proxy_fill = if self.proxy_enabled {
                                                NEKO_ACCENT
                                            } else {
                                                card_fill
                                            };
                                            let proxy_stroke = if self.proxy_enabled {
                                                NEKO_ACCENT
                                            } else {
                                                border
                                            };
                                            let proxy_txt = if self.proxy_enabled {
                                                egui::Color32::WHITE
                                            } else {
                                                match theme {
                                                    Theme::Light => egui::Color32::BLACK,
                                                    Theme::Dark => egui::Color32::WHITE,
                                                }
                                            };
                                            let proxy_label = "proxy";
                                            let proxy_btn = egui::Button::new(
                                                egui::RichText::new(proxy_label)
                                                    .size(11.0)
                                                    .color(proxy_txt),
                                            )
                                            .fill(proxy_fill)
                                            .stroke(egui::Stroke::new(1.0_f32, proxy_stroke))
                                            .corner_radius(egui::CornerRadius::same(R6));
                                            if ui
                                                .add_sized(egui::vec2(147.0, 26.0), proxy_btn)
                                                .clicked()
                                            {
                                                self.proxy_enabled = !self.proxy_enabled;
                                                self.app_config.proxy_enabled = self.proxy_enabled;
                                                let _ = rclash_config::save_app_config(
                                                    &self.app_config,
                                                );
                                                if self.core_child.is_some() {
                                                    apply_sys_proxy_state(self.proxy_enabled);
                                                }
                                            }
                                            let tun_fill = if self.tun_enabled {
                                                NEKO_ACCENT
                                            } else {
                                                card_fill
                                            };
                                            let tun_stroke = if self.tun_enabled {
                                                NEKO_ACCENT
                                            } else {
                                                border
                                            };
                                            let tun_txt = if self.tun_enabled {
                                                egui::Color32::WHITE
                                            } else {
                                                match theme {
                                                    Theme::Light => egui::Color32::BLACK,
                                                    Theme::Dark => egui::Color32::WHITE,
                                                }
                                            };
                                            let tun_label = "tun";
                                            let tun_btn = egui::Button::new(
                                                egui::RichText::new(tun_label)
                                                    .size(11.0)
                                                    .color(tun_txt),
                                            )
                                            .fill(tun_fill)
                                            .stroke(egui::Stroke::new(1.0_f32, tun_stroke))
                                            .corner_radius(egui::CornerRadius::same(R6));
                                            if ui
                                                .add_sized(egui::vec2(147.0, 26.0), tun_btn)
                                                .clicked()
                                            {
                                                let next = !self.tun_enabled;
                                                self.set_tun_enabled(next);
                                            }
                                        });
                                        let tgl_busy = self.core_op.is_some();
                                        let (tgl_label, tgl_fill) = if tgl_busy {
                                            ("Запуск…", NEKO_ACCENT)
                                        } else if self.core_enabled {
                                            ("ВКЛ • Выключить", NEKO_GREEN)
                                        } else {
                                            ("ВЫКЛ • Включить", NEKO_RED)
                                        };
                                        let tgl_btn = egui::Button::new(
                                            egui::RichText::new(tgl_label)
                                                .size(11.0)
                                                .color(egui::Color32::WHITE),
                                        )
                                        .fill(tgl_fill)
                                        .stroke(egui::Stroke::new(1.0_f32, tgl_fill))
                                        .corner_radius(egui::CornerRadius::same(R6));
                                        if ui.add_sized(egui::vec2(297.0, 30.0), tgl_btn).clicked()
                                        {
                                            if tgl_busy {
                                            } else if self.core_enabled {
                                                self.request_core_stop();
                                            } else {
                                                self.request_core_start();
                                            }
                                        }
                                        if let Some(e) = self.core_error.clone() {
                                            ui.label(
                                                egui::RichText::new(e).size(10.0).color(NEKO_RED),
                                            );
                                            let geo_btn = egui::Button::new(
                                                egui::RichText::new("Обновить геодату").size(11.0),
                                            )
                                            .fill(card_fill)
                                            .stroke(egui::Stroke::new(1.0_f32, border))
                                            .corner_radius(egui::CornerRadius::same(R6));
                                            if ui
                                                .add_sized(egui::vec2(297.0, 22.0), geo_btn)
                                                .clicked()
                                            {
                                                self.request_geodata_refresh();
                                            }
                                        }
                                        if let Some(w) = self.core_warning.clone() {
                                            ui.label(
                                                egui::RichText::new(w)
                                                    .size(10.0)
                                                    .color(NEKO_DELAY_MID),
                                            );
                                        }
                                    });
                                });
                                let update_frame = egui::Frame::new()
                                    .fill(egui::Color32::from_rgba_unmultiplied(
                                        0xE7, 0x4C, 0x3C, 40,
                                    ))
                                    .stroke(egui::Stroke::new(1.0_f32, NEKO_RED))
                                    .corner_radius(egui::CornerRadius::same(R4))
                                    .inner_margin(egui::Margin::same(8));
                                update_frame.show(ui, |ui| {
                                    ui.set_min_size(egui::vec2(295.0, 65.0));
                                    ui.set_max_size(egui::vec2(295.0, 65.0));
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                                        ui.add_space(11.5);
                                        ui.with_layout(
                                            egui::Layout::top_down(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    egui::RichText::new(
                                                        "Доступно обновление v0.2.8",
                                                    )
                                                    .size(11.0),
                                                );
        ui.add_space(8.0);
        if let Some(e) = self.settings_error.clone() {
            ui.label(egui::RichText::new(e).size(10.0).color(NEKO_RED));
            ui.add_space(4.0);
        }
                                                ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing =
                                                egui::vec2(8.0, 0.0);
                                            ui.add_space(47.5);
                                            let upd_btn = egui::Button::new(
                                                egui::RichText::new("Обновить")
                                                    .size(11.0)
                                                    .color(egui::Color32::WHITE),
                                            )
                                            .fill(NEKO_ACCENT)
                                            .stroke(egui::Stroke::new(
                                                1.0_f32, NEKO_ACCENT,
                                            ))
                                            .corner_radius(egui::CornerRadius::same(R6));
                                            if ui
                                                .add_sized(
                                                    egui::vec2(96.0, 22.0),
                                                    upd_btn,
                                                )
                                                .clicked()
                                            {
                                                log::info!("update clicked");
                                            }
                                            let later_btn = egui::Button::new(
                                                egui::RichText::new("Позже").size(11.0),
                                            )
                                            .fill(match theme {
                                                Theme::Light => egui::Color32::from_rgb(
                                                    0xE8, 0xEA, 0xED,
                                                ),
                                                Theme::Dark => egui::Color32::from_rgb(
                                                    0x3A, 0x3D, 0x45,
                                                ),
                                            })
                                            .stroke(egui::Stroke::new(
                                                1.0_f32, border,
                                            ))
                                            .corner_radius(egui::CornerRadius::same(R6));
                                            if ui
                                                .add_sized(
                                                    egui::vec2(96.0, 22.0),
                                                    later_btn,
                                                )
                                                .clicked()
                                            {
                                                log::info!("later clicked");
                                            }
                                        });
                                    });
                                });
                            });
                            });
                        });

                        ui.add_space(6.0);

                        ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(0.0, 8.0);
                                card_frame.show(ui, |ui| {
                                    ui.set_width(469.0);
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing =
                                                egui::vec2(8.0, 0.0);

                                            let avail = ui.available_width();
                                            let combo_w =
                                                (avail - 26.0 - 26.0 - 16.0).max(120.0);

                                        let profiles = self.profile_store.profiles.clone();
                                        let active_label = self
                                            .active_profile
                                            .clone()
                                            .unwrap_or_else(|| "Выберите профиль".to_owned());

                                        egui::ComboBox::from_id_salt("profiles_combo")
                                            .selected_text(
                                                egui::RichText::new(active_label).size(11.0),
                                            )
                                            .width(combo_w)
                                            .show_ui(ui, |ui| {
                                                if profiles.is_empty() {
                                                    ui.label(
                                                        egui::RichText::new("Нет профилей")
                                                            .size(11.0)
                                                            .weak(),
                                                    );
                                                } else {
                                                    for p in &profiles {
                                                        let is_active = Some(&p.name)
                                                            == self.active_profile.as_ref();
                                                        let label = if is_active {
                                                            format!("{} • активен", p.name)
                                                        } else if p.is_raw {
                                                            format!("{} • сырые", p.name)
                                                        } else {
                                                            p.name.clone()
                                                        };
                                                        if ui
                                                            .selectable_label(
                                                                is_active,
                                                                egui::RichText::new(label)
                                                                    .size(11.0),
                                                            )
                                                            .clicked()
                                                        {
                                                            self.active_profile =
                                                                Some(p.name.clone());
                                                            self.profile_store.active =
                                                                Some(p.name.clone());
                                                            let _ =
                                                                rclash_config::profile::save_profile_store(
                                                                    &self.profile_store,
                                                                );
                                                            if let Some((_, content)) =
                                                                active_db_content(
                                                                    &self.active_profile,
                                                                )
                                                            {
                                                                self.config_content = content;
                                                            }
                                                            log::info!(
                                                                "profile selected {}",
                                                                p.name
                                                            );
                                                            self.restart_core_if_running();
                                                        }
                                                    }
                                                }
                                                ui.separator();
                                                if ui
                                                    .selectable_label(
                                                        false,
                                                        egui::RichText::new(
                                                            "Управление профилями…",
                                                        )
                                                        .size(11.0),
                                                    )
                                                    .clicked()
                                                {
                                                    self.active_tab = Tab::Profiles;
                                                }
                                            });

                                        if ui
                                            .add_sized(
                                                [26.0, 22.0],
                                                egui::Button::new(
                                                    egui::RichText::new("↻").size(11.0),
                                                ),
                                            )
                                            .on_hover_text("Обновить")
                                            .clicked()
                                        {
                                            let url = self
                                                .active_profile
                                                .as_ref()
                                                .and_then(|n| self.profile_store.find(n))
                                                .and_then(|p| p.url.clone());
                                            if let Some(url) = url {
                                                let name = self
                                                    .active_profile
                                                    .clone()
                                                    .unwrap_or_else(|| "subscription".to_owned());
                                                self.input_error = String::new();
                                                self.sub_url = Some(url.clone());
                                                self.sub_fetch = Some(
                                                    poll_promise::Promise::spawn_thread(
                                                        "fetch-subscription",
                                                        move || fetch_subscription(url, name),
                                                    ),
                                                );
                                                log::info!("refresh subscription");
                                            } else if self.core_child.is_some() {
                                                self.refresh_proxies_from_core();
                                            } else {
                                                log::info!(
                                                    "refresh profile {:?}",
                                                    self.active_profile
                                                );
                                            }
                                        }

                                        let add_resp = ui
                                            .add_sized(
                                                [26.0, 22.0],
                                                egui::Button::new(
                                                    egui::RichText::new("+").size(11.0),
                                                ),
                                            )
                                            .on_hover_text("Добавить профиль");
                                        if add_resp.clicked() {
                                            self.add_menu_open = !self.add_menu_open;
                                        }
                                        add_btn_rect = Some(add_resp.rect);
                                    });

                                    ui.add_space(6.0);
                                    let sep_w = ui.available_width();
                                    let (sep_rect, _) = ui.allocate_exact_size(
                                        egui::vec2(sep_w, 1.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().hline(
                                        sep_rect.x_range(),
                                        sep_rect.center().y,
                                        egui::Stroke::new(1.0_f32, border),
                                    );
                                    ui.add_space(6.0);

                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                                        let avail = ui.available_width();
                                        let combo_w = (avail - 26.0 - 8.0).max(120.0);
                                        let groups = self.groups.clone();
                                        egui::ComboBox::from_id_salt("groups_combo")
                                            .selected_text(
                                                egui::RichText::new(self.selected_group.clone())
                                                    .size(11.0),
                                            )
                                            .width(combo_w)
                                            .show_ui(ui, |ui| {
                                                if ui
                                                    .selectable_label(
                                                        self.selected_group == "Все группы",
                                                        egui::RichText::new("Все группы")
                                                            .size(11.0),
                                                    )
                                                    .clicked()
                                                {
                                                    self.selected_group = "Все группы".to_owned();
                                                }
                                                for g in &groups {
                                                    if ui
                                                        .selectable_label(
                                                            self.selected_group == *g,
                                                            egui::RichText::new(g).size(11.0),
                                                        )
                                                        .clicked()
                                                    {
                                                        self.selected_group = g.clone();
                                                    }
                                                }
                                            });

                                        if ui
                                            .add_sized(
                                                [26.0, 22.0],
                                                egui::Button::new(
                                                    egui::RichText::new("◎").size(11.0),
                                                ),
                                            )
                                            .on_hover_text("Пинг видимых")
                                            .clicked()
                                        {
                                            if self.core_child.is_none() {
                                                log::info!("ping skipped: core not running");
                                            } else if self.core_op.is_none() {
                                                let names: Vec<String> = self
                                                    .proxies
                                                    .iter()
                                                    .filter(|p| {
                                                        self.selected_group == "Все группы"
                                                            || p.group == self.selected_group
                                                    })
                                                    .map(|p| p.name.clone())
                                                    .collect();
                                                let secret = secret_for_ops();
                                                let base = base_for_ops();
                                                log::info!("ping {} visible", names.len());
                                                self.core_op = Some(
                                                    poll_promise::Promise::spawn_thread(
                                                        "core-ping",
                                                        move || {
                                                            let delays = names
                                                                .into_iter()
                                                                .map(|n| {
                                                                    let d = rclash_core_manager::supervisor::test_delay_blocking(
                                                                        &base,
                                                                        &secret,
                                                                        &n,
                                                                        "http://www.gstatic.com/generate_204",
                                                                        5000,
                                                                    );
                                                                    (n, d)
                                                                })
                                                                .collect();
                                                            CoreOpOutcome::Delays { delays }
                                                        },
                                                    ),
                                                );
                                            }
                                        }
                                    });
                                });
                            });

                            let avail_h = ui.available_height().max(120.0);
                            let inner_h = (avail_h - 28.0 - 2.0).max(80.0);
                            card_frame.show(ui, |ui| {
                                ui.set_width(469.0);
                                ui.set_min_height(inner_h);
                                ui.set_max_height(inner_h);
                                ui.vertical(|ui| {
                                    ui.add_space(6.0);
                                    let scroll_h = (inner_h - 33.0).max(80.0);
                                    let mut to_select: Option<String> = None;
                                    egui::ScrollArea::vertical()
                                        .auto_shrink([false, false])
                                        .max_height(scroll_h)
                                        .show(ui, |ui| {
                                            ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0);
                                            if filtered.is_empty() {
                                                ui.add_space(24.0);
                                                ui.with_layout(
                                                    egui::Layout::top_down(egui::Align::Center),
                                                    |ui| {
                                                        ui.label(
                                                            egui::RichText::new("Нет локаций")
                                                                .size(11.0)
                                                                .weak(),
                                                        );
                                                    },
                                                );
                                            } else {
                                                for p in &filtered {
                                                    let is_sel =
                                                        Some(&p.name) == proxy_selected.as_ref();
                                                    let avail_w = ui.available_width();
                                                    let (rect, resp) = ui.allocate_exact_size(
                                                        egui::vec2(avail_w, 27.0),
                                                        egui::Sense::click(),
                                                    );
                                                    let hovered = resp.hovered();
                                                    let accent_bg =
                                                        egui::Color32::from_rgba_unmultiplied(
                                                            0x3B, 0x82, 0xF6, 36,
                                                        );
                                                    let hover_bg = match theme {
                                                        Theme::Light => {
                                                            egui::Color32::from_rgb(0xE8, 0xEA, 0xED)
                                                        }
                                                        Theme::Dark => {
                                                            egui::Color32::from_rgb(0x3A, 0x3D, 0x45)
                                                        }
                                                    };
                                                    let bg = if is_sel {
                                                        accent_bg
                                                    } else if hovered {
                                                        hover_bg
                                                    } else {
                                                        egui::Color32::TRANSPARENT
                                                    };
                                                    let stroke = if is_sel {
                                                        egui::Stroke::new(1.0_f32, NEKO_ACCENT)
                                                    } else {
                                                        egui::Stroke::NONE
                                                    };
                                                    ui.painter().rect(
                                                        rect,
                                                        egui::CornerRadius::same(R4),
                                                        bg,
                                                        stroke,
                                                        egui::StrokeKind::Inside,
                                                    );
                                                    if resp.clicked() {
                                                        to_select = Some(p.name.clone());
                                                    }
                                                    let inner =
                                                        rect.shrink2(egui::vec2(4.0_f32, 0.0_f32));
                                                    let name_color = if is_sel {
                                                        NEKO_ACCENT
                                                    } else {
                                                        match theme {
                                                            Theme::Light => egui::Color32::BLACK,
                                                            Theme::Dark => egui::Color32::WHITE,
                                                        }
                                                    };
                                                    let type_color = match theme {
                                                        Theme::Light => egui::Color32::BLACK,
                                                        Theme::Dark => egui::Color32::WHITE,
                                                    };
                                                    let delay_col = delay_color_for(p.delay, theme);
                                                    let delay_str = delay_text(p.delay);
                                                    ui.scope_builder(
                                                        egui::UiBuilder::new().max_rect(inner),
                                                        |ui| {
                                                            ui.columns(2, |cols| {
                                                                cols[0].horizontal(|ui| {
                                                                    ui.spacing_mut().item_spacing =
                                                                        egui::vec2(6.0_f32, 0.0_f32);
                                                                    let weak = match theme {
                                                                        Theme::Light => {
                                                                            egui::Color32::from_rgb(
                                                                                0x6B, 0x72, 0x80,
                                                                            )
                                                                        }
                                                                        Theme::Dark => {
                                                                            egui::Color32::from_rgb(
                                                                                0x9A, 0xA0, 0xA6,
                                                                            )
                                                                        }
                                                                    };
                                                                    ui.label(
                                                                        egui::RichText::new(
                                                                            country_code(&p.name),
                                                                        )
                                                                        .monospace()
                                                                        .size(10.0_f32)
                                                                        .color(weak),
                                                                    );
                                                                    let mut rt =
                                                                        egui::RichText::new(&p.name)
                                                                            .size(11.0_f32)
                                                                            .color(name_color);
                                                                    if is_sel {
                                                                        rt = rt.strong();
                                                                    }
                                                                    ui.add(
                                                                        egui::Label::new(rt).truncate(),
                                                                    );
                                                                });
                                                                cols[1].with_layout(
                                                                egui::Layout::right_to_left(
                                                                    egui::Align::Center,
                                                                ),
                                                                |ui| {
                                                                    ui.spacing_mut().item_spacing =
                                                                        egui::vec2(6.0_f32, 0.0_f32);
                                                                    let (rrect, _) = ui
                                                                        .allocate_exact_size(
                                                                            egui::vec2(
                                                                                14.0_f32, 14.0_f32,
                                                                            ),
                                                                            egui::Sense::hover(),
                                                                        );
                                                                    let ring = if is_sel {
                                                                        NEKO_ACCENT
                                                                    } else {
                                                                        egui::Color32::from_rgb(
                                                                            0x6B, 0x6F, 0x76,
                                                                        )
                                                                    };
                                                                    ui.painter_at(rrect).circle_stroke(
                                                                        rrect.center(),
                                                                        5.5_f32,
                                                                        egui::Stroke::new(
                                                                            1.0_f32, ring,
                                                                        ),
                                                                    );
                                                                    if is_sel {
                                                                        ui.painter_at(rrect)
                                                                            .circle_filled(
                                                                                rrect.center(),
                                                                                3.0_f32,
                                                                                NEKO_ACCENT,
                                                                            );
                                                                    }
                                                                    let delay_rt =
                                                                        egui::RichText::new(delay_str)
                                                                            .monospace()
                                                                            .size(11.0_f32)
                                                                            .color(delay_col);
                                                                    ui.add_sized(
                                                                        egui::vec2(50.0_f32, 14.0_f32),
                                                                        egui::Label::new(delay_rt),
                                                                    );
                                                                    let (sep_rect, _) = ui
                                                                        .allocate_exact_size(
                                                                            egui::vec2(
                                                                                8.0_f32, 14.0_f32,
                                                                            ),
                                                                            egui::Sense::hover(),
                                                                        );
                                                                    let sep_color = match theme {
                                                                        Theme::Light => {
                                                                            egui::Color32::from_rgb(
                                                                                0xD0, 0xD0, 0xD0,
                                                                            )
                                                                        }
                                                                        Theme::Dark => {
                                                                            egui::Color32::from_rgb(
                                                                                0x3C, 0x3F, 0x47,
                                                                            )
                                                                        }
                                                                    };
                                                                    ui.painter().vline(
                                                                        sep_rect.center().x,
                                                                        sep_rect.y_range(),
                                                                        egui::Stroke::new(
                                                                            1.0_f32, sep_color,
                                                                        ),
                                                                    );
                                                                    ui.label(
                                                                        egui::RichText::new(
                                                                            &p.proxy_type,
                                                                        )
                                                                        .monospace()
                                                                        .size(10.0_f32)
                                                                        .color(type_color),
                                                                    );
                                                                },
                                                            );
                                                            });
                                                        },
                                                    );
                                                }
                                            }
                                        });
                                    if let Some(n) = to_select {
                                        self.selected_proxy = Some(n.clone());
                                        log::info!("proxy selected {}", n);
                                        if self.core_child.is_some() && self.core_op.is_none() {
                                            let group = if self.selected_group == "Все группы" {
                                                "PROXY".to_owned()
                                            } else {
                                                self.selected_group.clone()
                                            };
                                            let secret = secret_for_ops();
                                            let base = base_for_ops();
                                            self.core_op = Some(
                                                poll_promise::Promise::spawn_thread(
                                                    "core-select",
                                                    move || {
                                                        match rclash_core_manager::supervisor::select_proxy_blocking(
                                                            &base, &secret, &group, &n,
                                                        ) {
                                                            Ok(()) => CoreOpOutcome::Done,
                                                            Err(e) => CoreOpOutcome::Failed(
                                                                format!("select: {e}"),
                                                            ),
                                                        }
                                                    },
                                                ),
                                            );
                                        }
                                    }
                                    ui.add_space(6.0);
                                    let sep_w = ui.available_width();
                                    let (sep_rect, _) = ui.allocate_exact_size(
                                        egui::vec2(sep_w, 1.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().hline(
                                        sep_rect.x_range(),
                                        sep_rect.center().y,
                                        egui::Stroke::new(1.0_f32, border),
                                    );
                                    ui.add_space(4.0);
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(format!("{count} локаций"))
                                                    .size(10.0)
                                                    .weak(),
                                            );
                                        },
                                    );
                                });
                            });
                        });
                    });
                    }

                    if self.add_menu_open {
                        if let Some(btn_rect) = add_btn_rect {
                            let pos = btn_rect.right_bottom() + egui::vec2(-160.0, 6.0);
                            egui::Area::new(egui::Id::new("add_menu"))
                                .fixed_pos(pos)
                                .order(egui::Order::Foreground)
                                .show(ctx, |ui| {
                                    egui::Frame::new()
                                        .fill(card_fill)
                                        .stroke(egui::Stroke::new(1.0_f32, border))
                                        .corner_radius(egui::CornerRadius::same(R6))
                                        .inner_margin(egui::Margin::same(6))
                                        .shadow(egui::Shadow {
                                            offset: [0, 10],
                                            blur: 30,
                                            spread: 0,
                                            color: egui::Color32::from_black_alpha(80),
                                        })
                                        .show(ui, |ui| {
                                            ui.set_min_width(160.0);
                                            ui.vertical(|ui| {
                                                ui.spacing_mut().item_spacing =
                                                    egui::vec2(0.0, 4.0);
                                                let items: [(&str, &str); 4] = [
                                                    ("Из буфера", "clipboard"),
                                                    ("URL", "url"),
                                                    ("Файл", "file"),
                                                    ("⌨ Сырой ключ", "raw"),
                                                ];
                                                let panel_bg =
                                                    panel_fill_for(self.app_config.theme);
                                                let hover_bg = match self.app_config.theme {
                                                    Theme::Light => {
                                                        egui::Color32::from_rgb(0xE8, 0xEA, 0xED)
                                                    }
                                                    Theme::Dark => {
                                                        egui::Color32::from_rgb(0x3A, 0x3D, 0x45)
                                                    }
                                                };
                                                ui.style_mut().visuals.widgets.inactive.bg_fill =
                                                    panel_bg;
                                                ui.style_mut().visuals.widgets.hovered.bg_fill =
                                                    hover_bg;
                                                ui.style_mut().visuals.widgets.active.bg_fill =
                                                    hover_bg;
                                                for (label, kind) in items {
                                                    if ui
                                                        .add_sized(
                                                            [160.0, 28.0],
                                                            egui::Button::new(
                                                                egui::RichText::new(label)
                                                                    .size(11.0),
                                                            ),
                                                        )
                                                        .clicked()
                                                    {
                                                        match kind {
                                                            "clipboard" => {
                                                                let text = arboard::Clipboard::new()
                                                                    .and_then(|mut cb| cb.get_text())
                                                                    .unwrap_or_default();
                                                                self.open_input(
                                                                    InputContext::Clipboard,
                                                                    text,
                                                                );
                                                            }
                                                            "url" => self.open_input(
                                                                InputContext::Url,
                                                                String::new(),
                                                            ),
                                                            "raw" => self.open_input(
                                                                InputContext::Raw,
                                                                String::new(),
                                                            ),
                                                            _ => {
                                                                self.add_menu_open = false;
                                                                if let Some(path) =
                                                                    rfd::FileDialog::new()
                                                                        .add_filter(
                                                                            "Config",
                                                                            &["yaml", "yml", "txt"],
                                                                        )
                                                                        .pick_file()
                                                                {
                                                                    match std::fs::read_to_string(
                                                                        &path,
                                                                    ) {
                                                                        Ok(content) => {
                                                                            let stem = path
                                                                                .file_stem()
                                                                                .and_then(|s| {
                                                                                    s.to_str()
                                                                                })
                                                                                .unwrap_or(
                                                                                    "import",
                                                                                )
                                                                                .to_owned();
                                                                            self.process_text(
                                                                                &content, &stem,
                                                                            );
                                                                            if !self.input_error.is_empty()
                                                                            {
                                                                                self.input_open =
                                                                                    true;
                                                                                self.input_ctx =
                                                                                    InputContext::Raw;
                                                                                self.input_text =
                                                                                    content;
                                                                            }
                                                                        }
                                                                        Err(e) => {
                                                                            log::warn!(
                                                                                "import file failed: {e}"
                                                                            );
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            });
                                        });
                                });
                            if ctx.input(|i| i.pointer.any_click()) {
                                if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                                    let menu_rect = egui::Rect::from_min_max(
                                        pos - egui::vec2(200.0, 200.0),
                                        pos + egui::vec2(200.0, 200.0),
                                    );
                                    let _ = menu_rect;
                                    if !btn_rect.contains(pos) {
                                        let menu_p =
                                            btn_rect.right_bottom() + egui::vec2(-160.0, 6.0);
                                        let menu_r = egui::Rect::from_min_size(
                                            menu_p,
                                            egui::vec2(172.0, 130.0),
                                        );
                                        if !menu_r.contains(pos) {
                                            self.add_menu_open = false;
                                        }
                                    }
                                }
                            }
                        }
                    }

                });
            });
        self.render_input_dialog(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logger::AppLogEntry;

    fn entry(level: &str, target: &str, message: &str) -> AppLogEntry {
        AppLogEntry {
            time: "18:26:50.771".to_owned(),
            level: level.to_owned(),
            target: target.to_owned(),
            message: message.to_owned(),
        }
    }

    #[test]
    fn log_visible_source_level() {
        let e = entry("info", "rclash_core_manager", "proxy selected");
        assert!(log_entry_visible(LogSource::All, 1, &e));
        assert!(log_entry_visible(LogSource::Core, 1, &e));
        assert!(!log_entry_visible(LogSource::App, 1, &e));
        assert!(!log_entry_visible(LogSource::All, 2, &e));
    }

    #[test]
    fn log_silent_hides_all() {
        let e = entry("error", "rclash", "boom");
        assert!(!log_entry_visible(LogSource::All, 4, &e));
        assert!(log_entry_visible(LogSource::All, 3, &e));
    }
}
