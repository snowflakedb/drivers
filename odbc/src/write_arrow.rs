use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::{CStr, c_char},
    marker::PhantomData,
    mem, slice, str,
    sync::Arc,
};

thread_local! {
    static SESSION_TIMEZONE: RefCell<Option<String>> = RefCell::new(None);
    static TIMESTAMP_TYPE_MAPPING: RefCell<TimestampType> =
        RefCell::new(TimestampType::Ltz);
}

pub fn set_session_timezone(tz: Option<String>) {
    SESSION_TIMEZONE.with(|cell| {
        *cell.borrow_mut() = tz.map(|name| crate::timezone::normalize_timezone_name(&name));
    });
}

fn get_session_timezone() -> Option<String> {
    SESSION_TIMEZONE.with(|cell| cell.borrow().clone())
}

pub fn set_timestamp_type_mapping(mapping: TimestampType) {
    TIMESTAMP_TYPE_MAPPING.with(|cell| {
        *cell.borrow_mut() = mapping;
    });
}

fn current_timestamp_type_mapping() -> TimestampType {
    TIMESTAMP_TYPE_MAPPING.with(|cell| *cell.borrow())
}

fn logical_type_from_sql_type(parameter_type: &sql::SqlDataType) -> Option<&'static str> {
    match parameter_type.0 {
        91 => Some("DATE"),
        92 => Some("TIME"),
        2000 => Some("TIMESTAMP_LTZ"),
        2001 => Some("TIMESTAMP_NTZ"),
        2002 => Some("TIMESTAMP_TZ"),
        93 => Some(match current_timestamp_type_mapping() {
            TimestampType::Ltz => "TIMESTAMP_LTZ",
            TimestampType::Ntz => "TIMESTAMP_NTZ",
            TimestampType::Tz => "TIMESTAMP_TZ",
        }),
        _ => None,
    }
}

fn timestamp_type_from_sql(parameter_type: &sql::SqlDataType) -> TimestampType {
    match parameter_type.0 {
        2000 => TimestampType::Ltz,
        2001 => TimestampType::Ntz,
        2002 => TimestampType::Tz,
        93 => current_timestamp_type_mapping(),
        _ => current_timestamp_type_mapping(),
    }
}

use arrow::{
    array::{
        Array, Date32Array, Float64Array, Int8Array, Int32Array, Int64Array, StringArray,
        StructArray, Time64NanosecondArray, TimestampNanosecondArray,
    },
    datatypes::{
        DataType, Field, Float64Type, Int32Type, TimeUnit, TimestampNanosecondType, Utf8Type,
    },
    ffi::{FFI_ArrowArray, FFI_ArrowSchema},
};

use crate::{
    api::{ParameterBinding, types::TimestampType},
    cdata_types::CDataType,
};
use chrono::{DateTime, Datelike, NaiveDateTime, Timelike};
use chrono_tz;
use odbc_sys as sql;
use odbc_sys::NULL_DATA;
use time;

/// Convert UTF-32 code points to a String
fn utf32_to_string(utf32_chars: &[u32]) -> Option<String> {
    utf32_chars
        .iter()
        .map(|&c| char::from_u32(c))
        .collect::<Option<String>>()
}

fn length_from_binding(binding: &ParameterBinding) -> Option<usize> {
    if !binding.str_len_or_ind_ptr.is_null() {
        let len = unsafe { *binding.str_len_or_ind_ptr };
        if len == sql::NTS || len == sql::NTSL || len < 0 {
            None
        } else {
            Some(len as usize)
        }
    } else if binding.buffer_length > 0 && binding.buffer_length != sql::NTS {
        Some(binding.buffer_length as usize)
    } else {
        None
    }
}

unsafe fn read_nts_len_u16(ptr: *const u16) -> usize {
    let mut len = 0usize;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
    }
    len
}

unsafe fn read_nts_len_u32(ptr: *const u32) -> usize {
    let mut len = 0usize;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
    }
    len
}

fn utf16_chars_from_binding(binding: &ParameterBinding) -> Result<Vec<u16>, ArrowBindingError> {
    let ptr = binding.parameter_value_ptr as *const u16;
    if ptr.is_null() {
        return Err(ArrowBindingError::InvalidParameterIndices);
    }

    let unit = mem::size_of::<u16>();
    let char_count = match length_from_binding(binding) {
        Some(len_bytes) if len_bytes >= unit => len_bytes / unit,
        Some(_) => 0,
        None => unsafe { read_nts_len_u16(ptr) },
    };

    let slice = unsafe { slice::from_raw_parts(ptr, char_count) };
    let mut chars = slice.to_vec();
    while matches!(chars.last(), Some(&0)) {
        chars.pop();
    }
    Ok(chars)
}

