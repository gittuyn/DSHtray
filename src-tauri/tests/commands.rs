use dshtray_lib::{
    commands::{
        guard_apply_proxy_change, guard_stop, prepare_proxy_change_for_test, sync_runtime_snapshot,
    },
    domain::{AppConfig, LifecycleState, Ownership, RuntimeSnapshot, TargetId},
};

fn snapshot(state: LifecycleState, ownership: Ownership, proxy_enabled: bool) -> RuntimeSnapshot {
    RuntimeSnapshot {
        state,
        target: TargetId::Source,
        pid: Some(4012),
        ownership,
        service_url: "http://127.0.0.1:3080".into(),
        proxy_enabled,
        last_error: None,
        started_at: None,
    }
}

#[test]
fn stop_rejects_unadopted_external_process() {
    let error = guard_stop(&snapshot(
        LifecycleState::External,
        Ownership::External,
        true,
    ))
    .expect_err("external process is not adopted");
    assert_eq!(error.code, "external_not_adopted");
}

#[test]
fn proxy_change_plan_requires_restart_only_for_managed_running_dsh() {
    let plan = prepare_proxy_change_for_test(
        &snapshot(LifecycleState::Running, Ownership::Managed, true),
        true,
        false,
    )
    .expect("prepare change");
    assert!(plan.requires_restart);

    let stopped = prepare_proxy_change_for_test(
        &snapshot(LifecycleState::Stopped, Ownership::None, true),
        true,
        false,
    )
    .expect("prepare stopped change");
    assert!(!stopped.requires_restart);
}

#[test]
fn apply_proxy_change_requires_confirmation_when_running() {
    let error = guard_apply_proxy_change(
        &snapshot(LifecycleState::Running, Ownership::Managed, true),
        false,
    )
    .expect_err("running DSH requires explicit confirmation");
    assert_eq!(error.code, "confirmation_required");
}

#[test]
fn confirmed_proxy_change_is_allowed_for_adopted_process() {
    guard_apply_proxy_change(
        &snapshot(LifecycleState::Running, Ownership::Adopted, true),
        true,
    )
    .expect("confirmed restart");
}

#[test]
fn stopped_runtime_snapshot_tracks_saved_config_changes() {
    let mut config = AppConfig::defaults();
    config.active_target = TargetId::Packaged;
    config.service.port = 3180;
    config.proxy.enabled = false;
    let mut runtime = snapshot(LifecycleState::Stopped, Ownership::None, true);

    sync_runtime_snapshot(&mut runtime, &config);

    assert_eq!(runtime.target, TargetId::Packaged);
    assert_eq!(runtime.service_url, "http://127.0.0.1:3180");
    assert!(!runtime.proxy_enabled);
}
