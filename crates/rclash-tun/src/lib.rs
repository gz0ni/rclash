pub mod common;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TunStatus {
    Disabled,
    Enabled { name: String },
    Error(String),
}

pub trait TunBackend {
    fn enable() -> Result<()>;
    fn disable() -> Result<()>;
    fn status() -> TunStatus;
}

pub fn enable() -> Result<()> {
    log::info!("TUN enable requested");
    #[cfg(target_os = "linux")]
    return linux::enable();
    #[cfg(target_os = "windows")]
    return windows::enable();
    #[cfg(target_os = "macos")]
    return macos::enable();
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        anyhow::bail!("TUN not supported on this OS");
    }
}

pub fn disable() -> Result<()> {
    log::info!("TUN disable requested");
    #[cfg(target_os = "linux")]
    return linux::disable();
    #[cfg(target_os = "windows")]
    return windows::disable();
    #[cfg(target_os = "macos")]
    return macos::disable();
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        anyhow::bail!("TUN not supported on this OS");
    }
}

pub fn status() -> TunStatus {
    #[cfg(target_os = "linux")]
    return linux::status();
    #[cfg(target_os = "windows")]
    return windows::status();
    #[cfg(target_os = "macos")]
    return macos::status();
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        TunStatus::Error("unsupported OS".into())
    }
}

pub fn is_enabled() -> bool {
    matches!(status(), TunStatus::Enabled { .. })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn status_not_panic() {
        let s = status();
        assert!(matches!(
            s,
            TunStatus::Disabled | TunStatus::Enabled { .. } | TunStatus::Error(_)
        ));
    }
}