fn utf32_chars_from_binding(binding: &ParameterBinding) -> Result<Vec<u32>, ArrowBindingError> {
    let ptr = binding.parameter_value_ptr as *const u32;
    if ptr.is_null() {
        return Err(ArrowBindingError::InvalidParameterIndices);
    }

    let unit = mem::size_of::<u32>();
    let char_count = match length_from_binding(binding) {
        Some(len_bytes) if len_bytes >= unit => len_bytes / unit,
        Some(_) => 0,
        None => unsafe { read_nts_len_u32(ptr) },
    };

    let slice = unsafe { slice::from_raw_parts(ptr, char_count) };
    let mut chars = slice.to_vec();
    while matches!(chars.last(), Some(&0)) {
        chars.pop();
    }
    Ok(chars)
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ArrowBindingError {
    InvalidParameterIndices,
    UnsupportedParameterType(sql::SqlDataType),
    UnsupportedCDataType(CDataType),
    InvalidColumnBufferLength,
    NullParameterValue,
    UnsupportedBindingMode,
    InvalidParameterValue,
    InvalidDateValue,
    InvalidTimeValue,
}

impl std::error::Error for ArrowBindingError {}

impl std::fmt::Display for ArrowBindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

struct Writer<T> {
    marker: PhantomData<T>,
}

impl<T> Writer<T> {
    fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

fn is_null(binding: &ParameterBinding) -> bool {
    if binding.str_len_or_ind_ptr.is_null() {
        false
    } else {
        unsafe { *binding.str_len_or_ind_ptr == NULL_DATA }
    }
}

trait ArrowWriter {
    fn arrow_type(&self) -> DataType;
    fn write(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        match binding.value_type {
            CDataType::Long => self.write_long(binding),
            CDataType::Char => self.write_char(binding),
            CDataType::WChar => self.write_wchar(binding),
            CDataType::Double => self.write_double(binding),
            CDataType::Bit => self.write_bit(binding),
            CDataType::Binary => self.write_binary(binding),
            CDataType::TypeTimestamp | CDataType::TimeStamp => self.write_timestamp(binding),
            CDataType::TypeDate | CDataType::Date => self.write_date(binding),
            CDataType::TypeTime | CDataType::Time => self.write_time(binding),
            _ => Err(ArrowBindingError::UnsupportedCDataType(binding.value_type)),
        }
    }

    fn write_long(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        Err(ArrowBindingError::UnsupportedCDataType(binding.value_type))
    }

    fn write_char(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        Err(ArrowBindingError::UnsupportedCDataType(binding.value_type))
    }

    fn write_wchar(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        Err(ArrowBindingError::UnsupportedCDataType(binding.value_type))
    }

    fn write_double(
        &self,
        binding: &ParameterBinding,
    ) -> Result<Arc<dyn Array>, ArrowBindingError> {
        Err(ArrowBindingError::UnsupportedCDataType(binding.value_type))
    }

    fn write_timestamp(
        &self,
        binding: &ParameterBinding,
    ) -> Result<Arc<dyn Array>, ArrowBindingError> {
        Err(ArrowBindingError::UnsupportedCDataType(binding.value_type))
    }

    fn write_date(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        Err(ArrowBindingError::UnsupportedCDataType(binding.value_type))
    }

    fn write_time(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        Err(ArrowBindingError::UnsupportedCDataType(binding.value_type))
    }

    fn write_bit(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        Err(ArrowBindingError::UnsupportedCDataType(binding.value_type))
    }

    fn write_binary(
        &self,
        binding: &ParameterBinding,
    ) -> Result<Arc<dyn Array>, ArrowBindingError> {
        Err(ArrowBindingError::UnsupportedCDataType(binding.value_type))
    }
}

impl ArrowWriter for Writer<Int8Array> {
    fn arrow_type(&self) -> DataType {
        DataType::Int8
    }
}

impl ArrowWriter for Writer<Int32Type> {
    fn arrow_type(&self) -> DataType {
        DataType::Int32
    }

    fn write_long(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        if is_null(binding) {
            return Ok(Arc::new(Int32Array::from(vec![None])));
        }
        Ok(Arc::new(Int32Array::from(vec![Some(unsafe {
            std::ptr::read(binding.parameter_value_ptr as *const i32)
        })])))
    }

    fn write_bit(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        // Bit is typically stored as a single byte (0 or 1)
        if is_null(binding) {
            return Ok(Arc::new(Int32Array::from(vec![None])));
        }
        let value = unsafe { std::ptr::read(binding.parameter_value_ptr as *const u8) };
        Ok(Arc::new(Int32Array::from(vec![Some(value as i32)])))
    }
}

impl ArrowWriter for Writer<Utf8Type> {
    fn arrow_type(&self) -> DataType {
        DataType::Utf8
    }

    fn write_char(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        if is_null(binding) {
            return Ok(Arc::new(StringArray::from(vec![None::<&str>])));
        }
        let value = if binding.buffer_length == sql::NTS {
            unsafe {
                CStr::from_ptr(binding.parameter_value_ptr as *const c_char)
                    .to_string_lossy()
                    .to_string()
            }
        } else {
            let raw_value = unsafe {
                str::from_utf8(slice::from_raw_parts(
                    binding.parameter_value_ptr as *const u8,
                    binding.buffer_length as usize,
                ))
                .unwrap()
                .to_string()
            };
            // Remove null terminator if present
            raw_value.trim_end_matches('\0').to_string()
        };
        // Only write length if pointer is not null
        if !binding.str_len_or_ind_ptr.is_null() {
            unsafe {
                std::ptr::write(binding.str_len_or_ind_ptr, value.len() as sql::Len);
            }
        }
        Ok(Arc::new(StringArray::from(vec![value])))
    }

    fn write_wchar(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        if is_null(binding) {
            return Ok(Arc::new(StringArray::from(vec![None::<&str>])));
        }

        // On macOS/Linux, wchar_t is 4 bytes (UTF-32), on Windows it's 2 bytes (UTF-16)
        // The odbc-sys crate defines WChar as u16, but iODBC uses wchar_t which is 4 bytes on macOS
        #[cfg(target_os = "macos")]
        let wchar_size = 4usize;
        #[cfg(not(target_os = "macos"))]
        let wchar_size = mem::size_of::<sql::WChar>();

        let value = match wchar_size {
            2 => {
                let chars = utf16_chars_from_binding(binding)?;
                if chars.is_empty() {
                    String::new()
                } else {
                    match String::from_utf16(&chars) {
                        Ok(s) => s,
                        Err(err) => {
                            tracing::error!(
                                "Failed to convert UTF-16 to string (len={}): {}",
                                chars.len(),
                                err
                            );
                            String::from_utf16_lossy(&chars)
                        }
                    }
                }
            }
            4 => {
                let chars = utf32_chars_from_binding(binding)?;
                if chars.is_empty() {
                    String::new()
                } else {
                    utf32_to_string(&chars).unwrap_or_else(|| {
                        tracing::error!("Failed to convert UTF-32 to string: {:?}", chars);
                        String::from("<error decoding utf32 string>")
                    })
                }
            }
            other => {
                tracing::error!("Unsupported SQLWCHAR size: {}", other);
                return Err(ArrowBindingError::UnsupportedCDataType(binding.value_type));
            }
        };

        Ok(Arc::new(StringArray::from(vec![value])))
    }

    fn write_timestamp(
        &self,
        binding: &ParameterBinding,
    ) -> Result<Arc<dyn Array>, ArrowBindingError> {
        // When binding a timestamp to a string column, convert to ISO format
        if is_null(binding) {
            return Ok(Arc::new(StringArray::from(vec![None::<&str>])));
        }
        #[repr(C)]
        struct SqlTimestampStruct {
            year: i16,
            month: u16,
            day: u16,
            hour: u16,
            minute: u16,
            second: u16,
            fraction: u32, // nanoseconds
        }

        let ts =
            unsafe { std::ptr::read(binding.parameter_value_ptr as *const SqlTimestampStruct) };

        // Format as ISO 8601 timestamp string
        eprintln!(
            "DEBUG write_timestamp_utf8: {:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:09}",
            ts.year, ts.month, ts.day, ts.hour, ts.minute, ts.second, ts.fraction
        );
        let value = format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:09}",
            ts.year, ts.month, ts.day, ts.hour, ts.minute, ts.second, ts.fraction
        );

        // Only write length if pointer is not null
        if !binding.str_len_or_ind_ptr.is_null() {
            unsafe {
                std::ptr::write(binding.str_len_or_ind_ptr, value.len() as sql::Len);
            }
        }
        Ok(Arc::new(StringArray::from(vec![value])))
    }

    fn write_binary(
        &self,
        binding: &ParameterBinding,
    ) -> Result<Arc<dyn Array>, ArrowBindingError> {
        // Read binary data and convert to hex string
        if is_null(binding) {
            return Ok(Arc::new(StringArray::from(vec![None::<&str>])));
        }
        let length = if binding.buffer_length == sql::NTS {
            // For binary, NTS doesn't make sense, but handle it
            0
        } else {
            binding.buffer_length as usize
        };

        let bytes =
            unsafe { slice::from_raw_parts(binding.parameter_value_ptr as *const u8, length) };

        // Convert to hex string
        let hex_string = bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        // Only write length if pointer is not null
        if !binding.str_len_or_ind_ptr.is_null() {
            unsafe {
                std::ptr::write(binding.str_len_or_ind_ptr, hex_string.len() as sql::Len);
            }
        }
        Ok(Arc::new(StringArray::from(vec![hex_string])))
    }
}

