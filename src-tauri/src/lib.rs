#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod app_error;
pub mod config;
pub mod domain;
pub mod proxy;
pub mod targets;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
