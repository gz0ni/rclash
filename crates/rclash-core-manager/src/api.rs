use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct CoreApi {
    base: String,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
pub struct VersionInfo {
    pub version: String,
}

impl CoreApi {
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_default() -> Self {
        Self::new("http://127.0.0.1:9090")
    }

    pub async fn version(&self) -> anyhow::Result<VersionInfo> {
        let url = format!("{}/version", self.base);
        let v = self
            .client
            .get(url)
            .send()
            .await?
            .json::<VersionInfo>()
            .await?;
        Ok(v)
    }

    pub async fn is_alive(&self) -> bool {
        self.version().await.is_ok()
    }
}
