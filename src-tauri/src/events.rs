use crate::{app_error::AppError, commands::AppStateDto};
use tauri::{AppHandle, Emitter, Runtime};

pub const STATE_CHANGED: &str = "state_changed";
pub const LOG_APPENDED: &str = "log_appended";
pub const STARTUP_PROGRESS: &str = "startup_progress";
pub const NOTIFICATION_REQUESTED: &str = "notification_requested";

pub fn emit_state_changed<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppStateDto,
) -> Result<(), AppError> {
    app.emit(STATE_CHANGED, state).map_err(|error| {
        AppError::with_details("event_emit_failed", "无法发布状态事件", error.to_string())
    })
}
