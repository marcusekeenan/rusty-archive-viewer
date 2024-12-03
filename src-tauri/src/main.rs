#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rusty_archive_viewer::{commands, AppState, Config};
use tauri::Manager;

fn main() {
    let context = tauri::generate_context!();
    let config = Config::default();
    let state = AppState::new(config);

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::fetch_data,
            commands::fetch_latest,
            commands::test_connection,
            commands::get_pv_metadata, // Add this line
        ])
        .manage(state)
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_window("main") {
                    window.open_devtools();
                    println!("Development mode active - LCLS Archiver Viewer");
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
        .expect("Failed to start application");
}
