use crate::{
    app_error::AppError,
    domain::{AppConfig, RuntimeSnapshot},
    logging::redact_url,
    targets::resolve_pnpm_command,
};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckStatus {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    pub remediation: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub healthy: bool,
    pub checks: Vec<DiagnosticCheck>,
}

impl DiagnosticReport {
    pub fn item(&self, name: &str) -> Option<&DiagnosticCheck> {
        self.checks.iter().find(|check| check.name == name)
    }
}

#[derive(Debug, Clone)]
pub struct PnpmResolver {
    command: Option<PathBuf>,
}

impl PnpmResolver {
    pub fn resolve() -> Self {
        Self {
            command: resolve_pnpm_command().ok(),
        }
    }

    pub fn missing() -> Self {
        Self { command: None }
    }

    pub fn command(&self) -> Option<&PathBuf> {
        self.command.as_ref()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticSnapshot {
    pub version: String,
    pub runtime: RuntimeSnapshot,
    pub target: String,
    pub service_url: String,
    pub proxy_url: String,
    pub pnpm_command: Option<String>,
    pub log_directory: Option<String>,
}

impl DiagnosticSnapshot {
    pub fn from_test_state_with_proxy(proxy_url: &str) -> Self {
        let config = AppConfig::defaults();
        Self {
            version: env!("CARGO_PKG_VERSION").into(),
            runtime: RuntimeSnapshot::stopped(&config),
            target: "source".into(),
            service_url: config.service.url(),
            proxy_url: redact_url(proxy_url),
            pnpm_command: None,
            log_directory: None,
        }
    }
}

pub fn run_self_test_with(resolver: PnpmResolver) -> DiagnosticReport {
    let pnpm_check = match resolver.command() {
        Some(path) => DiagnosticCheck {
            name: "pnpm".into(),
            status: CheckStatus::Passed,
            message: format!("已解析 pnpm: {}", path.display()),
            remediation: String::new(),
        },
        None => DiagnosticCheck {
            name: "pnpm".into(),
            status: CheckStatus::Failed,
            message: "PATH 中未找到 pnpm.cmd 或 pnpm.exe".into(),
            remediation: "安装 pnpm，并确认 pnpm 所在目录已加入当前用户 PATH，然后重启管理器"
                .into(),
        },
    };
    let checks = vec![pnpm_check];
    DiagnosticReport {
        healthy: checks
            .iter()
            .all(|check| check.status == CheckStatus::Passed),
        checks,
    }
}

#[allow(dead_code)]
fn _error_type_is_linked(_: AppError) {}
