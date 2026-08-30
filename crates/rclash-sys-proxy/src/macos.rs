use super::{ProxyState, SysProxy};

pub struct MacosProxy;

impl SysProxy for MacosProxy {
    fn get(&self) -> anyhow::Result<ProxyState> {
        anyhow::bail!("networksetup proxy get not yet implemented")
    }
    fn set(&self, _state: ProxyState, _addr: &str) -> anyhow::Result<()> {
        anyhow::bail!("networksetup proxy set not yet implemented (F1)")
    }
}
