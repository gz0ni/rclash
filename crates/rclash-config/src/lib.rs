use std::path::{Path, PathBuf};

pub mod custom;
pub mod profile;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UpdateInterval {
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
            Self::Manual => "Вручную",
            Self::Min30 => "30м",
            Self::H1 => "1ч",
            Self::H6 => "6ч",
            Self::H12 => "12ч",
            Self::H24 => "24ч",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
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
            "manual" | "вручную" | "0" => Some(Self::Manual),
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
            Self::Manual => None,
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
        };
        let s = serde_json::to_string(&cfg).unwrap();
        let back: AppConfig = serde_json::from_str(&s).unwrap();
        assert_eq!(back.theme, Theme::Light);
        assert!(!back.minimize_to_tray);
        assert_eq!(back.skipped_version.as_deref(), Some("v1.2.3"));
        assert_eq!(back.update_interval, UpdateInterval::H12);
        assert_eq!(back.log_level, LogLevel::Debug);
        assert!(back.tun_enabled);
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
}
