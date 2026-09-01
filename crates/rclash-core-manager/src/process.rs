use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::{Child, Command};

pub fn core_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "rclash-core.exe"
    } else {
        "rclash-core"
    }
}

pub fn resolve_core_path() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let cand = dir.join(core_binary_name());
            if cand.exists() {
                return Some(cand);
            }
            let cand_lower = dir.join("rclash-core");
            if cand_lower.exists() {
                return Some(cand_lower);
            }
            let cand_exe = dir.join("rclash-core.exe");
            if cand_exe.exists() {
                return Some(cand_exe);
            }
            return Some(cand);
        }
    }
    if let Some(cfg) = dirs::config_dir() {
        let cand = cfg.join("RClash").join(core_binary_name());
        if cand.exists() {
            return Some(cand);
        }
        let cand_plain = cfg.join("RClash").join("rclash-core");
        if cand_plain.exists() {
            return Some(cand_plain);
        }
        return Some(cand);
    }
    None
}

pub struct CoreProcess {
    child: Child,
}

impl CoreProcess {
    pub async fn spawn(
        binary: PathBuf,
        config_dir: PathBuf,
        config_file: PathBuf,
    ) -> anyhow::Result<Self> {
        let mut cmd = Command::new(&binary);
        cmd.arg("-d")
            .arg(&config_dir)
            .arg("-f")
            .arg(&config_file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let child = cmd.spawn()?;
        Ok(Self { child })
    }

    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    pub async fn kill(&mut self) -> anyhow::Result<()> {
        self.child.kill().await?;
        Ok(())
    }

    pub async fn wait(&mut self) -> anyhow::Result<Option<i32>> {
        Ok(self.child.wait().await?.code())
    }
}
