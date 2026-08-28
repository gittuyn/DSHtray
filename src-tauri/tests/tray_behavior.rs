use dshtray_lib::{
    domain::{LifecycleState, Ownership, RuntimeSnapshot, TargetId},
    process_flags::{windows_creation_flags, CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW},
    tray::{tray_event_action, TrayEventAction},
    tray_assets::{
        icon_bytes, icon_kind_for_proxy, icon_kind_for_runtime, tooltip_for_proxy,
        tooltip_for_runtime, TrayIconKind,
    },
};
use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconEvent, TrayIconId},
    PhysicalPosition, Rect,
};

fn snapshot(state: LifecycleState, ownership: Ownership, proxy_enabled: bool) -> RuntimeSnapshot {
    RuntimeSnapshot {
        state,
        target: TargetId::Source,
        pid: None,
        ownership,
        service_url: "http://127.0.0.1:3080".into(),
        proxy_enabled,
        last_error: None,
        started_at: None,
    }
}

fn tray_double_click(button: MouseButton) -> TrayIconEvent {
    TrayIconEvent::DoubleClick {
        id: TrayIconId::new("main"),
        position: PhysicalPosition::new(0.0, 0.0),
        rect: Rect::default(),
        button,
    }
}

fn tray_click(button: MouseButton, button_state: MouseButtonState) -> TrayIconEvent {
    TrayIconEvent::Click {
        id: TrayIconId::new("main"),
        position: PhysicalPosition::new(0.0, 0.0),
        rect: Rect::default(),
        button,
        button_state,
    }
}

#[test]
fn left_double_click_opens_manager_window() {
    assert_eq!(
        tray_event_action(&tray_double_click(MouseButton::Left)),
        TrayEventAction::ShowMainWindow
    );
}

#[test]
fn right_double_click_does_not_open_dsh_page() {
    assert_eq!(
        tray_event_action(&tray_double_click(MouseButton::Right)),
        TrayEventAction::Ignore
    );
}

#[test]
fn single_left_click_opens_dsh_page() {
    assert_eq!(
        tray_event_action(&tray_click(MouseButton::Left, MouseButtonState::Up)),
        TrayEventAction::OpenDshPage
    );
}

#[test]
fn mouse_down_does_not_trigger_an_action() {
    assert_eq!(
        tray_event_action(&tray_click(MouseButton::Left, MouseButtonState::Down)),
        TrayEventAction::Ignore
    );
}

#[test]
fn child_processes_hide_console_and_keep_process_group_support() {
    let flags = windows_creation_flags();
    assert_ne!(flags & CREATE_NO_WINDOW, 0);
    assert_ne!(flags & CREATE_NEW_PROCESS_GROUP, 0);
}

#[test]
fn stopped_dsh_uses_red_icon_regardless_of_proxy_setting() {
    let stopped = snapshot(LifecycleState::Stopped, Ownership::None, true);

    assert_eq!(icon_kind_for_runtime(&stopped), TrayIconKind::NotRunningRed);
    assert_eq!(tooltip_for_runtime(&stopped), "DSHtray · DSH 未启动");
}

#[test]
fn unadopted_external_dsh_uses_yellow_icon() {
    let external = snapshot(LifecycleState::External, Ownership::External, false);

    assert_eq!(
        icon_kind_for_runtime(&external),
        TrayIconKind::ExternalYellow
    );
    assert_eq!(tooltip_for_runtime(&external), "DSHtray · 等待确认接管");
}

#[test]
fn managed_running_dsh_keeps_existing_proxy_icon_mapping() {
    let proxy_on = snapshot(LifecycleState::Running, Ownership::Managed, true);
    let proxy_off = snapshot(LifecycleState::Running, Ownership::Managed, false);

    assert_eq!(icon_kind_for_runtime(&proxy_on), TrayIconKind::ProxyBlack);
    assert_eq!(
        icon_kind_for_runtime(&proxy_off),
        TrayIconKind::NonProxyBlue
    );
}

#[test]
fn proxy_off_uses_blue_icon_and_proxy_on_uses_black_icon() {
    assert_eq!(icon_kind_for_proxy(false), TrayIconKind::NonProxyBlue);
    assert_eq!(icon_kind_for_proxy(true), TrayIconKind::ProxyBlack);
    assert_eq!(tooltip_for_proxy(false), "DSHtray · 代理关闭");
    assert_eq!(tooltip_for_proxy(true), "DSHtray · 代理开启");
}

#[test]
fn all_embedded_tray_icons_are_png_images() {
    for kind in [
        TrayIconKind::NotRunningRed,
        TrayIconKind::ExternalYellow,
        TrayIconKind::NonProxyBlue,
        TrayIconKind::ProxyBlack,
    ] {
        assert_eq!(&icon_bytes(kind)[..8], b"\x89PNG\r\n\x1a\n");
    }
}
