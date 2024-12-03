use crate::constants::*;
use crate::epics::*;
use crate::types::*;
use chrono::NaiveDate;
use once_cell::sync::Lazy;
use prost::Message;
use std::collections::HashMap;
use std::time::Instant;

pub struct DecoderContext {
    points_capacity: usize,
    decode_buffer: Vec<u8>,
    current_point: Vec<u8>,
}

static YEAR_STARTS: Lazy<HashMap<i32, i64>> = Lazy::new(|| {
    let mut year_map = HashMap::new();
    for year in 2000..=2100 {
        let start_of_year = NaiveDate::from_ymd(year, 1, 1)
            .and_hms(0, 0, 0)
            .timestamp();
        year_map.insert(year, start_of_year);
    }
    year_map
});

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

    let start_time = Instant::now();

    let mut points = Vec::with_capacity(self.points_capacity);
    let mut current_point = Vec::with_capacity(128);

    let mut in_header = true;
    let mut in_escape = false;
    let mut payload_info = None;
    let mut meta = None;

    for &byte in raw_bytes {
        match (in_header, in_escape, byte) {
            (_, false, ESCAPE_CHAR) => {
                in_escape = true;
            }
            (_, true, escaped_char) => {
                current_point.push(match escaped_char {
                    ESCAPE_ESCAPE_CHAR => ESCAPE_CHAR,
                    NEWLINE_ESCAPE_CHAR => NEWLINE_CHAR,
                    CARRIAGERETURN_ESCAPE_CHAR => CARRIAGERETURN_CHAR,
                    other => other,
                });
                in_escape = false;
            }
            (true, false, NEWLINE_CHAR) => {
                let mut header_slice = &current_point[..];
                let info = PayloadInfo::decode(&mut header_slice)
                    .map_err(|e| Error::Decode(e.to_string()))?;
                meta = Some(self.process_metadata(&info)?);
                payload_info = Some(info);
                in_header = false;
                current_point.clear();
            }
            (false, false, NEWLINE_CHAR) => {
                if !current_point.is_empty() {
                    if let Some(ref info) = payload_info {
                        let mut point_buf = &current_point[..];
                        if let Ok(point) =
                            self.decode_point(&mut point_buf, info.r#type, info.year)
                        {
                            points.push(point);
                        }
                    }
                    current_point.clear();
                }
            }
            (_, false, b) => {
                current_point.push(b);
            }
        }
    }

    if !current_point.is_empty() && !in_header {
        if let Some(ref info) = payload_info {
            let mut point_buf = &current_point[..];
            if let Ok(point) = self.decode_point(&mut point_buf, info.r#type, info.year) {
                points.push(point);
            }
        }
    }

    let elapsed = start_time.elapsed();

    Ok(vec![PVData {
        meta: meta.ok_or_else(|| Error::Decode("Missing metadata".to_string()))?,
        data: points,
    }])
}
    #[inline(always)]
    fn process_metadata(&self, payload_info: &PayloadInfo) -> Result<Meta, Error> {
        let mut meta = Meta {
            name: payload_info.pvname.as_str().to_string(),
            ..Default::default()
        };

        for header in &payload_info.headers {
            match header.name.as_str() {
                "DRVH" => meta.DRVH = Some(header.val.as_str().to_string()),
                "EGU" => meta.EGU = Some(header.val.as_str().to_string()),
                "HIGH" => meta.HIGH = Some(header.val.as_str().to_string()),
                "HIHI" => meta.HIHI = Some(header.val.as_str().to_string()),
                "DRVL" => meta.DRVL = Some(header.val.as_str().to_string()),
                "PREC" => meta.PREC = Some(header.val.as_str().to_string()),
                "LOW" => meta.LOW = Some(header.val.as_str().to_string()),
                "LOLO" => meta.LOLO = Some(header.val.as_str().to_string()),
                "LOPR" => meta.LOPR = Some(header.val.as_str().to_string()),
                "HOPR" => meta.HOPR = Some(header.val.as_str().to_string()),
                "NELM" => meta.NELM = Some(header.val.as_str().to_string()),
                "DESC" => meta.DESC = Some(header.val.as_str().to_string()),
                _ => {}
            }
        }

        if meta.name.is_empty() {
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
        let start_of_year = *YEAR_STARTS
            .get(&year)
            .expect("Year out of range in YEAR_STARTS");

        let total_seconds = start_of_year + seconds_into_year as i64;
        total_seconds * 1000 + (nanos / 1_000_000) as i64
    }

    #[inline(always)]
    fn decode_scalar_float(&self, bytes: &mut &[u8], year: i32) -> Result<Point, Error> {
        let p = ScalarFloat::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
        let millis = self.convert_to_unix_ms(p.secondsintoyear, p.nano, year);

        Ok(Point {
            secs: millis / 1000,
            nanos: (millis % 1000 * 1_000_000) as i32,
            val: PointValue::Float(p.val),
            severity: p.severity.unwrap_or_default(),
            status: p.status.unwrap_or_default(),
        })
    }

    #[inline(always)]
    fn decode_scalar_double(&self, bytes: &mut &[u8], year: i32) -> Result<Point, Error> {
        let p = ScalarDouble::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
        let millis = self.convert_to_unix_ms(p.secondsintoyear, p.nano, year);

        Ok(Point {
            secs: millis / 1000,
            nanos: (millis % 1000 * 1_000_000) as i32,
            val: PointValue::Double(p.val),
            severity: p.severity.unwrap_or_default(),
            status: p.status.unwrap_or_default(),
        })
    }

    #[inline(always)]
    fn decode_scalar_int(&self, bytes: &mut &[u8], year: i32) -> Result<Point, Error> {
        let p = ScalarInt::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
        let millis = self.convert_to_unix_ms(p.secondsintoyear, p.nano, year);

        Ok(Point {
            secs: millis / 1000,
            nanos: (millis % 1000 * 1_000_000) as i32,
            val: PointValue::Int(p.val),
            severity: p.severity.unwrap_or_default(),
            status: p.status.unwrap_or_default(),
        })
    }

    #[inline(always)]
    fn decode_scalar_short(&self, bytes: &mut &[u8], year: i32) -> Result<Point, Error> {
        let p = ScalarShort::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
        let millis = self.convert_to_unix_ms(p.secondsintoyear, p.nano, year);

        Ok(Point {
            secs: millis / 1000,
            nanos: (millis % 1000 * 1_000_000) as i32,
            val: PointValue::Short(p.val as i16),
            severity: p.severity.unwrap_or_default(),
            status: p.status.unwrap_or_default(),
        })
    }

    #[inline(always)]
    fn decode_scalar_byte(&self, bytes: &mut &[u8], year: i32) -> Result<Point, Error> {
        let p = ScalarByte::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;

        let val = p.val.get(0).copied().ok_or_else(|| {
            Error::Decode("Expected a single byte in ScalarByte value".to_string())
        })?;

        let millis = self.convert_to_unix_ms(p.secondsintoyear, p.nano, year);

        Ok(Point {
            secs: millis / 1000,
            nanos: (millis % 1000 * 1_000_000) as i32,
            val: PointValue::Byte(val),
            severity: p.severity.unwrap_or_default(),
            status: p.status.unwrap_or_default(),
        })
    }

    #[inline(always)]
    fn decode_scalar_enum(&self, bytes: &mut &[u8], year: i32) -> Result<Point, Error> {
        let p = ScalarEnum::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
        let millis = self.convert_to_unix_ms(p.secondsintoyear, p.nano, year);

        Ok(Point {
            secs: millis / 1000,
            nanos: (millis % 1000 * 1_000_000) as i32,
            val: PointValue::Enum(p.val),
            severity: p.severity.unwrap_or_default(),
            status: p.status.unwrap_or_default(),
        })
    }

    #[inline(always)]
    fn decode_scalar_string(&self, bytes: &mut &[u8], year: i32) -> Result<Point, Error> {
        let p = ScalarString::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
        let millis = self.convert_to_unix_ms(p.secondsintoyear, p.nano, year);

        Ok(Point {
            secs: millis / 1000,
            nanos: (millis % 1000 * 1_000_000) as i32,
            val: PointValue::String(p.val),
            severity: p.severity.unwrap_or_default(),
            status: p.status.unwrap_or_default(),
        })
    }
}