impl ArrowWriter for Writer<Float64Type> {
    fn arrow_type(&self) -> DataType {
        DataType::Float64
    }

    fn write_double(
        &self,
        binding: &ParameterBinding,
    ) -> Result<Arc<dyn Array>, ArrowBindingError> {
        if is_null(binding) {
            return Ok(Arc::new(Float64Array::from(vec![None])));
        }
        Ok(Arc::new(Float64Array::from(vec![Some(unsafe {
            std::ptr::read(binding.parameter_value_ptr as *const f64)
        })])))
    }
}

impl ArrowWriter for Writer<TimestampNanosecondType> {
    fn arrow_type(&self) -> DataType {
        DataType::Timestamp(TimeUnit::Nanosecond, None)
    }

    fn write_char(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        // Parse timestamp from string (SQL_C_CHAR bound to SQL_TYPE_TIMESTAMP)
        if is_null(binding) {
            return Ok(Arc::new(TimestampNanosecondArray::from(vec![None])));
        }

        let timestamp_str = if binding.buffer_length == sql::NTS {
            unsafe {
                CStr::from_ptr(binding.parameter_value_ptr as *const c_char)
                    .to_string_lossy()
                    .to_string()
            }
        } else {
            let raw_value = unsafe {
                str::from_utf8(slice::from_raw_parts(
                    binding.parameter_value_ptr as *const u8,
                    binding.buffer_length as usize,
                ))
                .unwrap()
                .to_string()
            };
            raw_value.trim_end_matches('\0').to_string()
        };

        // Parse timestamp string - try common formats
        // Format: "YYYY-MM-DD HH:MM:SS.fff" or "YYYY-MM-DD HH:MM:SS"
        use time::format_description::well_known::Rfc3339;
        use time::{Date, Month, PrimitiveDateTime, Time};

        // Try RFC3339 first
        if let Ok(dt) = time::OffsetDateTime::parse(&timestamp_str, &Rfc3339) {
            return Ok(Arc::new(TimestampNanosecondArray::from(vec![Some(
                dt.unix_timestamp_nanos() as i64,
            )])));
        }

        // Try custom format: "YYYY-MM-DD HH:MM:SS.fff"
        let parts: Vec<&str> = timestamp_str.split_whitespace().collect();
        if parts.len() >= 2 {
            let date_parts: Vec<&str> = parts[0].split('-').collect();
            let time_parts: Vec<&str> = parts[1].split(':').collect();

            if date_parts.len() == 3 && time_parts.len() >= 2 {
                if let (Ok(year), Ok(month), Ok(day)) = (
                    date_parts[0].parse::<i32>(),
                    date_parts[1].parse::<u8>(),
                    date_parts[2].parse::<u8>(),
                ) {
                    if let (Ok(hour), Ok(minute)) =
                        (time_parts[0].parse::<u8>(), time_parts[1].parse::<u8>())
                    {
                        let second_and_frac = if time_parts.len() > 2 {
                            time_parts[2]
                        } else {
                            "0"
                        };
                        let sec_parts: Vec<&str> = second_and_frac.split('.').collect();
                        let second = sec_parts[0].parse::<u8>().unwrap_or(0);
                        let fraction = if sec_parts.len() > 1 {
                            // Pad or truncate to 9 digits
                            let frac_str = format!("{:0<9}", sec_parts[1]);
                            frac_str[..9].parse::<u32>().unwrap_or(0)
                        } else {
                            0
                        };

                        let month_enum = match month {
                            1 => Month::January,
                            2 => Month::February,
                            3 => Month::March,
                            4 => Month::April,
                            5 => Month::May,
                            6 => Month::June,
                            7 => Month::July,
                            8 => Month::August,
                            9 => Month::September,
                            10 => Month::October,
                            11 => Month::November,
                            12 => Month::December,
                            _ => Month::January,
                        };

                        if let Ok(date) = Date::from_calendar_date(year, month_enum, day) {
                            if let Ok(time) = Time::from_hms_nano(hour, minute, second, fraction) {
                                let datetime = PrimitiveDateTime::new(date, time);
                                let nanos = datetime.assume_utc().unix_timestamp_nanos() as i64;
                                return Ok(Arc::new(TimestampNanosecondArray::from(vec![Some(
                                    nanos,
                                )])));
                            }
                        }
                    }
                }
            }
        }

        // If parsing fails, return error
        Err(ArrowBindingError::UnsupportedCDataType(binding.value_type))
    }

