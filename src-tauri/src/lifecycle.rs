use crate::process_flags::windows_creation_flags;
use crate::{
    app_error::AppError,
    domain::{
        AppConfig, LifecycleState, Ownership, ProxyConfig, RuntimeSnapshot, ServiceConfig,
        TargetConfig, TargetKind,
    },
    health::{HealthChecker, HealthResult},
    network::{find_listener, ListenerOwner},
    process::{
        graceful_stop::GracefulStop,
        inspect::{ProcessIdentity, ProcessInspector},
        job::{JobOwner, PidTreeOwner},
    },
    proxy::build_child_environment,
    targets::{build_packaged_command, build_source_command, TargetCommand},
};
use std::{
    collections::HashMap,
    ffi::OsString,
    process::{Child, Command, Stdio},
    time::{Duration, SystemTime},
};

pub trait JobControl: Send {
    fn is_empty(&self) -> bool;
    fn terminate(&mut self) -> Result<(), AppError>;
}

pub struct LaunchedProcess {
    pub pid: u32,
    pub process_group_id: u32,
    pub job: Box<dyn JobControl>,
}

pub trait ProcessAdapter: Send {
    fn find_listener(&mut self, config: &ServiceConfig) -> Result<Option<ListenerOwner>, AppError>;
    fn inspect(&mut self, pid: u32) -> Result<ProcessIdentity, AppError>;
    fn inspect_chain(&mut self, pid: u32) -> Result<Vec<ProcessIdentity>, AppError> {
        let mut chain = Vec::new();
        let mut current = Some(pid);
        let mut seen = std::collections::HashSet::new();
        while let Some(current_pid) = current {
            if current_pid == 0 || !seen.insert(current_pid) || chain.len() >= 32 {
                break;
            }
            let identity = match self.inspect(current_pid) {
                Ok(identity) => identity,
                Err(error) if chain.is_empty() => return Err(error),
                Err(_) => break,
            };
            current = identity.parent_pid;
            chain.push(identity);
        }
        Ok(chain)
    }
    fn is_process_in_job(&mut self, _pid: u32) -> Result<bool, AppError> {
        Ok(false)
    }
    fn launch(
        &mut self,
        target: &TargetConfig,
        proxy: &ProxyConfig,
        environment: &[(OsString, OsString)],
    ) -> Result<LaunchedProcess, AppError>;
    fn adopt(&mut self, pid: u32) -> Result<LaunchedProcess, AppError>;
    fn adopt_tree(&mut self, pid: u32, _process_ids: &[u32]) -> Result<LaunchedProcess, AppError> {
        self.adopt(pid)
    }
    fn is_alive(&mut self, pid: u32) -> bool;
    fn request_graceful_stop(&mut self, process_group_id: u32);
}

pub trait HealthAdapter: Send {
    fn check(&mut self, config: &ServiceConfig) -> HealthResult;
}

pub trait Clock: Send {
    fn sleep(&mut self, duration: Duration);
}

pub struct LifecycleController<P, H, C>
where
    P: ProcessAdapter,
    H: HealthAdapter,
    C: Clock,
{
    config: AppConfig,
    backend: P,
    health: H,
    clock: C,
    snapshot: RuntimeSnapshot,
    process: Option<LaunchedProcess>,
    external_root_pid: Option<u32>,
    external_process_ids: Vec<u32>,
    listener_override: Option<Option<ListenerOwner>>,
    generation: u64,
}

