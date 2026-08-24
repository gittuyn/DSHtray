#![allow(dead_code)]

use dshtray_lib::{
    domain::{
        AppConfig, LifecycleState, Ownership, ProxyConfig, ServiceConfig, TargetConfig, TargetId,
        TargetKind,
    },
    health::HealthResult,
    lifecycle::{
        Clock, HealthAdapter, JobControl, LaunchedProcess, LifecycleController, ProcessAdapter,
    },
    network::ListenerOwner,
    process::inspect::ProcessIdentity,
};
use std::{ffi::OsString, path::PathBuf, time::Duration};

#[derive(Clone, Copy)]
enum LaunchMode {
    Ready,
    NoSpawn,
    Hanging,
    External,
    ListenerError,
}

struct FakeJob {
    empty: bool,
    terminate_calls: usize,
}

impl JobControl for FakeJob {
    fn is_empty(&self) -> bool {
        self.empty
    }

    fn terminate(&mut self) -> Result<(), dshtray_lib::app_error::AppError> {
        self.empty = true;
        self.terminate_calls += 1;
        Ok(())
    }
}

struct FakeAdapter {
    mode: LaunchMode,
    listener: Option<ListenerOwner>,
    terminate_calls: usize,
    launches: usize,
    last_target: Option<TargetId>,
    alive: bool,
}

impl FakeAdapter {
    fn ready() -> Self {
        Self {
            mode: LaunchMode::Ready,
            listener: None,
            terminate_calls: 0,
            launches: 0,
            last_target: None,
            alive: false,
        }
    }

    fn no_spawn() -> Self {
        Self {
            mode: LaunchMode::NoSpawn,
            ..Self::ready()
        }
    }

    fn hanging() -> Self {
        Self {
            mode: LaunchMode::Hanging,
            ..Self::ready()
        }
    }

    fn external() -> Self {
        Self {
            mode: LaunchMode::External,
            listener: Some(ListenerOwner {
                pid: 4012,
                local_address: "127.0.0.1".into(),
                port: 3080,
            }),
            ..Self::ready()
        }
    }

    fn listener_error() -> Self {
        Self {
            mode: LaunchMode::ListenerError,
            ..Self::ready()
        }
    }
}

impl ProcessAdapter for FakeAdapter {
    fn find_listener(
        &mut self,
        _: &ServiceConfig,
    ) -> Result<Option<ListenerOwner>, dshtray_lib::app_error::AppError> {
        if matches!(self.mode, LaunchMode::ListenerError) {
            return Err(dshtray_lib::app_error::AppError::new(
                "listener_query_failed",
                "fixture listener query failed",
            ));
        }
        Ok(self.listener.clone())
    }

    fn inspect(&mut self, pid: u32) -> Result<ProcessIdentity, dshtray_lib::app_error::AppError> {
        if matches!(self.mode, LaunchMode::External) && pid == 4012 {
            Ok(ProcessIdentity {
                pid,
                executable: PathBuf::from("C:/fixture/DSH.exe"),
                command_line: Some(format!(
                    "{} dsh web",
                    std::env::current_dir()
                        .expect("current directory")
                        .display()
                )),
                parent_pid: None,
            })
        } else {
            Err(dshtray_lib::app_error::AppError::new(
                "unknown_process",
                "unknown fixture process",
            ))
        }
    }

    fn launch(
        &mut self,
        target: &TargetConfig,
        _: &ProxyConfig,
        _: &[(OsString, OsString)],
    ) -> Result<LaunchedProcess, dshtray_lib::app_error::AppError> {
        if matches!(self.mode, LaunchMode::NoSpawn) {
            return Err(dshtray_lib::app_error::AppError::new(
                "launch_failed",
                "fixture refuses to spawn",
            ));
        }
        self.launches += 1;
        self.last_target = Some(match target.kind {
            TargetKind::Source => TargetId::Source,
            TargetKind::Packaged => TargetId::Packaged,
        });
        self.alive = true;
        Ok(LaunchedProcess {
            pid: 9001 + self.launches as u32,
            process_group_id: 9001 + self.launches as u32,
            job: Box::new(FakeJob {
                empty: false,
                terminate_calls: 0,
            }),
        })
    }

