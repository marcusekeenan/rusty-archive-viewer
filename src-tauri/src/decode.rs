use bytes::Bytes;
use prost::Message;
use serde_json::json;
use std::collections::HashMap;
use crate::constants::*;
use crate::types::*;
use crate::epics::*;
use crate::decode_helpers::unescape_new_lines;
use prost::DecodeError;

pub fn decode_response(raw_bytes: Bytes) -> Result<Vec<PVData>, Error> {
    let mut chunks = raw_bytes.split(|&b| b == NEWLINE_CHAR);
    let header = chunks.next().ok_or_else(|| Error::Decode("Empty response".to_string()))?;
    let unescaped_header = unescape_new_lines(header);
    let payload_info = PayloadInfo::decode(&unescaped_header[..])
        .map_err(|e: DecodeError| Error::Decode(e.to_string()))?;

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
        if let Ok(point) = decode_point(&unescaped[..], payload_info.r#type) {
            points.push(point);
        }
    }

    let json_response = json!([{
        "meta": json!(meta),
        "data": points.iter().map(|p| json!({
            "secs": p.secs,
            "nanos": p.nanos,
            "val": p.val,
            "severity": p.severity,
            "status": p.status
        })).collect::<Vec<_>>()
    }]);

    println!("JSON Response: {}", 
        serde_json::to_string_pretty(&json_response).unwrap()
    );

    Ok(vec![PVData { meta, data: points }])
}

fn decode_point(bytes: &[u8], type_id: i32) -> Result<Point, Error> {
    match type_id {
        x if x == PayloadType::ScalarString as i32 => decode_scalar_string(bytes),
        x if x == PayloadType::ScalarFloat as i32 => decode_scalar_float(bytes),
        x if x == PayloadType::ScalarDouble as i32 => decode_scalar_double(bytes),
        x if x == PayloadType::ScalarInt as i32 => decode_scalar_int(bytes),
        x if x == PayloadType::ScalarShort as i32 => decode_scalar_short(bytes),
        x if x == PayloadType::ScalarByte as i32 => decode_scalar_byte(bytes),
        x if x == PayloadType::ScalarEnum as i32 => decode_scalar_enum(bytes),
        x if x == PayloadType::WaveformString as i32 => decode_vector_string(bytes),
        x if x == PayloadType::WaveformFloat as i32 => decode_vector_float(bytes),
        x if x == PayloadType::WaveformDouble as i32 => decode_vector_double(bytes),
        x if x == PayloadType::WaveformInt as i32 => decode_vector_int(bytes),
        x if x == PayloadType::WaveformShort as i32 => decode_vector_short(bytes),
        x if x == PayloadType::WaveformEnum as i32 => decode_vector_enum(bytes),
        x if x == PayloadType::V4GenericBytes as i32 => decode_v4_generic_bytes(bytes),
        _ => Err(Error::Invalid(format!("Unsupported type: {}", type_id))),
    }
}

pub fn decode_scalar_string(bytes: &[u8]) -> Result<Point, Error> {
    let p = ScalarString::decode(bytes).map_err(|e: DecodeError| Error::Decode(e.to_string()))?;
    Ok(Point {
        secs: p.secondsintoyear as i64,
        nanos: p.nano as i32,
        val: json!(p.val),
        severity: p.severity.unwrap_or(0),
        status: p.status.unwrap_or(0),
    })
}

pub fn decode_scalar_float(bytes: &[u8]) -> Result<Point, Error> {
    let p = ScalarFloat::decode(bytes).map_err(|e: DecodeError| Error::Decode(e.to_string()))?;
    Ok(Point {
        secs: p.secondsintoyear as i64,
        nanos: p.nano as i32,
        val: json!(p.val),
        severity: p.severity.unwrap_or(0),
        status: p.status.unwrap_or(0),
    })
}

pub fn decode_scalar_double(bytes: &[u8]) -> Result<Point, Error> {
    let p = ScalarDouble::decode(bytes).map_err(|e: DecodeError| Error::Decode(e.to_string()))?;
    Ok(Point {
        secs: p.secondsintoyear as i64,
        nanos: p.nano as i32,
        val: json!(p.val),
        severity: p.severity.unwrap_or(0),
        status: p.status.unwrap_or(0),
    })
}

pub fn decode_scalar_int(bytes: &[u8]) -> Result<Point, Error> {
    let p = ScalarInt::decode(bytes).map_err(|e: DecodeError| Error::Decode(e.to_string()))?;
    Ok(Point {
        secs: p.secondsintoyear as i64,
        nanos: p.nano as i32,
        val: json!(p.val),
        severity: p.severity.unwrap_or(0),
        status: p.status.unwrap_or(0),
    })
}

