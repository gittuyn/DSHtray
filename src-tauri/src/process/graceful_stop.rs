#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GracefulStopResult {
    Requested,
    Unavailable { code: String, message: String },
}

pub struct GracefulStop;

impl GracefulStop {
    pub fn request(process_group_id: u32) -> GracefulStopResult {
        if process_group_id == 0 {
            return GracefulStopResult::Unavailable {
                code: "graceful-stop-unavailable".into(),
                message: "进程组 ID 为空，不能发送 CTRL_BREAK_EVENT".into(),
            };
        }

        #[cfg(windows)]
        {
            use windows::Win32::System::Console::{GenerateConsoleCtrlEvent, CTRL_BREAK_EVENT};
            match unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, process_group_id) } {
                Ok(()) => GracefulStopResult::Requested,
                Err(error) => GracefulStopResult::Unavailable {
                    code: "graceful-stop-unavailable".into(),
                    message: format!("{}: {}", error.code().0, error.message()),
                },
            }
        }

        #[cfg(not(windows))]
        {
            let _ = process_group_id;
            GracefulStopResult::Unavailable {
                code: "unsupported_platform".into(),
                message: "正常退出请求仅支持 Windows".into(),
            }
        }
    }
}
