#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use env_logger;
use log::{debug, error, info, warn};
use rusty_archive_viewer::archiver::{
    commands::*,
    constants::ERRORS,
    features::{DEBUG_ENABLED, MAX_CONCURRENT_REQUESTS},
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{Manager, State, Window};
use tokio::sync::Semaphore;

// Track active connections for resource management
struct ConnectionCounter(Arc<AtomicUsize>);

// Rate limiter for API requests
struct RateLimiter(Arc<Semaphore>);

/// Main entry point for the application
fn main() {
    let context = tauri::generate_context!();

    // Initialize connection counter and rate limiter
    let connection_counter = ConnectionCounter(Arc::new(AtomicUsize::new(0)));
    let rate_limiter = RateLimiter(Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS)));

    let mut builder = tauri::Builder::default()
        // Manage shared state
        .manage(connection_counter)
        .manage(rate_limiter)
        // Register command handlers
        .invoke_handler(tauri::generate_handler![
            // Data retrieval commands
            fetch_data,         // Historical data fetching
            fetch_data_at_time, // Point-in-time data retrieval
            // Metadata and validation
            get_pv_metadata, // PV metadata retrieval
            validate_pvs,    // PV name validation
            get_pv_status,   // PV status checking
            test_connection, // Connection testing
            // Utility commands
            export_data,         // Data export functionality
            toggle_debug_window, // Debug interface toggle
        ])
        // Application setup
        .setup(|app| {
            // Initialize logger
            setup_logging();

            // Configure main window
            if let Some(window) = app.get_window("main") {
                #[cfg(debug_assertions)]
                {
                    window.open_devtools();
                    log::info!("DevTools enabled in debug mode");
                }
            }

            // Log startup information
            log::info!(
                "Application started - Version: {}, Debug: {}",
                env!("CARGO_PKG_VERSION"),
                DEBUG_ENABLED
            );

            Ok(())
        })
        // Configure menu
        .menu(if cfg!(target_os = "macos") {
            tauri::Menu::os_default(&context.package_info().name)
        } else {
            tauri::Menu::default()
        })
        // Window event handling
        .on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event.event() {
                #[cfg(debug_assertions)]
                log::info!("Window close requested");

                // Allow the window to close
                api.prevent_close();

                // Clean up resources
                let window = event.window().clone();
                tokio::spawn(async move {
                    if let Err(e) = cleanup_resources(window.clone()).await {
                        log::error!("Error during cleanup: {}", e);
                    }
                    window.close().unwrap_or_else(|e| {
                        log::error!("Error closing window: {}", e);
                    });
                });
            }
        });

    // Run the application
    builder
        .build(context)
        .expect("Failed to build application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                log::info!("Application exit requested");
            }
        });
}

/// Sets up logging configuration
fn setup_logging() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
}

/// Handles window events
fn handle_window_event(event: &tauri::WindowEvent) {
    match event {
        tauri::WindowEvent::Focused(focused) => {
            log::debug!("Window focus changed: {}", focused);
        }
        tauri::WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
            log::debug!("Window scale factor changed: {}", scale_factor);
        }
        _ => {}
    }
}

/// Cleans up resources before window close
async fn cleanup_resources(window: Window) -> Result<(), String> {
    log::info!("Cleaning up resources for window: {}", window.label());

    // Perform any necessary cleanup here
    // No more live updates to clean up as they're handled by the frontend

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_counter() {
        let counter = ConnectionCounter(Arc::new(AtomicUsize::new(0)));
        assert_eq!(counter.0.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter(Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS)));
        assert_eq!(limiter.0.available_permits(), MAX_CONCURRENT_REQUESTS);
    }
}
