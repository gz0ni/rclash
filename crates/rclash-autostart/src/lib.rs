#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

pub trait AutoLaunch {
    fn is_enabled(&self) -> anyhow::Result<bool>;
    fn enable(&self) -> anyhow::Result<()>;
    fn disable(&self) -> anyhow::Result<()>;
}

pub fn current() -> Box<dyn AutoLaunch> {
    #[cfg(windows)]
    return Box::new(windows::WindowsAutostart);
    #[cfg(target_os = "linux")]
    return Box::new(linux::LinuxAutostart);
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacosAutostart);
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    return Box::new(UnsupportedAutostart);
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
struct UnsupportedAutostart;
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
impl AutoLaunch for UnsupportedAutostart {
    fn is_enabled(&self) -> anyhow::Result<bool> {
        anyhow::bail!("unsupported platform")
    }
    fn enable(&self) -> anyhow::Result<()> {
        anyhow::bail!("unsupported platform")
    }
    fn disable(&self) -> anyhow::Result<()> {
        anyhow::bail!("unsupported platform")
    }
}

pub(crate) fn current_exe_string() -> anyhow::Result<String> {
    let exe = std::env::current_exe()?;
    Ok(exe.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trait_object_current_does_not_panic() {
        let a = current();
        let _ = a.is_enabled();
    }
}