    fn adopt(&mut self, pid: u32) -> Result<LaunchedProcess, dshtray_lib::app_error::AppError> {
        Ok(LaunchedProcess {
            pid,
            process_group_id: pid,
            job: Box::new(FakeJob {
                empty: false,
                terminate_calls: 0,
            }),
        })
    }

    fn is_alive(&mut self, _: u32) -> bool {
        self.alive || matches!(self.mode, LaunchMode::Hanging)
    }

    fn request_graceful_stop(&mut self, _: u32) {
        if !matches!(self.mode, LaunchMode::Hanging) {
            self.alive = false;
        }
    }
}

struct FakeHealth {
    result: HealthResult,
    seen: Vec<ServiceConfig>,
}

impl FakeHealth {
    fn ready() -> Self {
        Self {
            result: HealthResult::Ready { status: 200 },
            seen: Vec::new(),
        }
    }

    fn unreachable() -> Self {
        Self {
            result: HealthResult::Unreachable {
                code: "fixture_unreachable".into(),
                message: "fixture unreachable".into(),
            },
            seen: Vec::new(),
        }
    }
}

impl HealthAdapter for FakeHealth {
    fn check(&mut self, config: &ServiceConfig) -> HealthResult {
        self.seen.push(config.clone());
        self.result.clone()
    }
}

#[derive(Default)]
struct FakeClock {
    elapsed: Duration,
}

impl Clock for FakeClock {
    fn sleep(&mut self, duration: Duration) {
        self.elapsed += duration;
    }
}

fn controller_with(
    adapter: FakeAdapter,
    health: FakeHealth,
) -> LifecycleController<FakeAdapter, FakeHealth, FakeClock> {
    let mut config = AppConfig::defaults();
    config.targets.source = TargetConfig::source(
        "fixture",
        std::env::current_dir().expect("current directory"),
    );
    LifecycleController::new(config, adapter, health, FakeClock::default())
}

#[test]
fn start_transitions_stopped_to_starting_to_running() {
    let mut controller = controller_with(FakeAdapter::ready(), FakeHealth::ready());
    assert_eq!(controller.snapshot().state, LifecycleState::Stopped);
    controller.start().expect("start fake target");
    assert_eq!(controller.snapshot().state, LifecycleState::Running);
}

#[test]
fn unknown_listener_enters_port_conflict_without_killing_pid() {
    let mut controller = controller_with(FakeAdapter::no_spawn(), FakeHealth::unreachable());
    controller.set_listener_for_test(Some(ListenerOwner {
        pid: 4012,
        local_address: "127.0.0.1".into(),
        port: 3080,
    }));
    let error = controller
        .start()
        .expect_err("unknown listener must block start");
    assert_eq!(error.code, "port_conflict");
    assert_eq!(controller.backend().terminate_calls, 0);
}

#[test]
fn listener_query_failure_does_not_launch_a_new_dsh() {
    let mut controller = controller_with(FakeAdapter::listener_error(), FakeHealth::ready());
    let error = controller
        .start()
        .expect_err("listener query failure must block start");
    assert_eq!(error.code, "listener_query_failed");
    assert_eq!(controller.backend().launches, 0);
}

#[test]
fn stop_waits_five_seconds_before_forcing_owned_job() {
    let mut controller = controller_with(FakeAdapter::hanging(), FakeHealth::ready());
    controller.start().expect("start fake target");
    controller.stop().expect("force after graceful wait");
    assert_eq!(controller.clock().elapsed, Duration::from_secs(5));
    assert_eq!(controller.backend().terminate_calls, 0);
}

#[test]
fn external_process_is_observe_only_until_adopted() {
    let mut controller = controller_with(FakeAdapter::external(), FakeHealth::ready());
    controller
        .refresh_external_state()
        .expect("observe external");
    assert_eq!(controller.snapshot().ownership, Ownership::External);
    let error = controller
        .stop()
        .expect_err("external process is observe-only");
    assert_eq!(error.code, "external_not_adopted");
}

#[test]
fn restart_uses_one_config_snapshot_for_command_and_health_url() {
    let mut controller = controller_with(FakeAdapter::ready(), FakeHealth::ready());
    controller.start().expect("initial start");
    controller.restart().expect("restart fake target");
    assert_eq!(controller.backend().launches, 2);
    assert!(controller
        .health()
        .seen
        .iter()
        .all(|service| service.port == 3080));
}
