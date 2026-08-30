#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyState {
    Enabled,
    Disabled,
}

pub trait SysProxy {
    fn get(&self) -> anyhow::Result<ProxyState>;
    fn set(&self, state: ProxyState, addr: &str) -> anyhow::Result<()>;
}

pub fn current() -> Box<dyn SysProxy> {
    #[cfg(windows)]
    return Box::new(windows::WindowsProxy);
    #[cfg(target_os = "linux")]
    return Box::new(linux::LinuxProxy);
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacosProxy);
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    return Box::new(UnsupportedProxy);
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
struct UnsupportedProxy;
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
impl SysProxy for UnsupportedProxy {
    fn get(&self) -> anyhow::Result<ProxyState> {
        anyhow::bail!("unsupported platform")
    }
    fn set(&self, _state: ProxyState, _addr: &str) -> anyhow::Result<()> {
        anyhow::bail!("unsupported platform")
    }
}
