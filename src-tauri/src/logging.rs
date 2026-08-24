use crate::app_error::AppError;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use url::Url;

pub fn redact_url(value: &str) -> String {
    let Ok(mut url) = Url::parse(value) else {
        return value.to_string();
    };
    if !url.username().is_empty() {
        let _ = url.set_username("***");
    }
    if url.password().is_some() {
        let _ = url.set_password(Some("***"));
    }
    let rendered = url.to_string();
    if rendered.ends_with('/') && !value.ends_with('/') {
        rendered.trim_end_matches('/').to_string()
    } else {
        rendered
    }
}

pub fn log_dsh_line(stream: &str, line: &str) -> String {
    let mut safe = line.to_string();
    for marker in [
        "Bearer ",
        "token=",
        "token:",
        "api_key=",
        "apikey=",
        "password=",
    ] {
        safe = redact_after_marker(&safe, marker);
    }
    format!("{} [{}] {}", timestamp(), stream, safe)
}

fn redact_after_marker(value: &str, marker: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let marker_lower = marker.to_ascii_lowercase();
    let Some(index) = lower.find(&marker_lower) else {
        return value.to_string();
    };
    let start = index + marker.len();
    let end = value[start..]
        .find(char::is_whitespace)
        .map(|offset| start + offset)
        .unwrap_or(value.len());
    format!("{}[REDACTED]{}", &value[..index], &value[end..])
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}

pub struct LogManager {
    manager: Mutex<File>,
    dsh: Mutex<File>,
    pub manager_path: PathBuf,
    pub dsh_path: PathBuf,
}

impl LogManager {
    pub fn init(directory: impl AsRef<Path>) -> Result<Self, AppError> {
        let directory = directory.as_ref();
        fs::create_dir_all(directory)?;
        let manager_path = directory.join("manager.log");
        let dsh_path = directory.join("dsh.log");
        let open = |path: &Path| {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(AppError::from)
        };
        Ok(Self {
            manager: Mutex::new(open(&manager_path)?),
            dsh: Mutex::new(open(&dsh_path)?),
            manager_path,
            dsh_path,
        })
    }

    pub fn log_manager_event(&self, event: &str) -> Result<(), AppError> {
        let mut file = self
            .manager
            .lock()
            .map_err(|_| AppError::new("log_lock_poisoned", "管理器日志锁已损坏"))?;
        writeln!(file, "{} [manager] {}", timestamp(), event).map_err(AppError::from)
    }

    pub fn log_dsh(&self, stream: &str, line: &str) -> Result<(), AppError> {
        let mut file = self
            .dsh
            .lock()
            .map_err(|_| AppError::new("log_lock_poisoned", "DSH 日志锁已损坏"))?;
        writeln!(file, "{}", log_dsh_line(stream, line)).map_err(AppError::from)
    }
}
