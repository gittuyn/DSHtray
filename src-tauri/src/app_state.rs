use crate::{
    app_error::AppError, commands::AppStateDto, config::ConfigStore,
    lifecycle::DefaultLifecycleController,
};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};

pub struct AppState {
    pub lifecycle: Mutex<DefaultLifecycleController>,
    pub config_path: PathBuf,
    first_run: AtomicBool,
}

impl AppState {
    pub fn new(
        lifecycle: DefaultLifecycleController,
        config_path: impl Into<PathBuf>,
        first_run: bool,
    ) -> Self {
        Self {
            lifecycle: Mutex::new(lifecycle),
            config_path: config_path.into(),
            first_run: AtomicBool::new(first_run),
        }
    }

    pub fn is_first_run(&self) -> bool {
        self.first_run.load(Ordering::Acquire)
    }

    pub fn mark_configured(&self) {
        self.first_run.store(false, Ordering::Release);
    }

    pub fn dto(&self) -> Result<AppStateDto, AppError> {
        let lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| AppError::new("state_lock_poisoned", "管理器状态锁已损坏"))?;
        Ok(AppStateDto::from_controller(
            lifecycle.config(),
            lifecycle.snapshot(),
            self.is_first_run(),
        ))
    }

    pub fn save_config(&self) -> Result<(), AppError> {
        let config = self
            .lifecycle
            .lock()
            .map_err(|_| AppError::new("state_lock_poisoned", "管理器状态锁已损坏"))?
            .config()
            .clone();
        ConfigStore::save(&self.config_path, &config)
    }

    pub fn ensure_config_parent(&self) -> Result<(), AppError> {
        let parent = self
            .config_path
            .parent()
            .ok_or_else(|| AppError::new("config_path_invalid", "配置文件路径没有父目录"))?;
        std::fs::create_dir_all(parent).map_err(AppError::from)
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }
}
