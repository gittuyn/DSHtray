use dshtray_lib::autostart::{reconcile_autostart, AutostartPort};
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
