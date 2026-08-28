use crate::{
    app_error::AppError,
    app_state::AppState,
    config::ConfigStore,
    diagnostics::{run_self_test_with, CheckStatus, PnpmResolver},
    discovery::{self, DiscoveredTarget},
    domain::{
        AppConfig, LifecycleState, ManagerConfig, Ownership, ProxyConfig, RuntimeSnapshot,
        TargetConfig, TargetId, TargetsConfig,
    },
    lifecycle::DefaultLifecycleController,
    tray,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime, State};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateDto {
    pub first_run: bool,
    pub runtime: RuntimeSnapshot,
    pub manager: ManagerConfig,
    pub active_target: TargetId,
    pub targets: TargetsConfig,
    pub service_host: String,
    pub service_port: u16,
    pub proxy: ProxyConfig,
}

impl AppStateDto {
    pub fn from_controller(config: &AppConfig, runtime: RuntimeSnapshot, first_run: bool) -> Self {
        Self {
            first_run,
            runtime,
            manager: config.manager.clone(),
            active_target: config.active_target,
            targets: config.targets.clone(),
            service_host: config.service.host.clone(),
            service_port: config.service.port,
            proxy: config.proxy.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyChangePlan {
    pub enabled: bool,
    pub current_enabled: bool,
    pub requires_restart: bool,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub start_on_login: Option<bool>,
    pub start_dsh_on_login: Option<bool>,
    pub service_port: Option<u16>,
    pub proxy_enabled: Option<bool>,
    pub proxy_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstRunSetup {
    pub source: Option<DiscoveredTarget>,
    pub packaged: Option<DiscoveredTarget>,
    pub active_target: TargetId,
    pub proxy_enabled: bool,
    pub proxy_url: String,
    pub start_on_login: bool,
    pub start_dsh_on_login: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfTestReport {
    pub healthy: bool,
    pub checks: Vec<SelfTestCheck>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfTestCheck {
    pub name: String,
    pub passed: bool,
    pub message: String,
}

pub fn guard_stop(snapshot: &RuntimeSnapshot) -> Result<(), AppError> {
    if snapshot.ownership == Ownership::External {
        return Err(AppError::new(
            "external_not_adopted",
            "外部 DSH 尚未被用户确认接管",
        ));
    }
    if snapshot.state == LifecycleState::PortConflict {
        return Err(AppError::new(
            "port_conflict",
            "服务端口已被占用，未执行停止或终止操作",
        ));
    }
    if snapshot.state == LifecycleState::Stopped {
        return Ok(());
    }
    if !matches!(snapshot.ownership, Ownership::Managed | Ownership::Adopted) {
        return Err(AppError::new("not_running", "没有可停止的受管理 DSH"));
    }
    Ok(())
}

pub fn prepare_proxy_change_for_test(
    snapshot: &RuntimeSnapshot,
    current_enabled: bool,
    enabled: bool,
) -> Result<ProxyChangePlan, AppError> {
    let requires_restart = matches!(
        (snapshot.state, snapshot.ownership),
        (
            LifecycleState::Running,
            Ownership::Managed | Ownership::Adopted
        )
    ) && current_enabled != enabled;
    Ok(ProxyChangePlan {
        enabled,
        current_enabled,
        requires_restart,
        message: if requires_restart {
            "需要重启 DSH，当前会话可能中断".into()
        } else {
            "将在下次启动 DSH 时生效".into()
        },
    })
}

pub fn guard_apply_proxy_change(
    snapshot: &RuntimeSnapshot,
    confirmed_restart: bool,
) -> Result<(), AppError> {
    let running = matches!(
        (snapshot.state, snapshot.ownership),
        (
            LifecycleState::Running,
            Ownership::Managed | Ownership::Adopted
        )
    );
    if running && !confirmed_restart {
        return Err(AppError::new(
            "confirmation_required",
            "运行中的 DSH 切换代理必须先确认重启",
        ));
    }
    Ok(())
}

pub fn sync_runtime_snapshot(snapshot: &mut RuntimeSnapshot, config: &AppConfig) {
    snapshot.target = config.active_target;
    snapshot.service_url = config.service.url();
    snapshot.proxy_enabled = config.proxy.enabled;
}

fn sync_tray_icon<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: &RuntimeSnapshot,
) -> Result<(), AppError> {
    tray::sync_icon(app, snapshot).map_err(|error| {
        AppError::with_details("tray_icon_failed", "无法更新托盘图标", error.to_string())
    })
}

#[tauri::command]
pub fn get_app_state(state: State<'_, AppState>) -> Result<AppStateDto, AppError> {
    state.dto()
}

#[tauri::command]
pub fn start_dsh(state: State<'_, AppState>, app: AppHandle) -> Result<RuntimeSnapshot, AppError> {
    start_dsh_with_app(state, &app)
}

pub(crate) fn start_dsh_with_app<R: Runtime>(
    state: State<'_, AppState>,
    app: &AppHandle<R>,
) -> Result<RuntimeSnapshot, AppError> {
    let (result, config, current_snapshot) = {
        let mut lifecycle = state
            .lifecycle
            .lock()
            .map_err(|_| AppError::new("state_lock_poisoned", "管理器状态锁已损坏"))?;
        let result = lifecycle.start();
        let config = lifecycle.config().clone();
        let current_snapshot = lifecycle.snapshot();
        (result, config, current_snapshot)
    };
    let snapshot = match result {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let _ = sync_tray_icon(app, &current_snapshot);
            return Err(error);
        }
    };
    ConfigStore::save(state.config_path(), &config)?;
    sync_tray_icon(app, &snapshot)?;
    Ok(snapshot)
}

#[tauri::command]
pub fn stop_dsh(state: State<'_, AppState>, app: AppHandle) -> Result<RuntimeSnapshot, AppError> {
    stop_dsh_with_app(state, &app)
}

pub(crate) fn stop_dsh_with_app<R: Runtime>(
    state: State<'_, AppState>,
    app: &AppHandle<R>,
) -> Result<RuntimeSnapshot, AppError> {
    let (result, current_snapshot) = {
        let mut lifecycle = state
            .lifecycle
            .lock()
            .map_err(|_| AppError::new("state_lock_poisoned", "管理器状态锁已损坏"))?;
        guard_stop(&lifecycle.snapshot())?;
        let result = lifecycle.stop();
        let current_snapshot = lifecycle.snapshot();
        (result, current_snapshot)
    };
    let snapshot = match result {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let _ = sync_tray_icon(app, &current_snapshot);
            return Err(error);
        }
    };
    sync_tray_icon(app, &snapshot)?;
    Ok(snapshot)
}

#[tauri::command]
pub fn restart_dsh(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<RuntimeSnapshot, AppError> {
    restart_dsh_with_app(state, &app)
}

pub(crate) fn restart_dsh_with_app<R: Runtime>(
    state: State<'_, AppState>,
    app: &AppHandle<R>,
) -> Result<RuntimeSnapshot, AppError> {
    let (result, current_snapshot) = {
        let mut lifecycle = state
            .lifecycle
            .lock()
            .map_err(|_| AppError::new("state_lock_poisoned", "管理器状态锁已损坏"))?;
        let result = lifecycle.restart();
        let current_snapshot = lifecycle.snapshot();
        (result, current_snapshot)
    };
    let snapshot = match result {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let _ = sync_tray_icon(app, &current_snapshot);
            return Err(error);
        }
    };
    sync_tray_icon(app, &snapshot)?;
    Ok(snapshot)
}

#[tauri::command]
pub fn prepare_proxy_change(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<ProxyChangePlan, AppError> {
    let lifecycle = state
        .lifecycle
        .lock()
        .map_err(|_| AppError::new("state_lock_poisoned", "管理器状态锁已损坏"))?;
    prepare_proxy_change_for_test(
        &lifecycle.snapshot(),
        lifecycle.config().proxy.enabled,
        enabled,
    )
}

#[tauri::command]
pub fn apply_proxy_change(
    state: State<'_, AppState>,
    app: AppHandle,
    enabled: bool,
    confirmed_restart: bool,
) -> Result<RuntimeSnapshot, AppError> {
    let mut lifecycle = state
        .lifecycle
        .lock()
        .map_err(|_| AppError::new("state_lock_poisoned", "管理器状态锁已损坏"))?;
    let snapshot = lifecycle.snapshot();
    guard_apply_proxy_change(&snapshot, confirmed_restart)?;
    lifecycle.config_mut().proxy.enabled = enabled;
    lifecycle.sync_snapshot_target();
    let result = if matches!(
        (snapshot.state, snapshot.ownership),
        (
            LifecycleState::Running,
            Ownership::Managed | Ownership::Adopted
        )
    ) {
        lifecycle.restart()?
    } else {
        lifecycle.snapshot()
    };
    let config = lifecycle.config().clone();
    drop(lifecycle);
    ConfigStore::save(state.config_path(), &config)?;
    sync_tray_icon(&app, &result)?;
    Ok(result)
}

#[tauri::command]
pub fn set_active_target(
    state: State<'_, AppState>,
    target_id: TargetId,
) -> Result<AppStateDto, AppError> {
    let mut lifecycle = state
        .lifecycle
        .lock()
        .map_err(|_| AppError::new("state_lock_poisoned", "管理器状态锁已损坏"))?;
    if lifecycle.snapshot().state != LifecycleState::Stopped {
        return Err(AppError::new(
            "stop_before_switching_target",
            "切换目标前必须先停止 DSH",
        ));
    }
    lifecycle.config_mut().active_target = target_id;
    lifecycle.config().validate_active_target()?;
    lifecycle.sync_snapshot_target();
    let config = lifecycle.config().clone();
    let snapshot = lifecycle.snapshot();
    drop(lifecycle);
    ConfigStore::save(state.config_path(), &config)?;
    Ok(AppStateDto::from_controller(
        &config,
        snapshot,
        state.is_first_run(),
    ))
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppState>,
    app: AppHandle,
    settings: SettingsPatch,
) -> Result<AppStateDto, AppError> {
    let mut lifecycle = state
        .lifecycle
        .lock()
        .map_err(|_| AppError::new("state_lock_poisoned", "管理器状态锁已损坏"))?;
    let config_snapshot = {
        let config = lifecycle.config_mut();
        if let Some(value) = settings.start_on_login {
            config.manager.start_on_login = value;
        }
        if let Some(value) = settings.start_dsh_on_login {
            config.manager.start_dsh_on_login = value;
        }
        if let Some(value) = settings.service_port {
            config.service.port = value;
        }
        if let Some(value) = settings.proxy_enabled {
            config.proxy.enabled = value;
        }
        if let Some(value) = settings.proxy_url {
            config.proxy.url = value;
        }
        config.validate()?;
        config.clone()
    };
    lifecycle.sync_snapshot_target();
    let snapshot = lifecycle.snapshot();
    drop(lifecycle);
    ConfigStore::save(state.config_path(), &config_snapshot)?;
    sync_tray_icon(&app, &snapshot)?;
    state.mark_configured();
    Ok(AppStateDto::from_controller(
        &config_snapshot,
        snapshot,
        state.is_first_run(),
    ))
}

#[tauri::command]
pub fn adopt_external_dsh(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<RuntimeSnapshot, AppError> {
    let (result, current_snapshot) = {
        let mut lifecycle = state
            .lifecycle
            .lock()
            .map_err(|_| AppError::new("state_lock_poisoned", "管理器状态锁已损坏"))?;
        let result = lifecycle.adopt_external();
        let current_snapshot = lifecycle.snapshot();
        (result, current_snapshot)
    };
    let snapshot = match result {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let _ = sync_tray_icon(&app, &current_snapshot);
            return Err(error);
        }
    };
    sync_tray_icon(&app, &snapshot)?;
    Ok(snapshot)
}

#[tauri::command]
pub fn open_dsh_url(state: State<'_, AppState>, app: AppHandle) -> Result<(), AppError> {
    open_dsh_url_with_app(state, &app)
}

pub(crate) fn open_dsh_url_with_app<R: Runtime>(
    state: State<'_, AppState>,
    app: &AppHandle<R>,
) -> Result<(), AppError> {
    let lifecycle = state
        .lifecycle
        .lock()
        .map_err(|_| AppError::new("state_lock_poisoned", "管理器状态锁已损坏"))?;
    let url = lifecycle.config().service.url();
    tauri_plugin_opener::OpenerExt::opener(app)
        .open_url(url, None::<String>)
        .map_err(|error| {
            AppError::with_details("open_url_failed", "无法打开 DSH 页面", error.to_string())
        })
}

#[tauri::command]
pub fn open_log_directory(app: AppHandle) -> Result<(), AppError> {
    let path = app.path().app_log_dir().map_err(|error| {
        AppError::with_details("log_path_failed", "无法解析日志目录", error.to_string())
    })?;
    std::fs::create_dir_all(&path).map_err(AppError::from)?;
    tauri_plugin_opener::OpenerExt::opener(&app)
        .open_path(path.to_string_lossy().to_string(), None::<String>)
        .map_err(|error| {
            AppError::with_details("open_log_failed", "无法打开日志目录", error.to_string())
        })
}

#[tauri::command]
pub fn scan_targets(state: State<'_, AppState>) -> Result<Vec<DiscoveredTarget>, AppError> {
    let lifecycle = state
        .lifecycle
        .lock()
        .map_err(|_| AppError::new("state_lock_poisoned", "管理器状态锁已损坏"))?;
    let mut candidates = discovery::default_candidates();
    candidates.push(lifecycle.config().targets.source.working_directory.clone());
    if !lifecycle
        .config()
        .targets
        .packaged
        .executable
        .as_os_str()
        .is_empty()
    {
        if let Some(parent) = lifecycle.config().targets.packaged.executable.parent() {
            candidates.push(parent.to_path_buf());
        }
    }
    Ok(discovery::discover_targets_from(candidates))
}

#[tauri::command]
pub fn complete_first_run(
    state: State<'_, AppState>,
    app: AppHandle,
    setup: FirstRunSetup,
) -> Result<AppStateDto, AppError> {
    let mut lifecycle = state
        .lifecycle
        .lock()
        .map_err(|_| AppError::new("state_lock_poisoned", "管理器状态锁已损坏"))?;
    {
        let config = lifecycle.config_mut();
        if let Some(source) = setup.source.as_ref() {
            config.targets.source = discovery::to_target_config(source);
        }
        if let Some(packaged) = setup.packaged.as_ref() {
            config.targets.packaged = discovery::to_target_config(packaged);
        }
        config.active_target = setup.active_target;
        config.proxy.enabled = setup.proxy_enabled;
        config.proxy.url = setup.proxy_url;
        config.manager.start_on_login = setup.start_on_login;
        config.manager.start_dsh_on_login = setup.start_dsh_on_login;
        config.validate()?;
    }
    lifecycle.sync_snapshot_target();
    let config_snapshot = lifecycle.config().clone();
    let snapshot = lifecycle.snapshot();
    drop(lifecycle);
    ConfigStore::save(state.config_path(), &config_snapshot)?;
    sync_tray_icon(&app, &snapshot)?;
    state.mark_configured();
    Ok(AppStateDto::from_controller(
        &config_snapshot,
        snapshot,
        false,
    ))
}

#[tauri::command]
pub fn run_self_test(state: State<'_, AppState>) -> Result<SelfTestReport, AppError> {
    let lifecycle = state
        .lifecycle
        .lock()
        .map_err(|_| AppError::new("state_lock_poisoned", "管理器状态锁已损坏"))?;
    let config_ok = lifecycle.config().validate().is_ok();
    let target_ok = lifecycle.config().active_target_config().is_configured();
    let mut checks = vec![
        SelfTestCheck {
            name: "config".into(),
            passed: config_ok,
            message: if config_ok {
                "配置有效"
            } else {
                "配置无效"
            }
            .into(),
        },
        SelfTestCheck {
            name: "target".into(),
            passed: target_ok,
            message: if target_ok {
                "目标已配置"
            } else {
                "目标尚未配置"
            }
            .into(),
        },
    ];
    let pnpm_report = run_self_test_with(PnpmResolver::resolve());
    for item in pnpm_report.checks {
        let message = if item.remediation.is_empty() {
            item.message
        } else {
            format!("{}；{}", item.message, item.remediation)
        };
        checks.push(SelfTestCheck {
            name: item.name,
            passed: item.status == CheckStatus::Passed,
            message,
        });
    }
    Ok(SelfTestReport {
        healthy: checks.iter().all(|check| check.passed),
        checks,
    })
}

#[allow(dead_code)]
fn _keep_types_linked(_: DefaultLifecycleController, _: TargetConfig) {}
