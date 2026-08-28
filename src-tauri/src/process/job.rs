use crate::app_error::AppError;

#[cfg(windows)]
mod platform {
    use super::AppError;
    use crate::process::inspect::{ProcessIdentity, ProcessInspector};
    use std::{ffi::OsStr, mem::size_of, os::windows::ffi::OsStrExt, slice};
    use windows::{
        core::{BOOL, PCWSTR},
        Win32::{
            Foundation::CloseHandle,
            System::{
                JobObjects::{
                    AssignProcessToJobObject, CreateJobObjectW, IsProcessInJob,
                    JobObjectBasicProcessIdList, QueryInformationJobObject, TerminateJobObject,
                    JOBOBJECT_BASIC_PROCESS_ID_LIST,
                },
                Threading::{
                    OpenProcess, TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION,
                    PROCESS_SET_QUOTA, PROCESS_TERMINATE,
                },
            },
        },
    };

    pub struct JobOwner {
        handle: windows::Win32::Foundation::HANDLE,
        name: String,
    }

    unsafe impl Send for JobOwner {}
    unsafe impl Sync for JobOwner {}

    impl std::fmt::Debug for JobOwner {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter
                .debug_struct("JobOwner")
                .field("name", &self.name)
                .finish_non_exhaustive()
        }
    }

    impl JobOwner {
        pub fn create_or_open(name: impl AsRef<str>) -> Result<Self, AppError> {
            let name = name.as_ref().to_owned();
            let wide: Vec<u16> = OsStr::new(&name).encode_wide().chain(Some(0)).collect();
            let handle =
                unsafe { CreateJobObjectW(None, PCWSTR(wide.as_ptr())) }.map_err(|error| {
                    win_error("job_create_failed", "无法创建或打开 Job Object", error)
                })?;
            Ok(Self { handle, name })
        }

        pub fn assign(&self, pid: u32) -> Result<(), AppError> {
            let process = unsafe {
                OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA | PROCESS_TERMINATE,
                    false,
                    pid,
                )
            }
            .map_err(|error| win_error("process_open_failed", "无法打开待归属进程", error))?;
            let result =
                unsafe { AssignProcessToJobObject(self.handle, process) }.map_err(|error| {
                    win_error("job_assign_failed", "无法把进程加入 Job Object", error)
                });
            unsafe {
                let _ = CloseHandle(process);
            }
            result
        }

        pub fn is_process_in_job(pid: u32) -> Result<bool, AppError> {
            let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
                .map_err(|error| {
                    win_error(
                        "job_query_process_failed",
                        "无法检查进程的 Job Object 归属",
                        error,
                    )
                })?;
            let mut in_job = BOOL(0);
            let result = unsafe { IsProcessInJob(process, None, &mut in_job) }.map_err(|error| {
                win_error(
                    "job_membership_query_failed",
                    "无法检查进程的 Job Object 归属",
                    error,
                )
            });
            unsafe {
                let _ = CloseHandle(process);
            }
            result.map(|_| in_job.as_bool())
        }

        pub fn process_ids(&self) -> Result<Vec<u32>, AppError> {
            let mut capacity = 16_usize;
            loop {
                let bytes = size_of::<JOBOBJECT_BASIC_PROCESS_ID_LIST>()
                    + (capacity.saturating_sub(1) * size_of::<usize>());
                let mut buffer = vec![0_u8; bytes];
                let mut returned = 0_u32;
                let result = unsafe {
                    QueryInformationJobObject(
                        Some(self.handle),
                        JobObjectBasicProcessIdList,
                        buffer.as_mut_ptr().cast(),
                        buffer.len() as u32,
                        Some(&mut returned),
                    )
                };
                if let Err(error) = result {
                    if returned as usize > buffer.len() {
                        capacity = (returned as usize / size_of::<usize>()).saturating_add(1);
                        continue;
                    }
                    return Err(win_error(
                        "job_query_failed",
                        "无法读取 Job Object 进程列表",
                        error,
                    ));
                }
                let list = unsafe { &*(buffer.as_ptr() as *const JOBOBJECT_BASIC_PROCESS_ID_LIST) };
                let count = (list.NumberOfProcessIdsInList as usize).min(capacity);
                let ids = unsafe { slice::from_raw_parts(list.ProcessIdList.as_ptr(), count) };
                return Ok(ids.iter().map(|pid| *pid as u32).collect());
            }
        }

        pub fn is_empty(&self) -> Result<bool, AppError> {
            Ok(self.process_ids()?.is_empty())
        }

        pub fn terminate(&self) -> Result<(), AppError> {
            unsafe { TerminateJobObject(self.handle, 1) }
                .map_err(|error| win_error("job_terminate_failed", "无法终止已归属进程树", error))
        }

        pub fn close_without_termination(self) {
            drop(self);
        }
    }

    impl Drop for JobOwner {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }

    #[derive(Debug)]
    pub(crate) struct ProcessTerminationHandle {
        pid: u32,
        handle: windows::Win32::Foundation::HANDLE,
    }

    unsafe impl Send for ProcessTerminationHandle {}

    impl ProcessTerminationHandle {
        fn open(pid: u32) -> Result<Self, AppError> {
            let handle = unsafe {
                OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE,
                    false,
                    pid,
                )
            }
            .map_err(|error| {
                win_error(
                    "external_process_open_failed",
                    "无法打开已确认的外部 DSH 进程",
                    error,
                )
            })?;
            Ok(Self { pid, handle })
        }

        fn terminate(&self) -> Result<(), AppError> {
            unsafe { TerminateProcess(self.handle, 1) }.map_err(|error| {
                win_error(
                    "external_process_terminate_failed",
                    "无法强制终止已确认的外部 DSH 进程",
                    error,
                )
            })
        }
    }

    impl Drop for ProcessTerminationHandle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }

    pub struct PidTreeOwner {
        root_pid: u32,
        processes: Vec<ProcessIdentity>,
    }

    impl std::fmt::Debug for PidTreeOwner {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter
                .debug_struct("PidTreeOwner")
                .field(
                    "pids",
                    &self
                        .processes
                        .iter()
                        .map(|process| process.pid)
                        .collect::<Vec<_>>(),
                )
                .finish()
        }
    }

    impl PidTreeOwner {
        pub fn new(root_pid: u32, processes: Vec<ProcessIdentity>) -> Self {
            Self {
                root_pid,
                processes,
            }
        }

        pub fn is_empty(&self) -> bool {
            self.processes
                .iter()
                .all(|expected| match ProcessInspector::inspect(expected.pid) {
                    Ok(current) => !same_process(expected, &current),
                    Err(_) => matches!(ProcessInspector::is_present(expected.pid), Ok(false)),
                })
        }

        pub(crate) fn preflight_termination(
            &self,
        ) -> Result<Vec<ProcessTerminationHandle>, AppError> {
            let mut plan = Vec::with_capacity(self.processes.len());
            for expected in self.processes.iter().rev() {
                let handle = match ProcessTerminationHandle::open(expected.pid) {
                    Ok(handle) => handle,
                    Err(open_error) => match ProcessInspector::is_present(expected.pid) {
                        Ok(false) => continue,
                        Ok(true) => {
                            return Err(AppError::with_details(
                                "external_process_recheck_failed",
                                "无法重新确认外部 DSH 进程，未执行强制终止",
                                format!("pid={}; open={open_error}", expected.pid),
                            ));
                        }
                        Err(presence_error) => {
                            return Err(AppError::with_details(
                                "external_process_recheck_failed",
                                "无法确认外部 DSH 进程是否仍存在，未执行强制终止",
                                format!(
                                    "pid={}; open={open_error}; presence={presence_error}",
                                    expected.pid
                                ),
                            ));
                        }
                    },
                };
                let current = match ProcessInspector::inspect(expected.pid) {
                    Ok(current) => current,
                    Err(error) => match ProcessInspector::is_present(expected.pid) {
                        Ok(false) => continue,
                        Ok(true) => {
                            return Err(AppError::with_details(
                                "external_process_recheck_failed",
                                "无法重新确认外部 DSH 进程，未执行强制终止",
                                format!("pid={}; {error}", expected.pid),
                            ));
                        }
                        Err(presence_error) => {
                            return Err(AppError::with_details(
                                "external_process_recheck_failed",
                                "无法确认外部 DSH 进程是否仍存在，未执行强制终止",
                                format!(
                                    "pid={}; inspect={error}; presence={presence_error}",
                                    expected.pid
                                ),
                            ));
                        }
                    },
                };
                if !same_process(expected, &current) {
                    return Err(AppError::with_details(
                        "pid_reused",
                        "接管期间进程 PID 已变化，未执行强制终止",
                        format!("pid={}", expected.pid),
                    ));
                }
                plan.push(handle);
            }
            Ok(plan)
        }

        pub fn terminate(&mut self) -> Result<(), AppError> {
            for pass in 0..3 {
                self.refresh_tree()?;
                let handles = self.preflight_termination()?;
                for handle in handles {
                    let pid = handle.pid;
                    handle.terminate().map_err(|error| {
                        AppError::with_details(
                            error.code,
                            error.message,
                            format!("pid={pid}; {}", error.details.unwrap_or_default()),
                        )
                    })?;
                }
                if self.is_empty() || pass == 2 {
                    return Ok(());
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            Ok(())
        }

        fn refresh_tree(&mut self) -> Result<(), AppError> {
            let tree = ProcessInspector::process_tree(self.root_pid)?;
            if tree.is_empty() {
                return Ok(());
            }
            let mut refreshed = Vec::with_capacity(tree.len() + self.processes.len());
            for pid in tree {
                let current = ProcessInspector::inspect(pid).map_err(|error| {
                    AppError::with_details(
                        "external_process_recheck_failed",
                        "无法重新确认外部 DSH 进程，未执行强制终止",
                        format!("pid={pid}; {error}"),
                    )
                })?;
                if let Some(expected) = self.processes.iter().find(|process| process.pid == pid) {
                    if !same_process(expected, &current) {
                        return Err(AppError::with_details(
                            "pid_reused",
                            "接管期间进程 PID 已变化，未执行强制终止",
                            format!("pid={pid}"),
                        ));
                    }
                }
                refreshed.push(current);
            }
            self.processes = refreshed;
            Ok(())
        }
    }

    fn same_process(expected: &ProcessIdentity, current: &ProcessIdentity) -> bool {
        expected == current
    }

    fn win_error(code: &str, message: &str, error: windows::core::Error) -> AppError {
        AppError::with_details(
            code,
            message,
            format!("{}: {}", error.code().0, error.message()),
        )
    }
}

#[cfg(not(windows))]
mod platform {
    use super::AppError;
    use crate::process::inspect::ProcessIdentity;

    #[derive(Debug)]
    pub struct JobOwner;

    #[derive(Debug)]
    pub struct PidTreeOwner;

    impl PidTreeOwner {
        pub fn new(_: u32, _: Vec<ProcessIdentity>) -> Self {
            Self
        }

        pub fn is_empty(&self) -> bool {
            true
        }

        pub fn terminate(&mut self) -> Result<(), AppError> {
            Err(AppError::new(
                "unsupported_platform",
                "精确 PID 树控制仅支持 Windows",
            ))
        }
    }

    impl JobOwner {
        pub fn create_or_open(_: impl AsRef<str>) -> Result<Self, AppError> {
            Err(AppError::new(
                "unsupported_platform",
                "Job Object 仅支持 Windows",
            ))
        }

        pub fn assign(&self, _: u32) -> Result<(), AppError> {
            Err(AppError::new(
                "unsupported_platform",
                "Job Object 仅支持 Windows",
            ))
        }

        pub fn is_process_in_job(_: u32) -> Result<bool, AppError> {
            Err(AppError::new(
                "unsupported_platform",
                "Job Object 仅支持 Windows",
            ))
        }

        pub fn process_ids(&self) -> Result<Vec<u32>, AppError> {
            Err(AppError::new(
                "unsupported_platform",
                "Job Object 仅支持 Windows",
            ))
        }

        pub fn is_empty(&self) -> Result<bool, AppError> {
            Err(AppError::new(
                "unsupported_platform",
                "Job Object 仅支持 Windows",
            ))
        }

        pub fn terminate(&self) -> Result<(), AppError> {
            Err(AppError::new(
                "unsupported_platform",
                "Job Object 仅支持 Windows",
            ))
        }

        pub fn close_without_termination(self) {}
    }
}

pub use platform::{JobOwner, PidTreeOwner};

#[cfg(all(test, windows))]
mod tests {
    use super::PidTreeOwner;
    use crate::process::inspect::ProcessInspector;

    #[test]
    fn preflight_rejects_later_identity_drift_before_building_a_termination_plan() {
        let current = ProcessInspector::inspect(std::process::id()).expect("inspect test process");
        let mut drifted = current.clone();
        drifted.command_line = Some("different-process".into());
        let owner = PidTreeOwner::new(std::process::id(), vec![current, drifted]);

        let error = owner
            .preflight_termination()
            .expect_err("a later identity mismatch must abort the whole preflight");

        assert_eq!(error.code, "pid_reused");
    }
}
