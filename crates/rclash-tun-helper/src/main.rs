use std::env;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("status");
    match cmd {
        "up" => {
            #[cfg(target_os = "linux")]
            rclash_tun::linux::helper_up()?;
            #[cfg(target_os = "windows")]
            rclash_tun::windows::helper_up()?;
            #[cfg(target_os = "macos")]
            rclash_tun::macos::helper_up()?;
            #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
            anyhow::bail!("unsupported OS");
            println!("enabled");
        }
        "down" => {
            #[cfg(target_os = "linux")]
            rclash_tun::linux::helper_down()?;
            #[cfg(target_os = "windows")]
            rclash_tun::windows::helper_down()?;
            #[cfg(target_os = "macos")]
            rclash_tun::macos::helper_down()?;
            #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
            anyhow::bail!("unsupported OS");
            println!("disabled");
        }
        "status" => {
            #[cfg(target_os = "linux")]
            println!("{}", rclash_tun::linux::helper_status());
            #[cfg(target_os = "windows")]
            println!("{}", rclash_tun::windows::helper_status());
            #[cfg(target_os = "macos")]
            println!("{}", rclash_tun::macos::helper_status());
            #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
            println!("disabled");
        }
        "service" => {
            #[cfg(target_os = "windows")]
            rclash_tun::windows::helper_service_run()?;
            #[cfg(not(target_os = "windows"))]
            anyhow::bail!("service only on Windows");
        }
        _ => {
            eprintln!("Usage: rclash-tun-helper {{up|down|status|service}}");
            std::process::exit(1);
        }
    }
    Ok(())
}
