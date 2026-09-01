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
use std::{collections::HashSet, ffi::OsString, path::PathBuf, time::Duration};

#[derive(Clone, Copy)]
enum LaunchMode {
    Ready,
    Exited,
    NoSpawn,
    Hanging,
    TerminateError,
    External,
    SourceListener,
    ListenerError,
}

struct FakeJob {
    empty: bool,
    terminate_calls: usize,
    termination_error: Option<dshtray_lib::app_error::AppError>,
}

impl JobControl for FakeJob {
    fn is_empty(&self) -> bool {
        self.empty
    }

    fn terminate(&mut self) -> Result<(), dshtray_lib::app_error::AppError> {
        if let Some(error) = &self.termination_error {
            self.terminate_calls += 1;
            return Err(error.clone());
        }
        self.empty = true;
        self.terminate_calls += 1;
        Ok(())
    }
}

struct FakeAdapter {
    mode: LaunchMode,
    listener: Option<ListenerOwner>,
    listener_sequence: Vec<Option<ListenerOwner>>,
    chain: Option<Vec<ProcessIdentity>>,
    terminate_calls: usize,
    launches: usize,
    last_target: Option<TargetId>,
    last_adopted_pid: Option<u32>,
    last_adopted_pids: Vec<u32>,
    last_adoption_was_direct: bool,
    job_membership: HashSet<u32>,
    alive: bool,
}

impl FakeAdapter {
    fn ready() -> Self {
        Self {
            mode: LaunchMode::Ready,
            listener: None,
            listener_sequence: Vec::new(),
            chain: None,
            terminate_calls: 0,
            launches: 0,
            last_target: None,
            last_adopted_pid: None,
            last_adopted_pids: Vec::new(),
            last_adoption_was_direct: false,
            job_membership: HashSet::new(),
            alive: false,
        }
    }

    fn no_spawn() -> Self {
        Self {
            mode: LaunchMode::NoSpawn,
            ..Self::ready()
        }
    }

    fn exited() -> Self {
        Self {
            mode: LaunchMode::Exited,
            ..Self::ready()
        }
    }

    fn hanging() -> Self {
        Self {
            mode: LaunchMode::Hanging,
            ..Self::ready()
        }
    }

