use crate::constants::*;
use crate::epics::*;
use crate::types::*;
use bytes::Bytes;
use chrono::NaiveDate;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use prost::Message;
use std::collections::HashMap;

// Cache for year start timestamps
static YEAR_STARTS: Lazy<DashMap<i32, i64>> = Lazy::new(DashMap::new);

pub struct DecoderContext {
    points_capacity: usize,
}

impl DecoderContext {
    pub fn new(points_capacity: usize) -> Self {
        Self { points_capacity }
    }

    pub fn decode_response(&self, raw_bytes: &[u8]) -> Result<Vec<PVData>, Error> {
        if raw_bytes.is_empty() {
            return Err(Error::Decode("Empty response".to_string()));
        }
    
        // Split the response into lines
        let mut iter = raw_bytes.split(|&b| b == NEWLINE_CHAR);
    
        // Parse header
        let header = iter
            .next()
            .ok_or_else(|| Error::Decode("Missing header".to_string()))?;
        let mut header_buffer = Vec::with_capacity(header.len());
        self.unescape_new_lines(header, &mut header_buffer);
        let payload_info =
            PayloadInfo::decode(&header_buffer[..]).map_err(|e| Error::Decode(e.to_string()))?;
    
        let year = payload_info.year;
        let type_id = payload_info.r#type;
    
        // Prepare metadata
        let mut meta_map = HashMap::new();
        meta_map.insert("name".to_string(), Some(payload_info.pvname)); // Example: mandatory field
        for header in payload_info.headers {
            meta_map.insert(header.name, Some(header.val));
        }
    
        let meta = Meta {
            name: meta_map
                .remove("name")
                .flatten()
                .ok_or_else(|| Error::Decode("Missing 'name' in metadata".to_string()))?,
            DRVH: meta_map.remove("DRVH").flatten(),
            EGU: meta_map.remove("EGU").flatten(),
            HIGH: meta_map.remove("HIGH").flatten(),
            HIHI: meta_map.remove("HIHI").flatten(),
            DRVL: meta_map.remove("DRVL").flatten(),
            PREC: meta_map.remove("PREC").flatten(),
            LOW: meta_map.remove("LOW").flatten(),
            LOLO: meta_map.remove("LOLO").flatten(),
            LOPR: meta_map.remove("LOPR").flatten(),
            HOPR: meta_map.remove("HOPR").flatten(),
            NELM: meta_map.remove("NELM").flatten(),
            DESC: meta_map.remove("DESC").flatten(),
        };
    
        // Skip empty line
        iter.next();
    
        // Preallocate points vector
        let mut points = Vec::with_capacity(self.points_capacity);
    
        // Process points
        let mut point_buffer = Vec::with_capacity(1024);
        for chunk in iter {
            if chunk.is_empty() {
                continue;
            }
    
            point_buffer.clear();
            self.unescape_new_lines(chunk, &mut point_buffer);
            if let Ok(point) = self.decode_point(&point_buffer, type_id, year) {
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
            x if x == PayloadType::ScalarInt as i32 => self.decode_scalar_int(bytes, year),
            x if x == PayloadType::ScalarShort as i32 => self.decode_scalar_short(bytes, year),
            x if x == PayloadType::ScalarByte as i32 => self.decode_scalar_byte(bytes, year),
            x if x == PayloadType::ScalarEnum as i32 => self.decode_scalar_enum(bytes, year),
            // Handle other types if needed
            _ => Err(Error::Invalid(format!("Unsupported type: {}", type_id))),
        }
    }

    #[inline(always)]
    fn unescape_new_lines(&self, input: &[u8], output: &mut Vec<u8>) {
        let mut i = 0;
        while i < input.len() {
            let b = input[i];
            if b != ESCAPE_CHAR {
                output.push(b);
                i += 1;
            } else {
                i += 1;
                if i < input.len() {
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

    // Implement decoder functions for each scalar type
    #[inline(always)]
    fn decode_scalar_float(&self, bytes: &[u8], year: i32) -> Result<Point, Error> {
        let p = ScalarFloat::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
        let millis = self.convert_to_unix_ms(p.secondsintoyear, p.nano, year);

        Ok(Point {
            secs: millis / 1000,
            nanos: ((millis % 1000) * 1_000_000) as i32,
            val: PointValue::Float(p.val),
            severity: p.severity.unwrap_or(0),
            status: p.status.unwrap_or(0),
        })
    }

    #[inline(always)]
    fn decode_scalar_double(&self, bytes: &[u8], year: i32) -> Result<Point, Error> {
        let p = ScalarDouble::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
        let millis = self.convert_to_unix_ms(p.secondsintoyear, p.nano, year);

        Ok(Point {
            secs: millis / 1000,
            nanos: ((millis % 1000) * 1_000_000) as i32,
            val: PointValue::Double(p.val),
            severity: p.severity.unwrap_or(0),
            status: p.status.unwrap_or(0),
        })
    }

    #[inline(always)]
    fn decode_scalar_int(&self, bytes: &[u8], year: i32) -> Result<Point, Error> {
        let p = ScalarInt::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
        let millis = self.convert_to_unix_ms(p.secondsintoyear, p.nano, year);

        Ok(Point {
            secs: millis / 1000,
            nanos: ((millis % 1000) * 1_000_000) as i32,
            val: PointValue::Int(p.val),
            severity: p.severity.unwrap_or(0),
            status: p.status.unwrap_or(0),
        })
    }

    #[inline(always)]
    fn decode_scalar_short(&self, bytes: &[u8], year: i32) -> Result<Point, Error> {
        let p = ScalarShort::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
        let val: i16 = p
            .val
            .try_into()
            .map_err(|_| Error::Decode("Value out of range for i16".to_string()))?;
        let millis = self.convert_to_unix_ms(p.secondsintoyear, p.nano, year);

        Ok(Point {
            secs: millis / 1000,
            nanos: ((millis % 1000) * 1_000_000) as i32,
            val: PointValue::Short(val),
            severity: p.severity.unwrap_or(0),
            status: p.status.unwrap_or(0),
        })
    }

    #[inline(always)]
fn decode_scalar_byte(&self, bytes: &[u8], year: i32) -> Result<Point, Error> {
    let p = ScalarByte::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
    
    // Ensure that p.val contains exactly one byte
    let val = if p.val.len() == 1 {
        p.val[0]
    } else {
        return Err(Error::Decode("Expected a single byte in ScalarByte value".to_string()));
    };

    let millis = self.convert_to_unix_ms(p.secondsintoyear, p.nano, year);

    Ok(Point {
        secs: millis / 1000,
        nanos: ((millis % 1000) * 1_000_000) as i32,
        val: PointValue::Byte(val),
        severity: p.severity.unwrap_or(0),
        status: p.status.unwrap_or(0),
    })
}


    #[inline(always)]
    fn decode_scalar_enum(&self, bytes: &[u8], year: i32) -> Result<Point, Error> {
        let p = ScalarEnum::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
        let val = p.val;
        let millis = self.convert_to_unix_ms(p.secondsintoyear, p.nano, year);

        Ok(Point {
            secs: millis / 1000,
            nanos: ((millis % 1000) * 1_000_000) as i32,
            val: PointValue::Enum(val),
            severity: p.severity.unwrap_or(0),
            status: p.status.unwrap_or(0),
        })
    }

    #[inline(always)]
    fn decode_scalar_string(&self, bytes: &[u8], year: i32) -> Result<Point, Error> {
        let p = ScalarString::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
        let millis = self.convert_to_unix_ms(p.secondsintoyear, p.nano, year);

        Ok(Point {
            secs: millis / 1000,
            nanos: ((millis % 1000) * 1_000_000) as i32,
            val: PointValue::String(p.val),
            severity: p.severity.unwrap_or(0),
            status: p.status.unwrap_or(0),
        })
    }
}
