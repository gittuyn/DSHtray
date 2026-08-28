use crate::app_error::AppError;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub executable: PathBuf,
    pub command_line: Option<String>,
    pub parent_pid: Option<u32>,
    pub working_directory: Option<PathBuf>,
}

pub struct ProcessInspector;

impl ProcessInspector {
    pub fn inspect(pid: u32) -> Result<ProcessIdentity, AppError> {
        #[cfg(windows)]
        {
            inspect_windows(pid)
        }
        #[cfg(not(windows))]
        {
            let _ = pid;
            Err(AppError::new(
                "unsupported_platform",
                "进程检查仅支持 Windows",
            ))
        }
    }

    pub fn process_tree(root_pid: u32) -> Result<Vec<u32>, AppError> {
        #[cfg(windows)]
        {
            let processes = snapshot_processes()?;
            if !processes.iter().any(|(pid, _)| *pid == root_pid) {
                return Ok(Vec::new());
            }
            let mut tree = vec![root_pid];
            let mut cursor = 0;
            while cursor < tree.len() {
                let parent_pid = tree[cursor];
                for (pid, candidate_parent) in processes.iter().copied() {
                    if candidate_parent == parent_pid && !tree.contains(&pid) {
                        tree.push(pid);
                    }
                }
                cursor += 1;
            }
            Ok(tree)
        }
        #[cfg(not(windows))]
        {
            let _ = root_pid;
            Err(AppError::new(
                "unsupported_platform",
                "进程树检查仅支持 Windows",
            ))
        }
    }

    pub fn is_present(pid: u32) -> Result<bool, AppError> {
        #[cfg(windows)]
        {
            find_parent_pid(pid).map(|parent| parent.is_some())
        }
        #[cfg(not(windows))]
        {
            let _ = pid;
            Err(AppError::new(
                "unsupported_platform",
                "进程检查仅支持 Windows",
            ))
        }
    }
}

#[cfg(windows)]
fn inspect_windows(pid: u32) -> Result<ProcessIdentity, AppError> {
    use std::os::windows::ffi::OsStringExt;
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    use windows::{
        core::PWSTR,
        Win32::{
            Foundation::CloseHandle,
            System::Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
    };

    let process =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.map_err(|error| {
            AppError::with_details(
                "process_open_failed",
                "无法打开待检查进程",
                format!("{}: {}", error.code().0, error.message()),
            )
        })?;
    let mut path_buffer = vec![0_u16; 32_768];
    let mut path_len = path_buffer.len() as u32;
    let path_result = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(path_buffer.as_mut_ptr()),
            &mut path_len,
        )
    };
    unsafe {
        let _ = CloseHandle(process);
    }
    path_result.map_err(|error| {
        AppError::with_details(
            "process_path_failed",
            "无法读取进程可执行路径",
            format!("{}: {}", error.code().0, error.message()),
        )
    })?;
    let executable = PathBuf::from(std::ffi::OsString::from_wide(
        &path_buffer[..path_len as usize],
    ));

    let parent_pid = find_parent_pid(pid)?;
    let mut system = System::new();
    let process_ids = [Pid::from_u32(pid)];
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&process_ids),
        true,
        ProcessRefreshKind::nothing()
            .with_cmd(UpdateKind::Always)
            .with_cwd(UpdateKind::Always),
    );
    let command_line = system.process(Pid::from_u32(pid)).map(|process| {
        process
            .cmd()
            .iter()
            .map(|part| part.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ")
    });
    let working_directory = system
        .process(Pid::from_u32(pid))
        .and_then(|process| process.cwd().map(PathBuf::from));

    Ok(ProcessIdentity {
        pid,
        executable,
        command_line,
        parent_pid,
        working_directory,
    })
}

#[cfg(windows)]
fn find_parent_pid(pid: u32) -> Result<Option<u32>, AppError> {
    Ok(snapshot_processes()?
        .into_iter()
        .find_map(|(process_pid, parent_pid)| (process_pid == pid).then_some(parent_pid)))
}

#[cfg(windows)]
fn snapshot_processes() -> Result<Vec<(u32, u32)>, AppError> {
    use std::mem::size_of;
    use windows::Win32::{
        Foundation::CloseHandle,
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
    };

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }.map_err(|error| {
        AppError::with_details(
            "process_snapshot_failed",
            "无法读取进程快照",
            format!("{}: {}", error.code().0, error.message()),
        )
    })?;
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let first = unsafe { Process32FirstW(snapshot, &mut entry) };
    if let Err(error) = first {
        unsafe {
            let _ = CloseHandle(snapshot);
        }
        return Err(AppError::with_details(
            "process_snapshot_failed",
            "无法读取进程快照",
            format!("{}: {}", error.code().0, error.message()),
        ));
    }
    let mut result = Vec::new();
    loop {
        result.push((entry.th32ProcessID, entry.th32ParentProcessID));
        match unsafe { Process32NextW(snapshot, &mut entry) } {
            Ok(()) => {}
            Err(error) if is_process_snapshot_end(error.code()) => break,
            Err(error) => {
                unsafe {
                    let _ = CloseHandle(snapshot);
                }
                return Err(AppError::with_details(
                    "process_snapshot_failed",
                    "无法读取进程快照",
                    format!("{}: {}", error.code().0, error.message()),
                ));
            }
        }
    }
    unsafe {
        let _ = CloseHandle(snapshot);
    }
    Ok(result)
}

#[cfg(windows)]
fn is_process_snapshot_end(code: windows::core::HRESULT) -> bool {
    code == windows::Win32::Foundation::ERROR_NO_MORE_FILES.to_hresult()
}

#[cfg(all(test, windows))]
mod tests {
    use super::is_process_snapshot_end;
    use windows::Win32::Foundation::ERROR_NO_MORE_FILES;

    #[test]
    fn only_no_more_files_is_a_normal_snapshot_end() {
        assert!(is_process_snapshot_end(ERROR_NO_MORE_FILES.to_hresult()));
        assert!(!is_process_snapshot_end(windows::core::HRESULT(0)));
    }
}
