use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub mod custom;
pub mod profile;
pub mod runtime;

pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("RClash"))
}

pub fn ensure_config_dir() -> anyhow::Result<PathBuf> {
    let dir = config_dir().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn validate_yaml(path: &Path) -> anyhow::Result<serde_yaml::Value> {
    let s = std::fs::read_to_string(path)?;
    let v: serde_yaml::Value = serde_yaml::from_str(&s)?;
    Ok(v)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    #[default]
    Dark,
}

impl Theme {
    pub fn label_ru(&self) -> &'static str {
        match self {
            Self::Light => "Светлая",
            Self::Dark => "Тёмная",
        }
    }
    pub fn all() -> &'static [Self] {
        &[Self::Light, Self::Dark]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UpdateInterval {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "30m")]
    Min30,
    #[serde(rename = "1h")]
    H1,
    #[serde(rename = "6h")]
    H6,
    #[serde(rename = "12h")]
    H12,
    #[default]
    #[serde(rename = "24h")]
    H24,
}

impl UpdateInterval {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Manual => "manual",
            Self::Min30 => "30m",
            Self::H1 => "1h",
            Self::H6 => "6h",
            Self::H12 => "12h",
            Self::H24 => "24h",
        }
    }

    pub fn label_ru(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Manual => "off",
            Self::Min30 => "30м",
            Self::H1 => "1ч",
            Self::H6 => "6ч",
            Self::H12 => "12ч",
            Self::H24 => "24ч",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Auto,
            Self::Manual,
            Self::Min30,
            Self::H1,
            Self::H6,
            Self::H12,
            Self::H24,
        ]
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "auto" | "авто" => Some(Self::Auto),
            "manual" | "вручную" | "off" | "0" => Some(Self::Manual),
            "30m" | "30м" => Some(Self::Min30),
            "1h" | "1ч" => Some(Self::H1),
            "6h" | "6ч" => Some(Self::H6),
            "12h" | "12ч" => Some(Self::H12),
            "24h" | "24ч" => Some(Self::H24),
            _ => None,
        }
    }

    pub fn duration_secs(&self) -> Option<u64> {
        match self {
            Self::Auto | Self::Manual => None,
            Self::Min30 => Some(30 * 60),
            Self::H1 => Some(3600),
            Self::H6 => Some(6 * 3600),
            Self::H12 => Some(12 * 3600),
            Self::H24 => Some(24 * 3600),
        }
    }

    pub fn effective(subscription: Option<Self>, app: Self) -> Self {
        subscription.unwrap_or(app)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Silent,
    Error,
    Warning,
    #[default]
    Info,
    Debug,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Silent => "silent",
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }

    pub fn label_ru(&self) -> &'static str {
        match self {
            Self::Silent => "Тихо",
            Self::Error => "Ошибки",
            Self::Warning => "Предупр.",
            Self::Info => "Инфо",
            Self::Debug => "Отладка",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Silent,
            Self::Error,
            Self::Warning,
            Self::Info,
            Self::Debug,
        ]
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "silent" | "тихо" => Some(Self::Silent),
            "error" | "ошибки" => Some(Self::Error),
            "warning" | "warn" | "предупр." | "предупреждения" => {
                Some(Self::Warning)
            }
            "info" | "инфо" => Some(Self::Info),
            "debug" | "отладка" => Some(Self::Debug),
            _ => None,
        }
    }

    pub fn to_log_filter(&self) -> log::LevelFilter {
        match self {
            Self::Silent => log::LevelFilter::Off,
            Self::Error => log::LevelFilter::Error,
            Self::Warning => log::LevelFilter::Warn,
            Self::Info => log::LevelFilter::Info,
            Self::Debug => log::LevelFilter::Debug,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CoreMode {
    #[default]
    Rule,
    Global,
    Direct,
}

impl CoreMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rule => "rule",
            Self::Global => "global",
            Self::Direct => "direct",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::Rule, Self::Global, Self::Direct]
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "rule" => Some(Self::Rule),
            "global" => Some(Self::Global),
            "direct" => Some(Self::Direct),
            _ => None,
        }
    }
}

fn default_enhanced_mode() -> String {
    "fake-ip".to_owned()
}

fn default_dns_listen() -> String {
    "0.0.0.0:1053".to_owned()
}

fn default_fake_ip_range() -> String {
    "198.18.0.1/16".to_owned()
}

