#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rusty_archive_viewer::archiver::commands::*;
use tauri::{Manager, Window};

fn main() {
    let context = tauri::generate_context!();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // Core data fetching
            fetch_binned_data,
            fetch_data_with_operator,
            fetch_optimized_data,
            fetch_raw_data,
            get_data_at_time,
            
            // Metadata and status
            get_pv_metadata,
            get_pv_status,
            get_health_status,
            
            // Testing and utilities
            test_connection,
            // export_data,
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_window("main") {
                    window.open_devtools();
                }
            }
        
            // Example of initializing HealthMonitor here if needed
            tauri::async_runtime::spawn(async {
                if let Ok(monitor) = get_health_monitor().await {
                    if let Err(e) = monitor.start().await {
                        eprintln!("Failed to start HealthMonitor: {}", e);
                    }
                } else {
                    eprintln!("Failed to initialize HealthMonitor");
                }
            });
        
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