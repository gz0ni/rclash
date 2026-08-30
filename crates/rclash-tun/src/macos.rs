use crate::{common, TunStatus};
use anyhow::Result;

pub fn enable() -> Result<()> {
    log::info!("macOS TUN enable via osascript");
    common::run_helper_osascript(&["up"])?;
    Ok(())
}

pub fn disable() -> Result<()> {
    log::info!("macOS TUN disable via osascript");
    common::run_helper_osascript(&["down"])?;
    Ok(())
}

pub fn status() -> TunStatus {
    match common::run_helper(&["status"]) {
        Ok(s) => {
            let t = s.trim().to_lowercase();
            if t.contains("enabled") || t.contains("up") {
                TunStatus::Enabled {
                    name: "utun3".into(),
                }
            } else {
                TunStatus::Disabled
            }
        }
        Err(e) => TunStatus::Error(format!("{e}")),
    }
}

pub fn helper_up() -> Result<()> {
    log::info!("helper up: macOS utun");
    let out = std::process::Command::new("ifconfig").arg("utun3").output();
    if let Ok(o) = out {
        if o.status.success() {
            log::info!("utun3 already exists");
            return Ok(());
        }
    }
    let dev = std::process::Command::new("ifconfig")
        .args(["utun3", "create"])
        .output();
    if let Ok(o) = dev {
        log::info!(
            "ifconfig utun3 create: {}",
            String::from_utf8_lossy(&o.stdout)
        );
    }
    let _ = std::process::Command::new("ifconfig")
        .args(["utun3", "inet", "198.18.0.1", "198.18.0.2", "up"])
        .output();
    let _ = std::process::Command::new("route")
        .args(["add", "-net", "198.18.0.0/16", "-interface", "utun3"])
        .output();
    Ok(())
}

pub fn helper_down() -> Result<()> {
    log::info!("helper down: macOS utun");
    let _ = std::process::Command::new("ifconfig")
        .args(["utun3", "destroy"])
        .output();
    let _ = std::process::Command::new("route")
        .args(["delete", "-net", "198.18.0.0/16"])
        .output();
    Ok(())
}

pub fn helper_status() -> String {
    let out = std::process::Command::new("ifconfig").arg("utun3").output();
    match out {
        Ok(o) if o.status.success() => "enabled".to_owned(),
        _ => "disabled".to_owned(),
    }
}
