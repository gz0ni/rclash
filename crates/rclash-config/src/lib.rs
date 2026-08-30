use std::path::{Path, PathBuf};

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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_dir_ends_with_rclash() {
        let dir = config_dir().unwrap();
        assert!(dir.ends_with("RClash"));
    }
}
