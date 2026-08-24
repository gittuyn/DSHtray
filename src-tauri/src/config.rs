use crate::{app_error::AppError, domain::AppConfig};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug)]
pub struct ConfigLoad {
    pub config: AppConfig,
    pub recovered: bool,
    pub backup_path: Option<PathBuf>,
}

pub struct ConfigStore;

impl ConfigStore {
    pub fn load(path: &Path) -> Result<ConfigLoad, AppError> {
        if !path.exists() {
            return Ok(ConfigLoad {
                config: AppConfig::defaults(),
                recovered: false,
                backup_path: None,
            });
        }

        let data = fs::read_to_string(path)?;
        match serde_json::from_str::<AppConfig>(&data) {
            Ok(config) => {
                config.validate()?;
                Ok(ConfigLoad {
                    config,
                    recovered: false,
                    backup_path: None,
                })
            }
            Err(error) => {
                let backup_path = backup_corrupt_file(path).map_err(|recovery_error| {
                    AppError::with_details(
                        "config_recovery_failed",
                        "配置损坏且无法恢复",
                        format!("{}; 原始解析错误: {}", recovery_error, error),
                    )
                })?;
                Ok(ConfigLoad {
                    config: AppConfig::defaults(),
                    recovered: true,
                    backup_path: Some(backup_path),
                })
            }
        }
    }

    pub fn save(path: &Path, config: &AppConfig) -> Result<(), AppError> {
        config.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp_path = temporary_path(path);
        let payload = serde_json::to_vec_pretty(config)?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)?;
        file.write_all(&payload)?;
        file.sync_all()?;
        drop(file);
        replace_file(&temp_path, path)?;
        Ok(())
    }
}

fn temporary_path(path: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config.json");
    path.with_file_name(format!("{file_name}.tmp-{stamp}"))
}

fn backup_corrupt_file(path: &Path) -> Result<PathBuf, AppError> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config.json");
    let backup = path.with_file_name(format!("{file_name}.corrupt-{stamp}"));
    fs::rename(path, &backup)?;
    Ok(backup)
}

fn replace_file(temp: &Path, destination: &Path) -> Result<(), AppError> {
    if destination.exists() {
        fs::remove_file(destination)?;
    }
    fs::rename(temp, destination)?;
    Ok(())
}
