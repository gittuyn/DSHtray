use tauri::image::Image;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayIconKind {
    NonProxyBlue,
    ProxyBlack,
}

pub fn icon_kind_for_proxy(proxy_enabled: bool) -> TrayIconKind {
    if proxy_enabled {
        TrayIconKind::ProxyBlack
    } else {
        TrayIconKind::NonProxyBlue
    }
}

pub fn tooltip_for_proxy(proxy_enabled: bool) -> &'static str {
    match icon_kind_for_proxy(proxy_enabled) {
        TrayIconKind::NonProxyBlue => "DSHtray · 代理关闭",
        TrayIconKind::ProxyBlack => "DSHtray · 代理开启",
    }
}

pub fn icon_bytes(kind: TrayIconKind) -> &'static [u8] {
    match kind {
        TrayIconKind::NonProxyBlue => include_bytes!("../icons/tray-deepseek-blue.png"),
        TrayIconKind::ProxyBlack => include_bytes!("../icons/tray-deepseek-black.png"),
    }
}

pub fn image_for_proxy(proxy_enabled: bool) -> tauri::Result<Image<'static>> {
    Image::from_bytes(icon_bytes(icon_kind_for_proxy(proxy_enabled)))
}
