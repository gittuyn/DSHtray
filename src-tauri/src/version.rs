use crate::domain::TargetConfig;
use serde::Deserialize;
use std::{fs, path::PathBuf};

#[derive(Debug, Deserialize)]
struct PackageManifest {
    version: String,
}

pub fn read_target_version(target: &TargetConfig) -> Option<String> {
    let mut candidates = Vec::<PathBuf>::with_capacity(2);
    if !target.working_directory.as_os_str().is_empty() {
        candidates.push(target.working_directory.join("package.json"));
    }
    if !target.executable.as_os_str().is_empty() {
        if let Some(parent) = target.executable.parent() {
            candidates.push(parent.join("package.json"));
        }
    }

    candidates.into_iter().find_map(|path| {
        let manifest: PackageManifest =
            serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
        let version = manifest.version.trim();
        (!version.is_empty()).then(|| version.to_owned())
    })
}