fn default_nameservers() -> Vec<String> {
    vec!["223.5.5.5".to_owned(), "8.8.8.8".to_owned()]
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DnsConfig {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default = "default_enhanced_mode")]
    pub enhanced_mode: String,
    #[serde(default = "default_dns_listen")]
    pub listen: String,
    #[serde(default)]
    pub ipv6: bool,
    #[serde(default = "default_fake_ip_range")]
    pub fake_ip_range: String,
    #[serde(default = "default_nameservers")]
    pub nameservers: Vec<String>,
    #[serde(default)]
    pub fallback: Vec<String>,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            enable: true,
            enhanced_mode: default_enhanced_mode(),
            listen: default_dns_listen(),
            ipv6: false,
            fake_ip_range: default_fake_ip_range(),
            nameservers: default_nameservers(),
            fallback: Vec::new(),
        }
    }
}

impl DnsConfig {
    pub fn ui_mode(&self) -> &'static str {
        if self.enhanced_mode == "redir-host" {
            "RedirHost"
        } else {
            "FakeIP"
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "enable": self.enable,
            "listen": self.listen,
            "ipv6": self.ipv6,
            "enhanced-mode": self.enhanced_mode,
            "fake-ip-range": self.fake_ip_range,
            "nameserver": self.nameservers,
            "fallback": self.fallback,
        })
    }
}

pub fn api_base(external_controller: &Option<String>) -> String {
    let ctrl = external_controller
        .clone()
        .unwrap_or_else(|| "127.0.0.1:9090".to_owned());
    format!("http://{ctrl}")
}

pub fn validate_port_text(s: &str) -> Option<u16> {
    s.trim().parse::<u16>().ok()
}

pub fn validate_listen(s: &str) -> bool {
    let s = s.trim();
    let Some((_host, port)) = s.rsplit_once(':') else {
        return false;
    };
    if port.is_empty() {
        return false;
    }
    port.parse::<u16>().is_ok()
}

pub fn validate_cidr(s: &str) -> bool {
    let s = s.trim();
    let Some((ip, mask)) = s.split_once('/') else {
        return false;
    };
    let Ok(addr) = ip.parse::<std::net::IpAddr>() else {
        return false;
    };
    let Ok(bits) = mask.parse::<u8>() else {
        return false;
    };
    match addr {
        std::net::IpAddr::V4(_) => bits <= 32,
        std::net::IpAddr::V6(_) => bits <= 128,
    }
}

pub fn parse_nameserver_list(s: &str) -> Vec<String> {
    s.split([',', '\n', ';'])
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect()
}

pub fn parse_hosts_text(s: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in s.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (k, v) = match line.split_once('=') {
            Some((a, b)) => (a.trim(), b.trim()),
            None => match line.split_once(char::is_whitespace) {
                Some((a, b)) => (a.trim(), b.trim()),
                None => continue,
            },
        };
        if k.is_empty() || v.parse::<std::net::IpAddr>().is_err() {
            continue;
        }
        out.insert(k.to_owned(), v.to_owned());
    }
    out
}

pub fn hosts_to_text(hosts: &BTreeMap<String, String>) -> String {
    hosts
        .iter()
        .map(|(k, v)| format!("{k} = {v}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub theme: Theme,
    #[serde(default = "default_true")]
    pub minimize_to_tray: bool,
    #[serde(default)]
    pub skipped_version: Option<String>,
    #[serde(default)]
    pub last_check: Option<String>,
    #[serde(default)]
    pub update_interval: UpdateInterval,
    #[serde(default)]
    pub log_level: LogLevel,
    #[serde(default)]
    pub tun_enabled: bool,
    #[serde(default)]
    pub proxy_enabled: bool,
    #[serde(default)]
    pub master_enabled: bool,
    #[serde(default = "default_true")]
    pub show_traffic_graph: bool,
    #[serde(default)]
    pub mixed_port: Option<u16>,
    #[serde(default)]
    pub socks_port: Option<u16>,
    #[serde(default)]
    pub external_controller: Option<String>,
    #[serde(default)]
    pub allow_lan: Option<bool>,
    #[serde(default)]
    pub ipv6: Option<bool>,
    #[serde(default)]
    pub unified_delay: Option<bool>,
    #[serde(default)]
    pub tcp_concurrent: Option<bool>,
    #[serde(default)]
    pub keep_alive_interval: Option<u32>,
    #[serde(default)]
    pub geodata_loader: Option<String>,
    #[serde(default)]
    pub find_process_mode: Option<String>,
    #[serde(default)]
    pub core_secret: Option<String>,
    #[serde(default)]
    pub mode: CoreMode,
    #[serde(default)]
    pub dns: DnsConfig,
    #[serde(default)]
    pub hosts: BTreeMap<String, String>,
    #[serde(default)]
    pub geodata_strict: bool,
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            minimize_to_tray: true,
            skipped_version: None,
            last_check: None,
            update_interval: UpdateInterval::H24,
            log_level: LogLevel::Info,
            tun_enabled: false,
            proxy_enabled: false,
            master_enabled: false,
            show_traffic_graph: true,
            mixed_port: None,
            socks_port: None,
            external_controller: None,
            allow_lan: None,
            ipv6: None,
            unified_delay: None,
            tcp_concurrent: None,
            keep_alive_interval: None,
            geodata_loader: None,
            find_process_mode: None,
            core_secret: None,
            mode: CoreMode::Rule,
            dns: DnsConfig::default(),
            hosts: BTreeMap::new(),
            geodata_strict: false,
        }
    }
}

