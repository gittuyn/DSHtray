use crate::app_error::AppError;

#[cfg(windows)]
mod platform {
    use super::AppError;
    use std::{ffi::OsStr, mem::size_of, os::windows::ffi::OsStrExt, slice};
    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::CloseHandle,
            System::{
                JobObjects::{
                    AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicProcessIdList,
                    QueryInformationJobObject, TerminateJobObject, JOBOBJECT_BASIC_PROCESS_ID_LIST,
                },
                Threading::{
                    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA,
                    PROCESS_TERMINATE,
                },
            },
        },
    };

    pub struct JobOwner {
        handle: windows::Win32::Foundation::HANDLE,
        name: String,
    }

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

    #[derive(Debug)]
    pub struct JobOwner;

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

pub use platform::JobOwner;