    fn write_timestamp(
        &self,
        binding: &ParameterBinding,
    ) -> Result<Arc<dyn Array>, ArrowBindingError> {
        // Read SQL_TIMESTAMP_STRUCT
        if is_null(binding) {
            return Ok(Arc::new(TimestampNanosecondArray::from(vec![None])));
        }
        #[repr(C)]
        struct SqlTimestampStruct {
            year: i16,
            month: u16,
            day: u16,
            hour: u16,
            minute: u16,
            second: u16,
            fraction: u32, // nanoseconds
        }

        let ts =
            unsafe { std::ptr::read(binding.parameter_value_ptr as *const SqlTimestampStruct) };

        // Convert to nanoseconds since epoch using the time crate
        use time::{Date, Month, PrimitiveDateTime, Time};

        let month = match ts.month {
            1 => Month::January,
            2 => Month::February,
            3 => Month::March,
            4 => Month::April,
            5 => Month::May,
            6 => Month::June,
            7 => Month::July,
            8 => Month::August,
            9 => Month::September,
            10 => Month::October,
            11 => Month::November,
            12 => Month::December,
            _ => Month::January, // fallback
        };

        // For TIMESTAMP_LTZ, interpret the timestamp in the session timezone
        // If no session timezone is available, fall back to UTC
        let tz_opt = get_session_timezone();
        let nanos = if let Some(ref tz_name) = tz_opt {
            use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
            use chrono_tz::Tz;

            // Try to parse the timezone name (e.g., "America/New_York")
            if let Ok(tz) = tz_name.parse::<Tz>() {
                let naive_date =
                    NaiveDate::from_ymd_opt(ts.year as i32, ts.month as u32, ts.day as u32)
                        .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
                let naive_time = NaiveTime::from_hms_nano_opt(
                    ts.hour as u32,
                    ts.minute as u32,
                    ts.second as u32,
                    ts.fraction,
                )
                .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap());
                let naive_datetime = NaiveDateTime::new(naive_date, naive_time);

                // Interpret the naive datetime in the session timezone
                let datetime_with_tz = tz
                    .from_local_datetime(&naive_datetime)
                    .single()
                    .unwrap_or_else(|| tz.from_utc_datetime(&naive_datetime));

                let nanos = datetime_with_tz.timestamp_nanos_opt().unwrap_or(0);
                tracing::debug!(
                    "write_timestamp: interpreted {:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:09} in timezone {} -> {} nanos",
                    ts.year,
                    ts.month,
                    ts.day,
                    ts.hour,
                    ts.minute,
                    ts.second,
                    ts.fraction,
                    tz_name,
                    nanos
                );
                nanos
            } else {
                tracing::warn!(
                    "write_timestamp: failed to parse timezone '{}', falling back to UTC",
                    tz_name
                );
                let date = Date::from_calendar_date(ts.year as i32, month, ts.day as u8)
                    .unwrap_or_else(|_| Date::from_calendar_date(1970, Month::January, 1).unwrap());
                let time = Time::from_hms_nano(
                    ts.hour as u8,
                    ts.minute as u8,
                    ts.second as u8,
                    ts.fraction,
                )
                .unwrap_or_else(|_| Time::from_hms(0, 0, 0).unwrap());
                let datetime = PrimitiveDateTime::new(date, time);
                datetime.assume_utc().unix_timestamp_nanos() as i64
            }
        } else {
            // No session timezone - use UTC
            let date = Date::from_calendar_date(ts.year as i32, month, ts.day as u8)
                .unwrap_or_else(|_| Date::from_calendar_date(1970, Month::January, 1).unwrap());
            let time =
                Time::from_hms_nano(ts.hour as u8, ts.minute as u8, ts.second as u8, ts.fraction)
                    .unwrap_or_else(|_| Time::from_hms(0, 0, 0).unwrap());
            let datetime = PrimitiveDateTime::new(date, time);
            let nanos = datetime.assume_utc().unix_timestamp_nanos() as i64;
            tracing::debug!(
                "write_timestamp: no session timezone, using UTC: {:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:09} -> {} nanos",
                ts.year,
                ts.month,
                ts.day,
                ts.hour,
                ts.minute,
                ts.second,
                ts.fraction,
                nanos
            );
            nanos
        };

        eprintln!(
            "DEBUG write_timestamp_single: {:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:09} -> {} (tz={:?})",
            ts.year,
            ts.month,
            ts.day,
            ts.hour,
            ts.minute,
            ts.second,
            ts.fraction,
            nanos,
            tz_opt.as_deref()
        );
        Ok(Arc::new(TimestampNanosecondArray::from(vec![Some(nanos)])))
    }

    fn write_date(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        // Read SQL_DATE_STRUCT
        if is_null(binding) {
            return Ok(Arc::new(TimestampNanosecondArray::from(vec![None])));
        }
        #[repr(C)]
        struct SqlDateStruct {
            year: i16,
            month: u16,
            day: u16,
        }

        let date_struct =
            unsafe { std::ptr::read(binding.parameter_value_ptr as *const SqlDateStruct) };

        // Convert to nanoseconds since epoch (midnight on that date) using the time crate
        use time::{Date, Month, PrimitiveDateTime, Time};

        let month = match date_struct.month {
            1 => Month::January,
            2 => Month::February,
            3 => Month::March,
            4 => Month::April,
            5 => Month::May,
            6 => Month::June,
            7 => Month::July,
            8 => Month::August,
            9 => Month::September,
            10 => Month::October,
            11 => Month::November,
            12 => Month::December,
            _ => Month::January, // fallback
        };

        let date = Date::from_calendar_date(date_struct.year as i32, month, date_struct.day as u8)
            .unwrap_or_else(|_| Date::from_calendar_date(1970, Month::January, 1).unwrap());
        let time = Time::from_hms(0, 0, 0).unwrap();
        let datetime = PrimitiveDateTime::new(date, time);
        let nanos = datetime.assume_utc().unix_timestamp_nanos() as i64;

        Ok(Arc::new(TimestampNanosecondArray::from(vec![Some(nanos)])))
    }

    fn write_time(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        // Read SQL_TIME_STRUCT
        if is_null(binding) {
            return Ok(Arc::new(TimestampNanosecondArray::from(vec![None])));
        }
        #[repr(C)]
        struct SqlTimeStruct {
            hour: u16,
            minute: u16,
            second: u16,
        }

        let time = unsafe { std::ptr::read(binding.parameter_value_ptr as *const SqlTimeStruct) };

        // Convert to nanoseconds since midnight
        let nanos = time.hour as i64 * 3600 * 1_000_000_000
            + time.minute as i64 * 60 * 1_000_000_000
            + time.second as i64 * 1_000_000_000;

        Ok(Arc::new(TimestampNanosecondArray::from(vec![Some(nanos)])))
    }
}

