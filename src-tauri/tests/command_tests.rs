// use chrono::Utc;
// use tauri::Manager;

// use rusty_archive_viewer::{
//     AppState,
//     Config,
//     fetch_data,
//     fetch_latest,
//     test_connection,
//     get_pv_metadata,
//     constants::{DEFAULT_BASE_URL, DEFAULT_TIMEOUT},
// };

// // Known good PVs for testing
// const TEST_PVS: [&str; 4] = [
//     "ROOM:LI30:1:OUTSIDE_TEMP",
//     "CPT:PSI5:5205:PRESS",
//     "CFT:PSI8:8601:FLOW",
//     "CTE:PSI5:5504:TEMP"
// ];

// #[cfg(test)]
// mod tests {
//     use super::*;

//     /// Sets up the Tauri app with `AppState`.
//     async fn setup_tauri() -> tauri::App<tauri::Wry> {
//         let app = tauri::Builder::default()
//             .manage(AppState::new(Config {
//                 url: DEFAULT_BASE_URL.to_string(),
//                 timeout_secs: DEFAULT_TIMEOUT.as_secs(),
//             }))
//             .build(tauri::generate_context!())
//             .expect("Failed to build app");
//         app
//     }

//     /// Tests `fetch_data` and verifies results for a valid PV.
//     #[tokio::test]
//     async fn test_fetch_data_command() {
//         let app = setup_tauri().await;
//         let state = app.state::<AppState>();

//         let now = Utc::now().timestamp();
//         let one_hour_ago = now - 3600;

//         println!("\n=== Testing fetch_data command ===");
//         let result = fetch_data(
//             state.clone(),
//             vec![TEST_PVS[0].to_string()],
//             one_hour_ago,
//             now,
//         )
//         .await;

//         match result {
//             Ok(data) => {
//                 println!("Fetched data for {}", TEST_PVS[0]);
//                 println!("Number of points: {}", data[0].data.len());
//                 if let Some(first) = data[0].data.first() {
//                     println!("First point:");
//                     println!("  Time: {}s {}ns", first.secs, first.nanos);
//                     println!("  Value: {}", first.val);
//                 }
//             }
//             Err(e) => panic!("Failed to fetch data: {}", e),
//         }
//     }

//     /// Tests `fetch_latest` and verifies the most recent point.
//     #[tokio::test]
//     async fn test_fetch_latest_command() {
//         let app = setup_tauri().await;
//         let state = app.state::<AppState>();

//         println!("\n=== Testing fetch_latest command ===");
//         let result = fetch_latest(state.clone(), TEST_PVS[0].to_string()).await;

//         match result {
//             Ok(point) => {
//                 println!("Latest point for {}:", TEST_PVS[0]);
//                 println!("  Time: {}s {}ns", point.secs, point.nanos);
//                 println!("  Value: {}", point.val);
//             }
//             Err(e) => panic!("Failed to fetch latest point: {}", e),
//         }
//     }

//     /// Tests `test_connection` to verify connectivity.
//     #[tokio::test]
//     async fn test_connection_command() {
//         let app = setup_tauri().await;
//         let state = app.state::<AppState>();

//         println!("\n=== Testing connection command ===");
//         let result = test_connection(state.clone()).await;
//         assert!(result.unwrap_or(false), "Connection test failed");
//     }

//     /// Tests `get_pv_metadata` to verify metadata fetching.
//     #[tokio::test]
//     async fn test_get_pv_metadata_command() {
//         let app = setup_tauri().await;
//         let state = app.state::<AppState>();

//         println!("\n=== Testing metadata command ===");
//         let result = get_pv_metadata(state.clone(), TEST_PVS[0].to_string()).await;

//         match result {
//             Ok(metadata) => {
//                 println!("Metadata for {}:", TEST_PVS[0]);
//                 for (key, value) in metadata.0.iter() {
//                     println!("  {}: {}", key, value);
//                 }
//             }
//             Err(e) => panic!("Failed to fetch metadata: {}", e),
//         }
//     }

//     /// Tests invalid inputs for `fetch_data`.
//     #[tokio::test]
//     async fn test_error_cases() {
//         let app = setup_tauri().await;
//         let state = app.state::<AppState>();
//         let now = Utc::now().timestamp();

//         println!("\n=== Testing Error Cases ===");

//         // Invalid PV
//         println!("\nTesting invalid PV:");
//         let result = fetch_data(
//             state.clone(),
//             vec!["INVALID:PV:NAME".to_string()],
//             now - 3600,
//             now,
//         )
//         .await;
//         assert!(result.is_err(), "Expected error for invalid PV");

//         // Invalid time range
//         println!("\nTesting invalid time range:");
//         let result = fetch_data(
//             state.clone(),
//             vec![TEST_PVS[0].to_string()],
//             now,
//             now - 3600, // End before start
//         )
//         .await;
//         assert!(result.is_err(), "Expected error for invalid time range");

//         // Empty PV list
//         println!("\nTesting empty PV list:");
//         let result = fetch_data(state.clone(), vec![], now - 3600, now).await;
//         assert!(result.is_err(), "Expected error for empty PV list");
//     }
// }
