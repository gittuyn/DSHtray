use dshtray_lib::{
    discovery::{discover_targets_from, validate_packaged_executable},
    domain::TargetKind,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn discovery_returns_candidates_without_starting_them() {
    let root = tempdir().expect("temp root");
    fs::write(root.path().join("package.json"), "{}").expect("package json");
    let result = discover_targets_from(vec![root.path().to_path_buf()]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].kind, TargetKind::Source);
    assert!(result[0].needs_user_confirmation);
    assert!(result[0].valid);
}

#[test]
fn packaged_candidate_requires_dsh_exe_file() {
    let root = tempdir().expect("temp root");
    let executable = root.path().join("DSH.exe");
    assert!(!validate_packaged_executable(&executable).is_valid);
    fs::write(&executable, b"fixture").expect("exe fixture");
    assert!(validate_packaged_executable(&executable).is_valid);
}
