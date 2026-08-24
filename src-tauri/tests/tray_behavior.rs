use dshtray_lib::{
    process_flags::{windows_creation_flags, CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW},
    tray_assets::{icon_bytes, icon_kind_for_proxy, tooltip_for_proxy, TrayIconKind},
};

#[test]
fn child_processes_hide_console_and_keep_process_group_support() {
    let flags = windows_creation_flags();
    assert_ne!(flags & CREATE_NO_WINDOW, 0);
    assert_ne!(flags & CREATE_NEW_PROCESS_GROUP, 0);
}

#[test]
fn proxy_off_uses_blue_icon_and_proxy_on_uses_black_icon() {
    assert_eq!(icon_kind_for_proxy(false), TrayIconKind::NonProxyBlue);
    assert_eq!(icon_kind_for_proxy(true), TrayIconKind::ProxyBlack);
    assert_eq!(tooltip_for_proxy(false), "DSHtray · 代理关闭");
    assert_eq!(tooltip_for_proxy(true), "DSHtray · 代理开启");
}

#[test]
fn both_embedded_tray_icons_are_png_images() {
    for kind in [TrayIconKind::NonProxyBlue, TrayIconKind::ProxyBlack] {
        assert_eq!(&icon_bytes(kind)[..8], b"\x89PNG\r\n\x1a\n");
    }
}
