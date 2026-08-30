use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::{Child, Command};

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
