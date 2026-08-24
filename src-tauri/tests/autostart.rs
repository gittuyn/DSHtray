use dshtray_lib::autostart::{reconcile_autostart, should_start_dsh_on_login, AutostartPort};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct FakeAutostart {
    calls: Arc<Mutex<Vec<&'static str>>>,
}

impl AutostartPort for FakeAutostart {
    fn enable(&mut self) -> Result<(), String> {
        self.calls.lock().unwrap().push("enable");
        Ok(())
    }

    fn disable(&mut self) -> Result<(), String> {
        self.calls.lock().unwrap().push("disable");
        Ok(())
    }
}

#[test]
fn reconcile_does_not_start_dsh_when_manager_autostart_is_enabled() {
    let mut port = FakeAutostart::default();
    reconcile_autostart(&mut port, true, false).expect("reconcile");
    assert_eq!(&*port.calls.lock().unwrap(), &["enable"]);
}

#[test]
fn reconcile_disables_only_manager_autostart() {
    let mut port = FakeAutostart::default();
    reconcile_autostart(&mut port, false, true).expect("reconcile");
    assert_eq!(&*port.calls.lock().unwrap(), &["disable"]);
}

#[test]
fn dsh_login_flag_is_independent_from_manager_autostart() {
    let mut manager = dshtray_lib::domain::ManagerConfig {
        start_on_login: true,
        start_dsh_on_login: false,
        close_to_tray: true,
        confirm_restart_on_proxy_change: true,
    };
    assert!(!should_start_dsh_on_login(&manager));
    manager.start_dsh_on_login = true;
    assert!(should_start_dsh_on_login(&manager));
}