pub fn decode_scalar_short(bytes: &[u8]) -> Result<Point, Error> {
    let p = ScalarShort::decode(bytes).map_err(|e: DecodeError| Error::Decode(e.to_string()))?;
    Ok(Point {
        secs: p.secondsintoyear as i64,
        nanos: p.nano as i32,
        val: json!(p.val),
        severity: p.severity.unwrap_or(0),
        status: p.status.unwrap_or(0),
    })
}

pub fn decode_scalar_byte(bytes: &[u8]) -> Result<Point, Error> {
    let p = ScalarByte::decode(bytes).map_err(|e: DecodeError| Error::Decode(e.to_string()))?;
    Ok(Point {
        secs: p.secondsintoyear as i64,
        nanos: p.nano as i32,
        val: json!(p.val),
        severity: p.severity.unwrap_or(0),
        status: p.status.unwrap_or(0),
    })
}

pub fn decode_scalar_enum(bytes: &[u8]) -> Result<Point, Error> {
    let p = ScalarEnum::decode(bytes).map_err(|e: DecodeError| Error::Decode(e.to_string()))?;
    Ok(Point {
        secs: p.secondsintoyear as i64,
        nanos: p.nano as i32,
        val: json!(p.val),
        severity: p.severity.unwrap_or(0),
        status: p.status.unwrap_or(0),
    })
}

pub fn decode_vector_string(bytes: &[u8]) -> Result<Point, Error> {
    let p = VectorString::decode(bytes).map_err(|e: DecodeError| Error::Decode(e.to_string()))?;
    Ok(Point {
        secs: p.secondsintoyear as i64,
        nanos: p.nano as i32,
        val: json!(p.val),
        severity: p.severity.unwrap_or(0),
        status: p.status.unwrap_or(0),
    })
}

pub fn decode_vector_float(bytes: &[u8]) -> Result<Point, Error> {
    let p = VectorFloat::decode(bytes).map_err(|e: DecodeError| Error::Decode(e.to_string()))?;
    Ok(Point {
        secs: p.secondsintoyear as i64,
        nanos: p.nano as i32,
        val: json!(p.val),
        severity: p.severity.unwrap_or(0),
        status: p.status.unwrap_or(0),
    })
}

pub fn decode_vector_double(bytes: &[u8]) -> Result<Point, Error> {
    let p = VectorDouble::decode(bytes).map_err(|e: DecodeError| Error::Decode(e.to_string()))?;
    Ok(Point {
        secs: p.secondsintoyear as i64,
        nanos: p.nano as i32,
        val: json!(p.val),
        severity: p.severity.unwrap_or(0),
        status: p.status.unwrap_or(0),
    })
}

pub fn decode_vector_int(bytes: &[u8]) -> Result<Point, Error> {
    let p = VectorInt::decode(bytes).map_err(|e: DecodeError| Error::Decode(e.to_string()))?;
    Ok(Point {
        secs: p.secondsintoyear as i64,
        nanos: p.nano as i32,
        val: json!(p.val),
        severity: p.severity.unwrap_or(0),
        status: p.status.unwrap_or(0),
    })
}

pub fn decode_vector_short(bytes: &[u8]) -> Result<Point, Error> {
    let p = VectorShort::decode(bytes).map_err(|e: DecodeError| Error::Decode(e.to_string()))?;
    Ok(Point {
        secs: p.secondsintoyear as i64,
        nanos: p.nano as i32,
        val: json!(p.val),
        severity: p.severity.unwrap_or(0),
        status: p.status.unwrap_or(0),
    })
}

pub fn decode_vector_enum(bytes: &[u8]) -> Result<Point, Error> {
    let p = VectorEnum::decode(bytes).map_err(|e: DecodeError| Error::Decode(e.to_string()))?;
    Ok(Point {
        secs: p.secondsintoyear as i64,
        nanos: p.nano as i32,
        val: json!(p.val),
        severity: p.severity.unwrap_or(0),
        status: p.status.unwrap_or(0),
    })
}

pub fn decode_v4_generic_bytes(bytes: &[u8]) -> Result<Point, Error> {
    let p = V4GenericBytes::decode(bytes).map_err(|e: DecodeError| Error::Decode(e.to_string()))?;
    Ok(Point {
        secs: p.secondsintoyear as i64,
        nanos: p.nano as i32,
        val: json!(p.val),
        severity: p.severity.unwrap_or(0),
        status: p.status.unwrap_or(0),
    })
}