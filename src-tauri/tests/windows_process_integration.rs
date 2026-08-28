#![cfg(windows)]

use dshtray_lib::{
    lifecycle::{ProcessAdapter, WindowsProcessAdapter},
    process::{
        graceful_stop::GracefulStop,
        inspect::ProcessInspector,
        job::{JobOwner, PidTreeOwner},
    },
};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

struct FixtureProcess {
    child: Child,
    stdin: ChildStdin,
}

impl FixtureProcess {
    fn spawn_parent_with_child() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_dsh-test-fixture"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn fixture parent");
        let stdout = child.stdout.take().expect("fixture stdout");
        let mut reader = BufReader::new(stdout);
        let mut ready = String::new();
        reader
            .read_line(&mut ready)
            .expect("read fixture readiness");
        assert_eq!(ready.trim(), "READY");
        let stdin = child.stdin.take().expect("fixture stdin");
        Self { child, stdin }
    }

    fn start_child(&mut self) {
        self.stdin
            .write_all(b"start\n")
            .expect("start fixture child");
        self.stdin.flush().expect("flush fixture start");
        thread::sleep(Duration::from_millis(100));
    }

    fn parent_pid(&self) -> u32 {
        self.child.id()
    }

    fn is_alive(&mut self) -> bool {
        self.child
            .try_wait()
            .expect("query fixture state")
            .is_none()
    }

    fn terminate_tree_for_test(&mut self) {
        let _ = self.stdin.write_all(b"stop\n");
        let _ = self.stdin.flush();
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if !self.is_alive() {
                return;
            }
            thread::sleep(Duration::from_millis(25));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for FixtureProcess {
    fn drop(&mut self) {
        self.terminate_tree_for_test();
    }
}

fn unique_job_name() -> String {
    static NEXT_JOB_ID: AtomicU64 = AtomicU64::new(0);
    format!(
        "Local\\DeepSeekHarnessManager-test-{}-{}",
        std::process::id(),
        NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed),
    )
}

#[test]
fn closing_manager_handle_does_not_kill_owned_fixture() {
    let mut fixture = FixtureProcess::spawn_parent_with_child();
    let job = JobOwner::create_or_open(unique_job_name()).expect("create job");
    job.assign(fixture.parent_pid()).expect("assign fixture");
    fixture.start_child();
    job.close_without_termination();
    assert!(fixture.is_alive());
    fixture.terminate_tree_for_test();
}

#[test]
fn terminate_only_kills_processes_assigned_to_the_job() {
    let mut owned = FixtureProcess::spawn_parent_with_child();
    let mut unrelated = FixtureProcess::spawn_parent_with_child();
    let job = JobOwner::create_or_open(unique_job_name()).expect("create job");
    job.assign(owned.parent_pid()).expect("assign owned parent");
    owned.start_child();
    unrelated.start_child();
    job.terminate().expect("terminate owned job");
    let deadline = Instant::now() + Duration::from_secs(2);
    while owned.is_alive() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(25));
    }
    assert!(!owned.is_alive());
    assert!(unrelated.is_alive());
    unrelated.terminate_tree_for_test();
}

#[test]
fn process_inspector_reads_the_fixture_identity() {
    let mut fixture = FixtureProcess::spawn_parent_with_child();
    let identity = ProcessInspector::inspect(fixture.parent_pid()).expect("inspect fixture");
    assert_eq!(identity.pid, fixture.parent_pid());
    assert!(identity.executable.is_file());
    assert_eq!(identity.working_directory, std::env::current_dir().ok());
    fixture.terminate_tree_for_test();
}

#[test]
fn process_tree_probe_includes_fixture_child() {
    let mut fixture = FixtureProcess::spawn_parent_with_child();
    fixture.start_child();
    let root = fixture.parent_pid();
    let tree = ProcessInspector::process_tree(root).expect("inspect fixture process tree");
    assert_eq!(tree.first().copied(), Some(root));
    assert!(tree.iter().skip(1).any(|pid| {
        ProcessInspector::inspect(*pid)
            .map(|identity| identity.parent_pid == Some(root))
            .unwrap_or(false)
    }));
    fixture.terminate_tree_for_test();
}

