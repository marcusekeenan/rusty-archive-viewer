// main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rusty_archive_viewer::archiver::commands::*;
use tauri::{Manager, Window, WindowUrl};

/// Main entry point for the application
fn main() {
    let context = tauri::generate_context!();

    tauri::Builder::default()
        // Register all command handlers
        .invoke_handler(tauri::generate_handler![
            // Data retrieval commands
            fetch_data,         // Historical data
            fetch_data_at_time, // Point-in-time data
            // Metadata and validation commands
            get_pv_metadata, // PV metadata
            validate_pvs,    // PV validation
            get_pv_status,   // PV status
            test_connection, // Connection testing
            // Export and utility commands
            export_data,         // Data export
            toggle_debug_window, // Debug interface
            start_live_updates,
            stop_live_updates,
        ])
        // Initial setup
        .setup(|app| {
            // Enable DevTools in debug mode
            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_window("main") {
                    window.open_devtools();

                    // Log application startup in debug mode
                    println!("Application started in debug mode");
                }
            }

            Ok(())
        })
        // Menu configuration
        .menu(if cfg!(target_os = "macos") {
            tauri::Menu::os_default(&context.package_info().name)
        } else {
            tauri::Menu::default()
        })
        // Error handling and window management settings
        .on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event.event() {
                // Handle window close events
                #[cfg(debug_assertions)]
                println!("Window close requested");
            }
        })
        // Run the application
        .run(context)
        .expect("Failed to start application");
}
