use crate::{
    app_state::AppState,
    commands,
    tray_assets::{image_for_proxy, tooltip_for_proxy},
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, Runtime,
};

pub fn setup<R: Runtime>(app: &tauri::App<R>, proxy_enabled: bool) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "打开管理器", true, None::<&str>)?;
    let start = MenuItem::with_id(app, "start", "启动 DSH", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "停止 DSH", true, None::<&str>)?;
    let restart = MenuItem::with_id(app, "restart", "重启 DSH", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = PredefinedMenuItem::quit(app, Some("退出管理器"))?;
    let menu = Menu::with_items(app, &[&open, &start, &stop, &restart, &separator, &quit])?;
    let icon = image_for_proxy(proxy_enabled)?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip(tooltip_for_proxy(proxy_enabled))
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| handle_menu_event(app, event.id.as_ref()))
        .build(app)?;
    Ok(())
}

pub fn sync_icon<R: Runtime>(app: &AppHandle<R>, proxy_enabled: bool) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id("main") else {
        return Ok(());
    };
    tray.set_icon(Some(image_for_proxy(proxy_enabled)?))?;
    tray.set_tooltip(Some(tooltip_for_proxy(proxy_enabled)))?;
    Ok(())
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        "open" => show_main_window(app),
        "start" => {
            let state = app.state::<AppState>();
            let _ = commands::start_dsh(state);
        }
        "stop" => {
            let state = app.state::<AppState>();
            let _ = commands::stop_dsh(state);
        }
        "restart" => {
            let state = app.state::<AppState>();
            let _ = commands::restart_dsh(state);
        }
        _ => {}
    }
}

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