    fn termination_error() -> Self {
        Self {
            mode: LaunchMode::TerminateError,
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

    fn source_listener() -> Self {
        Self {
            mode: LaunchMode::SourceListener,
            listener: Some(ListenerOwner {
                pid: 4012,
                local_address: "127.0.0.1".into(),
                port: 3080,
            }),
            ..Self::ready()
        }
    }

    fn source_chain() -> Self {
        let working_directory = std::env::current_dir().expect("current directory");
        Self {
            listener: Some(ListenerOwner {
                pid: 4012,
                local_address: "127.0.0.1".into(),
                port: 3080,
            }),
            chain: Some(vec![
                ProcessIdentity {
                    pid: 4012,
                    executable: PathBuf::from("C:/Program Files/nodejs/node.exe"),
                    command_line: Some("node --import tsx/esm apps/cli/src/bin.ts \"web\"".into()),
                    parent_pid: Some(4020),
                    working_directory: Some(working_directory.clone()),
                },
                ProcessIdentity {
                    pid: 4020,
                    executable: PathBuf::from("C:/Program Files/nodejs/pnpm.cjs"),
                    command_line: Some("pnpm dsh web".into()),
                    parent_pid: None,
                    working_directory: Some(working_directory),
                },
            ]),
            ..Self::ready()
        }
    }

    fn packaged_chain() -> Self {
        Self {
            listener: Some(ListenerOwner {
                pid: 4012,
                local_address: "127.0.0.1".into(),
                port: 3080,
            }),
            chain: Some(vec![
                ProcessIdentity {
                    pid: 4012,
                    executable: PathBuf::from("C:/Program Files/nodejs/node.exe"),
                    command_line: Some("node web-server.js".into()),
                    parent_pid: Some(4020),
                    working_directory: Some(PathBuf::from("C:/fixture")),
                },
                ProcessIdentity {
                    pid: 4020,
                    executable: PathBuf::from("C:/Windows/System32/cmd.exe"),
                    command_line: Some("cmd /c pnpm run web".into()),
                    parent_pid: Some(4030),
                    working_directory: Some(PathBuf::from("C:/fixture")),
                },
                ProcessIdentity {
                    pid: 4030,
                    executable: PathBuf::from("C:/fixture/DSH.exe"),
                    command_line: Some("C:/fixture/DSH.exe".into()),
                    parent_pid: None,
                    working_directory: Some(PathBuf::from("C:/fixture")),
                },
            ]),
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
        if !self.listener_sequence.is_empty() {
            return Ok(self.listener_sequence.remove(0));
        }
        Ok(self.listener.clone())
    }

    fn inspect(&mut self, pid: u32) -> Result<ProcessIdentity, dshtray_lib::app_error::AppError> {
        if let Some(chain) = &self.chain {
            if let Some(identity) = chain.iter().find(|identity| identity.pid == pid) {
                return Ok(identity.clone());
            }
        }
        if matches!(self.mode, LaunchMode::External | LaunchMode::SourceListener) && pid == 4012 {
            let command_line = if matches!(self.mode, LaunchMode::SourceListener) {
                "node --import tsx/esm apps/cli/src/bin.ts \"web\"".into()
            } else {
                format!(
                    "{} pnpm dsh web",
                    std::env::current_dir()
                        .expect("current directory")
                        .display()
                )
            };
            Ok(ProcessIdentity {
                pid,
                executable: PathBuf::from("C:/fixture/DSH.exe"),
                command_line: Some(command_line),
                parent_pid: None,
                working_directory: Some(std::env::current_dir().expect("current directory")),
            })
        } else {
            Err(dshtray_lib::app_error::AppError::new(
                "unknown_process",
                "unknown fixture process",
            ))
        }
    }

    fn is_process_in_job(&mut self, pid: u32) -> Result<bool, dshtray_lib::app_error::AppError> {
        Ok(self.job_membership.contains(&pid))
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
        self.alive = !matches!(self.mode, LaunchMode::Exited);
        Ok(LaunchedProcess {
            pid: 9001 + self.launches as u32,
            process_group_id: 9001 + self.launches as u32,
            job: Box::new(FakeJob {
                empty: matches!(self.mode, LaunchMode::Exited),
                terminate_calls: 0,
                termination_error: matches!(self.mode, LaunchMode::TerminateError).then(|| {
                    dshtray_lib::app_error::AppError::new(
                        "fixture_termination_failed",
                        "fixture termination failed",
                    )
                }),
            }),
        })
    }

    fn adopt(&mut self, pid: u32) -> Result<LaunchedProcess, dshtray_lib::app_error::AppError> {
        self.last_adopted_pid = Some(pid);
        Ok(LaunchedProcess {
            pid,
            process_group_id: pid,
            job: Box::new(FakeJob {
                empty: false,
                terminate_calls: 0,
                termination_error: matches!(self.mode, LaunchMode::TerminateError).then(|| {
                    dshtray_lib::app_error::AppError::new(
                        "fixture_termination_failed",
                        "fixture termination failed",
                    )
                }),
            }),
        })
    }

    fn adopt_tree(
        &mut self,
        pid: u32,
        process_ids: &[u32],
    ) -> Result<LaunchedProcess, dshtray_lib::app_error::AppError> {
        self.last_adopted_pid = Some(pid);
        self.last_adopted_pids = process_ids.to_vec();
        self.last_adoption_was_direct = process_ids
            .iter()
            .any(|process_id| self.job_membership.contains(process_id));
        self.adopt(pid)
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
fn runtime_refresh_marks_managed_dsh_stopped_after_owned_tree_exits() {
    let mut controller = controller_with(FakeAdapter::exited(), FakeHealth::ready());
    controller.start().expect("start fake target");

    let snapshot = controller
        .refresh_runtime_state()
        .expect("refresh managed runtime state");

    assert_eq!(snapshot.state, LifecycleState::Stopped);
    assert_eq!(snapshot.ownership, Ownership::None);
    assert_eq!(snapshot.pid, None);
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
fn restart_on_port_conflict_does_not_try_to_stop_a_missing_owned_process() {
    let mut controller = controller_with(FakeAdapter::no_spawn(), FakeHealth::ready());
    controller.backend_mut().listener = Some(ListenerOwner {
        pid: 4012,
        local_address: "127.0.0.1".into(),
        port: 3080,
    });
    let start_error = controller
        .start()
        .expect_err("fixture listener must produce a port conflict");
    assert_eq!(start_error.code, "port_conflict");

    let restart_error = controller
        .restart()
        .expect_err("restart must preserve the port conflict");
    assert_eq!(restart_error.code, "port_conflict");
}

#[test]
fn restart_on_reclassified_external_dsh_requires_adoption() {
    let mut controller = controller_with(FakeAdapter::no_spawn(), FakeHealth::ready());
    controller.backend_mut().listener = Some(ListenerOwner {
        pid: 4012,
        local_address: "127.0.0.1".into(),
        port: 3080,
    });
    controller
        .start()
        .expect_err("unknown listener must produce a port conflict");
    controller.backend_mut().mode = LaunchMode::External;

    let restart_error = controller
        .restart()
        .expect_err("reclassified external DSH must require adoption");
    assert_eq!(restart_error.code, "external_not_adopted");
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
fn stop_termination_failure_leaves_a_failed_state_and_retains_control() {
    let mut controller = controller_with(FakeAdapter::termination_error(), FakeHealth::ready());
    controller.start().expect("start fake target");

    let error = controller
        .stop()
        .expect_err("fixture termination should fail");

    assert_eq!(error.code, "fixture_termination_failed");
    assert_eq!(controller.snapshot().state, LifecycleState::Failed);
    assert_eq!(controller.snapshot().ownership, Ownership::Managed);
}

#[test]
fn stop_listener_timeout_leaves_control_for_a_retry() {
    let mut controller = controller_with(FakeAdapter::ready(), FakeHealth::ready());
    controller.start().expect("start fake target");
    controller.backend_mut().listener = Some(ListenerOwner {
        pid: 4012,
        local_address: "127.0.0.1".into(),
        port: 3080,
    });

    let error = controller
        .stop()
        .expect_err("a stuck listener should fail the stop");
    assert_eq!(error.code, "stop_timeout");
    assert_eq!(controller.snapshot().state, LifecycleState::Failed);
    assert_eq!(controller.snapshot().ownership, Ownership::Managed);

    controller.backend_mut().listener_sequence = vec![None];
    controller
        .stop()
        .expect("retained control should allow retry");
    assert_eq!(controller.snapshot().state, LifecycleState::Stopped);
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
fn source_listener_entrypoint_is_recognized_as_external_dsh() {
    let mut controller = controller_with(FakeAdapter::source_listener(), FakeHealth::ready());
    let snapshot = controller
        .refresh_external_state()
        .expect("inspect source listener");
    assert_eq!(snapshot.state, LifecycleState::External);
    assert_eq!(snapshot.ownership, Ownership::External);
}

#[test]
fn windows_style_source_listener_entrypoint_is_recognized_as_external_dsh() {
    let mut adapter = FakeAdapter::source_chain();
    let chain = adapter.chain.as_mut().expect("source chain");
    chain[0].command_line = Some(r#"node --import tsx/esm apps\cli\src\bin.ts "web""#.into());
    chain[0].working_directory = Some(
        std::env::current_dir()
            .expect("current directory")
            .join("apps")
            .join("cli")
            .join("src"),
    );
    chain[1].command_line = Some("cmd.exe /c web-runner".into());

    let mut controller = controller_with(adapter, FakeHealth::ready());
    let snapshot = controller
        .refresh_external_state()
        .expect("inspect Windows source listener");

    assert_eq!(snapshot.state, LifecycleState::External);
    assert_eq!(snapshot.ownership, Ownership::External);
}

#[test]
fn source_listener_chain_adopts_the_pnpm_root_pid() {
    let mut controller = controller_with(FakeAdapter::source_chain(), FakeHealth::ready());
    controller
        .refresh_external_state()
        .expect("inspect source process chain");
    controller
        .adopt_external()
        .expect("adopt source process chain");
    assert_eq!(controller.backend().last_adopted_pid, Some(4020));
    assert_eq!(controller.backend().last_adopted_pids, vec![4020, 4012]);
}

#[test]
fn external_adoption_uses_direct_control_for_existing_job_membership() {
    let mut adapter = FakeAdapter::source_chain();
    adapter.job_membership.insert(4020);
    let mut controller = controller_with(adapter, FakeHealth::ready());
    controller
        .refresh_external_state()
        .expect("observe source process chain");

    let snapshot = controller
        .adopt_external()
        .expect("existing Job Object should use direct PID control");

    assert_eq!(snapshot.ownership, Ownership::Adopted);
    assert_eq!(controller.backend().last_adopted_pid, Some(4020));
    assert!(controller.backend().last_adoption_was_direct);
}

#[test]
fn external_adoption_rechecks_listener_before_control() {
    let mut controller = controller_with(FakeAdapter::source_chain(), FakeHealth::ready());
    controller
        .refresh_external_state()
        .expect("observe source process chain");
    controller.backend_mut().listener_sequence = vec![None];

    let error = controller
        .adopt_external()
        .expect_err("adoption must recheck a listener that may have exited");

    assert_eq!(error.code, "external_not_found");
    assert_eq!(controller.backend().last_adopted_pid, None);
    assert_eq!(controller.snapshot().state, LifecycleState::Stopped);
}

#[test]
fn packaged_listener_chain_adopts_the_dsh_exe_root_pid() {
    let mut controller = controller_with(FakeAdapter::packaged_chain(), FakeHealth::ready());
    controller.config_mut().active_target = TargetId::Packaged;
    controller.config_mut().targets.packaged =
        TargetConfig::packaged("fixture DSH", PathBuf::from("C:/fixture/DSH.exe"));
    controller
        .refresh_external_state()
        .expect("inspect packaged process chain");
    controller
        .adopt_external()
        .expect("adopt packaged process chain");
    assert_eq!(controller.backend().last_adopted_pid, Some(4030));
    assert_eq!(
        controller.backend().last_adopted_pids,
        vec![4030, 4020, 4012]
    );
}

#[test]
fn restart_waits_until_owned_listener_is_gone_before_starting_again() {
    let listener = ListenerOwner {
        pid: 4012,
        local_address: "127.0.0.1".into(),
        port: 3080,
    };
    let mut controller = controller_with(FakeAdapter::ready(), FakeHealth::ready());
    controller.start().expect("initial start");
    controller.backend_mut().listener_sequence = vec![Some(listener.clone()), Some(listener), None];

    let snapshot = controller
        .restart()
        .expect("restart waits for the old listener to disappear");

    assert_eq!(snapshot.state, LifecycleState::Running);
    assert_eq!(controller.backend().launches, 2);
    assert!(controller.clock().elapsed >= Duration::from_millis(200));
}

#[test]
fn source_chain_survives_an_unreadable_ancestor_after_matching_root() {
    let mut adapter = FakeAdapter::source_chain();
    adapter
        .chain
        .as_mut()
        .expect("source chain")
        .last_mut()
        .expect("source root")
        .parent_pid = Some(99999);
    let mut controller = controller_with(adapter, FakeHealth::ready());

    let snapshot = controller
        .refresh_external_state()
        .expect("refresh external state");

    assert_eq!(snapshot.state, LifecycleState::External);
    controller.adopt_external().expect("adopt external");
    assert_eq!(controller.backend().last_adopted_pid, Some(4020));
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
