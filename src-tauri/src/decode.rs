use crate::constants::*;
use crate::epics::*;
use crate::types::*;
// use bytes::{Buf, Bytes};
use chrono::NaiveDate;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use prost::Message;
use std::time::Instant;

static YEAR_STARTS: Lazy<DashMap<i32, i64>> = Lazy::new(DashMap::new);

pub struct DecoderContext {
    points_capacity: usize,
    decode_buffer: Vec<u8>,
    current_point: Vec<u8>,
}

impl DecoderContext {
    pub fn new(points_capacity: usize) -> Self {
        Self { 
            points_capacity,
            decode_buffer: Vec::with_capacity(1024),
            current_point: Vec::with_capacity(128),
        }
    }

    pub fn decode_response(&mut self, raw_bytes: &[u8]) -> Result<Vec<PVData>, Error> {
        if raw_bytes.is_empty() {
            return Err(Error::Decode("Empty response".to_string()));
        }

        let start_time = Instant::now(); // Start timing the decode process

        let mut points = Vec::with_capacity(self.points_capacity);
        self.decode_buffer.clear();
        self.current_point.clear();

        // State tracking
        let mut in_header = true;
        let mut in_escape = false;
        let mut payload_info = None;
        let mut meta = None;

        for &byte in raw_bytes {
            match (in_header, in_escape, byte) {
                // Handle escape sequences
                (_, false, ESCAPE_CHAR) => {
                    in_escape = true;
                    continue;
                },
                (_, true, ESCAPE_ESCAPE_CHAR) => {
                    self.current_point.push(ESCAPE_CHAR);
                    in_escape = false;
                },
                (_, true, NEWLINE_ESCAPE_CHAR) => {
                    self.current_point.push(NEWLINE_CHAR);
                    in_escape = false;
                },
                (_, true, CARRIAGERETURN_ESCAPE_CHAR) => {
                    self.current_point.push(CARRIAGERETURN_CHAR);
                    in_escape = false;
                },
                (_, true, b) => {
                    self.current_point.push(b);
                    in_escape = false;
                },

                // Handle header completion
                (true, false, NEWLINE_CHAR) => {
                    let mut header_slice = self.current_point.as_slice();
                    let info = PayloadInfo::decode(&mut header_slice)
                        .map_err(|e| Error::Decode(e.to_string()))?;
                    meta = Some(self.process_metadata(&info)?);
                    payload_info = Some(info);
                    in_header = false;
                    self.current_point.clear();
                },

                // Handle point completion
                (false, false, NEWLINE_CHAR) => {
                    if !self.current_point.is_empty() {
                        if let Some(ref info) = payload_info {
                            let mut point_buf = self.current_point.as_slice();
                            if let Ok(point) = self.decode_point(
                                &mut point_buf, 
                                info.r#type, 
                                info.year
                            ) {
                                points.push(point);
                            }
                        }
                        self.current_point.clear();
                    }
                },

                // Normal byte collection
                (_, false, b) => {
                    self.current_point.push(b);
                },
            }
        }

        if !self.current_point.is_empty() && !in_header {
            if let Some(ref info) = payload_info {
                let mut point_buf = self.current_point.as_slice();
                if let Ok(point) = self.decode_point(
                    &mut point_buf, 
                    info.r#type, 
                    info.year
                ) {
                    points.push(point);
                }
            }
        }

        let elapsed = start_time.elapsed(); // Measure elapsed time
        //  println!("Decoding completed in {:?}", elapsed);

        Ok(vec![PVData { 
            meta: meta.ok_or_else(|| Error::Decode("Missing metadata".to_string()))?,
            data: points
        }])
    }
    
