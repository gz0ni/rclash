use crate::{common, TunStatus};
use anyhow::Result;

pub fn enable() -> Result<()> {
    log::info!("Linux TUN enable via pkexec");
    common::run_helper_pkexec(&["up"])?;
    Ok(())
}

pub fn disable() -> Result<()> {
    log::info!("Linux TUN disable via pkexec");
    common::run_helper_pkexec(&["down"])?;
    Ok(())
}

pub fn status() -> TunStatus {
    match common::run_helper(&["status"]) {
        Ok(s) => {
            let t = s.trim().to_lowercase();
            if t.contains("enabled") || t.contains("up") {
                TunStatus::Enabled {
                    name: "tun0".into(),
                }
            } else {
                TunStatus::Disabled
            }
        }
        Err(e) => TunStatus::Error(format!("{e}")),
    }
}

pub fn helper_up() -> Result<()> {
    log::info!("helper up: Linux tun setup");
    let steps: Vec<(&str, Vec<&str>)> = vec![
        ("ip", vec!["tuntap", "add", "dev", "tun0", "mode", "tun"]),
        ("ip", vec!["link", "set", "dev", "tun0", "up"]),
        ("ip", vec!["addr", "add", "198.18.0.1/16", "dev", "tun0"]),
        (
            "iptables",
            vec![
                "-t",
                "nat",
                "-C",
                "POSTROUTING",
                "-o",
                "tun0",
                "-j",
                "MASQUERADE",
            ],
        ),
    ];
    for (bin, args) in steps {
        let is_check = args.contains(&"-C");
        let out = std::process::Command::new(bin).args(&args).output();
        match out {
            Ok(o) if o.status.success() => {
                if is_check {
                    continue;
                }
                log::info!("{bin} {:?} ok", args);
            }
            Ok(o) if is_check => {
                let add_args: Vec<&str> = args
                    .iter()
                    .map(|s| if *s == "-C" { "-A" } else { s })
                    .copied()
                    .collect();
                let out2 = std::process::Command::new(bin).args(&add_args).output();
                if let Ok(o2) = out2 {
                    if o2.status.success() {
                        log::info!("{bin} {:?} added", add_args);
                    } else {
                        log::warn!(
                            "{bin} {:?} failed: {}",
                            add_args,
                            String::from_utf8_lossy(&o2.stderr)
                        );
                    }
                }
            }
            Ok(o) => {
                log::warn!(
                    "{bin} {:?} failed: {}",
                    args,
                    String::from_utf8_lossy(&o.stderr)
                );
            }
            Err(e) => {
                log::warn!("{bin} not found: {e}");
            }
        }
    }
    Ok(())
}

pub fn helper_down() -> Result<()> {
    log::info!("helper down: Linux tun teardown");
    for (bin, args) in [
        (
            "iptables",
            vec![
                "-t",
                "nat",
                "-D",
                "POSTROUTING",
                "-o",
                "tun0",
                "-j",
                "MASQUERADE",
            ],
        ),
        ("ip", vec!["link", "set", "dev", "tun0", "down"]),
        ("ip", vec!["link", "delete", "dev", "tun0"]),
    ] {
        let out = std::process::Command::new(bin).args(&args).output();
        if let Ok(o) = out {
            if o.status.success() {
                log::info!("{bin} {:?} ok", args);
            } else {
                log::warn!(
                    "{bin} {:?} failed: {}",
                    args,
                    String::from_utf8_lossy(&o.stderr)
                );
            }
        }
    }
    Ok(())
}

pub fn helper_status() -> String {
    let out = std::process::Command::new("ip")
        .args(["link", "show", "dev", "tun0"])
        .output();
    match out {
        Ok(o) if o.status.success() => "enabled".to_owned(),
        _ => "disabled".to_owned(),
    }
}