#[test]
fn process_presence_probe_distinguishes_exited_fixture() {
    let mut fixture = FixtureProcess::spawn_parent_with_child();
    let pid = fixture.parent_pid();
    assert!(ProcessInspector::is_present(pid).expect("query fixture presence"));
    fixture.terminate_tree_for_test();
    assert!(!ProcessInspector::is_present(pid).expect("query exited fixture presence"));
}

#[test]
fn job_membership_probe_reports_assigned_fixture_without_changing_it() {
    let mut fixture = FixtureProcess::spawn_parent_with_child();
    let job = JobOwner::create_or_open(unique_job_name()).expect("create job");
    job.assign(fixture.parent_pid()).expect("assign fixture");

    assert!(JobOwner::is_process_in_job(fixture.parent_pid()).expect("query job membership"));
    assert!(fixture.is_alive());

    job.close_without_termination();
    fixture.terminate_tree_for_test();
}

#[test]
fn direct_tree_control_does_not_terminate_a_process_outside_the_current_tree() {
    let mut fixture = FixtureProcess::spawn_parent_with_child();
    let mut unrelated = Command::new("ping.exe")
        .args(["127.0.0.1", "-t"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn unrelated process");
    let root = fixture.parent_pid();
    let processes = vec![
        ProcessInspector::inspect(root).expect("inspect fixture root"),
        ProcessInspector::inspect(unrelated.id()).expect("inspect unrelated process"),
    ];
    let mut owner = PidTreeOwner::new(root, processes);

    owner
        .terminate()
        .expect("terminate only the current root tree");

    let deadline = Instant::now() + Duration::from_secs(2);
    while fixture.is_alive() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(25));
    }
    assert!(!fixture.is_alive());
    assert!(ProcessInspector::is_present(unrelated.id()).expect("query unrelated process"));

    let _ = unrelated.kill();
    let _ = unrelated.wait();
}

#[test]
fn direct_tree_adoption_controls_fixture_already_assigned_to_a_job() {
    let mut fixture = FixtureProcess::spawn_parent_with_child();
    fixture.start_child();
    let root = fixture.parent_pid();
    let child_pid = ProcessInspector::process_tree(root)
        .expect("inspect direct-control tree")
        .into_iter()
        .find(|pid| *pid != root)
        .expect("fixture child pid");
    let existing_job = JobOwner::create_or_open(unique_job_name()).expect("create existing job");
    existing_job
        .assign(root)
        .expect("assign fixture to existing job");
    let mut adapter = WindowsProcessAdapter::default();

    let mut adopted = adapter
        .adopt_tree(root, &[root])
        .expect("existing Job Object should use direct PID control");

    assert!(!adopted.job.is_empty());
    adopted
        .job
        .terminate()
        .expect("terminate exact fixture PID tree");
    let deadline = Instant::now() + Duration::from_secs(2);
    while (fixture.is_alive() || ProcessInspector::is_present(child_pid).unwrap_or(true))
        && Instant::now() < deadline
    {
        thread::sleep(Duration::from_millis(25));
    }
    assert!(!fixture.is_alive());
    assert!(!ProcessInspector::is_present(child_pid).expect("query terminated child"));

    existing_job.close_without_termination();
}

#[test]
fn direct_tree_adoption_reuses_a_configured_manager_job() {
    let mut fixture = FixtureProcess::spawn_parent_with_child();
    let job_name = unique_job_name();
    let existing_job = JobOwner::create_or_open(&job_name).expect("create manager job");
    existing_job
        .assign(fixture.parent_pid())
        .expect("assign fixture to manager job");
    let mut adapter = WindowsProcessAdapter::with_job_name(job_name);

    let adopted = adapter
        .adopt_tree(fixture.parent_pid(), &[fixture.parent_pid()])
        .expect("reopen the manager-owned job");

    assert!(!adopted.job.is_empty());
    drop(adopted);
    existing_job.close_without_termination();
    fixture.terminate_tree_for_test();
}

#[test]
fn graceful_stop_reports_a_real_request_or_unavailable_status() {
    let result = GracefulStop::request(0);
    assert!(matches!(
        result,
        dshtray_lib::process::graceful_stop::GracefulStopResult::Unavailable { .. }
    ));
}
