use crate::{
    app_error::AppError,
    domain::{TargetConfig, TargetKind},
};
use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetCommand {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub working_directory: PathBuf,
}

pub fn build_source_command(target: &TargetConfig) -> Result<TargetCommand, AppError> {
    if target.kind != TargetKind::Source {
        return Err(AppError::new("invalid_target_kind", "当前目标不是源码模式"));
    }
    if target.working_directory.as_os_str().is_empty() {
        return Err(AppError::new(
            "target_not_configured",
            "源码目标目录尚未配置",
        ));
    }

    let program = if target.command.trim().is_empty()
        || target.command.eq_ignore_ascii_case("pnpm")
        || target.command.eq_ignore_ascii_case("pnpm.cmd")
    {
        resolve_pnpm_command()?
    } else {
        PathBuf::from(&target.command)
    };
    let args = if target.arguments.is_empty() {
        vec![OsString::from("dsh"), OsString::from("web")]
    } else {
        target.arguments.iter().map(OsString::from).collect()
    };
    Ok(TargetCommand {
        program,
        args,
        working_directory: target.working_directory.clone(),
    })
}

pub fn build_packaged_command(target: &TargetConfig) -> Result<TargetCommand, AppError> {
    if target.kind != TargetKind::Packaged {
        return Err(AppError::new("invalid_target_kind", "当前目标不是打包模式"));
    }
    if target.executable.as_os_str().is_empty() {
        return Err(AppError::new(
            "target_not_configured",
            "DSH.exe 路径尚未配置",
        ));
    }
    let working_directory = target
        .executable
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| AppError::new("invalid_packaged_executable", "DSH.exe 缺少所在目录"))?;
    Ok(TargetCommand {
        program: target.executable.clone(),
        args: target.arguments.iter().map(OsString::from).collect(),
        working_directory,
    })
}

pub fn resolve_pnpm_command() -> Result<PathBuf, AppError> {
    let path = env::var_os("PATH")
        .ok_or_else(|| AppError::new("pnpm_not_found", "环境变量 PATH 不存在"))?;
    for directory in env::split_paths(&path) {
        for name in pnpm_names() {
            let candidate = directory.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(AppError::new(
        "pnpm_not_found",
        "PATH 中找不到 pnpm.cmd 或 pnpm.exe",
    ))
}

fn pnpm_names() -> [&'static str; 3] {
    ["pnpm.cmd", "pnpm.exe", "pnpm"]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_target(path: PathBuf) -> TargetConfig {
        TargetConfig::source("DSH 源码", path)
    }

    fn packaged_target(path: PathBuf) -> TargetConfig {
        TargetConfig::packaged("DSH.exe", path)
    }

    #[test]
    fn source_command_preserves_pnpm_dsh_web_argv() {
        let target = source_target(PathBuf::from(r"C:\deepseek-harness"));
        let command = build_source_command(&target).expect("valid source target");
        assert_eq!(
            command.args,
            vec![OsString::from("dsh"), OsString::from("web")]
        );
        assert_eq!(
            command.working_directory,
            PathBuf::from(r"C:\deepseek-harness")
        );
    }

    #[test]
    fn packaged_command_uses_executable_directory_as_cwd() {
        let target = packaged_target(PathBuf::from(r"C:\DSH\DSH.exe"));
        let command = build_packaged_command(&target).expect("valid packaged target");
        assert_eq!(command.program, PathBuf::from(r"C:\DSH\DSH.exe"));
        assert_eq!(command.working_directory, PathBuf::from(r"C:\DSH"));
        assert!(command.args.is_empty());
    }

    #[test]
    fn source_command_rejects_wrong_target_kind() {
        let target = packaged_target(PathBuf::from(r"C:\DSH\DSH.exe"));
        let error = build_source_command(&target).expect_err("packaged target cannot be source");
        assert_eq!(error.code, "invalid_target_kind");
    }
}
