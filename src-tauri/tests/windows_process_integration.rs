#![cfg(windows)]

use dshtray_lib::process::{graceful_stop::GracefulStop, inspect::ProcessInspector, job::JobOwner};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
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
    format!(
        "Local\\DeepSeekHarnessManager-test-{}-{}",
        std::process::id(),
        Instant::now().elapsed().as_nanos()
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
