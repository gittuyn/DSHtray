use crate::app_error::AppError;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub executable: PathBuf,
    pub command_line: Option<String>,
    pub parent_pid: Option<u32>,
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
}

#[cfg(windows)]
fn inspect_windows(pid: u32) -> Result<ProcessIdentity, AppError> {
    use std::os::windows::ffi::OsStringExt;
    use sysinfo::{Pid, ProcessesToUpdate, System};
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
    system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]), true);
    let command_line = system.process(Pid::from_u32(pid)).map(|process| {
        process
            .cmd()
            .iter()
            .map(|part| part.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ")
    });

    Ok(ProcessIdentity {
        pid,
        executable,
        command_line,
        parent_pid,
    })
}

#[cfg(windows)]
fn find_parent_pid(pid: u32) -> Result<Option<u32>, AppError> {
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
    let mut result = None;
    let first = unsafe { Process32FirstW(snapshot, &mut entry) };
    if first.is_ok() {
        loop {
            if entry.th32ProcessID == pid {
                result = Some(entry.th32ParentProcessID);
                break;
            }
            if unsafe { Process32NextW(snapshot, &mut entry) }.is_err() {
                break;
            }
        }
    }
    unsafe {
        let _ = CloseHandle(snapshot);
    }
    Ok(result)
}
