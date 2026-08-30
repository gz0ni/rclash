use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct CoreApi {
    base: String,
    secret: Option<String>,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
pub struct VersionInfo {
    pub version: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct TrafficInfo {
    #[serde(default)]
    pub up: u64,
    #[serde(default)]
    pub down: u64,
    #[serde(default, alias = "upTotal")]
    pub up_total: u64,
    #[serde(default, alias = "downTotal")]
    pub down_total: u64,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Snapshot {
    #[serde(default, alias = "downloadTotal")]
    pub download_total: i64,
    #[serde(default, alias = "uploadTotal")]
    pub upload_total: i64,
    #[serde(default)]
    pub connections: Vec<TrackerInfo>,
    #[serde(default)]
    pub memory: u64,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct TrackerInfo {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub upload: i64,
    #[serde(default)]
    pub download: i64,
    #[serde(default)]
    pub start: String,
    #[serde(default)]
    pub chains: Vec<String>,
    #[serde(default)]
    pub rule: String,
    #[serde(default, alias = "rulePayload")]
    pub rule_payload: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct LogEntry {
    #[serde(rename = "type", alias = "level", default)]
    pub level: String,
    #[serde(alias = "payload", alias = "message", default)]
    pub payload: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionsSort {
    StartDesc,
    UploadDesc,
    DownloadDesc,
    HostAsc,
}

impl ConnectionsSort {
    pub fn label_ru(&self) -> &'static str {
        match self {
            Self::StartDesc => "Время ↓",
            Self::UploadDesc => "↑ Трафик ↓",
            Self::DownloadDesc => "↓ Трафик ↓",
            Self::HostAsc => "Хост ↑",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::StartDesc,
            Self::UploadDesc,
            Self::DownloadDesc,
            Self::HostAsc,
        ]
    }
}

pub fn format_bytes(b: u64) -> String {
    if b < 1024 {
        format!("{b} B")
    } else if b < 1_048_576 {
        format!("{:.1} KB", b as f64 / 1024.0)
    } else if b < 1_073_741_824 {
        format!("{:.1} MB", b as f64 / 1_048_576.0)
    } else {
        format!("{:.2} GB", b as f64 / 1_073_741_824.0)
    }
}

pub fn log_level_color(level: &str) -> egui::Color32 {
    match level.to_ascii_lowercase().as_str() {
        "error" => egui::Color32::from_rgb(220, 80, 80),
        "warning" | "warn" => egui::Color32::from_rgb(220, 180, 60),
        "debug" => egui::Color32::from_rgb(140, 140, 140),
        "info" => egui::Color32::from_rgb(120, 180, 255),
        _ => egui::Color32::from_gray(180),
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ProxiesInfo {
    #[serde(default)]
    pub proxies: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    Rule,
    Global,
    Direct,
}

impl ProxyMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rule => "rule",
            Self::Global => "global",
            Self::Direct => "direct",
        }
    }

    pub fn label_ru(&self) -> &'static str {
        match self {
            Self::Rule => "Правило",
            Self::Global => "Глобальный",
            Self::Direct => "Прямой",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::Rule, Self::Global, Self::Direct]
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "rule" | "правило" => Some(Self::Rule),
            "global" | "глобальный" => Some(Self::Global),
            "direct" | "прямой" => Some(Self::Direct),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DelayResult {
    pub delay: Option<u64>,
    #[serde(default)]
    pub message: Option<String>,
}

pub fn delay_color(delay_ms: Option<u64>) -> egui::Color32 {
    match delay_ms {
        None => egui::Color32::from_gray(120),
        Some(d) if d < 100 => egui::Color32::from_rgb(80, 200, 120),
        Some(d) if d < 300 => egui::Color32::from_rgb(220, 180, 60),
        Some(_) => egui::Color32::from_rgb(220, 80, 80),
    }
}

pub fn delay_text(delay_ms: Option<u64>) -> String {
    match delay_ms {
        None => "—".to_owned(),
        Some(d) => format!("{d} ms"),
    }
}

impl CoreApi {
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            secret: None,
            client: reqwest::Client::new(),
        }
    }

    pub fn with_secret(mut self, secret: impl Into<String>) -> Self {
        self.secret = Some(secret.into());
        self
    }

    pub fn with_default() -> Self {
        Self::new("http://127.0.0.1:9090")
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(s) = &self.secret {
            req.header("Authorization", format!("Bearer {s}"))
        } else {
            req
        }
    }

    pub async fn version(&self) -> anyhow::Result<VersionInfo> {
        let url = format!("{}/version", self.base);
        let v = self.auth(self.client.get(url)).send().await?.json().await?;
        Ok(v)
    }

    pub async fn is_alive(&self) -> bool {
        self.version().await.is_ok()
    }

    pub async fn traffic(&self) -> anyhow::Result<TrafficInfo> {
        let url = format!("{}/traffic", self.base);
        let v = self.auth(self.client.get(url)).send().await?.json().await?;
        Ok(v)
    }

    pub async fn proxies(&self) -> anyhow::Result<serde_json::Value> {
        let url = format!("{}/proxies", self.base);
        let v = self.auth(self.client.get(url)).send().await?.json().await?;
        Ok(v)
    }

    pub async fn connections(&self) -> anyhow::Result<serde_json::Value> {
        let url = format!("{}/connections", self.base);
        let v = self.auth(self.client.get(url)).send().await?.json().await?;
        Ok(v)
    }

    pub async fn reload(&self) -> anyhow::Result<()> {
        let url = format!("{}/configs?force=true", self.base);
        self.auth(self.client.put(url))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn get_configs(&self) -> anyhow::Result<serde_json::Value> {
        let url = format!("{}/configs", self.base);
        let v = self.auth(self.client.get(url)).send().await?.json().await?;
        Ok(v)
    }

    pub async fn set_mode(&self, mode: ProxyMode) -> anyhow::Result<()> {
        let url = format!("{}/configs", self.base);
        let body = serde_json::json!({"mode": mode.as_str()});
        self.auth(self.client.patch(url).json(&body))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn get_mode(&self) -> anyhow::Result<ProxyMode> {
        let cfg = self.get_configs().await?;
        let mode_str = cfg.get("mode").and_then(|v| v.as_str()).unwrap_or("rule");
        Ok(ProxyMode::from_str(mode_str).unwrap_or(ProxyMode::Rule))
    }

    pub async fn test_delay(
        &self,
        proxy_name: &str,
        test_url: &str,
        timeout_ms: u64,
    ) -> anyhow::Result<DelayResult> {
        let url = format!(
            "{}/proxies/{}/delay?url={}&timeout={}",
            self.base,
            percent_encode(proxy_name),
            percent_encode(test_url),
            timeout_ms
        );
        let resp = self.auth(self.client.get(url)).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("delay check failed {status}: {text}");
        }
        let v: DelayResult = resp.json().await?;
        Ok(v)
    }

    pub async fn close_connection(&self, id: &str) -> anyhow::Result<()> {
        let url = format!("{}/connections/{}", self.base, percent_encode(id));
        self.auth(self.client.delete(url))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn close_all_connections(&self) -> anyhow::Result<()> {
        let url = format!("{}/connections", self.base);
        self.auth(self.client.delete(url))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proxy_mode_roundtrip() {
        assert_eq!(ProxyMode::from_str("rule"), Some(ProxyMode::Rule));
        assert_eq!(ProxyMode::from_str("Global"), Some(ProxyMode::Global));
        assert_eq!(ProxyMode::Rule.as_str(), "rule");
        assert_eq!(ProxyMode::Global.label_ru(), "Глобальный");
    }

    #[test]
    fn delay_color_thresholds() {
        assert_eq!(delay_color(None), egui::Color32::from_gray(120));
        assert_eq!(delay_color(Some(50)), egui::Color32::from_rgb(80, 200, 120));
        assert_eq!(
            delay_color(Some(150)),
            egui::Color32::from_rgb(220, 180, 60)
        );
        assert_eq!(delay_color(Some(500)), egui::Color32::from_rgb(220, 80, 80));
    }

    #[test]
    fn percent_encode_basic() {
        assert_eq!(percent_encode("hello world"), "hello%20world");
        assert_eq!(percent_encode("PROXY"), "PROXY");
    }

    #[test]
    fn traffic_info_deser() {
        let s = r#"{"up":123,"down":456,"upTotal":789,"downTotal":101}"#;
        let v: TrafficInfo = serde_json::from_str(s).unwrap();
        assert_eq!(v.up, 123);
        assert_eq!(v.down, 456);
        assert_eq!(v.up_total, 789);
        assert_eq!(v.down_total, 101);
    }

    #[test]
    fn snapshot_deser() {
        let s = r#"{"downloadTotal":100,"uploadTotal":200,"connections":[{"id":"abc","metadata":{"host":"example.com"},"upload":10,"download":20,"start":"2026-01-01T00:00:00Z","chains":["PROXY"],"rule":"DOMAIN","rulePayload":"example.com"}],"memory":12345}"#;
        let v: Snapshot = serde_json::from_str(s).unwrap();
        assert_eq!(v.connections.len(), 1);
        assert_eq!(v.connections[0].id, "abc");
        assert_eq!(v.connections[0].chains[0], "PROXY");
    }

    #[test]
    fn log_entry_deser() {
        let s = r#"{"type":"info","payload":"hello"}"#;
        let v: LogEntry = serde_json::from_str(s).unwrap();
        assert_eq!(v.level, "info");
        assert_eq!(v.payload, "hello");
    }

    #[test]
    fn format_bytes_cases() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(2_097_152), "2.0 MB");
    }

    #[test]
    fn connections_sort_labels() {
        assert_eq!(ConnectionsSort::StartDesc.label_ru(), "Время ↓");
        assert_eq!(ConnectionsSort::all().len(), 4);
    }
}
