use bytes::Bytes;
use prost::Message;
use serde_json::json;
use std::collections::HashMap;
use chrono::{NaiveDate, NaiveDateTime};
use crate::constants::*;
use crate::types::*;
use crate::epics::*;
use crate::decode_helpers::unescape_new_lines;
use prost::DecodeError;

/// Converts "seconds into the year" and nanoseconds into Unix milliseconds.
fn convert_to_unix_ms(secondsintoyear: u32, nanos: u32, year: i32) -> i64 {
    let start_of_year = NaiveDate::from_ymd_opt(year, 1, 1)
        .expect("Invalid year")
        .and_hms_opt(0, 0, 0)
        .expect("Invalid time")
        .timestamp();

    let total_seconds = start_of_year + secondsintoyear as i64;
    total_seconds * 1000 + (nanos / 1_000_000) as i64
}

pub fn decode_response(raw_bytes: Bytes) -> Result<Vec<PVData>, Error> {
    let mut chunks = raw_bytes.split(|&b| b == NEWLINE_CHAR);
    let header = chunks.next().ok_or_else(|| Error::Decode("Empty response".to_string()))?;
    let unescaped_header = unescape_new_lines(header);
    let payload_info = PayloadInfo::decode(&unescaped_header[..])
        .map_err(|e: DecodeError| Error::Decode(e.to_string()))?;

    let year = payload_info.year;

    let mut meta_map = HashMap::new();
    meta_map.insert("name".to_string(), payload_info.pvname.clone());
    for header in &payload_info.headers {
        meta_map.insert(header.name.clone(), header.val.clone());
    }

    let meta = Meta(meta_map);

    let mut points = Vec::new();
    chunks.next(); // Skip the empty line after the header

    for chunk in chunks {
        if chunk.is_empty() { continue; }

        let unescaped = unescape_new_lines(chunk);
        if let Ok(point) = decode_point(&unescaped[..], payload_info.r#type, year) {
            points.push(point);
        }
    }

    Ok(vec![PVData { meta, data: points }])
}

fn decode_point(bytes: &[u8], type_id: i32, year: i32) -> Result<Point, Error> {
    match type_id {
        x if x == PayloadType::ScalarString as i32 => decode_scalar_string(bytes, year),
        x if x == PayloadType::ScalarFloat as i32 => decode_scalar_float(bytes, year),
        x if x == PayloadType::ScalarDouble as i32 => decode_scalar_double(bytes, year),
        x if x == PayloadType::ScalarInt as i32 => decode_scalar_int(bytes, year),
        x if x == PayloadType::ScalarShort as i32 => decode_scalar_short(bytes, year),
        x if x == PayloadType::ScalarByte as i32 => decode_scalar_byte(bytes, year),
        x if x == PayloadType::ScalarEnum as i32 => decode_scalar_enum(bytes, year),
        x if x == PayloadType::WaveformString as i32 => decode_vector_string(bytes, year),
        x if x == PayloadType::WaveformFloat as i32 => decode_vector_float(bytes, year),
        x if x == PayloadType::WaveformDouble as i32 => decode_vector_double(bytes, year),
        x if x == PayloadType::WaveformInt as i32 => decode_vector_int(bytes, year),
        x if x == PayloadType::WaveformShort as i32 => decode_vector_short(bytes, year),
        x if x == PayloadType::WaveformEnum as i32 => decode_vector_enum(bytes, year),
        x if x == PayloadType::V4GenericBytes as i32 => decode_v4_generic_bytes(bytes, year),
        _ => Err(Error::Invalid(format!("Unsupported type: {}", type_id))),
    }
}

macro_rules! define_decoder {
    ($name:ident, $struct_type:ty) => {
        pub fn $name(bytes: &[u8], year: i32) -> Result<Point, Error> {
            let p = <$struct_type>::decode(bytes).map_err(|e: DecodeError| Error::Decode(e.to_string()))?;
            let millis = convert_to_unix_ms(p.secondsintoyear, p.nano, year);

            Ok(Point {
                secs: millis / 1000,
                nanos: (millis % 1000) as i32 * 1_000_000,
                val: json!(p.val),
                severity: p.severity.unwrap_or(0),
                status: p.status.unwrap_or(0),
            })
        }
    };
}

define_decoder!(decode_scalar_string, ScalarString);
define_decoder!(decode_scalar_float, ScalarFloat);
define_decoder!(decode_scalar_double, ScalarDouble);
define_decoder!(decode_scalar_int, ScalarInt);
define_decoder!(decode_scalar_short, ScalarShort);
define_decoder!(decode_scalar_byte, ScalarByte);
define_decoder!(decode_scalar_enum, ScalarEnum);
define_decoder!(decode_vector_string, VectorString);
define_decoder!(decode_vector_float, VectorFloat);
define_decoder!(decode_vector_double, VectorDouble);
define_decoder!(decode_vector_int, VectorInt);
define_decoder!(decode_vector_short, VectorShort);
define_decoder!(decode_vector_enum, VectorEnum);
define_decoder!(decode_v4_generic_bytes, V4GenericBytes);
