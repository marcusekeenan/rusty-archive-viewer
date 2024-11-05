// //! Test module for the EPICS Archiver Appliance interface
// //! Includes both unit tests and integration tests

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::archiver::{api::ArchiverClient, commands, export::*, types::*};
//     use chrono::{DateTime, TimeZone, Utc};
//     use pretty_assertions::assert_eq;
//     use std::time::SystemTime;

//     // Test constants
//     const TEST_PVS: [&str; 3] = [
//         "ROOM:LI30:1:OUTSIDE_TEMP", // Temperature reading
//         "VPIO:IN20:111:VRAW",       // Voltage reading
//         "YAGS:UND1:1005:Y_BM_CTR",  // Beam position
//     ];

//     const TEST_TIMEFRAMES: [(i64, i64); 3] = [
//         // Recent data (5 minutes)
//         (1710287585, 1710287899), // 2024-03-12T23:53:05Z to 23:58:19Z
//         // Hour range
//         (1710284285, 1710287885), // 2024-03-12T22:53:05Z to 23:53:05Z
//         // Day range
//         (1710201485, 1710287885), // 2024-03-12T00:53:05Z to 23:53:05Z
//     ];

//     /// Helper function to create test data
//     fn create_test_data() -> NormalizedPVData {
//         NormalizedPVData {
//             meta: Meta {
//                 name: TEST_PVS[0].to_string(),
//                 egu: "C".to_string(),
//                 description: Some("Test temperature sensor".to_string()),
//                 precision: Some(2),
//                 archive_parameters: Some(ArchiveParameters {
//                     sampling_period: 1.0,
//                     sampling_method: "MONITOR".to_string(),
//                     last_modified: SystemTime::now(),
//                 }),
//                 display_limits: Some(DisplayLimits {
//                     low: 0.0,
//                     high: 100.0,
//                 }),
//                 alarm_limits: Some(AlarmLimits {
//                     low: 10.0,
//                     high: 90.0,
//                     lolo: 5.0,
//                     hihi: 95.0,
//                 }),
//             },
//             data: vec![
//                 ProcessedPoint {
//                     timestamp: TEST_TIMEFRAMES[0].0 * 1000,
//                     severity: 0,
//                     status: 0,
//                     value: 23.5,
//                     min: 23.5,
//                     max: 23.5,
//                     stddev: 0.0,
//                     count: 1,
//                 },
//                 ProcessedPoint {
//                     timestamp: (TEST_TIMEFRAMES[0].0 + 60) * 1000,
//                     severity: 0,
//                     status: 0,
//                     value: 23.6,
//                     min: 23.6,
//                     max: 23.6,
//                     stddev: 0.0,
//                     count: 1,
//                 },
//             ],
//             statistics: Some(Statistics {
//                 mean: 23.55,
//                 std_dev: 0.05,
//                 min: 23.5,
//                 max: 23.6,
//                 count: 2,
//                 first_timestamp: TEST_TIMEFRAMES[0].0 * 1000,
//                 last_timestamp: (TEST_TIMEFRAMES[0].0 + 60) * 1000,
//             }),
//         }
//     }

//     // Command Tests

//     #[tokio::test]
//     async fn test_fetch_binned_data_command() {
//         let (start, end) = TEST_TIMEFRAMES[0];
//         let options = ExtendedFetchOptions {
//             operator: Some("raw".to_string()),
//             ..Default::default()
//         };

//         let result = commands::fetch_binned_data(
//             TEST_PVS.iter().map(|&s| s.to_string()).collect(),
//             start,
//             end,
//             Some(options),
//         )
//         .await;

//         assert!(result.is_ok(), "Command failed: {}", result.unwrap_err());
//         let data = result.unwrap();
//         assert_eq!(data.len(), TEST_PVS.len());
//     }

//     #[tokio::test]
//     async fn test_fetch_data_with_operator_command() {
//         let (start, end) = TEST_TIMEFRAMES[1];

//         let result = commands::fetch_data_with_operator(
//             TEST_PVS.iter().map(|&s| s.to_string()).collect(),
//             start,
//             end,
//             "mean_60".to_string(),
//             None,
//         )
//         .await;

//         assert!(result.is_ok(), "Command failed: {}", result.unwrap_err());
//         let data = result.unwrap();
//         assert!(!data.is_empty());
//     }

//     #[tokio::test]
//     async fn test_fetch_raw_data_command() {
//         let (start, end) = TEST_TIMEFRAMES[0];

//         let result = commands::fetch_raw_data(
//             TEST_PVS.iter().map(|&s| s.to_string()).collect(),
//             start,
//             end,
//         )
//         .await;

//         assert!(result.is_ok(), "Command failed: {}", result.unwrap_err());
//         let data = result.unwrap();
//         assert!(!data.is_empty());
//     }

