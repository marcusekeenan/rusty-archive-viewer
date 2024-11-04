#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rusty_archive_viewer::archiver::commands::*;
use tauri::{Manager, Window};

fn main() {
    let context = tauri::generate_context!();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            fetch_binned_data,
            get_pv_metadata,
            get_data_at_time,
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_window("main") {
                    window.open_devtools();
                }
            }
            Ok(())
        })
        .menu(if cfg!(target_os = "macos") {
            tauri::Menu::os_default(&context.package_info().name)
        } else {
            tauri::Menu::default()
        })
        .run(context)
        .expect("error while running tauri application");
}