use crate::app_error::AppError;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::SystemTime};
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TargetId {
    Source,
    Packaged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TargetKind {
    Source,
    Packaged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerConfig {
    pub start_on_login: bool,
    pub start_dsh_on_login: bool,
    pub close_to_tray: bool,
    pub confirm_restart_on_proxy_change: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetConfig {
    pub label: String,
    pub kind: TargetKind,
    pub working_directory: PathBuf,
    pub command: String,
    pub arguments: Vec<String>,
    pub executable: PathBuf,
}

impl TargetConfig {
    pub fn source(label: impl Into<String>, working_directory: PathBuf) -> Self {
        Self {
            label: label.into(),
            kind: TargetKind::Source,
            working_directory,
            command: "pnpm".into(),
            arguments: vec!["dsh".into(), "web".into()],
            executable: PathBuf::new(),
        }
    }

    pub fn packaged(label: impl Into<String>, executable: PathBuf) -> Self {
        let working_directory = executable.parent().map(PathBuf::from).unwrap_or_default();
        Self {
            label: label.into(),
            kind: TargetKind::Packaged,
            working_directory,
            command: String::new(),
            arguments: Vec::new(),
            executable,
        }
    }

    pub fn is_configured(&self) -> bool {
        match self.kind {
            TargetKind::Source => !self.working_directory.as_os_str().is_empty(),
            TargetKind::Packaged => !self.executable.as_os_str().is_empty(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetsConfig {
    pub source: TargetConfig,
    pub packaged: TargetConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceConfig {
    pub host: String,
    pub port: u16,
}

impl ServiceConfig {
    pub fn url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    pub enabled: bool,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    pub manager: ManagerConfig,
    pub active_target: TargetId,
    pub targets: TargetsConfig,
    pub service: ServiceConfig,
    pub proxy: ProxyConfig,
}

impl AppConfig {
    pub fn defaults() -> Self {
        Self {
            version: 1,
            manager: ManagerConfig {
                start_on_login: true,
                start_dsh_on_login: false,
                close_to_tray: true,
                confirm_restart_on_proxy_change: true,
            },
            active_target: TargetId::Source,
            targets: TargetsConfig {
                source: TargetConfig::source("DSH 源码", PathBuf::new()),
                packaged: TargetConfig {
                    label: "DSH.exe".into(),
                    kind: TargetKind::Packaged,
                    working_directory: PathBuf::new(),
                    command: String::new(),
                    arguments: Vec::new(),
                    executable: PathBuf::new(),
                },
            },
            service: ServiceConfig {
                host: "127.0.0.1".into(),
                port: 3080,
            },
            proxy: ProxyConfig {
                enabled: true,
                url: "http://127.0.0.1:7897".into(),
            },
        }
    }

    pub fn active_target_config(&self) -> &TargetConfig {
        match self.active_target {
            TargetId::Source => &self.targets.source,
            TargetId::Packaged => &self.targets.packaged,
        }
    }

    pub fn active_target_config_mut(&mut self) -> &mut TargetConfig {
        match self.active_target {
            TargetId::Source => &mut self.targets.source,
            TargetId::Packaged => &mut self.targets.packaged,
        }
    }

    pub fn validate(&self) -> Result<(), AppError> {
        if self.version != 1 {
            return Err(AppError::new(
                "unsupported_config_version",
                "配置文件版本不受支持",
            ));
        }
        if self.service.host != "127.0.0.1" && self.service.host != "localhost" {
            return Err(AppError::new(
                "invalid_service_host",
                "服务地址只能使用 127.0.0.1 或 localhost",
            ));
        }
        if self.service.port == 0 {
            return Err(AppError::new(
                "invalid_service_port",
                "服务端口必须在 1 到 65535 之间",
            ));
        }
        validate_proxy_url(&self.proxy.url)?;
        validate_target(self.active_target_config())?;
        Ok(())
    }

    pub fn validate_active_target(&self) -> Result<(), AppError> {
        self.validate()?;
        let target = self.active_target_config();
        if !target.is_configured() {
            return Err(AppError::new(
                "target_not_configured",
                "当前 DSH 目标尚未配置",
            ));
        }
        Ok(())
    }
}

fn validate_target(target: &TargetConfig) -> Result<(), AppError> {
    match target.kind {
        TargetKind::Source => {
            if !target.working_directory.as_os_str().is_empty()
                && (!target.working_directory.is_dir())
            {
                return Err(AppError::new(
                    "invalid_source_directory",
                    "源码目标目录不存在或不是目录",
                ));
            }
        }
        TargetKind::Packaged => {
            if !target.executable.as_os_str().is_empty()
                && (!target.executable.is_file()
                    || target
                        .executable
                        .extension()
                        .is_none_or(|extension| !extension.eq_ignore_ascii_case("exe")))
            {
                return Err(AppError::new(
                    "invalid_packaged_executable",
                    "打包目标必须是存在的 .exe 文件",
                ));
            }
            if !target.working_directory.as_os_str().is_empty()
                && !target.working_directory.is_dir()
            {
                return Err(AppError::new(
                    "invalid_working_directory",
                    "工作目录不存在或不是目录",
                ));
            }
        }
    }
    Ok(())
}

fn validate_proxy_url(value: &str) -> Result<(), AppError> {
    let parsed =
        Url::parse(value).map_err(|_| AppError::new("invalid_proxy_url", "代理 URL 格式无效"))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(AppError::new(
            "invalid_proxy_url",
            "代理 URL 只能使用 http 或 https 且必须包含主机",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LifecycleState {
    Stopped,
    Starting,
    Running,
    External,
    Stopping,
    Failed,
    PortConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Ownership {
    None,
    Managed,
    Adopted,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSnapshot {
    pub state: LifecycleState,
    pub target: TargetId,
    pub pid: Option<u32>,
    pub ownership: Ownership,
    pub service_url: String,
    pub proxy_enabled: bool,
    pub last_error: Option<crate::app_error::AppError>,
    #[serde(skip)]
    pub started_at: Option<SystemTime>,
}

impl RuntimeSnapshot {
    pub fn stopped(config: &AppConfig) -> Self {
        Self {
            state: LifecycleState::Stopped,
            target: config.active_target,
            pid: None,
            ownership: Ownership::None,
            service_url: config.service.url(),
            proxy_enabled: config.proxy.enabled,
            last_error: None,
            started_at: None,
        }
    }
}
