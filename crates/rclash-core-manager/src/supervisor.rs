use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub const DEFAULT_BASE: &str = "http://127.0.0.1:9090";

pub struct Supervisor {
    child: Option<Child>,
    binary: Option<PathBuf>,
    base: String,
    secret: String,
    client: reqwest::blocking::Client,
}

impl Supervisor {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            child: None,
            binary: None,
            base: DEFAULT_BASE.to_owned(),
            secret: secret.into(),
            client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()
                .unwrap_or_else(|_| reqwest::blocking::Client::new()),
        }
    }

    pub fn is_running(&self) -> bool {
        self.child.is_some()
    }

    pub fn api(&self) -> CoreApi {
        CoreApi::new(&self.base).with_secret(&self.secret)
    }

    pub fn version_blocking(&self) -> anyhow::Result<String> {
        let url = format!("{}/version", self.base);
        let mut req = self.client.get(url);
        if !self.secret.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.secret));
        }
        let v: serde_json::Value = req.send()?.error_for_status()?.json()?;
        Ok(v.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_owned())
    }

    pub fn is_alive(&self) -> bool {
        self.version_blocking().is_ok()
    }

    pub fn start(
        &mut self,
        binary: PathBuf,
        config_dir: &Path,
        config_file: &Path,
    ) -> anyhow::Result<String> {
        self.stop();
        if !binary.exists() {
            anyhow::bail!("core binary not found: {}", binary.display());
        }
        let mut cmd = Command::new(&binary);
        cmd.arg("-d")
            .arg(config_dir)
            .arg("-f")
            .arg(config_file)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let child = cmd.spawn()?;
        self.child = Some(child);
        self.binary = Some(binary);

        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(child) = self.child.as_mut() {
                if let Some(status) = child.try_wait()? {
                    self.child = None;
                    anyhow::bail!("core exited early: {status}");
                }
            }
            match self.version_blocking() {
                Ok(v) => return Ok(v),
                Err(_) => {
                    if Instant::now() >= deadline {
                        self.stop();
                        anyhow::bail!("core healthcheck timeout");
                    }
                    std::thread::sleep(Duration::from_millis(300));
                }
            }
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn restart(&mut self, config_dir: &Path, config_file: &Path) -> anyhow::Result<String> {
        let binary = self
            .binary
            .clone()
            .or_else(crate::process::resolve_core_path)
            .ok_or_else(|| anyhow::anyhow!("core binary not found"))?;
        self.start(binary, config_dir, config_file)
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        self.stop();
    }
}

fn blocking_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

fn authed(
    req: reqwest::blocking::RequestBuilder,
    secret: &str,
) -> reqwest::blocking::RequestBuilder {
    if secret.is_empty() {
        req
    } else {
        req.header("Authorization", format!("Bearer {secret}"))
    }
}

pub fn precheck(binary: &Path, config_dir: &Path, config_file: &Path) -> anyhow::Result<()> {
    if !binary.exists() {
        anyhow::bail!("core binary not found: {}", binary.display());
    }
    let test_out = Command::new(binary)
        .arg("-d")
        .arg(config_dir)
        .arg("-f")
        .arg(config_file)
        .arg("-t")
        .stdin(Stdio::null())
        .output()?;
    if !test_out.status.success() {
        let mut err = String::from_utf8_lossy(&test_out.stderr).into_owned();
        if err.trim().is_empty() {
            err = String::from_utf8_lossy(&test_out.stdout).into_owned();
        }
        let tail: String = err.lines().rev().take(12).collect::<Vec<_>>().join("\n");
        anyhow::bail!("config test failed: {}", tail.trim());
    }
    Ok(())
}

