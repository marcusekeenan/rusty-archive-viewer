// main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rusty_archive_viewer::archiver::commands::*;
use tauri::{Manager, Window, WindowUrl};

fn main() {
    let context = tauri::generate_context!();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_pv_metadata,
            validate_pvs,
            get_pv_status,
            test_connection,
            fetch_data,
            fetch_live_data,
            export_data,
            toggle_debug_window
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_window("main") {
                    window.open_devtools();
                }
            }

            // No need to pre-create the debug window
            // It will be created on-demand when toggled

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
