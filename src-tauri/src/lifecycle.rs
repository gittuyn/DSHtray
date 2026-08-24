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
        job::JobOwner,
    },
    proxy::build_child_environment,
    targets::{build_packaged_command, build_source_command, TargetCommand},
};
use std::{
    collections::HashMap,
    ffi::OsString,
    process::{Child, Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
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
    fn launch(
        &mut self,
        target: &TargetConfig,
        proxy: &ProxyConfig,
        environment: &[(OsString, OsString)],
    ) -> Result<LaunchedProcess, AppError>;
    fn adopt(&mut self, pid: u32) -> Result<LaunchedProcess, AppError>;
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
        if let Some(listener) = listener {
            if let Ok(identity) = self.backend.inspect(listener.pid) {
                if identity_matches(&target, &identity) {
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
            process.job.terminate()?;
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
            return Err(error);
        }
        self.snapshot = RuntimeSnapshot::stopped(&self.config);
        Ok(self.snapshot())
    }

    pub fn restart(&mut self) -> Result<RuntimeSnapshot, AppError> {
        let config_snapshot = self.config.clone();
        if self.snapshot.state != LifecycleState::Stopped {
            self.stop()?;
        }
        self.config = config_snapshot;
        self.start()
    }

    pub fn refresh_external_state(&mut self) -> Result<RuntimeSnapshot, AppError> {
        let listener = self.backend.find_listener(&self.config.service)?;
        let Some(listener) = listener else {
            self.snapshot = RuntimeSnapshot::stopped(&self.config);
            return Ok(self.snapshot());
        };
        let target = self.config.active_target_config();
        let identity = self.backend.inspect(listener.pid).ok();
        if identity
            .as_ref()
            .is_some_and(|identity| identity_matches(target, identity))
        {
            self.snapshot.state = LifecycleState::External;
            self.snapshot.target = self.config.active_target;
            self.snapshot.pid = Some(listener.pid);
            self.snapshot.ownership = Ownership::External;
            self.snapshot.service_url = self.config.service.url();
            self.snapshot.proxy_enabled = self.config.proxy.enabled;
            self.snapshot.last_error = None;
            return Ok(self.snapshot());
        }
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
        let pid = self
            .snapshot
            .pid
            .ok_or_else(|| AppError::new("external_pid_missing", "外部 DSH PID 不可用"))?;
        self.process = Some(self.backend.adopt(pid)?);
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

fn identity_matches(target: &TargetConfig, identity: &ProcessIdentity) -> bool {
    match target.kind {
        TargetKind::Packaged => {
            normalize_path(&target.executable) == normalize_path(&identity.executable)
        }
        TargetKind::Source => {
            let command_line = identity
                .command_line
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase();
            let root = normalize_path(&target.working_directory).to_ascii_lowercase();
            command_line.contains(&root)
                && command_line.contains("dsh")
                && command_line.contains("web")
        }
    }
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

struct WindowsJobControl(JobOwner);

impl JobControl for WindowsJobControl {
    fn is_empty(&self) -> bool {
        self.0.is_empty().unwrap_or(false)
    }

    fn terminate(&mut self) -> Result<(), AppError> {
        self.0.terminate()
    }
}

#[derive(Default)]
pub struct WindowsProcessAdapter {
    children: HashMap<u32, Child>,
}

impl ProcessAdapter for WindowsProcessAdapter {
    fn find_listener(&mut self, config: &ServiceConfig) -> Result<Option<ListenerOwner>, AppError> {
        find_listener(config)
    }

    fn inspect(&mut self, pid: u32) -> Result<ProcessIdentity, AppError> {
        ProcessInspector::inspect(pid)
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
        let job = new_job(pid)?;
        job.assign(pid)?;
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
        let job = new_job(pid)?;
        job.assign(pid)?;
        self.children.insert(pid, child);
        Ok(LaunchedProcess {
            pid,
            process_group_id: pid,
            job: Box::new(WindowsJobControl(job)),
        })
    }
}

fn new_job(pid: u32) -> Result<JobOwner, AppError> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    JobOwner::create_or_open(format!("Local\\DeepSeekHarnessManager-{}-{}", pid, stamp))
}

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