pub fn app_config_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("app.json"))
}

pub fn load_app_config() -> AppConfig {
    let Some(p) = app_config_path() else {
        return AppConfig::default();
    };
    if !p.exists() {
        return AppConfig::default();
    }
    let Ok(s) = std::fs::read_to_string(&p) else {
        return AppConfig::default();
    };
    serde_json::from_str(&s).unwrap_or_default()
}

pub fn save_app_config(cfg: &AppConfig) -> anyhow::Result<()> {
    let dir = ensure_config_dir()?;
    let path = dir.join("app.json");
    let s = serde_json::to_string_pretty(cfg)?;
    atomic_write(&path, s.as_bytes())?;
    Ok(())
}

pub fn ensure_core_secret(cfg: &mut AppConfig) -> anyhow::Result<String> {
    if let Some(ref s) = cfg.core_secret {
        if !s.is_empty() {
            return Ok(s.clone());
        }
    }
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut h1 = DefaultHasher::new();
    (nanos, std::process::id(), 1u8).hash(&mut h1);
    let mut h2 = DefaultHasher::new();
    (nanos.reverse_bits(), std::process::id(), 2u8).hash(&mut h2);
    let secret = format!("{:016x}{:016x}", h1.finish(), h2.finish());
    cfg.core_secret = Some(secret.clone());
    save_app_config(cfg)?;
    Ok(secret)
}