impl<P, H, C> LifecycleController<P, H, C>
where
    P: ProcessAdapter,
    H: HealthAdapter,
    C: Clock,
{
    pub fn new(config: AppConfig, backend: P, health: H, clock: C) -> Self {
        let snapshot = RuntimeSnapshot::stopped(&config);
        Self {
            config,
            backend,
            health,
            clock,
            snapshot,
            process: None,
            external_root_pid: None,
            external_process_ids: Vec::new(),
            listener_override: None,
            generation: 0,
        }
    }

    pub fn snapshot(&self) -> RuntimeSnapshot {
        self.snapshot.clone()
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut AppConfig {
        &mut self.config
    }

    pub fn sync_snapshot_target(&mut self) {
        self.snapshot.target = self.config.active_target;
        self.snapshot.service_url = self.config.service.url();
        self.snapshot.proxy_enabled = self.config.proxy.enabled;
    }

    pub fn backend(&self) -> &P {
        &self.backend
    }

    pub fn health(&self) -> &H {
        &self.health
    }

    pub fn clock(&self) -> &C {
        &self.clock
    }

    pub fn backend_mut(&mut self) -> &mut P {
        &mut self.backend
    }

    pub fn set_listener_for_test(&mut self, listener: Option<ListenerOwner>) {
        self.listener_override = Some(listener);
    }

    pub fn start(&mut self) -> Result<RuntimeSnapshot, AppError> {
        if matches!(
            self.snapshot.state,
            LifecycleState::Starting
                | LifecycleState::Running
                | LifecycleState::External
                | LifecycleState::Stopping
        ) {
            return Err(AppError::new("already_running", "DSH 当前已经在运行"));
        }
        self.config.validate_active_target()?;
        let target_id = self.config.active_target;
        let target = self.config.active_target_config().clone();
        let listener = self.take_listener()?;
        self.external_root_pid = None;
        self.external_process_ids.clear();
        if let Some(listener) = listener {
            let chain = self.backend.inspect_chain(listener.pid).ok();
            if let Some(chain) = chain.as_deref() {
                if let Some(root_pid) = target_root_pid(&target, chain) {
                    self.external_root_pid = Some(root_pid);
                    self.external_process_ids = chain_pids_to_root(chain, root_pid);
                    self.snapshot = RuntimeSnapshot {
                        state: LifecycleState::External,
                        target: target_id,
                        pid: Some(listener.pid),
                        ownership: Ownership::External,
                        service_url: self.config.service.url(),
                        proxy_enabled: self.config.proxy.enabled,
                        last_error: None,
                        started_at: None,
                    };
                    return Ok(self.snapshot());
                }
            }
            let error = AppError::with_details(
                "port_conflict",
                "服务端口已被未知进程占用，未执行停止或终止操作",
                format!("pid={}", listener.pid),
            );
            self.snapshot.state = LifecycleState::PortConflict;
            self.snapshot.last_error = Some(error.clone());
            return Err(error);
        }

        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        self.snapshot = RuntimeSnapshot {
            state: LifecycleState::Starting,
            target: target_id,
            pid: None,
            ownership: Ownership::None,
            service_url: self.config.service.url(),
            proxy_enabled: self.config.proxy.enabled,
            last_error: None,
            started_at: None,
        };
        let inherited: Vec<(OsString, OsString)> = std::env::vars_os().collect();
        let launched = match self.backend.launch(
            &target,
            &self.config.proxy,
            &build_child_environment(&self.config.proxy, &inherited),
        ) {
            Ok(launched) => launched,
            Err(error) => return Err(self.fail_start(error)),
        };
        self.snapshot.pid = Some(launched.pid);
        self.snapshot.ownership = Ownership::Managed;
        let pid = launched.pid;
        let mut launched = Some(launched);
        for _ in 0..180 {
            if generation != self.generation {
                let error = AppError::new("stale_start", "启动任务已过期");
                if let Some(mut process) = launched.take() {
                    let _ = process.job.terminate();
                }
                return Err(self.fail_start(error));
            }
            match self.health.check(&self.config.service) {
                HealthResult::Ready { .. } => {
                    self.process = launched.take();
                    self.snapshot.state = LifecycleState::Running;
                    self.snapshot.started_at = Some(SystemTime::now());
                    return Ok(self.snapshot());
                }
                HealthResult::Unreachable { .. } | HealthResult::UnexpectedStatus { .. } => {
                    if !self.backend.is_alive(pid) {
                        let error = AppError::new("startup_failed", "DSH 进程在服务就绪前退出");
                        if let Some(mut process) = launched.take() {
                            let _ = process.job.terminate();
                        }
                        return Err(self.fail_start(error));
                    }
                    self.clock.sleep(Duration::from_millis(500));
                }
            }
        }
        if let Some(mut process) = launched.take() {
            let _ = process.job.terminate();
        }
        Err(self.fail_start(AppError::new(
            "startup_timeout",
            "等待 DSH 服务就绪超时（90 秒）",
        )))
    }

    pub fn stop(&mut self) -> Result<RuntimeSnapshot, AppError> {
        if self.snapshot.ownership == Ownership::External {
            return Err(AppError::new(
                "external_not_adopted",
                "外部 DSH 尚未被用户确认接管",
            ));
        }
        let Some(mut process) = self.process.take() else {
            if self.snapshot.state == LifecycleState::Stopped {
                return Ok(self.snapshot());
            }
            return Err(AppError::new("not_running", "没有可停止的 DSH 进程"));
        };
        self.snapshot.state = LifecycleState::Stopping;
        self.backend.request_graceful_stop(process.process_group_id);
        let mut elapsed = Duration::ZERO;
        while elapsed < Duration::from_secs(5) && !process.job.is_empty() {
            self.clock.sleep(Duration::from_millis(100));
            elapsed += Duration::from_millis(100);
        }
        if !process.job.is_empty() {
            if let Err(error) = process.job.terminate() {
                self.snapshot.state = LifecycleState::Failed;
                self.snapshot.last_error = Some(error.clone());
                self.process = Some(process);
                return Err(error);
            }
        }
        let mut force_elapsed = Duration::ZERO;
        while force_elapsed < Duration::from_secs(5) && !process.job.is_empty() {
            self.clock.sleep(Duration::from_millis(100));
            force_elapsed += Duration::from_millis(100);
        }
        if !process.job.is_empty() {
            let error = AppError::new("stop_timeout", "强制停止 DSH 进程树超时");
            self.snapshot.state = LifecycleState::Failed;
            self.snapshot.last_error = Some(error.clone());
            self.process = Some(process);
            return Err(error);
        }
        if let Err(error) = self.wait_for_listener_to_clear() {
            self.process = Some(process);
            return Err(error);
        }
        self.snapshot = RuntimeSnapshot::stopped(&self.config);
        Ok(self.snapshot())
    }

    fn wait_for_listener_to_clear(&mut self) -> Result<(), AppError> {
        for _ in 0..50 {
            match self.backend.find_listener(&self.config.service)? {
                None => return Ok(()),
                Some(_) => self.clock.sleep(Duration::from_millis(100)),
            }
        }
        let error = AppError::new("stop_timeout", "等待 DSH 服务端口释放超时");
        self.snapshot.state = LifecycleState::Failed;
        self.snapshot.last_error = Some(error.clone());
        Err(error)
    }

    pub fn restart(&mut self) -> Result<RuntimeSnapshot, AppError> {
        let config_snapshot = self.config.clone();
        if self.snapshot.state != LifecycleState::Stopped {
            match self.snapshot.ownership {
                Ownership::Managed | Ownership::Adopted => {
                    self.stop()?;
                }
                Ownership::External => {
                    return Err(AppError::new(
                        "external_not_adopted",
                        "外部 DSH 尚未被用户确认接管",
                    ));
                }
                Ownership::None => {
                    let refreshed = self.refresh_external_state()?;
                    if refreshed.state == LifecycleState::External {
                        return Err(AppError::new(
                            "external_not_adopted",
                            "外部 DSH 尚未被用户确认接管",
                        ));
                    }
                    if refreshed.state != LifecycleState::Stopped {
                        return Err(refreshed.last_error.unwrap_or_else(|| {
                            AppError::new("restart_not_ready", "DSH 当前状态不允许重启")
                        }));
                    }
                }
            }
        }
        self.config = config_snapshot;
        self.start()
    }

    pub fn refresh_external_state(&mut self) -> Result<RuntimeSnapshot, AppError> {
        let listener = self.backend.find_listener(&self.config.service)?;
        let Some(listener) = listener else {
            self.external_root_pid = None;
            self.external_process_ids.clear();
            self.snapshot = RuntimeSnapshot::stopped(&self.config);
            return Ok(self.snapshot());
        };
        let target = self.config.active_target_config();
        let chain = self.backend.inspect_chain(listener.pid).ok();
        if let Some(chain) = chain.as_deref() {
            if let Some(root_pid) = target_root_pid(target, chain) {
                self.external_root_pid = Some(root_pid);
                self.external_process_ids = chain_pids_to_root(chain, root_pid);
                self.snapshot.state = LifecycleState::External;
                self.snapshot.target = self.config.active_target;
                self.snapshot.pid = Some(listener.pid);
                self.snapshot.ownership = Ownership::External;
                self.snapshot.service_url = self.config.service.url();
                self.snapshot.proxy_enabled = self.config.proxy.enabled;
                self.snapshot.last_error = None;
                return Ok(self.snapshot());
            }
        }
        self.external_root_pid = None;
        self.external_process_ids.clear();
        let error = AppError::with_details(
            "port_conflict",
            "服务端口已被未知进程占用",
            format!("pid={}", listener.pid),
        );
        self.snapshot.state = LifecycleState::PortConflict;
        self.snapshot.last_error = Some(error);
        Ok(self.snapshot())
    }

    pub fn adopt_external(&mut self) -> Result<RuntimeSnapshot, AppError> {
        if self.snapshot.ownership != Ownership::External {
            return Err(AppError::new("external_not_found", "没有可接管的外部 DSH"));
        }
        let refreshed = self.refresh_external_state()?;
        if refreshed.state != LifecycleState::External {
            return Err(refreshed.last_error.unwrap_or_else(|| {
                AppError::new("external_not_found", "外部 DSH 已退出或状态已变化")
            }));
        }
        let pid = self
            .external_root_pid
            .or(self.snapshot.pid)
            .ok_or_else(|| AppError::new("external_pid_missing", "外部 DSH PID 不可用"))?;
        let mut process_ids = self.external_process_ids.clone();
        if !process_ids.contains(&pid) {
            process_ids.push(pid);
        }
        let adopted = match self.backend.adopt_tree(pid, &process_ids) {
            Ok(process) => process,
            Err(error) => {
                self.snapshot.last_error = Some(error.clone());
                return Err(error);
            }
        };
        self.process = Some(adopted);
        self.external_root_pid = None;
        self.external_process_ids.clear();
        self.snapshot.ownership = Ownership::Adopted;
        self.snapshot.state = LifecycleState::Running;
        Ok(self.snapshot())
    }

    fn fail_start(&mut self, error: AppError) -> AppError {
        self.snapshot.state = LifecycleState::Failed;
        self.snapshot.last_error = Some(error.clone());
        self.process = None;
        error
    }

    fn take_listener(&mut self) -> Result<Option<ListenerOwner>, AppError> {
        match self.listener_override.take() {
            Some(listener) => Ok(listener),
            None => self.backend.find_listener(&self.config.service),
        }
    }
}

fn target_root_pid(target: &TargetConfig, chain: &[ProcessIdentity]) -> Option<u32> {
    match target.kind {
        TargetKind::Packaged => chain
            .iter()
            .find(|identity| {
                normalize_path(&target.executable) == normalize_path(&identity.executable)
            })
            .map(|identity| identity.pid),
        TargetKind::Source => {
            let root = normalize_path(&target.working_directory);
            let source_matches = |identity: &ProcessIdentity| {
                let command_line = identity
                    .command_line
                    .as_deref()
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .replace('\\', "/");
                let working_directory_matches = identity
                    .working_directory
                    .as_deref()
                    .is_some_and(|path| path_is_same_or_descendant(path, &root));
                let entrypoint =
                    command_line.contains("apps/cli/src/bin.ts") && command_line.contains("web");
                let launcher = command_line.contains("pnpm")
                    && command_line.contains("dsh")
                    && command_line.contains("web");
                working_directory_matches && (entrypoint || launcher)
            };
            chain
                .iter()
                .rev()
                .find(|identity| {
                    let command_line = identity
                        .command_line
                        .as_deref()
                        .unwrap_or_default()
                        .to_ascii_lowercase();
                    identity
                        .working_directory
                        .as_deref()
                        .is_some_and(|path| normalize_path(path) == root)
                        && command_line.contains("pnpm")
                        && command_line.contains("dsh")
                        && command_line.contains("web")
                })
                .or_else(|| chain.iter().rev().find(|identity| source_matches(identity)))
                .map(|identity| identity.pid)
        }
    }
}

fn chain_pids_to_root(chain: &[ProcessIdentity], root_pid: u32) -> Vec<u32> {
    let Some(root_index) = chain.iter().position(|identity| identity.pid == root_pid) else {
        return vec![root_pid];
    };
    chain[..=root_index]
        .iter()
        .rev()
        .map(|identity| identity.pid)
        .collect()
}

fn unique_process_ids(root_pid: u32, process_ids: &[u32]) -> Vec<u32> {
    let mut unique = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for process_id in process_ids.iter().copied().chain(std::iter::once(root_pid)) {
        if process_id != 0 && seen.insert(process_id) {
            unique.push(process_id);
        }
    }
    unique
}

fn normalize_path(path: &std::path::Path) -> String {
    let mut normalized = std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase();
    if let Some(stripped) = normalized.strip_prefix("\\\\?\\") {
        normalized = stripped.to_owned();
    }
    normalized
}

fn path_is_same_or_descendant(path: &std::path::Path, root: &str) -> bool {
    let normalized = normalize_path(path);
    normalized == root
        || normalized
            .strip_prefix(root)
            .is_some_and(|suffix| suffix.starts_with('\\'))
}

struct WindowsJobControl(JobOwner);

impl JobControl for WindowsJobControl {
    fn is_empty(&self) -> bool {
        self.0.is_empty().unwrap_or(false)
    }

    fn terminate(&mut self) -> Result<(), AppError> {
        self.0.terminate()
    }
}

struct WindowsPidTreeControl(PidTreeOwner);

impl JobControl for WindowsPidTreeControl {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn terminate(&mut self) -> Result<(), AppError> {
        self.0.terminate()
    }
}

pub struct WindowsProcessAdapter {
    children: HashMap<u32, Child>,
    manager_job_name: String,
}

impl Default for WindowsProcessAdapter {
    fn default() -> Self {
        Self::with_job_name(MANAGED_JOB_NAME)
    }
}

impl WindowsProcessAdapter {
    pub fn with_job_name(name: impl Into<String>) -> Self {
        Self {
            children: HashMap::new(),
            manager_job_name: name.into(),
        }
    }

    fn manager_job(&self) -> Result<JobOwner, AppError> {
        JobOwner::create_or_open(&self.manager_job_name)
    }
}

impl ProcessAdapter for WindowsProcessAdapter {
    fn find_listener(&mut self, config: &ServiceConfig) -> Result<Option<ListenerOwner>, AppError> {
        find_listener(config)
    }

    fn inspect(&mut self, pid: u32) -> Result<ProcessIdentity, AppError> {
        ProcessInspector::inspect(pid)
    }

    fn is_process_in_job(&mut self, pid: u32) -> Result<bool, AppError> {
        let manager_job = self.manager_job()?;
        if manager_job.process_ids()?.contains(&pid) {
            return Ok(false);
        }
        JobOwner::is_process_in_job(pid)
    }

    fn launch(
        &mut self,
        target: &TargetConfig,
        _: &ProxyConfig,
        environment: &[(OsString, OsString)],
    ) -> Result<LaunchedProcess, AppError> {
        let command = match target.kind {
            TargetKind::Source => build_source_command(target)?,
            TargetKind::Packaged => build_packaged_command(target)?,
        };
        self.spawn_command(command, environment)
    }

    fn adopt(&mut self, pid: u32) -> Result<LaunchedProcess, AppError> {
        self.adopt_tree(pid, &[pid])
    }

    fn adopt_tree(&mut self, pid: u32, process_ids: &[u32]) -> Result<LaunchedProcess, AppError> {
        let unique_process_ids = unique_process_ids(pid, process_ids);
        let job = self.manager_job()?;
        let manager_process_ids = job.process_ids()?;
        let mut external_job_found = false;
        for process_id in unique_process_ids.iter().copied() {
            if manager_process_ids.contains(&process_id) {
                continue;
            }
            if JobOwner::is_process_in_job(process_id)? {
                external_job_found = true;
                break;
            }
        }
        if external_job_found {
            return self.adopt_with_pid_control(pid, &unique_process_ids);
        }
        for process_id in unique_process_ids {
            if manager_process_ids.contains(&process_id) {
                continue;
            }
            if let Err(error) = job.assign(process_id) {
                if matches!(JobOwner::is_process_in_job(process_id), Ok(true)) {
                    return self.adopt_with_pid_control(pid, process_ids);
                }
                return Err(error);
            }
        }
        Ok(LaunchedProcess {
            pid,
            process_group_id: pid,
            job: Box::new(WindowsJobControl(job)),
        })
    }

    fn is_alive(&mut self, pid: u32) -> bool {
        if let Some(child) = self.children.get_mut(&pid) {
            match child.try_wait() {
                Ok(None) => true,
                Ok(Some(_)) | Err(_) => {
                    self.children.remove(&pid);
                    false
                }
            }
        } else {
            ProcessInspector::inspect(pid).is_ok()
        }
    }

    fn request_graceful_stop(&mut self, process_group_id: u32) {
        let _ = GracefulStop::request(process_group_id);
    }
}

impl WindowsProcessAdapter {
    fn adopt_with_pid_control(
        &self,
        pid: u32,
        process_ids: &[u32],
    ) -> Result<LaunchedProcess, AppError> {
        let unique_process_ids = unique_process_ids(pid, process_ids);
        let processes = unique_process_ids
            .into_iter()
            .map(|process_id| {
                ProcessInspector::inspect(process_id).map_err(|error| {
                    AppError::with_details(
                        "external_process_recheck_failed",
                        "无法重新确认外部 DSH 进程，未建立强制控制",
                        format!("pid={process_id}; {error}"),
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(LaunchedProcess {
            pid,
            process_group_id: pid,
            job: Box::new(WindowsPidTreeControl(PidTreeOwner::new(pid, processes))),
        })
    }

    fn spawn_command(
        &mut self,
        command: TargetCommand,
        environment: &[(OsString, OsString)],
    ) -> Result<LaunchedProcess, AppError> {
        let mut process = Command::new(command.program);
        process.args(command.args);
        process.current_dir(command.working_directory);
        process.envs(environment.iter().cloned());
        process
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            process.creation_flags(windows_creation_flags());
        }
        let child = process.spawn().map_err(|error| {
            AppError::with_details("launch_failed", "无法启动 DSH 目标", error.to_string())
        })?;
        let pid = child.id();
        let job = self.manager_job()?;
        job.assign(pid)?;
        self.children.insert(pid, child);
        Ok(LaunchedProcess {
            pid,
            process_group_id: pid,
            job: Box::new(WindowsJobControl(job)),
        })
    }
}

const MANAGED_JOB_NAME: &str = "Local\\DeepSeekHarnessManager";

#[derive(Clone)]
pub struct BlockingHealthAdapter {
    checker: HealthChecker,
}

impl Default for BlockingHealthAdapter {
    fn default() -> Self {
        Self {
            checker: HealthChecker::with_proxy_disabled(),
        }
    }
}

impl HealthAdapter for BlockingHealthAdapter {
    fn check(&mut self, config: &ServiceConfig) -> HealthResult {
        let checker = self.checker.clone();
        let config = config.clone();
        std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("create health runtime")
                .block_on(checker.check(&config))
        })
        .join()
        .unwrap_or_else(|_| HealthResult::Unreachable {
            code: "health_worker_failed".into(),
            message: "健康检查线程异常退出".into(),
        })
    }
}

#[derive(Default)]
pub struct RealClock;

impl Clock for RealClock {
    fn sleep(&mut self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

pub type DefaultLifecycleController =
    LifecycleController<WindowsProcessAdapter, BlockingHealthAdapter, RealClock>;