fn arrow_writer_from_sql_type(
    parameter_type: &sql::SqlDataType,
) -> Result<Box<dyn ArrowWriter + Send + Sync>, ArrowBindingError> {
    use sql::SqlDataType;
    let type_value = parameter_type.0; // Get the underlying i16 value
    match *parameter_type {
        SqlDataType::INTEGER => {
            Ok(Box::new(Writer::<Int32Type>::new()) as Box<dyn ArrowWriter + Send + Sync>)
        }
        SqlDataType::VARCHAR | SqlDataType::CHAR => {
            Ok(Box::new(Writer::<Utf8Type>::new()) as Box<dyn ArrowWriter + Send + Sync>)
        }
        SqlDataType::DOUBLE | SqlDataType::FLOAT => {
            Ok(Box::new(Writer::<Float64Type>::new()) as Box<dyn ArrowWriter + Send + Sync>)
        }
        // SQL_BIT = -7, SQL_BINARY = -2, SQL_WCHAR = -8, SQL_WVARCHAR = -9, SQL_WLONGVARCHAR = -10
        _ if type_value == -7 => {
            Ok(Box::new(Writer::<Int32Type>::new()) as Box<dyn ArrowWriter + Send + Sync>)
        }
        _ if type_value == -2 || type_value == -8 || type_value == -9 || type_value == -10 => {
            Ok(Box::new(Writer::<Utf8Type>::new()) as Box<dyn ArrowWriter + Send + Sync>)
        }
        // SQL_TYPE_DATE = 91
        _ if type_value == 91 => {
            Ok(Box::new(DateWriter::new()) as Box<dyn ArrowWriter + Send + Sync>)
        }
        // SQL_TYPE_TIME = 92
        _ if type_value == 92 => {
            Ok(Box::new(TimeWriter::new()) as Box<dyn ArrowWriter + Send + Sync>)
        }
        _ if type_value == 93 => {
            Ok(Box::new(TimestampStructWriter::new()) as Box<dyn ArrowWriter + Send + Sync>)
        }
        // Snowflake extended timestamp types (SQL_SF_TIMESTAMP_LTZ = 2000, etc.)
        _ if type_value >= 2000 && type_value <= 2002 => {
            Ok(Box::new(TimestampStructWriter::new()) as Box<dyn ArrowWriter + Send + Sync>)
        }
        // For any other unknown type, default to Timestamp writer
        _ => Ok(Box::new(Writer::<TimestampNanosecondType>::new())
            as Box<dyn ArrowWriter + Send + Sync>),
    }
}

pub fn odbc_bindings_to_arrow_bindings(
    bindings: &HashMap<u16, ParameterBinding>,
) -> Result<(Box<FFI_ArrowSchema>, Box<FFI_ArrowArray>), ArrowBindingError> {
    odbc_bindings_to_arrow_bindings_batch(bindings, 1)
}

/// Convert ODBC parameter bindings to Arrow arrays with support for batch (array) binding
pub fn odbc_bindings_to_arrow_bindings_batch(
    bindings: &HashMap<u16, ParameterBinding>,
    row_count: usize,
) -> Result<(Box<FFI_ArrowSchema>, Box<FFI_ArrowArray>), ArrowBindingError> {
    eprintln!(
        "DEBUG odbc_bindings_to_arrow_bindings_batch: START, row_count={}, num_bindings={}",
        row_count,
        bindings.len()
    );
    let mut schema_fields = Vec::new();
    let mut arrays = Vec::new();
    let max_key = *bindings.keys().max().unwrap_or(&0);
    let min_key = *bindings.keys().min().unwrap_or(&1);
    eprintln!(
        "DEBUG odbc_bindings_to_arrow_bindings_batch: min_key={}, max_key={}",
        min_key, max_key
    );

    for param_num in min_key..=max_key {
        let binding = bindings.get(&param_num);
        if binding.is_none() {
            tracing::error!(
                "SQLExecute: parameter #{param_num} not found. Make sure parameter bindings are contiguous and start at 1.",
            );
            return Err(ArrowBindingError::InvalidParameterIndices);
        }
        let binding = binding.unwrap();
        let writer = arrow_writer_from_sql_type(&binding.parameter_type)?;

        // For batch binding, we need to write multiple rows
        let array = if row_count <= 1 {
            writer.write(binding)?
        } else {
            write_batch_array(&*writer, binding, row_count)?
        };

        // Use the actual array's data type for the schema field
        let mut field = arrow::datatypes::Field::new(
            format!("param_{param_num}"),
            array.data_type().clone(),
            true,
        );

        if let Some(logical_type) = logical_type_from_sql_type(&binding.parameter_type) {
            let mut metadata = HashMap::new();
            metadata.insert("logicalType".to_string(), logical_type.to_string());
            field = field.with_metadata(metadata);
        }

        schema_fields.push(field.clone());

        eprintln!(
            "DEBUG odbc_bindings_to_arrow_bindings_batch: param_{} array len = {}, writer_type={:?}, array_type={:?}",
            param_num,
            array.len(),
            writer.arrow_type(),
            array.data_type()
        );

        // Use the actual array's data type for the field, not the writer's expected type
        // This handles cases where the array type differs (e.g., Date32 vs TimestampNanosecond)
        arrays.push((Arc::new(field), array));
    }
    eprintln!("DEBUG odbc_bindings_to_arrow_bindings_batch: Creating schema and array");
    let schema = arrow::datatypes::Schema::new(schema_fields);
    eprintln!("DEBUG odbc_bindings_to_arrow_bindings_batch: Schema created");
    let schema = Box::new(arrow::ffi::FFI_ArrowSchema::try_from(&schema).unwrap());
    eprintln!("DEBUG odbc_bindings_to_arrow_bindings_batch: FFI schema created");
    let array = arrow::array::StructArray::from(arrays);
    eprintln!("DEBUG odbc_bindings_to_arrow_bindings_batch: StructArray created");
    let array = Box::new(arrow::ffi::FFI_ArrowArray::new(&array.into_data()));
    eprintln!("DEBUG odbc_bindings_to_arrow_bindings_batch: FFI array created, DONE");
    Ok((schema, array))
}

