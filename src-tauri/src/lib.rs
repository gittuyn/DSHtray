#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod app_error;
pub mod app_state;
pub mod autostart;
pub mod commands;
pub mod config;
pub mod diagnostics;
pub mod discovery;
pub mod domain;
pub mod events;
pub mod health;
pub mod lifecycle;
pub mod logging;
pub mod network;
pub mod process;
pub mod process_flags;
pub mod proxy;
pub mod targets;
pub mod tray;
pub mod tray_assets;

use app_state::AppState;
use commands::{
    adopt_external_dsh, apply_proxy_change, complete_first_run, get_app_state, open_dsh_url,
    open_log_directory, prepare_proxy_change, restart_dsh, run_self_test, save_settings,
    scan_targets, set_active_target, start_dsh, stop_dsh,
};
use config::ConfigStore;
use lifecycle::{
    BlockingHealthAdapter, DefaultLifecycleController, RealClock, WindowsProcessAdapter,
};
use tauri::{Manager, WindowEvent};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config_dir = app
                .path()
                .app_config_dir()
                .map_err(|error| std::io::Error::other(format!("无法解析配置目录: {error}")))?;
            let config_path = config_dir.join("config.json");
            let missing_before_load = !config_path.exists();
            let loaded = ConfigStore::load(&config_path)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let first_run = missing_before_load
                || (!loaded.config.targets.source.is_configured()
                    && !loaded.config.targets.packaged.is_configured());
            let start_on_login = loaded.config.manager.start_on_login;
            let start_dsh_on_login = loaded.config.manager.start_dsh_on_login;
            let mut lifecycle = DefaultLifecycleController::new(
                loaded.config,
                WindowsProcessAdapter::default(),
                BlockingHealthAdapter::default(),
                RealClock,
            );
            if !first_run {
                lifecycle
                    .refresh_external_state()
                    .map_err(|error| std::io::Error::other(error.to_string()))?;
            }
            let initial_runtime = lifecycle.snapshot();
            app.manage(AppState::new(lifecycle, config_path, first_run));
            let mut autostart = autostart::TauriAutostart { app };
            autostart::reconcile_autostart(&mut autostart, start_on_login, false)
                .map_err(std::io::Error::other)?;
            tray::setup(app, &initial_runtime)?;
            if start_dsh_on_login {
                let state = app.state::<AppState>();
                commands::start_dsh(state, app.handle().clone())
                    .map_err(|error| std::io::Error::other(error.to_string()))?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_app_state,
            start_dsh,
            stop_dsh,
            restart_dsh,
            prepare_proxy_change,
            apply_proxy_change,
            set_active_target,
            save_settings,
            scan_targets,
            complete_first_run,
            adopt_external_dsh,
            open_dsh_url,
            open_log_directory,
            run_self_test
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
