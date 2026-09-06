use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub const GEOSITE_JSDELIVR: &str =
    "https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geosite.dat";
pub const GEOIP_JSDELIVR: &str =
    "https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geoip.dat";
pub const GEOSITE_GITHUB: &str =
    "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geosite.dat";
pub const GEOIP_GITHUB: &str =
    "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geoip.dat";

pub fn geodata_paths(home: &Path) -> (PathBuf, PathBuf) {
    (home.join("GeoSite.dat"), home.join("GeoIP.dat"))
}

fn download_one(url: &str, dest: &Path) -> Result<()> {
    log::info!("geodata download {url}");
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let resp = client.get(url).send().context("GET geodata")?;
    if !resp.status().is_success() {
        anyhow::bail!("geodata fetch {}: {}", resp.status(), url);
    }
    let bytes = resp.bytes().context("read geodata body")?;
    if bytes.is_empty() {
        anyhow::bail!("geodata empty: {url}");
    }
    rclash_config::atomic_write(dest, &bytes)?;
    Ok(())
}

fn ensure_one(name: &str, dest: &Path, urls: &[&str], force: bool) -> Result<bool> {
    if !force && dest.exists() {
        if let Ok(meta) = std::fs::metadata(dest) {
            if meta.len() > 0 {
                return Ok(false);
            }
        }
    }
    let mut last_err = anyhow::anyhow!("no urls");
    for url in urls {
        match download_one(url, dest) {
            Ok(()) => {
                log::info!("geodata {name} ok");
                return Ok(true);
            }
            Err(e) => {
                log::warn!("geodata {name} failed: {e}");
                last_err = e;
            }
        }
    }
    Err(last_err.context(format!("geodata {name} download failed")))
}

pub fn ensure_geodata(home: &Path, force: bool) -> Result<bool> {
    std::fs::create_dir_all(home)?;
    let (site, ip) = geodata_paths(home);
    let site_dl = ensure_one(
        "GeoSite.dat",
        &site,
        &[GEOSITE_JSDELIVR, GEOSITE_GITHUB],
        force,
    )?;
    let ip_dl = ensure_one("GeoIP.dat", &ip, &[GEOIP_JSDELIVR, GEOIP_GITHUB], force)?;
    Ok(site_dl || ip_dl)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geodata_urls_point_at_meta_rules_dat() {
        for u in [
            GEOSITE_JSDELIVR,
            GEOIP_JSDELIVR,
            GEOSITE_GITHUB,
            GEOIP_GITHUB,
        ] {
            assert!(u.contains("meta-rules-dat"), "{u}");
        }
        assert!(GEOSITE_JSDELIVR.ends_with("geosite.dat"));
        assert!(GEOIP_JSDELIVR.ends_with("geoip.dat"));
    }

    #[test]
    fn geodata_paths_use_core_filenames() {
        let (site, ip) = geodata_paths(Path::new("/tmp/x"));
        assert_eq!(site.file_name().unwrap(), "GeoSite.dat");
        assert_eq!(ip.file_name().unwrap(), "GeoIP.dat");
    }

    #[test]
    fn ensure_one_skips_existing_without_network() {
        let dir = std::env::temp_dir().join("rclash-geodata-test");
        let _ = std::fs::create_dir_all(&dir);
        let dest = dir.join("GeoSite.dat");
        std::fs::write(&dest, b"dummy").unwrap();
        let dl = ensure_one("GeoSite.dat", &dest, &["http://127.0.0.1:9/x"], false).unwrap();
        assert!(!dl);
        let _ = std::fs::remove_file(&dest);
    }
}