/// Write a batch of values from a column-wise binding to an Arrow array
fn write_batch_array(
    writer: &dyn ArrowWriter,
    binding: &ParameterBinding,
    row_count: usize,
) -> Result<Arc<dyn Array>, ArrowBindingError> {
    eprintln!(
        "DEBUG write_batch_array: value_type={:?}, param_type={:?}, row_count={}",
        binding.value_type, binding.parameter_type, row_count
    );
    use arrow::array::{
        Date32Builder, Float64Builder, Int32Builder, Int64Builder, StringBuilder,
        Time64NanosecondBuilder,
    };

    // Match on value_type (how the data is stored in memory) first
    match binding.value_type {
        CDataType::Long => {
            let mut builder = Int32Builder::with_capacity(row_count);
            for row_idx in 0..row_count {
                let is_null = is_null_at_row(binding, row_idx);
                if is_null {
                    builder.append_null();
                } else {
                    let ptr = get_value_ptr_at_row::<i32>(binding, row_idx);
                    let value = unsafe { *ptr };
                    builder.append_value(value);
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        CDataType::Double => {
            let mut builder = Float64Builder::with_capacity(row_count);
            for row_idx in 0..row_count {
                let is_null = is_null_at_row(binding, row_idx);
                if is_null {
                    builder.append_null();
                } else {
                    let ptr = get_value_ptr_at_row::<f64>(binding, row_idx);
                    let value = unsafe { *ptr };
                    builder.append_value(value);
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        CDataType::Char => {
            let mut builder = StringBuilder::with_capacity(row_count, row_count * 64);
            for row_idx in 0..row_count {
                let is_null = is_null_at_row(binding, row_idx);
                if is_null {
                    builder.append_null();
                } else {
                    let value = get_string_at_row(binding, row_idx)?;
                    builder.append_value(&value);
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        CDataType::Bit => {
            // Bit values are stored as single bytes, convert to Int32 for Arrow
            let mut builder = Int32Builder::with_capacity(row_count);
            for row_idx in 0..row_count {
                let is_null = is_null_at_row(binding, row_idx);
                if is_null {
                    builder.append_null();
                } else {
                    let ptr = get_value_ptr_at_row::<u8>(binding, row_idx);
                    let value = unsafe { *ptr };
                    builder.append_value(value as i32);
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        CDataType::Binary => {
            // Binary data is sent as hex-encoded string for Snowflake
            let mut builder = StringBuilder::with_capacity(row_count, row_count * 64);
            for row_idx in 0..row_count {
                let is_null = is_null_at_row(binding, row_idx);
                if is_null {
                    builder.append_null();
                } else {
                    let bytes = get_binary_at_row(binding, row_idx)?;
                    // Convert to hex string
                    let hex: String = bytes.iter().map(|b| format!("{:02X}", b)).collect();
                    builder.append_value(&hex);
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        CDataType::TypeDate | CDataType::Date => {
            // Date values - convert to Date32 (days since epoch)
            let mut builder = Date32Builder::with_capacity(row_count);
            for row_idx in 0..row_count {
                let is_null = is_null_at_row(binding, row_idx);
                if is_null {
                    builder.append_null();
                } else {
                    let ptr = get_value_ptr_at_row::<sql::Date>(binding, row_idx);
                    let date_struct = unsafe { *ptr };
                    let days = date_struct_to_days(&date_struct)?;
                    builder.append_value(days);
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        CDataType::TypeTime | CDataType::Time => {
            // Time values - convert to Time64Nanosecond (nanoseconds since midnight)
            let mut builder = Time64NanosecondBuilder::with_capacity(row_count);
            for row_idx in 0..row_count {
                let is_null = is_null_at_row(binding, row_idx);
                if is_null {
                    builder.append_null();
                } else {
                    let ptr = get_value_ptr_at_row::<sql::Time>(binding, row_idx);
                    let time_struct = unsafe { *ptr };
                    let nanos = time_struct_to_nanos(&time_struct);
                    builder.append_value(nanos);
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        CDataType::TypeTimestamp | CDataType::TimeStamp => {
            let mut epoch_builder = Int64Builder::with_capacity(row_count);
            let mut fraction_builder = Int32Builder::with_capacity(row_count);
            for row_idx in 0..row_count {
                let is_null = is_null_at_row(binding, row_idx);
                if is_null {
                    epoch_builder.append_null();
                    fraction_builder.append_null();
                } else {
                    let ptr = get_value_ptr_at_row::<sql::Timestamp>(binding, row_idx);
                    let ts = unsafe { *ptr };
                    let (epoch, fraction) =
                        timestamp_struct_to_epoch_fraction(&ts, &binding.parameter_type)?;
                    epoch_builder.append_value(epoch);
                    fraction_builder.append_value(fraction);
                }
            }
            let epoch_array = Arc::new(epoch_builder.finish()) as Arc<dyn Array>;
            let fraction_array = Arc::new(fraction_builder.finish()) as Arc<dyn Array>;
            let struct_array = StructArray::from(vec![
                (
                    Arc::new(Field::new("epoch", DataType::Int64, true)),
                    epoch_array,
                ),
                (
                    Arc::new(Field::new("fraction", DataType::Int32, true)),
                    fraction_array,
                ),
            ]);
            Ok(Arc::new(struct_array))
        }
        _ => {
            // Fallback: use single-row write for each row and concatenate
            // This is less efficient but handles all types
            let mut row_arrays = Vec::with_capacity(row_count);
            for row_idx in 0..row_count {
                let row_binding = create_row_binding(binding, row_idx)?;
                row_arrays.push(writer.write(&row_binding)?);
            }
            arrow::compute::concat(&row_arrays.iter().map(|a| a.as_ref()).collect::<Vec<_>>())
                .map_err(|_| ArrowBindingError::InvalidParameterValue)
        }
    }
}

struct DateWriter;
struct TimeWriter;
struct TimestampStructWriter;

impl DateWriter {
    fn new() -> Self {
        Self
    }
}

impl TimeWriter {
    fn new() -> Self {
        Self
    }
}

impl TimestampStructWriter {
    fn new() -> Self {
        Self
    }
}

impl ArrowWriter for DateWriter {
    fn arrow_type(&self) -> DataType {
        DataType::Date32
    }

    fn write_date(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        if is_null(binding) {
            return Ok(Arc::new(Date32Array::from(vec![None])));
        }
        let date_struct = unsafe { *(binding.parameter_value_ptr as *const sql::Date) };
        let days = date_struct_to_days(&date_struct)?;
        Ok(Arc::new(Date32Array::from(vec![Some(days)])))
    }
}

impl ArrowWriter for TimeWriter {
    fn arrow_type(&self) -> DataType {
        DataType::Time64(TimeUnit::Nanosecond)
    }

    fn write_time(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        if is_null(binding) {
            return Ok(Arc::new(Time64NanosecondArray::from(vec![None])));
        }
        let time_struct = unsafe { *(binding.parameter_value_ptr as *const sql::Time) };
        let nanos = time_struct_to_nanos(&time_struct);
        Ok(Arc::new(Time64NanosecondArray::from(vec![Some(nanos)])))
    }
}

fn build_timestamp_struct_array(epoch: Option<i64>, fraction: Option<i32>) -> Arc<dyn Array> {
    let epoch_array = Arc::new(Int64Array::from(vec![epoch]));
    let fraction_array = Arc::new(Int32Array::from(vec![fraction]));
    Arc::new(StructArray::from(vec![
        (
            Arc::new(Field::new("epoch", DataType::Int64, true)),
            epoch_array as Arc<dyn Array>,
        ),
        (
            Arc::new(Field::new("fraction", DataType::Int32, true)),
            fraction_array as Arc<dyn Array>,
        ),
    ]))
}

impl ArrowWriter for TimestampStructWriter {
    fn arrow_type(&self) -> DataType {
        DataType::Struct(
            vec![
                Field::new("epoch", DataType::Int64, true),
                Field::new("fraction", DataType::Int32, true),
            ]
            .into(),
        )
    }

    fn write_timestamp(
        &self,
        binding: &ParameterBinding,
    ) -> Result<Arc<dyn Array>, ArrowBindingError> {
        if is_null(binding) {
            return Ok(build_timestamp_struct_array(None, None));
        }

        let ts = unsafe { std::ptr::read(binding.parameter_value_ptr as *const sql::Timestamp) };
        let (epoch, fraction) = timestamp_struct_to_epoch_fraction(&ts, &binding.parameter_type)?;
        Ok(build_timestamp_struct_array(Some(epoch), Some(fraction)))
    }

    fn write_char(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        if is_null(binding) {
            return Ok(build_timestamp_struct_array(None, None));
        }
        let value = read_char_string(binding)?;
        let ts = parse_timestamp_string(&value)?;
        let (epoch, fraction) = timestamp_struct_to_epoch_fraction(&ts, &binding.parameter_type)?;
        Ok(build_timestamp_struct_array(Some(epoch), Some(fraction)))
    }

    fn write_wchar(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        if is_null(binding) {
            return Ok(build_timestamp_struct_array(None, None));
        }
        let value = read_wchar_string(binding)?;
        let ts = parse_timestamp_string(&value)?;
        let (epoch, fraction) = timestamp_struct_to_epoch_fraction(&ts, &binding.parameter_type)?;
        Ok(build_timestamp_struct_array(Some(epoch), Some(fraction)))
    }
}

/// Check if a value is null at a specific row index
fn is_null_at_row(binding: &ParameterBinding, row_idx: usize) -> bool {
    if binding.str_len_or_ind_ptr.is_null() {
        false
    } else {
        unsafe { *binding.str_len_or_ind_ptr.add(row_idx) == NULL_DATA }
    }
}

/// Get a pointer to the value at a specific row index
fn get_value_ptr_at_row<T>(binding: &ParameterBinding, row_idx: usize) -> *const T {
    let element_size = binding.buffer_length as usize;
    let actual_size = if element_size > 0 {
        element_size
    } else {
        mem::size_of::<T>()
    };
    unsafe { (binding.parameter_value_ptr as *const u8).add(row_idx * actual_size) as *const T }
}

/// Get a string value at a specific row index
fn get_string_at_row(
    binding: &ParameterBinding,
    row_idx: usize,
) -> Result<String, ArrowBindingError> {
    let element_size = binding.buffer_length as usize;
    let base_ptr =
        unsafe { (binding.parameter_value_ptr as *const u8).add(row_idx * element_size) };

    // Get the length for this row
    let len = if !binding.str_len_or_ind_ptr.is_null() {
        let len_val = unsafe { *binding.str_len_or_ind_ptr.add(row_idx) };
        if len_val == sql::NTS || len_val == sql::NTSL || len_val < 0 {
            // Null-terminated string
            unsafe {
                CStr::from_ptr(base_ptr as *const c_char)
                    .to_string_lossy()
                    .to_string()
            }
        } else {
            let actual_len = len_val as usize;
            let bytes = unsafe { slice::from_raw_parts(base_ptr, actual_len) };
            String::from_utf8_lossy(bytes)
                .trim_end_matches('\0')
                .to_string()
        }
    } else {
        // No length indicator, use buffer_length
        let bytes = unsafe { slice::from_raw_parts(base_ptr, element_size) };
        String::from_utf8_lossy(bytes)
            .trim_end_matches('\0')
            .to_string()
    };

    Ok(len)
}

/// Get binary data at a specific row index
fn get_binary_at_row(
    binding: &ParameterBinding,
    row_idx: usize,
) -> Result<Vec<u8>, ArrowBindingError> {
    let element_size = binding.buffer_length as usize;
    let base_ptr =
        unsafe { (binding.parameter_value_ptr as *const u8).add(row_idx * element_size) };

    let len = if !binding.str_len_or_ind_ptr.is_null() {
        let len_val = unsafe { *binding.str_len_or_ind_ptr.add(row_idx) };
        if len_val < 0 {
            element_size
        } else {
            len_val as usize
        }
    } else {
        element_size
    };

    let bytes = unsafe { slice::from_raw_parts(base_ptr, len) };
    Ok(bytes.to_vec())
}

/// Convert a SQL_DATE_STRUCT to days since Unix epoch (1970-01-01)
fn date_struct_to_days(date_struct: &sql::Date) -> Result<i32, ArrowBindingError> {
    use time::{Date, Month};

    let month = Month::try_from(date_struct.month as u8)
        .map_err(|_| ArrowBindingError::InvalidDateValue)?;
    let date = Date::from_calendar_date(date_struct.year as i32, month, date_struct.day as u8)
        .map_err(|_| ArrowBindingError::InvalidDateValue)?;
    Ok((date.to_julian_day() - 2440588) as i32)
}

/// Convert a SQL_TIME_STRUCT to nanoseconds since midnight
fn time_struct_to_nanos(time_struct: &sql::Time) -> i64 {
    (time_struct.hour as i64 * 3600 + time_struct.minute as i64 * 60 + time_struct.second as i64)
        * 1_000_000_000
}

fn timestamp_struct_to_epoch_fraction(
    ts: &sql::Timestamp,
    parameter_type: &sql::SqlDataType,
) -> Result<(i64, i32), ArrowBindingError> {
    use chrono::{LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
    use chrono_tz::Tz;

    let date = NaiveDate::from_ymd_opt(ts.year as i32, ts.month as u32, ts.day as u32)
        .ok_or(ArrowBindingError::InvalidParameterValue)?;
    let time = NaiveTime::from_hms_nano_opt(
        ts.hour as u32,
        ts.minute as u32,
        ts.second as u32,
        ts.fraction,
    )
    .ok_or(ArrowBindingError::InvalidParameterValue)?;
    let dt = NaiveDateTime::new(date, time);

    let timestamp_type = timestamp_type_from_sql(parameter_type);
    let seconds = match timestamp_type {
        TimestampType::Ntz => dt.and_utc().timestamp(),
        _ => {
            if let Some(tz_name) = get_session_timezone() {
                if let Ok(tz) = tz_name.parse::<Tz>() {
                    match tz.from_local_datetime(&dt) {
                        LocalResult::Single(local_dt) => local_dt.timestamp(),
                        LocalResult::Ambiguous(local_dt, _) => local_dt.timestamp(),
                        LocalResult::None => tz.from_utc_datetime(&dt).timestamp(),
                    }
                } else {
                    dt.and_utc().timestamp()
                }
            } else {
                dt.and_utc().timestamp()
            }
        }
    };

    eprintln!(
        "DEBUG timestamp_struct_to_epoch_fraction: input={:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:09}, seconds={}, fraction={}",
        ts.year, ts.month, ts.day, ts.hour, ts.minute, ts.second, ts.fraction, seconds, ts.fraction
    );

    Ok((seconds, ts.fraction as i32))
}

fn read_char_string(binding: &ParameterBinding) -> Result<String, ArrowBindingError> {
    if !binding.str_len_or_ind_ptr.is_null() {
        let len = unsafe { *binding.str_len_or_ind_ptr };
        if len == sql::NTS || len == sql::NTSL {
            return Ok(unsafe {
                CStr::from_ptr(binding.parameter_value_ptr as *const c_char)
                    .to_string_lossy()
                    .to_string()
            });
        } else if len > 0 {
            let bytes = unsafe {
                slice::from_raw_parts(binding.parameter_value_ptr as *const u8, len as usize)
            };
            return Ok(String::from_utf8_lossy(bytes)
                .trim_end_matches('\0')
                .to_string());
        }
    }

    if binding.buffer_length == sql::NTS || binding.buffer_length <= 0 {
        Ok(unsafe {
            CStr::from_ptr(binding.parameter_value_ptr as *const c_char)
                .to_string_lossy()
                .to_string()
        })
    } else {
        let bytes = unsafe {
            slice::from_raw_parts(
                binding.parameter_value_ptr as *const u8,
                binding.buffer_length as usize,
            )
        };
        Ok(String::from_utf8_lossy(bytes)
            .trim_end_matches('\0')
            .to_string())
    }
}

fn read_wchar_string(binding: &ParameterBinding) -> Result<String, ArrowBindingError> {
    #[cfg(target_os = "macos")]
    let wchar_size = 4usize;
    #[cfg(not(target_os = "macos"))]
    let wchar_size = mem::size_of::<sql::WChar>();

    match wchar_size {
        2 => {
            let chars = utf16_chars_from_binding(binding)?;
            if chars.is_empty() {
                Ok(String::new())
            } else {
                String::from_utf16(&chars).map_err(|_| ArrowBindingError::InvalidParameterValue)
            }
        }
        4 => {
            let chars = utf32_chars_from_binding(binding)?;
            if chars.is_empty() {
                Ok(String::new())
            } else {
                utf32_to_string(&chars).ok_or(ArrowBindingError::InvalidParameterValue)
            }
        }
        _ => Err(ArrowBindingError::UnsupportedCDataType(CDataType::WChar)),
    }
}

fn parse_timestamp_string(value: &str) -> Result<sql::Timestamp, ArrowBindingError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ArrowBindingError::InvalidParameterValue);
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        let naive = dt.naive_local();
        return Ok(sql::Timestamp {
            year: naive.date().year() as i16,
            month: naive.date().month() as u16,
            day: naive.date().day() as u16,
            hour: naive.time().hour() as u16,
            minute: naive.time().minute() as u16,
            second: naive.time().second() as u16,
            fraction: naive.time().nanosecond(),
        });
    }

    const PATTERNS: [&str; 4] = [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
    ];

    for pattern in PATTERNS {
        if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, pattern) {
            return Ok(sql::Timestamp {
                year: dt.date().year() as i16,
                month: dt.date().month() as u16,
                day: dt.date().day() as u16,
                hour: dt.time().hour() as u16,
                minute: dt.time().minute() as u16,
                second: dt.time().second() as u16,
                fraction: dt.time().nanosecond(),
            });
        }
    }

    Err(ArrowBindingError::InvalidParameterValue)
}

/// Create a single-row binding from a batch binding at a specific row index
fn create_row_binding(
    binding: &ParameterBinding,
    row_idx: usize,
) -> Result<ParameterBinding, ArrowBindingError> {
    let element_size = if binding.buffer_length > 0 {
        binding.buffer_length as usize
    } else {
        8
    };
    let value_ptr = unsafe {
        (binding.parameter_value_ptr as *const u8).add(row_idx * element_size) as sql::Pointer
    };
    let ind_ptr = if binding.str_len_or_ind_ptr.is_null() {
        std::ptr::null_mut()
    } else {
        unsafe { binding.str_len_or_ind_ptr.add(row_idx) }
    };

    Ok(ParameterBinding {
        parameter_value_ptr: value_ptr,
        str_len_or_ind_ptr: ind_ptr,
        buffer_length: binding.buffer_length,
        value_type: binding.value_type,
        parameter_type: binding.parameter_type,
        owned_buffer: None,
    })
}