pub fn atomic_write(path: &Path, data: &[u8]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data)?;
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_dir_ends_with_rclash() {
        let dir = config_dir().unwrap();
        assert!(dir.ends_with("RClash"));
    }

    #[test]
    fn app_config_roundtrip() {
        let cfg = AppConfig {
            theme: Theme::Light,
            minimize_to_tray: false,
            skipped_version: Some("v1.2.3".into()),
            last_check: Some("2026-08-30T00:00:00Z".into()),
            update_interval: UpdateInterval::H12,
            log_level: LogLevel::Debug,
            tun_enabled: true,
            proxy_enabled: true,
            master_enabled: true,
            show_traffic_graph: false,
            mixed_port: Some(7890),
            socks_port: None,
            external_controller: None,
            allow_lan: None,
            ipv6: None,
            unified_delay: None,
            tcp_concurrent: None,
            keep_alive_interval: None,
            geodata_loader: None,
            find_process_mode: None,
            core_secret: None,
            mode: CoreMode::Global,
            dns: DnsConfig::default(),
            hosts: BTreeMap::new(),
            geodata_strict: true,
        };
        let s = serde_json::to_string(&cfg).unwrap();
        let back: AppConfig = serde_json::from_str(&s).unwrap();
        assert_eq!(back.theme, Theme::Light);
        assert!(!back.minimize_to_tray);
        assert_eq!(back.skipped_version.as_deref(), Some("v1.2.3"));
        assert_eq!(back.update_interval, UpdateInterval::H12);
        assert_eq!(back.log_level, LogLevel::Debug);
        assert!(back.tun_enabled);
        assert!(!back.show_traffic_graph);
        assert_eq!(back.mode, CoreMode::Global);
        assert!(back.dns.enable);
        assert_eq!(back.dns.enhanced_mode, "fake-ip");
        assert!(back.geodata_strict);
    }

    #[test]
    fn app_config_default_is_dark() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.theme, Theme::Dark);
        assert!(cfg.minimize_to_tray);
        assert_eq!(cfg.update_interval, UpdateInterval::H24);
        assert!(cfg.skipped_version.is_none());
        assert_eq!(cfg.log_level, LogLevel::Info);
        assert!(!cfg.tun_enabled);
    }

    #[test]
    fn log_level_roundtrip() {
        assert_eq!(LogLevel::from_str("debug"), Some(LogLevel::Debug));
        assert_eq!(LogLevel::from_str("INFO"), Some(LogLevel::Info));
        assert_eq!(LogLevel::Info.as_str(), "info");
        assert_eq!(LogLevel::Warning.label_ru(), "Предупр.");
        assert_eq!(LogLevel::Info.to_log_filter(), log::LevelFilter::Info);
    }

    #[test]
    fn update_interval_effective() {
        assert_eq!(
            UpdateInterval::effective(Some(UpdateInterval::H1), UpdateInterval::H24),
            UpdateInterval::H1
        );
        assert_eq!(
            UpdateInterval::effective(None, UpdateInterval::H6),
            UpdateInterval::H6
        );
    }

    #[test]
    fn update_interval_from_str() {
        assert_eq!(UpdateInterval::from_str("30m"), Some(UpdateInterval::Min30));
        assert_eq!(
            UpdateInterval::from_str("manual"),
            Some(UpdateInterval::Manual)
        );
        assert_eq!(UpdateInterval::from_str("24ч"), Some(UpdateInterval::H24));
        assert_eq!(UpdateInterval::from_str("unknown"), None);
    }

    #[test]
    fn update_interval_duration() {
        assert_eq!(UpdateInterval::Manual.duration_secs(), None);
        assert_eq!(UpdateInterval::H1.duration_secs(), Some(3600));
        assert_eq!(UpdateInterval::H24.duration_secs(), Some(86400));
    }

    #[test]
    fn core_mode_roundtrip() {
        assert_eq!(CoreMode::from_str("global"), Some(CoreMode::Global));
        assert_eq!(CoreMode::from_str("DIRECT"), Some(CoreMode::Direct));
        assert_eq!(CoreMode::from_str("nope"), None);
        assert_eq!(CoreMode::Rule.as_str(), "rule");
        assert_eq!(CoreMode::all().len(), 3);
    }

    #[test]
    fn legacy_app_json_loads_with_mode_defaults() {
        let back: AppConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(back.mode, CoreMode::Rule);
        assert!(back.dns.enable);
        assert!(back.hosts.is_empty());
    }

    #[test]
    fn api_base_default_and_custom() {
        assert_eq!(api_base(&None), "http://127.0.0.1:9090");
        assert_eq!(
            api_base(&Some("127.0.0.1:9091".to_owned())),
            "http://127.0.0.1:9091"
        );
    }

    #[test]
    fn validators() {
        assert_eq!(validate_port_text("7890"), Some(7890));
        assert_eq!(validate_port_text("0"), Some(0));
        assert_eq!(validate_port_text("99999"), None);
        assert_eq!(validate_port_text("abc"), None);
        assert!(validate_listen("0.0.0.0:1053"));
        assert!(validate_listen(":1053"));
        assert!(!validate_listen("1053"));
        assert!(!validate_listen("host:abc"));
        assert!(validate_cidr("198.18.0.1/16"));
        assert!(validate_cidr("::1/128"));
        assert!(!validate_cidr("198.18.0.1/33"));
        assert!(!validate_cidr("not-a-cidr"));
    }

    #[test]
    fn nameserver_and_hosts_parsing() {
        let ns = parse_nameserver_list("223.5.5.5, 8.8.8.8\n1.1.1.1");
        assert_eq!(ns, vec!["223.5.5.5", "8.8.8.8", "1.1.1.1"]);
        let hosts = parse_hosts_text("example.com = 1.2.3.4\nbad-line\nx.io 5.6.7.8");
        assert_eq!(
            hosts.get("example.com").map(String::as_str),
            Some("1.2.3.4")
        );
        assert_eq!(hosts.get("x.io").map(String::as_str), Some("5.6.7.8"));
        assert!(!hosts.contains_key("bad-line"));
        assert!(hosts_to_text(&hosts).contains("example.com = 1.2.3.4"));
    }
}
