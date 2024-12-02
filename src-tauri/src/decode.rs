use bytes::Bytes;
use prost::Message;
use serde_json::json;
use std::collections::HashMap;
use chrono::NaiveDate;
use once_cell::sync::Lazy;
use dashmap::DashMap;
use crate::constants::*;
use crate::types::*;
use crate::epics::*;

// Cache for year start timestamps
static YEAR_STARTS: Lazy<DashMap<i32, i64>> = Lazy::new(DashMap::new);

pub struct DecoderContext {
    points_capacity: usize,
}

impl DecoderContext {
    pub fn new(points_capacity: usize) -> Self {
        Self {
            points_capacity,
        }
    }

    pub fn decode_response(&self, raw_bytes: Bytes) -> Result<Vec<PVData>, Error> {
        if raw_bytes.len() < 1024 {
            return self.decode_response_fast(raw_bytes);
        }

        let mut iter = raw_bytes.split(|&b| b == NEWLINE_CHAR);
        
        // Parse header
        let header = iter.next()
            .ok_or_else(|| Error::Decode("Empty response".to_string()))?;
        let mut header_buffer = Vec::with_capacity(header.len());
        self.unescape_new_lines_optimized(header, &mut header_buffer);
        let payload_info = PayloadInfo::decode(&header_buffer[..])
            .map_err(|e| Error::Decode(e.to_string()))?;

        let year = payload_info.year;
        let type_id = payload_info.r#type;

        // Prepare metadata with capacity
        let mut meta_map = HashMap::with_capacity(payload_info.headers.len() + 1);
        meta_map.insert("name".to_string(), payload_info.pvname);
        for header in payload_info.headers {
            meta_map.insert(header.name, header.val);
        }

        let meta = Meta(meta_map);

        // Skip empty line and preallocate points vector
        iter.next();
        let mut points = Vec::with_capacity(self.points_capacity);

        // Process points
        let mut point_buffer = Vec::with_capacity(1024);
        for chunk in iter {
            if chunk.is_empty() { continue; }
            
            point_buffer.clear();
            self.unescape_new_lines_optimized(chunk, &mut point_buffer);
            if let Ok(point) = self.decode_point(&point_buffer, type_id, year) {
                points.push(point);
            }
        }

        Ok(vec![PVData { meta, data: points }])
    }
    #[inline(always)]
    fn decode_response_fast(&self, raw_bytes: Bytes) -> Result<Vec<PVData>, Error> {
        let mut iter = raw_bytes.split(|&b| b == NEWLINE_CHAR);
        
        // Parse header
        let header = iter.next()
            .ok_or_else(|| Error::Decode("Empty response".to_string()))?;
        let payload_info = PayloadInfo::decode(header)
            .map_err(|e| Error::Decode(e.to_string()))?;

        let year = payload_info.year;

        // Prepare metadata
        let mut meta_map = HashMap::with_capacity(payload_info.headers.len() + 1);
        meta_map.insert("name".to_string(), payload_info.pvname);
        for header in payload_info.headers {
            meta_map.insert(header.name, header.val);
        }

        let meta = Meta(meta_map);

        // Skip empty line
        iter.next();

        // Process points
        let mut points = Vec::with_capacity(16);  // Assume small number of points for fast path

        for chunk in iter {
            if chunk.is_empty() { continue; }
            
            if let Ok(point) = self.decode_point(chunk, payload_info.r#type, year) {
                points.push(point);
            }
        }

        Ok(vec![PVData { meta, data: points }])
    }

    #[inline(always)]
    fn decode_point(&self, bytes: &[u8], type_id: i32, year: i32) -> Result<Point, Error> {
        match type_id {
            x if x == PayloadType::ScalarString as i32 => self.decode_scalar_string(bytes, year),
            x if x == PayloadType::ScalarFloat as i32 => self.decode_scalar_float(bytes, year),
            x if x == PayloadType::ScalarDouble as i32 => self.decode_scalar_double(bytes, year),
            // ... other type matches
            _ => Err(Error::Invalid(format!("Unsupported type: {}", type_id))),
        }
    }

    #[inline(always)]
    fn unescape_new_lines_optimized(&self, input: &[u8], output: &mut Vec<u8>) {
        output.reserve(input.len());
        let mut i = 0;
        while i < input.len() {
            let b = input[i];
            if b != ESCAPE_CHAR {
                output.push(b);
                i += 1;
                continue;
            }
            
            if i + 1 < input.len() {
                i += 1;
                match input[i] {
                    ESCAPE_ESCAPE_CHAR => output.push(ESCAPE_CHAR),
                    NEWLINE_ESCAPE_CHAR => output.push(NEWLINE_CHAR),
                    CARRIAGERETURN_ESCAPE_CHAR => output.push(CARRIAGERETURN_CHAR),
                    b => output.push(b),
                }
            }
            i += 1;
        }
    }

    #[inline(always)]
    fn convert_to_unix_ms(&self, seconds_into_year: u32, nanos: u32, year: i32) -> i64 {
        let start_of_year = *YEAR_STARTS.entry(year).or_insert_with(|| {
            NaiveDate::from_ymd_opt(year, 1, 1)
                .expect("Invalid year")
                .and_hms_opt(0, 0, 0)
                .expect("Invalid time")
                .timestamp()
        });

        let total_seconds = start_of_year + seconds_into_year as i64;
        total_seconds * 1000 + (nanos / 1_000_000) as i64
    }
}

// Decoder implementations using the context
macro_rules! define_decoder {
    ($name:ident, $struct_type:ty) => {
        impl DecoderContext {
            #[inline(always)]
            fn $name(&self, bytes: &[u8], year: i32) -> Result<Point, Error> {
                let p = <$struct_type>::decode(bytes)
                    .map_err(|e| Error::Decode(e.to_string()))?;
                let millis = self.convert_to_unix_ms(p.secondsintoyear, p.nano, year);

                Ok(Point {
                    secs: millis / 1000,
                    nanos: (millis % 1000) as i32 * 1_000_000,
                    val: json!(p.val),
                    severity: p.severity.unwrap_or(0),
                    status: p.status.unwrap_or(0),
                })
            }
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