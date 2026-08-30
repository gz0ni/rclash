use super::{ProxyState, SysProxy};

pub struct LinuxProxy;

impl SysProxy for LinuxProxy {
    fn get(&self) -> anyhow::Result<ProxyState> {
        anyhow::bail!("gsettings proxy get not yet implemented")
    }
    fn set(&self, _state: ProxyState, _addr: &str) -> anyhow::Result<()> {
        anyhow::bail!("gsettings proxy set not yet implemented (F1)")
    }
}
