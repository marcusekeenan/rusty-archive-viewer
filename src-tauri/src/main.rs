#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod archiver;

use tauri::Manager;
use crate::archiver::commands::*;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            fetch_pv_data,
            fetch_binned_data,
            get_pv_metadata,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}