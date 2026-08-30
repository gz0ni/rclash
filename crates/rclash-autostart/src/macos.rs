use super::AutoLaunch;
use std::path::{Path, PathBuf};

pub struct MacosAutostart;

fn launch_agents_dir_with_home(home: &Path) -> PathBuf {
    home.join("Library").join("LaunchAgents")
}

fn plist_path_with_home(home: &Path) -> PathBuf {
    launch_agents_dir_with_home(home).join("com.rclash.app.plist")
}

fn default_home() -> Option<PathBuf> {
    dirs::home_dir()
}

pub fn plist_path() -> Option<PathBuf> {
    default_home().map(|h| plist_path_with_home(&h))
}

fn plist_content(exec: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>com.rclash.app</string>
    <key>ProgramArguments</key><array><string>{exec}</string><string>--minimized</string></array>
    <key>RunAtLoad</key><true/>
    <key>KeepAlive</key><false/>
</dict>
</plist>
"#
    )
}

impl AutoLaunch for MacosAutostart {
    fn is_enabled(&self) -> anyhow::Result<bool> {
        let Some(p) = plist_path() else {
            anyhow::bail!("no home dir")
        };
        Ok(p.exists())
    }

    fn enable(&self) -> anyhow::Result<()> {
        let Some(home) = default_home() else {
            anyhow::bail!("no home dir")
        };
        enable_with_home(&home, &super::current_exe_string()?)
    }

    fn disable(&self) -> anyhow::Result<()> {
        let Some(home) = default_home() else {
            anyhow::bail!("no home dir")
        };
        disable_with_home(&home)
    }
}

pub(crate) fn enable_with_home(home: &Path, exec: &str) -> anyhow::Result<()> {
    let dir = launch_agents_dir_with_home(home);
    std::fs::create_dir_all(&dir)?;
    let file = plist_path_with_home(home);
    std::fs::write(file, plist_content(exec))?;
    Ok(())
}

pub(crate) fn disable_with_home(home: &Path) -> anyhow::Result<()> {
    let file = plist_path_with_home(home);
    if file.exists() {
        std::fs::remove_file(file)?;
    }
    Ok(())
}

pub(crate) fn is_enabled_with_home(home: &Path) -> bool {
    plist_path_with_home(home).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_contains_exec_and_minimized() {
        let c = plist_content("/Applications/RClash.app/Contents/MacOS/rclash");
        assert!(c.contains("/Applications/RClash.app/Contents/MacOS/rclash"));
        assert!(c.contains("--minimized"));
        assert!(c.contains("com.rclash.app"));
    }

    #[test]
    fn enable_disable_with_tempdir() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        assert!(!is_enabled_with_home(home));
        enable_with_home(home, "/tmp/rclash").unwrap();
        assert!(is_enabled_with_home(home));
        let content = std::fs::read_to_string(plist_path_with_home(home)).unwrap();
        assert!(content.contains("/tmp/rclash"));
        disable_with_home(home).unwrap();
        assert!(!is_enabled_with_home(home));
    }
}
