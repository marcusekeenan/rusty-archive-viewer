use crate::constants::*;
use std::collections::HashMap;
use crate::types::{PVData, Meta, Point, Error};
use crate::epics::PayloadInfo;
use chrono::{DateTime, Utc};

pub fn normalize_pv_data(payload_info: PayloadInfo, data_points: Vec<Point>) -> PVData {
    let mut meta_map = HashMap::new();
    meta_map.insert("name".to_string(), payload_info.pvname);
    for header in payload_info.headers {
        meta_map.insert(header.name, header.val);
    }

    PVData {
        meta: Meta(meta_map),
        data: data_points,
    }
}

pub fn create_point(seconds_into_year: u32, nano: u32, val: impl Into<serde_json::Value>, severity: Option<i32>, status: Option<i32>) -> Point {
    Point {
        secs: seconds_into_year as i64,
        nanos: nano as i32,
        val: val.into(),
        severity: severity.unwrap_or(0),
        status: status.unwrap_or(0),
    }
}

pub fn format_date_for_archiver(timestamp_ms: i64) -> Option<String> {
    use chrono::{TimeZone, Utc};
    Utc.timestamp_millis_opt(timestamp_ms)
        .single()
        .map(|dt| dt.to_rfc3339().replace("Z", "-00:00"))
}

pub fn unescape_new_lines(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        let b = input[i];
        if b == ESCAPE_CHAR {
            i += 1;
            if i >= input.len() {
                break;
            }
            match input[i] {
                ESCAPE_ESCAPE_CHAR => output.push(ESCAPE_CHAR),
                NEWLINE_ESCAPE_CHAR => output.push(NEWLINE_CHAR),
                CARRIAGERETURN_ESCAPE_CHAR => output.push(CARRIAGERETURN_CHAR),
                b => output.push(b),
            }
        } else {
            output.push(b);
        }
        i += 1;
    }
    output
}

pub fn format_timestamp(timestamp: i64) -> Result<String, Error> {
    DateTime::<Utc>::from_timestamp(timestamp, 0)
        .ok_or_else(|| Error::Invalid(format!("Invalid timestamp: {}", timestamp)))
        .map(|dt| dt.to_rfc3339())
}