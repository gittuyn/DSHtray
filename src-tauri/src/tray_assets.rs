use crate::domain::{LifecycleState, Ownership, RuntimeSnapshot};
use tauri::image::Image;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayIconKind {
    NotRunningRed,
    ExternalYellow,
    NonProxyBlue,
    ProxyBlack,
}

pub fn icon_kind_for_runtime(snapshot: &RuntimeSnapshot) -> TrayIconKind {
    match (snapshot.state, snapshot.ownership) {
        (LifecycleState::External, Ownership::External) => TrayIconKind::ExternalYellow,
        (LifecycleState::Stopped, _) | (LifecycleState::PortConflict, _) => {
            TrayIconKind::NotRunningRed
        }
        (LifecycleState::Failed, Ownership::None) => TrayIconKind::NotRunningRed,
        _ => icon_kind_for_proxy(snapshot.proxy_enabled),
    }
}

pub fn icon_kind_for_proxy(proxy_enabled: bool) -> TrayIconKind {
    if proxy_enabled {
        TrayIconKind::ProxyBlack
    } else {
        TrayIconKind::NonProxyBlue
    }
}

pub fn tooltip_for_runtime(snapshot: &RuntimeSnapshot) -> &'static str {
    match icon_kind_for_runtime(snapshot) {
        TrayIconKind::NotRunningRed => "DSHtray · DSH 未启动",
        TrayIconKind::ExternalYellow => "DSHtray · 等待确认接管",
        TrayIconKind::NonProxyBlue => "DSHtray · 代理关闭",
        TrayIconKind::ProxyBlack => "DSHtray · 代理开启",
    }
}

pub fn tooltip_for_proxy(proxy_enabled: bool) -> &'static str {
    if proxy_enabled {
        "DSHtray · 代理开启"
    } else {
        "DSHtray · 代理关闭"
    }
}

pub fn icon_bytes(kind: TrayIconKind) -> &'static [u8] {
    match kind {
        TrayIconKind::NotRunningRed => include_bytes!("../icons/tray-deepseek-red.png"),
        TrayIconKind::ExternalYellow => include_bytes!("../icons/tray-deepseek-yellow.png"),
        TrayIconKind::NonProxyBlue => include_bytes!("../icons/tray-deepseek-blue.png"),
        TrayIconKind::ProxyBlack => include_bytes!("../icons/tray-deepseek-black.png"),
    }
}

pub fn image_for_runtime(snapshot: &RuntimeSnapshot) -> tauri::Result<Image<'static>> {
    Image::from_bytes(icon_bytes(icon_kind_for_runtime(snapshot)))
}

pub fn image_for_proxy(proxy_enabled: bool) -> tauri::Result<Image<'static>> {
    Image::from_bytes(icon_bytes(icon_kind_for_proxy(proxy_enabled)))
}
