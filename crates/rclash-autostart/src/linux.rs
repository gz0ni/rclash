use super::AutoLaunch;
use std::path::{Path, PathBuf};

pub struct LinuxAutostart;

fn autostart_dir_with_base(base: &Path) -> PathBuf {
    base.join("autostart")
}

fn autostart_file_with_base(base: &Path) -> PathBuf {
    autostart_dir_with_base(base).join("RClash.desktop")
}

fn default_base() -> Option<PathBuf> {
    dirs::config_dir()
}

pub fn autostart_file_path() -> Option<PathBuf> {
    default_base().map(|b| autostart_file_with_base(&b))
}

fn desktop_entry_content(exec: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=RClash\nExec={exec} --minimized\nHidden=false\nNoDisplay=false\nX-GNOME-Autostart-enabled=true\n"
    )
}

fn ensure_content(exec: &str) -> String {
    desktop_entry_content(exec)
}

impl AutoLaunch for LinuxAutostart {
    fn is_enabled(&self) -> anyhow::Result<bool> {
        let Some(p) = autostart_file_path() else {
            anyhow::bail!("no config dir")
        };
        Ok(p.exists())
    }

    fn enable(&self) -> anyhow::Result<()> {
        let Some(base) = default_base() else {
            anyhow::bail!("no config dir")
        };
        enable_with_base(&base, &super::current_exe_string()?)
    }

    fn disable(&self) -> anyhow::Result<()> {
        let Some(base) = default_base() else {
            anyhow::bail!("no config dir")
        };
        disable_with_base(&base)
    }
}

pub(crate) fn enable_with_base(base: &Path, exec: &str) -> anyhow::Result<()> {
    let dir = autostart_dir_with_base(base);
    std::fs::create_dir_all(&dir)?;
    let file = autostart_file_with_base(base);
    std::fs::write(file, ensure_content(exec))?;
    Ok(())
}

pub(crate) fn disable_with_base(base: &Path) -> anyhow::Result<()> {
    let file = autostart_file_with_base(base);
    if file.exists() {
        std::fs::remove_file(file)?;
    }
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn is_enabled_with_base(base: &Path) -> bool {
    autostart_file_with_base(base).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_entry_contains_exec() {
        let c = desktop_entry_content("/usr/bin/rclash");
        assert!(c.contains("Exec=/usr/bin/rclash --minimized"));
        assert!(c.contains("X-GNOME-Autostart-enabled=true"));
    }

    #[test]
    fn enable_disable_with_tempdir() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        assert!(!is_enabled_with_base(base));
        enable_with_base(base, "/tmp/rclash").unwrap();
        assert!(is_enabled_with_base(base));
        let content = std::fs::read_to_string(autostart_file_with_base(base)).unwrap();
        assert!(content.contains("/tmp/rclash"));
        disable_with_base(base).unwrap();
        assert!(!is_enabled_with_base(base));
    }

    #[test]
    fn disable_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        disable_with_base(tmp.path()).unwrap();
    }
}
