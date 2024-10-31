// lib.rs

mod archiver;

use archiver::{
    commands::{fetch_archiver_data, fetch_binned_data, set_archiver_url_command},
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            fetch_archiver_data,
            fetch_binned_data,
            set_archiver_url_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
