use crate::{
    app_state::AppState,
    commands,
    domain::RuntimeSnapshot,
    tray_assets::{image_for_runtime, tooltip_for_runtime},
};
use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

const SINGLE_CLICK_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayEventAction {
    OpenDshPage,
    ShowMainWindow,
    Ignore,
}

pub fn tray_event_action(event: &TrayIconEvent) -> TrayEventAction {
    match event {
        TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
        } => TrayEventAction::ShowMainWindow,
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } => TrayEventAction::OpenDshPage,
        _ => TrayEventAction::Ignore,
    }
}

#[derive(Default)]
struct TrayClickState {
    generation: u64,
    suppress_next_left_up_until: Option<Instant>,
}

pub fn setup<R: Runtime>(app: &tauri::App<R>, runtime: &RuntimeSnapshot) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "打开管理器", true, None::<&str>)?;
    let start = MenuItem::with_id(app, "start", "启动 DSH", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "停止 DSH", true, None::<&str>)?;
    let restart = MenuItem::with_id(app, "restart", "重启 DSH", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = PredefinedMenuItem::quit(app, Some("退出管理器"))?;
    let menu = Menu::with_items(app, &[&open, &start, &stop, &restart, &separator, &quit])?;
    let icon = image_for_runtime(runtime)?;
    let click_state = Arc::new(Mutex::new(TrayClickState::default()));

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip(tooltip_for_runtime(runtime))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_menu_event(app, event.id.as_ref()))
        .on_tray_icon_event(move |tray, event| {
            handle_tray_event(tray, event, click_state.clone());
        })
        .build(app)?;
    Ok(())
}

pub fn sync_icon<R: Runtime>(app: &AppHandle<R>, runtime: &RuntimeSnapshot) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id("main") else {
        return Ok(());
    };
    tray.set_icon(Some(image_for_runtime(runtime)?))?;
    tray.set_tooltip(Some(tooltip_for_runtime(runtime)))?;
    Ok(())
}

fn handle_tray_event<R: Runtime>(
    tray: &TrayIcon<R>,
    event: TrayIconEvent,
    click_state: Arc<Mutex<TrayClickState>>,
) {
    match tray_event_action(&event) {
        TrayEventAction::OpenDshPage => {
            schedule_single_click_open_dsh_page(tray.app_handle().clone(), click_state);
        }
        TrayEventAction::ShowMainWindow => {
            cancel_pending_single_click(&click_state);
            show_main_window(tray.app_handle());
        }
        TrayEventAction::Ignore => {}
    }
}

fn cancel_pending_single_click(click_state: &Mutex<TrayClickState>) {
    let mut state = click_state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.generation = state.generation.wrapping_add(1);
    state.suppress_next_left_up_until = Some(Instant::now() + SINGLE_CLICK_DELAY);
}

fn schedule_single_click_open_dsh_page<R: Runtime>(
    app: AppHandle<R>,
    click_state: Arc<Mutex<TrayClickState>>,
) {
    let generation = {
        let mut state = click_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(deadline) = state.suppress_next_left_up_until {
            if Instant::now() <= deadline {
                state.suppress_next_left_up_until = None;
                return;
            }
            state.suppress_next_left_up_until = None;
        }
        state.generation = state.generation.wrapping_add(1);
        state.generation
    };

    let _ = thread::Builder::new()
        .name("dshtray-tray-click".into())
        .spawn(move || {
            thread::sleep(SINGLE_CLICK_DELAY);
            let should_open = click_state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .generation
                == generation;
            if should_open {
                let state = app.state::<AppState>();
                let _ = commands::open_dsh_url_with_app(state, &app);
            }
        });
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        "open" => show_main_window(app),
        "start" => {
            let state = app.state::<AppState>();
            let _ = commands::start_dsh_with_app(state, app);
        }
        "stop" => {
            let state = app.state::<AppState>();
            let _ = commands::stop_dsh_with_app(state, app);
        }
        "restart" => {
            let state = app.state::<AppState>();
            let _ = commands::restart_dsh_with_app(state, app);
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
