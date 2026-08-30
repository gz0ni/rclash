use std::path::PathBuf;

pub fn helper_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = if cfg!(windows) {
                dir.join("rclash-tun-helper.exe")
            } else {
                dir.join("rclash-tun-helper")
            };
            if p.exists() {
                return p;
            }
        }
    }
    PathBuf::from("rclash-tun-helper")
}

pub fn run_helper(args: &[&str]) -> anyhow::Result<String> {
    let helper = helper_path();
    log::info!("run helper {:?} args {:?}", helper, args);
    let out = std::process::Command::new(&helper).args(args).output()?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("helper {:?} failed: {}", args, stderr);
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn run_helper_pkexec(args: &[&str]) -> anyhow::Result<String> {
    let helper = helper_path();
    log::info!("pkexec helper {:?} args {:?}", helper, args);
    let mut cmd = std::process::Command::new("pkexec");
    cmd.arg(&helper);
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output()?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("pkexec helper failed: {}", stderr);
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn run_helper_osascript(args: &[&str]) -> anyhow::Result<String> {
    let helper = helper_path();
    let helper_str = helper.display().to_string();
    let args_str = args.join(" ");
    let script =
        format!("do shell script \"{helper_str} {args_str}\" with administrator privileges");
    log::info!("osascript helper {}", script);
    let out = std::process::Command::new("osascript")
        .args(["-e", &script])
        .output()?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("osascript helper failed: {}", stderr);
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}
