use dshtray_lib::diagnostics::{run_self_test_with, CheckStatus, DiagnosticSnapshot, PnpmResolver};

#[test]
fn diagnostic_output_does_not_include_environment_map() {
    let snapshot = DiagnosticSnapshot::from_test_state_with_proxy("http://127.0.0.1:7897");
    let text = serde_json::to_string(&snapshot).expect("serialize diagnostics");
    assert!(!text.contains("environment"));
    assert!(!text.contains("NODE_USE_ENV_PROXY"));
}

#[test]
fn self_test_reports_missing_pnpm_as_actionable_item() {
    let report = run_self_test_with(PnpmResolver::missing());
    let item = report.item("pnpm").expect("pnpm check");
    assert_eq!(item.status, CheckStatus::Failed);
    assert!(!item.remediation.is_empty());
}
