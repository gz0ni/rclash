#![allow(dead_code)]
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone)]
pub struct AppLogEntry {
    pub time: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

const MAX_BUF: usize = 2000;
const MAX_FILE_BYTES: u64 = 5 * 1024 * 1024;

static BUF: OnceLock<Mutex<VecDeque<AppLogEntry>>> = OnceLock::new();
static FILE: OnceLock<Mutex<Option<File>>> = OnceLock::new();

struct AppLogger;

impl log::Log for AppLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let time = chrono_str();
        let level = record.level().as_str().to_ascii_lowercase();
        let target = record.target().to_owned();
        let message = format!("{}", record.args());
        let entry = AppLogEntry {
            time: time.clone(),
            level: level.clone(),
            target: target.clone(),
            message: message.clone(),
        };
        if let Some(m) = BUF.get() {
            if let Ok(mut b) = m.lock() {
                if b.len() >= MAX_BUF {
                    b.pop_front();
                }
                b.push_back(entry);
            }
        }
        let line = format!("[{time}][{level}][{target}] {message}\n");
        if let Some(m) = FILE.get() {
            if let Ok(mut f) = m.lock() {
                if let Some(file) = f.as_mut() {
                    let _ = file.write_all(line.as_bytes());
                    let _ = file.flush();
                }
            }
        }
    }

    fn flush(&self) {
        if let Some(m) = FILE.get() {
            if let Ok(mut f) = m.lock() {
                if let Some(file) = f.as_mut() {
                    let _ = file.flush();
                }
            }
        }
    }
}

static LOGGER: AppLogger = AppLogger;

fn chrono_str() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let millis = now.subsec_millis();
    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    let s = secs % 60;
    format!("{hours:02}:{mins:02}:{s:02}.{millis:03}")
}

pub fn logs_dir() -> Option<PathBuf> {
    rclash_config::config_dir().map(|p| p.join("logs"))
}

pub fn log_file_path() -> Option<PathBuf> {
    logs_dir().map(|p| p.join("app.log"))
}

fn rotate_if_needed(path: &PathBuf) {
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() > MAX_FILE_BYTES {
            let rotated = path.with_extension("log.1");
            let _ = std::fs::remove_file(&rotated);
            let _ = std::fs::rename(path, &rotated);
        }
    }
}

pub fn init(level: log::LevelFilter) -> anyhow::Result<()> {
    BUF.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_BUF + 1)));
    let path = log_file_path().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    rotate_if_needed(&path);
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    FILE.get_or_init(|| Mutex::new(Some(file)));
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(level);
    log::info!("logger initialized level={} file={}", level, path.display());
    Ok(())
}

pub fn set_level(level: log::LevelFilter) {
    log::set_max_level(level);
    log::info!("log level changed to {}", level);
}

pub fn snapshot() -> Vec<AppLogEntry> {
    BUF.get()
        .and_then(|m| m.lock().ok().map(|b| b.iter().cloned().collect()))
        .unwrap_or_default()
}

pub fn clear() {
    if let Some(m) = BUF.get() {
        if let Ok(mut b) = m.lock() {
            b.clear();
        }
    }
    log::info!("app logs cleared");
}
