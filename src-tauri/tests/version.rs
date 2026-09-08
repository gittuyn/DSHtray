use dshtray_lib::{domain::TargetConfig, version::read_target_version};
use std::fs;
use tempfile::tempdir;

#[test]
fn reads_version_for_source_target() {
    let directory = tempdir().expect("temporary directory");
    fs::write(
        directory.path().join("package.json"),
        r#"{"name":"dsh-root","version":"0.1.2-rc.1"}"#,
    )
    .expect("write package manifest");

    let target = TargetConfig::source("source", directory.path().to_path_buf());

    assert_eq!(read_target_version(&target).as_deref(), Some("0.1.2-rc.1"));
}

#[test]
fn reads_version_for_packaged_target_from_its_directory() {
    let directory = tempdir().expect("temporary directory");
    fs::write(
        directory.path().join("package.json"),
        r#"{"name":"dsh-root","version":"0.1.2-rc.1"}"#,
    )
    .expect("write package manifest");
    let executable = directory.path().join("DSH.exe");
    fs::write(&executable, b"fixture").expect("write executable fixture");

    let target = TargetConfig::packaged("packaged", executable);

    assert_eq!(read_target_version(&target).as_deref(), Some("0.1.2-rc.1"));
}

#[test]
fn missing_or_invalid_manifest_returns_unknown_without_an_error() {
    let directory = tempdir().expect("temporary directory");
    fs::write(directory.path().join("package.json"), "{invalid").expect("write manifest");

    let target = TargetConfig::source("source", directory.path().to_path_buf());

    assert_eq!(read_target_version(&target), None);
}
