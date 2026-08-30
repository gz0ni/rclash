use super::{ProxyState, SysProxy};

pub struct WindowsProxy;

impl SysProxy for WindowsProxy {
    fn get(&self) -> anyhow::Result<ProxyState> {
        let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        let key =
            hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings")?;
        let enabled: u32 = key.get_value("ProxyEnable").unwrap_or(0);
        Ok(if enabled == 1 {
            ProxyState::Enabled
        } else {
            ProxyState::Disabled
        })
    }

    fn set(&self, state: ProxyState, addr: &str) -> anyhow::Result<()> {
        let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        let (key, _) =
            hkcu.create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings")?;
        match state {
            ProxyState::Enabled => {
                key.set_value("ProxyEnable", &1u32)?;
                key.set_value("ProxyServer", &addr)?;
            }
            ProxyState::Disabled => {
                key.set_value("ProxyEnable", &0u32)?;
            }
        }
        Ok(())
    }
}
