use crate::domain::ManagerConfig;
use tauri::Runtime;
use tauri_plugin_autostart::ManagerExt as TauriAutostartExt;

pub trait AutostartPort {
    fn enable(&mut self) -> Result<(), String>;
    fn disable(&mut self) -> Result<(), String>;
}

pub fn should_start_dsh_on_login(manager: &ManagerConfig) -> bool {
    manager.start_dsh_on_login
}

pub fn reconcile_autostart<P: AutostartPort>(
    port: &mut P,
    start_on_login: bool,
    _start_dsh_on_login: bool,
) -> Result<(), String> {
    if start_on_login {
        port.enable()
    } else {
        port.disable()
    }
}

pub struct TauriAutostart<'a, R: Runtime> {
    pub app: &'a tauri::App<R>,
}

impl<R: Runtime> AutostartPort for TauriAutostart<'_, R> {
    fn enable(&mut self) -> Result<(), String> {
        self.app
            .autolaunch()
            .enable()
            .map_err(|error| error.to_string())
    }

    fn disable(&mut self) -> Result<(), String> {
        self.app
            .autolaunch()
            .disable()
            .map_err(|error| error.to_string())
    }
}