pub fn spawn_and_wait(
    binary: &Path,
    config_dir: &Path,
    config_file: &Path,
    base: &str,
    secret: &str,
) -> anyhow::Result<(Child, String)> {
    precheck(binary, config_dir, config_file)?;
    let mut cmd = Command::new(binary);
    cmd.arg("-d")
        .arg(config_dir)
        .arg("-f")
        .arg(config_file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut child = cmd.spawn()?;
    let client = blocking_client();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = child.try_wait()? {
            anyhow::bail!("core exited early: {status}");
        }
        let url = format!("{base}/version");
        match authed(client.get(url), secret)
            .send()
            .and_then(|r| r.error_for_status())
        {
            Ok(resp) => {
                let v: serde_json::Value = resp.json().unwrap_or_default();
                let version = v
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_owned();
                return Ok((child, version));
            }
            Err(_) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    anyhow::bail!("core healthcheck timeout");
                }
                std::thread::sleep(Duration::from_millis(300));
            }
        }
    }
}

pub fn stop_child(child: &mut Option<Child>) {
    if let Some(mut c) = child.take() {
        let _ = c.kill();
        let _ = c.wait();
    }
}

pub fn child_alive(child: &mut Option<Child>) -> bool {
    match child.as_mut() {
        Some(c) => matches!(c.try_wait(), Ok(None)),
        None => false,
    }
}

pub fn version_blocking(base: &str, secret: &str) -> anyhow::Result<String> {
    let client = blocking_client();
    let url = format!("{base}/version");
    let v: serde_json::Value = authed(client.get(url), secret)
        .send()?
        .error_for_status()?
        .json()?;
    Ok(v.get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_owned())
}

pub fn proxies_blocking(base: &str, secret: &str) -> anyhow::Result<serde_json::Value> {
    let client = blocking_client();
    let url = format!("{base}/proxies");
    Ok(authed(client.get(url), secret)
        .send()?
        .error_for_status()?
        .json()?)
}

fn percent_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub fn select_proxy_blocking(
    base: &str,
    secret: &str,
    group: &str,
    name: &str,
) -> anyhow::Result<()> {
    let client = blocking_client();
    let url = format!("{base}/proxies/{}", percent_encode(group));
    let body = serde_json::json!({"name": name});
    authed(client.put(url).json(&body), secret)
        .send()?
        .error_for_status()?;
    Ok(())
}

pub fn set_mode_blocking(base: &str, secret: &str, mode: &str) -> anyhow::Result<()> {
    let client = blocking_client();
    let url = format!("{base}/configs");
    let body = serde_json::json!({"mode": mode});
    authed(client.patch(url).json(&body), secret)
        .send()?
        .error_for_status()?;
    Ok(())
}

pub fn patch_configs_blocking(
    base: &str,
    secret: &str,
    body: &serde_json::Value,
) -> anyhow::Result<()> {
    let client = blocking_client();
    let url = format!("{base}/configs");
    authed(client.patch(url).json(body), secret)
        .send()?
        .error_for_status()?;
    Ok(())
}

pub fn get_configs_blocking(base: &str, secret: &str) -> anyhow::Result<serde_json::Value> {
    let client = blocking_client();
    let url = format!("{base}/configs");
    Ok(authed(client.get(url), secret)
        .send()?
        .error_for_status()?
        .json()?)
}

pub fn test_delay_blocking(
    base: &str,
    secret: &str,
    proxy: &str,
    test_url: &str,
    timeout_ms: u64,
) -> Option<u64> {
    let client = blocking_client();
    let url = format!(
        "{base}/proxies/{}/delay?url={test_url}&timeout={timeout_ms}",
        percent_encode(proxy)
    );
    let resp = authed(client.get(url), secret).send().ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: serde_json::Value = resp.json().ok()?;
    v.get("delay").and_then(|d| d.as_u64())
}

pub fn reload_blocking(base: &str, secret: &str) -> anyhow::Result<()> {
    let client = blocking_client();
    let url = format!("{base}/configs?force=true");
    authed(client.put(url).json(&serde_json::json!({})), secret)
        .send()?
        .error_for_status()?;
    Ok(())
}

use super::api::CoreApi;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn supervisor_starts_stopped() {
        let s = Supervisor::new("test-secret");
        assert!(!s.is_running());
        assert!(!s.is_alive());
    }

    #[test]
    fn start_missing_binary_errors() {
        let mut s = Supervisor::new("x");
        let missing = PathBuf::from("definitely-not-here-rclash-core-test");
        let err = s
            .start(missing, Path::new("."), Path::new("config.yaml"))
            .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }
}
