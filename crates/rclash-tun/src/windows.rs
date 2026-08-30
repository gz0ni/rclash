use crate::{common, TunStatus};
use anyhow::Result;

pub fn enable() -> Result<()> {
    log::info!("Windows TUN enable via helper");
    let helper = common::helper_path();
    let out = std::process::Command::new(&helper).arg("up").output()?;
    if !out.status.success() {
        anyhow::bail!("helper up failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(())
}

pub fn disable() -> Result<()> {
    log::info!("Windows TUN disable via helper");
    let helper = common::helper_path();
    let out = std::process::Command::new(&helper).arg("down").output()?;
    if !out.status.success() {
        anyhow::bail!(
            "helper down failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

pub fn status() -> TunStatus {
    match common::run_helper(&["status"]) {
        Ok(s) => {
            let t = s.trim().to_lowercase();
            if t.contains("enabled") || t.contains("up") {
                TunStatus::Enabled {
                    name: "RClash".into(),
                }
            } else {
                TunStatus::Disabled
            }
        }
        Err(e) => TunStatus::Error(format!("{e}")),
    }
}

pub fn helper_up() -> Result<()> {
    log::info!("helper up: Windows wintun");
    let out = std::process::Command::new("sc")
        .args(["query", "RClashTun"])
        .output();
    if let Ok(o) = out {
        if String::from_utf8_lossy(&o.stdout).contains("RUNNING") {
            log::info!("RClashTun service already running");
            return Ok(());
        }
    }
    if std::path::Path::new("wintun.dll").exists() {
        log::info!("wintun.dll found, would create adapter RClash");
    } else {
        log::warn!("wintun.dll not found — stub tun up");
    }
    let _ = std::process::Command::new("sc")
        .args([
            "create",
            "RClashTun",
            "binPath=",
            "\"rclash-tun-helper.exe service\"",
            "start=",
            "auto",
        ])
        .output();
    let _ = std::process::Command::new("sc")
        .args(["start", "RClashTun"])
        .output();
    Ok(())
}

pub fn helper_down() -> Result<()> {
    log::info!("helper down: Windows wintun");
    let _ = std::process::Command::new("sc")
        .args(["stop", "RClashTun"])
        .output();
    Ok(())
}

pub fn helper_status() -> String {
    let out = std::process::Command::new("sc")
        .args(["query", "RClashTun"])
        .output();
    match out {
        Ok(o) if String::from_utf8_lossy(&o.stdout).contains("RUNNING") => "enabled".to_owned(),
        _ => "disabled".to_owned(),
    }
}

pub fn helper_service_run() -> Result<()> {
    log::info!("helper service run (stub)");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
