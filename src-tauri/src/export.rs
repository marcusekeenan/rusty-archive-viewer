// export.rs

use crate::archiver::types::*;
use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Write;

/// Exports data to CSV format
pub fn export_to_csv(data: &[NormalizedPVData]) -> Result<String, String> {
    let mut wtr = csv::Writer::from_writer(vec![]);

    // Write header
    wtr.write_record(&[
        "timestamp",
        "pv_name",
        "value",
        "severity",
        "status",
        "units",
        "min",
        "max",
        "stddev",
        "count",
    ])
    .map_err(|e| format!("Failed to write CSV header: {}", e))?;

    // Write data for each PV
    for pv_data in data {
        let units = &pv_data.meta.EGU;

        for point in &pv_data.data {
            let timestamp = format_timestamp(point.timestamp);

            wtr.write_record(&[
                timestamp,
                pv_data.meta.name.clone(),
                point.value.to_string(),
                point.severity.to_string(),
                point.status.to_string(),
                units.clone(),
                point.min.to_string(),
                point.max.to_string(),
                point.stddev.to_string(),
                point.count.to_string(),
            ])
            .map_err(|e| format!("Failed to write CSV record: {}", e))?;
        }
    }

    String::from_utf8(wtr.into_inner().map_err(|e| e.to_string())?)
        .map_err(|e| format!("Failed to create CSV string: {}", e))
}

/// Exports data to MATLAB format
pub fn export_to_matlab(data: &[NormalizedPVData]) -> Result<String, String> {
    #[derive(Serialize)]
    struct MatlabData<'a> {
        name: &'a str,
        timestamps: Vec<f64>,
        values: Vec<f64>,
        units: &'a str,
        metadata: HashMap<&'static str, String>,
    }

    let mut matlab_data = Vec::new();

    for pv_data in data {
        let timestamps: Vec<f64> = pv_data
            .data
            .iter()
            .map(|p| p.timestamp as f64 / 1000.0) // Convert to seconds
            .collect();

        let values: Vec<f64> = pv_data.data.iter().map(|p| p.value).collect();

        let mut metadata = HashMap::new();
        metadata.insert(
            "description",
            pv_data.meta.description.clone().unwrap_or_default(),
        );
        if let Some(prec) = pv_data.meta.precision {
            metadata.insert("precision", prec.to_string());
        }

        matlab_data.push(MatlabData {
            name: &pv_data.meta.name,
            timestamps,
            values,
            units: &pv_data.meta.EGU,
            metadata,
        });
    }

    serde_json::to_string_pretty(&matlab_data)
        .map_err(|e| format!("Failed to create MATLAB data: {}", e))
}

/// Exports data to text format
pub fn export_to_text(data: &[NormalizedPVData]) -> Result<String, String> {
    let mut output = String::new();

    for pv_data in data {
        writeln!(output, "PV: {}", pv_data.meta.name)
            .map_err(|e| format!("Failed to write text: {}", e))?;
        writeln!(output, "Units: {}", pv_data.meta.EGU)
            .map_err(|e| format!("Failed to write text: {}", e))?;

        if let Some(desc) = &pv_data.meta.description {
            writeln!(output, "Description: {}", desc)
                .map_err(|e| format!("Failed to write text: {}", e))?;
        }

        writeln!(output, "\nTimestamp\t\t\tValue\tSeverity\tStatus")
            .map_err(|e| format!("Failed to write text: {}", e))?;

        for point in &pv_data.data {
            writeln!(
                output,
                "{}\t{:.6}\t{}\t{}",
                format_timestamp(point.timestamp),
                point.value,
                point.severity,
                point.status
            )
            .map_err(|e| format!("Failed to write text: {}", e))?;
        }

        if let Some(stats) = &pv_data.statistics {
            writeln!(output, "\nStatistics:")
                .map_err(|e| format!("Failed to write text: {}", e))?;
            writeln!(output, "Mean: {:.6}", stats.mean)
                .map_err(|e| format!("Failed to write text: {}", e))?;
            writeln!(output, "Standard Deviation: {:.6}", stats.std_dev)
                .map_err(|e| format!("Failed to write text: {}", e))?;
            writeln!(output, "Minimum: {:.6}", stats.min)
                .map_err(|e| format!("Failed to write text: {}", e))?;
            writeln!(output, "Maximum: {:.6}", stats.max)
                .map_err(|e| format!("Failed to write text: {}", e))?;
            writeln!(output, "Count: {}", stats.count)
                .map_err(|e| format!("Failed to write text: {}", e))?;
        }

        writeln!(output, "\n").map_err(|e| format!("Failed to write text: {}", e))?;
    }

    Ok(output)
}

/// Exports data to SVG format
pub fn export_to_svg(data: &[NormalizedPVData]) -> Result<String, String> {
    const WIDTH: i32 = 800;
    const HEIGHT: i32 = 400;
    const MARGIN: i32 = 50;
    const PLOT_WIDTH: i32 = WIDTH - 2 * MARGIN;
    const PLOT_HEIGHT: i32 = HEIGHT - 2 * MARGIN;

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}">"#,
        WIDTH, HEIGHT
    ));

    svg.push_str(
        r#"
        <style>
            .axis { stroke: #333; stroke-width: 1; }
            .data { fill: none; stroke-width: 1.5; }
            .label { font-family: Arial; font-size: 12px; }
        </style>
    "#,
    );

    for (i, pv_data) in data.iter().enumerate() {
        // ... rest of SVG plotting code ...
    }

    svg.push_str("</svg>");
    Ok(svg)
}

/// Helper function to format timestamps
fn format_timestamp(ms: i64) -> String {
    let dt = Utc.timestamp_millis_opt(ms).unwrap();
    dt.format("%Y-%m-%d %H:%M:%S%.3f").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_export() {
        // Add tests...
    }

    #[test]
    fn test_matlab_export() {
        // Add tests...
    }

    #[test]
    fn test_text_export() {
        // Add tests...
    }

    #[test]
    fn test_svg_export() {
        // Add tests...
    }
}
