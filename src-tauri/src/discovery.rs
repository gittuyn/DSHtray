use crate::domain::{TargetConfig, TargetId, TargetKind};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredTarget {
    pub id: TargetId,
    pub kind: TargetKind,
    pub label: String,
    pub working_directory: String,
    pub executable: Option<String>,
    pub valid: bool,
    pub needs_user_confirmation: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathValidation {
    pub is_valid: bool,
}

pub fn validate_packaged_executable(path: &Path) -> PathValidation {
    PathValidation {
        is_valid: path.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe")),
    }
}

pub fn discover_targets_from(candidates: Vec<PathBuf>) -> Vec<DiscoveredTarget> {
    let mut results = Vec::new();
    let mut seen = Vec::<PathBuf>::new();
    for root in candidates {
        let normalized = std::fs::canonicalize(&root).unwrap_or(root.clone());
        if seen.iter().any(|item| item == &normalized) {
            continue;
        }
        seen.push(normalized);
        let label = root
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("DSH")
            .to_string();
        let root_string = root.to_string_lossy().to_string();
        if root.join("package.json").is_file() {
            results.push(DiscoveredTarget {
                id: TargetId::Source,
                kind: TargetKind::Source,
                label: format!("{label} 源码"),
                working_directory: root_string.clone(),
                executable: None,
                valid: true,
                needs_user_confirmation: true,
                reason: "发现 package.json；启动命令需要用户确认".into(),
            });
        }
        let executable = root.join("DSH.exe");
        if validate_packaged_executable(&executable).is_valid {
            results.push(DiscoveredTarget {
                id: TargetId::Packaged,
                kind: TargetKind::Packaged,
                label: format!("{label} 打包版"),
                working_directory: root_string,
                executable: Some(executable.to_string_lossy().to_string()),
                valid: true,
                needs_user_confirmation: true,
                reason: "发现 DSH.exe；可执行目标需要用户确认".into(),
            });
        }
    }
    results
}

pub fn default_candidates() -> Vec<PathBuf> {
    let mut candidates = vec![PathBuf::from(
        r"C:\Users\Tony\Documents\Default Project\deepseek-harness",
    )];
    if let Some(user_profile) = std::env::var_os("USERPROFILE") {
        let profile = PathBuf::from(user_profile);
        candidates.push(
            profile
                .join("Documents")
                .join("Default Project")
                .join("deepseek-harness"),
        );
        candidates.push(
            profile
                .join("Documents")
                .join("BaiduSyncdisk")
                .join("DSH")
                .join("deepseek-harness"),
        );
    }
    candidates
}

pub fn discover_targets() -> Vec<DiscoveredTarget> {
    discover_targets_from(default_candidates())
}

pub fn to_target_config(candidate: &DiscoveredTarget) -> TargetConfig {
    match candidate.id {
        TargetId::Source => TargetConfig::source(
            candidate.label.clone(),
            PathBuf::from(&candidate.working_directory),
        ),
        TargetId::Packaged => TargetConfig::packaged(
            candidate.label.clone(),
            candidate
                .executable
                .as_deref()
                .map(PathBuf::from)
                .unwrap_or_default(),
        ),
    }
}