//     #[tokio::test]
//     async fn test_fetch_optimized_data_command() {
//         let (start, end) = TEST_TIMEFRAMES[2];

//         let result = commands::fetch_optimized_data(
//             TEST_PVS.iter().map(|&s| s.to_string()).collect(),
//             start,
//             end,
//             800, // chart width
//         )
//         .await;

//         assert!(result.is_ok(), "Command failed: {}", result.unwrap_err());
//         let data = result.unwrap();
//         assert!(!data.is_empty());
//     }

//     #[tokio::test]
//     async fn test_export_data_command() {
//         let (start, end) = TEST_TIMEFRAMES[0];

//         for format in [
//             DataFormat::Csv,
//             DataFormat::Text,
//             DataFormat::Matlab,
//             DataFormat::Svg,
//         ] {
//             let result = commands::export_data(
//                 TEST_PVS.iter().map(|&s| s.to_string()).collect(),
//                 start,
//                 end,
//                 format.clone(),
//                 None,
//             )
//             .await;

//             assert!(
//                 result.is_ok(),
//                 "Export failed for format {:?}: {}",
//                 format,
//                 result.unwrap_err()
//             );
//         }
//     }

//     //  #[tokio::test]
//     async fn test_get_pv_metadata_command() {
//         // Test successful cases
//         for &pv in &TEST_PVS {
//             let result = commands::get_pv_metadata(pv.to_string()).await;
//             match result {
//                 Ok(meta) => {
//                     // Verify metadata fields
//                     assert_eq!(meta.name, pv, "PV name mismatch");
//                     assert!(!meta.egu.is_empty(), "Units missing");

//                     // Optional fields
//                     if let Some(prec) = meta.precision {
//                         assert!(prec >= 0, "Invalid precision for {}", pv);
//                     }

//                     if let Some(params) = meta.archive_parameters {
//                         assert!(params.sampling_period > 0.0, "Invalid sampling period");
//                     }
//                 }
//                 Err(e) => panic!("Failed to fetch metadata for {}: {}", pv, e),
//             }
//         }

//         // Test error case
//         let invalid_result = commands::get_pv_metadata("INVALID:PV:NAME".to_string()).await;
//         assert!(invalid_result.is_err(), "Expected error for invalid PV");
//     }
//     // Export Tests

//     #[test]
//     fn test_csv_export() {
//         let test_data = vec![create_test_data()];
//         let result = export_to_csv(&test_data).unwrap();
//         assert!(result.contains("timestamp,pv_name,value"));
//         assert!(result.contains(TEST_PVS[0]));
//         assert!(result.contains("23.5"));
//     }

//     // #[test]
//     fn test_matlab_export() {
//         let test_data = vec![create_test_data()];
//         let result = export_to_matlab(&test_data).unwrap();
//         assert!(result.contains("timestamps"));
//         assert!(result.contains("values"));
//         assert!(result.contains(TEST_PVS[0]));
//     }

//     #[test]
//     fn test_text_export() {
//         let test_data = vec![create_test_data()];
//         let result = export_to_text(&test_data).unwrap();
//         assert!(result.contains("PV:"));
//         assert!(result.contains("Units:"));
//         assert!(result.contains(TEST_PVS[0]));
//         assert!(result.contains("23.5"));
//     }

//     //#[test]
//     fn test_svg_export() {
//         let test_data = vec![create_test_data()];
//         let result = export_to_svg(&test_data).unwrap();
//         assert!(result.contains("<svg"));
//         assert!(result.contains("</svg>"));
//         assert!(result.contains("path"));
//     }

//     // Error Tests

//     // #[tokio::test]
//     async fn test_error_handling() {
//         // Test invalid PV
//         let result = commands::get_pv_metadata("INVALID:PV:NAME".to_string()).await;
//         assert!(result.is_err());

//         // Test invalid time range
//         let result = commands::fetch_binned_data(
//             vec![TEST_PVS[0].to_string()],
//             TEST_TIMEFRAMES[0].1, // end
//             TEST_TIMEFRAMES[0].0, // start
//             None,
//         )
//         .await;
//         assert!(result.is_err());

//         // Test invalid operator
//         let result = commands::fetch_data_with_operator(
//             vec![TEST_PVS[0].to_string()],
//             TEST_TIMEFRAMES[0].0,
//             TEST_TIMEFRAMES[0].1,
//             "invalid_operator".to_string(),
//             None,
//         )
//         .await;
//         assert!(result.is_err());

//         // Test empty PV list
//         let result =
//             commands::fetch_binned_data(vec![], TEST_TIMEFRAMES[0].0, TEST_TIMEFRAMES[0].1, None)
//                 .await;
//         assert!(result.is_err());
//     }
// }
