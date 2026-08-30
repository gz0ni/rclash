use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone)]
pub struct Manifest {
    pub version: String,
    #[serde(default, alias = "buildTime")]
    pub build_time: Option<String>,
    #[serde(rename = "coreSha256", alias = "coreSha256", default)]
    pub core_sha256: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub sha256: String,
    pub url: String,
}

pub fn current_triple() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    match (os, arch) {
        ("linux", "x86_64") => "linux-amd64".into(),
        ("linux", "aarch64") => "linux-arm64".into(),
        ("windows", "x86_64") => "windows-amd64".into(),
        ("macos", "x86_64") => "darwin-amd64".into(),
        ("macos", "aarch64") => "darwin-arm64".into(),
        _ => format!("{os}-{arch}"),
    }
}

pub fn manifest_url() -> String {
    "https://github.com/RClash/rclash/releases/download/core-nightly/manifest.json".to_owned()
}

pub fn core_download_url(version: &str, triple: &str) -> String {
    let ext = if triple.starts_with("windows") {
        ".exe"
    } else {
        ""
    };
    format!(
        "https://github.com/RClash/rclash/releases/download/{}/rclash-core-{}{}",
        version, triple, ext
    )
}

pub fn fetch_manifest(url: &str) -> Result<Manifest> {
    log::info!("fetch manifest {}", url);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let resp = client.get(url).send().context("GET manifest")?;
    if !resp.status().is_success() {
        anyhow::bail!("manifest fetch {}: {}", resp.status(), url);
    }
    let m: Manifest = resp.json().context("parse manifest")?;
    log::info!(
        "manifest version {} triples {:?}",
        m.version,
        m.core_sha256.keys()
    );
    Ok(m)
}

pub fn check_for_update(manifest_url: &str) -> Result<Option<UpdateInfo>> {
    let m = fetch_manifest(manifest_url)?;
    let triple = current_triple();
    let sha = m.core_sha256.get(&triple);
    match sha {
        Some(s) => Ok(Some(UpdateInfo {
            version: m.version.clone(),
            sha256: s.clone(),
            url: core_download_url(&m.version, &triple),
        })),
        None => {
            log::warn!("no sha for triple {} in manifest", triple);
            Ok(None)
        }
    }
}

pub fn download_and_verify(url: &str, expected_sha256: &str, dest: &Path) -> Result<()> {
    log::info!("download {} -> {}", url, dest.display());
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let mut resp = client.get(url).send().context("GET core")?;
    if !resp.status().is_success() {
        anyhow::bail!("download {}: {}", resp.status(), url);
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = dest.with_extension("tmp");
    let mut file = std::fs::File::create(&tmp)?;
    use std::io::Write;
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    let mut buf = [0u8; 8192];
    loop {
        use std::io::Read;
        let n = resp.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        file.write_all(&buf[..n])?;
    }
    file.flush()?;
    drop(file);
    let got = format!("{:x}", hasher.finalize());
    if got.to_lowercase() != expected_sha256.to_lowercase() {
        let _ = std::fs::remove_file(&tmp);
        anyhow::bail!("sha256 mismatch expected {} got {}", expected_sha256, got);
    }
    log::info!("sha256 verified {}", got);
    if dest.exists() {
        let _ = std::fs::remove_file(dest);
    }
    std::fs::rename(&tmp, dest)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o755));
    }
    Ok(())
}

pub fn core_bin_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("RClash").join("rclash-core"))
}

pub fn should_check(last_check: Option<&str>, interval: rclash_config::UpdateInterval) -> bool {
    if interval == rclash_config::UpdateInterval::Manual {
        return false;
    }
    let Some(secs) = interval.duration_secs() else {
        return false;
    };
    let Some(s) = last_check else {
        return true;
    };
    if let Ok(t) = parse_last_check(s) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        return now.saturating_sub(t) >= secs;
    }
    true
}

fn parse_last_check(s: &str) -> Result<u64> {
    let s = s.trim().trim_matches('"');
    if let Ok(v) = s.parse::<u64>() {
        return Ok(v);
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Ok(dt.timestamp() as u64);
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ") {
        return Ok(dt.and_utc().timestamp() as u64);
    }
    anyhow::bail!("parse time {s}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn triple_not_empty() {
        assert!(!current_triple().is_empty());
    }
    #[test]
    fn manifest_url_contains_core_nightly() {
        assert!(manifest_url().contains("core-nightly"));
    }
    #[test]
    fn core_download_url_format() {
        let u = core_download_url("v0.1.0", "linux-amd64");
        assert!(u.contains("rclash-core-linux-amd64"));
        let uw = core_download_url("v0.1.0", "windows-amd64");
        assert!(uw.ends_with(".exe"));
    }
    #[test]
    fn should_check_manual_false() {
        assert!(!should_check(None, rclash_config::UpdateInterval::Manual));
        assert!(!should_check(
            Some("0"),
            rclash_config::UpdateInterval::Manual
        ));
    }
    #[test]
    fn should_check_interval() {
        assert!(should_check(None, rclash_config::UpdateInterval::H24));
    }
}