    #[inline(always)]
    fn process_metadata(&self, payload_info: &PayloadInfo) -> Result<Meta, Error> {
        let mut meta_vec: Vec<(String, Option<String>)> = payload_info.headers.iter()
            .map(|h| (h.name.clone(), Some(h.val.clone())))
            .collect();

        meta_vec.push(("name".to_string(), Some(payload_info.pvname.clone())));

        let mut meta = Meta {
            name: String::new(),
            DRVH: None,
            EGU: None,
            HIGH: None,
            HIHI: None,
            DRVL: None,
            PREC: None,
            LOW: None,
            LOLO: None,
            LOPR: None,
            HOPR: None,
            NELM: None,
            DESC: None,
        };

        for (key, value) in meta_vec {
            match key.as_str() {
                "name" => meta.name = value.unwrap_or(String::new()),
                "DRVH" => meta.DRVH = value,
                "EGU" => meta.EGU = value,
                "HIGH" => meta.HIGH = value,
                "HIHI" => meta.HIHI = value,
                "DRVL" => meta.DRVL = value,
                "PREC" => meta.PREC = value,
                "LOW" => meta.LOW = value,
                "LOLO" => meta.LOLO = value,
                "LOPR" => meta.LOPR = value,
                "HOPR" => meta.HOPR = value,
                "NELM" => meta.NELM = value,
                "DESC" => meta.DESC = value,
                _ => {},
            }
        }

        if meta.name.is_empty() || meta.name == "" {
            return Err(Error::Decode("Missing 'name' in metadata".to_string()));
        }

        Ok(meta)
    }


    #[inline(always)]
    fn decode_point(&self, bytes: &mut &[u8], type_id: i32, year: i32) -> Result<Point, Error> {
        match type_id {
            x if x == PayloadType::ScalarString as i32 => self.decode_scalar_string(bytes, year),
            x if x == PayloadType::ScalarFloat as i32 => self.decode_scalar_float(bytes, year),
            x if x == PayloadType::ScalarDouble as i32 => self.decode_scalar_double(bytes, year),
            x if x == PayloadType::ScalarInt as i32 => self.decode_scalar_int(bytes, year),
            x if x == PayloadType::ScalarShort as i32 => self.decode_scalar_short(bytes, year),
            x if x == PayloadType::ScalarByte as i32 => self.decode_scalar_byte(bytes, year),
            x if x == PayloadType::ScalarEnum as i32 => self.decode_scalar_enum(bytes, year),
            _ => Err(Error::Invalid(format!("Unsupported type: {}", type_id))),
        }
    }

    #[inline(always)]
    fn convert_to_unix_ms(&self, seconds_into_year: u32, nanos: u32, year: i32) -> i64 {
        let start_of_year = *YEAR_STARTS.entry(year).or_insert_with(|| {
            NaiveDate::from_ymd_opt(year, 1, 1)
                .expect("Invalid year")
                .and_hms_opt(0, 0, 0)
                .expect("Invalid time")
                .and_utc()
                .timestamp()
        });

        let total_seconds = start_of_year + seconds_into_year as i64;
        total_seconds * 1000 + (nanos / 1_000_000) as i64
    }

    #[inline(always)]
    fn decode_scalar_float(&self, bytes: &mut &[u8], year: i32) -> Result<Point, Error> {
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
    fn decode_scalar_double(&self, bytes: &mut &[u8], year: i32) -> Result<Point, Error> {
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
    fn decode_scalar_int(&self, bytes: &mut &[u8], year: i32) -> Result<Point, Error> {
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
    fn decode_scalar_short(&self, bytes: &mut &[u8], year: i32) -> Result<Point, Error> {
        let p = ScalarShort::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
        let val: i16 = p.val
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
    fn decode_scalar_byte(&self, bytes: &mut &[u8], year: i32) -> Result<Point, Error> {
        let p = ScalarByte::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
        
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
    fn decode_scalar_enum(&self, bytes: &mut &[u8], year: i32) -> Result<Point, Error> {
        let p = ScalarEnum::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
        let millis = self.convert_to_unix_ms(p.secondsintoyear, p.nano, year);

        Ok(Point {
            secs: millis / 1000,
            nanos: ((millis % 1000) * 1_000_000) as i32,
            val: PointValue::Enum(p.val),
            severity: p.severity.unwrap_or(0),
            status: p.status.unwrap_or(0),
        })
    }

    #[inline(always)]
    fn decode_scalar_string(&self, bytes: &mut &[u8], year: i32) -> Result<Point, Error> {
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