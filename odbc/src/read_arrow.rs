use arrow::{
    array::{Array, ArrowPrimitiveType, BooleanArray, GenericByteArray, PrimitiveArray},
    datatypes::{
        ByteArrayType, DataType, Date32Type, Decimal128Type, Field, Float32Type, Float64Type,
        Int8Type, Int16Type, Int32Type, Int64Type, Time64NanosecondType, TimeUnit,
        TimestampNanosecondType, Utf8Type,
    },
};
use chrono::{Datelike, NaiveDate, NaiveDateTime, TimeZone, Timelike, offset::Offset};
use chrono_tz::Tz;
use odbc_sys as sql;

use crate::api::types::TimestampLtzFormat;
use crate::cdata_types::{CDataType, Double, Real, SBigInt, UBigInt};
use std::{any::TypeId, cell::RefCell, fmt::Display, io::Write};

thread_local! {
    static READ_SESSION_TIMEZONE: RefCell<Option<String>> = RefCell::new(None);
    static READ_TIMESTAMP_LTZ_FORMAT: RefCell<TimestampLtzFormat> =
        RefCell::new(TimestampLtzFormat::new(true, false));
    static READ_TIMESTAMP_NTZ_FORMAT: RefCell<TimestampLtzFormat> =
        RefCell::new(TimestampLtzFormat::new(false, false));
    static READ_TIMESTAMP_TZ_FORMAT: RefCell<TimestampLtzFormat> =
        RefCell::new(TimestampLtzFormat::new(true, true));
}

pub fn set_read_session_timezone(tz: Option<String>) {
    READ_SESSION_TIMEZONE.with(|cell| {
        *cell.borrow_mut() = tz.map(|name| crate::timezone::normalize_timezone_name(&name));
    });
}

pub fn set_read_timestamp_ltz_format(format: TimestampLtzFormat) {
    READ_TIMESTAMP_LTZ_FORMAT.with(|cell| {
        *cell.borrow_mut() = format;
    });
}

pub fn set_read_timestamp_ntz_format(format: TimestampLtzFormat) {
    READ_TIMESTAMP_NTZ_FORMAT.with(|cell| {
        *cell.borrow_mut() = format;
    });
}

pub fn set_read_timestamp_tz_format(format: TimestampLtzFormat) {
    READ_TIMESTAMP_TZ_FORMAT.with(|cell| {
        *cell.borrow_mut() = format;
    });
}

fn get_read_timestamp_ltz_format() -> TimestampLtzFormat {
    READ_TIMESTAMP_LTZ_FORMAT.with(|cell| *cell.borrow())
}

fn get_read_timestamp_ntz_format() -> TimestampLtzFormat {
    READ_TIMESTAMP_NTZ_FORMAT.with(|cell| *cell.borrow())
}

fn get_read_timestamp_tz_format() -> TimestampLtzFormat {
    READ_TIMESTAMP_TZ_FORMAT.with(|cell| *cell.borrow())
}

fn get_read_session_timezone() -> Option<Tz> {
    READ_SESSION_TIMEZONE.with(|cell| {
        cell.borrow()
            .as_ref()
            .and_then(|tz_name| tz_name.parse::<Tz>().ok())
    })
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum ExtractError {
    UnsupportedArrowType(DataType),
    UnsupportedTargetType(CDataType),
    DowncastError,
    ErrorParsingFieldMeta(Box<Field>, String),
    UnsupportedFieldMeta(FieldMeta, DataType),
    ConversionError(String),
}

impl std::error::Error for ExtractError {}

impl Display for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtractError::UnsupportedArrowType(dt) => {
                write!(f, "Unsupported Arrow type: {:?}", dt)
            }
            ExtractError::UnsupportedTargetType(ct) => {
                write!(f, "Unsupported target C type: {:?}", ct)
            }
            ExtractError::DowncastError => {
                write!(f, "Failed to downcast Arrow array to expected type")
            }
            ExtractError::ErrorParsingFieldMeta(field, msg) => {
                write!(
                    f,
                    "Error parsing field metadata for '{}': {}",
                    field.name(),
                    msg
                )
            }
            ExtractError::UnsupportedFieldMeta(meta, dt) => {
                write!(
                    f,
                    "Unsupported field metadata {:?} for Arrow type {:?}",
                    meta, dt
                )
            }
            ExtractError::ConversionError(msg) => {
                write!(f, "Conversion error: {}", msg)
            }
        }
    }
}

fn get_value<T: ArrowPrimitiveType>(
    array: &dyn Array,
    row_idx: usize,
) -> Result<T::Native, ExtractError> {
    Ok(array
        .as_any()
        .downcast_ref::<PrimitiveArray<T>>()
        .ok_or(ExtractError::DowncastError)?
        .value(row_idx))
}

fn get_byte_array_value<T: ByteArrayType>(
    array: &dyn Array,
    row_idx: usize,
) -> Result<&T::Native, ExtractError> {
    Ok(array
        .as_any()
        .downcast_ref::<GenericByteArray<T>>()
        .unwrap()
        .value(row_idx))
}

fn get_binary_value(array: &dyn Array, row_idx: usize) -> Result<&[u8], ExtractError> {
    Ok(array
        .as_any()
        .downcast_ref::<arrow::array::BinaryArray>()
        .ok_or(ExtractError::DowncastError)?
        .value(row_idx))
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum FieldMeta {
    Fixed {
        scale: u32,
        precision: u32,
        logical_type: Option<String>,
    },
    Text,
    Other {
        logical_type: Option<String>,
        scale: Option<u32>,
    },
}

impl FieldMeta {
    fn logical_type(&self) -> Option<&str> {
        match self {
            FieldMeta::Fixed { logical_type, .. } => logical_type.as_deref(),
            FieldMeta::Other { logical_type, .. } => logical_type.as_deref(),
            FieldMeta::Text => None,
        }
    }
}

fn get_field_meta(field: &Field) -> Result<FieldMeta, ExtractError> {
    let metadata = field.metadata();
    let logical_type_raw = metadata.get("logicalType").cloned();
    let logical_type_norm = logical_type_raw
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("NONE")
        .to_uppercase();

    match logical_type_norm.as_str() {
        "FIXED" | "TIME" => {
            let scale = metadata
                .get("scale")
                .ok_or(ExtractError::ErrorParsingFieldMeta(
                    Box::new(field.clone()),
                    "scale not found".to_string(),
                ))?;
            let precision = metadata.get("precision");
            Ok(FieldMeta::Fixed {
                scale: scale.parse::<u32>().map_err(|_| {
                    ExtractError::ErrorParsingFieldMeta(
                        Box::new(field.clone()),
                        "scale not a valid u32".to_string(),
                    )
                })?,
                precision: precision
                    .map(|p| p.parse::<u32>())
                    .transpose()
                    .map_err(|_| {
                        ExtractError::ErrorParsingFieldMeta(
                            Box::new(field.clone()),
                            "precision not a valid u32".to_string(),
                        )
                    })?
                    .unwrap_or(0),
                logical_type: logical_type_raw.clone(),
            })
        }
        "TEXT" => Ok(FieldMeta::Text),
        _ => {
            let logical_type = logical_type_raw.filter(|s| s.as_str() != "NONE");
            if let Some(ref lt) = logical_type {
                tracing::debug!("Field logical type: {}", lt);
            }
            let scale = metadata.get("scale").and_then(|s| s.parse::<u32>().ok());
            Ok(FieldMeta::Other {
                logical_type,
                scale,
            })
        }
    }
}

pub trait ReadArrowValue<T>: Sized {
    fn read(self, array: &dyn Array, field: &Field, row_idx: usize) -> Result<(), ExtractError> {
        // Check if the value is NULL
        if array.is_null(row_idx) {
            return self.read_null();
        }

        match array.data_type() {
            DataType::Int16 => self.read_int16(
                &get_field_meta(field)?,
                get_value::<Int16Type>(array, row_idx)?,
            ),
            DataType::Int32 => self.read_int32(
                &get_field_meta(field)?,
                get_value::<Int32Type>(array, row_idx)?,
            ),
            DataType::Int8 => self.read_int8(
                &get_field_meta(field)?,
                get_value::<Int8Type>(array, row_idx)?,
            ),
            DataType::Int64 => self.read_int64(
                &get_field_meta(field)?,
                get_value::<Int64Type>(array, row_idx)?,
            ),
            DataType::Utf8 => self.read_utf8(
                &get_field_meta(field)?,
                get_byte_array_value::<Utf8Type>(array, row_idx)?,
            ),
            DataType::Binary => {
                self.read_binary(&get_field_meta(field)?, get_binary_value(array, row_idx)?)
            }
            DataType::Decimal128(precision, scale) => self.read_decimal128(
                &get_field_meta(field)?,
                get_value::<Decimal128Type>(array, row_idx)?,
                *precision,
                *scale,
            ),
            DataType::Date32 => self.read_date32(
                &get_field_meta(field)?,
                get_value::<Date32Type>(array, row_idx)?,
            ),
            DataType::Float32 => self.read_float32(
                &get_field_meta(field)?,
                get_value::<Float32Type>(array, row_idx)?,
            ),
            DataType::Float64 => self.read_float64(
                &get_field_meta(field)?,
                get_value::<Float64Type>(array, row_idx)?,
            ),
            DataType::Boolean => {
                let bool_array = array
                    .as_any()
                    .downcast_ref::<BooleanArray>()
                    .ok_or(ExtractError::DowncastError)?;
                self.read_boolean(&get_field_meta(field)?, bool_array.value(row_idx))
            }
            DataType::Time64(TimeUnit::Nanosecond) => self.read_time64_nanosecond(
                &get_field_meta(field)?,
                get_value::<Time64NanosecondType>(array, row_idx)?,
            ),
            DataType::Timestamp(TimeUnit::Nanosecond, _) => {
                // Handle timestamps with or without timezone - treat as Int64 nanoseconds
                self.read_int64(
                    &get_field_meta(field)?,
                    get_value::<TimestampNanosecondType>(array, row_idx)?,
                )
            }
            DataType::Struct(_) => {
                // Handle struct types (e.g., TIMESTAMP_LTZ with epoch/fraction/timezone fields)
                self.read_struct(&get_field_meta(field)?, array, row_idx)
            }
            _ => Err(ExtractError::UnsupportedArrowType(
                array.data_type().clone(),
            )),
        }
    }

    fn read_null(self) -> Result<(), ExtractError> {
        // Default implementation: do nothing for NULL values
        // Implementations can override this to set SQL_NULL_DATA
        Ok(())
    }
    fn read_int8(self, _field: &FieldMeta, _value: i8) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Int8))
    }
    fn read_int16(self, _field: &FieldMeta, _value: i16) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Int16))
    }
    fn read_int32(self, _field: &FieldMeta, _value: i32) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Int32))
    }
    fn read_int64(self, _field: &FieldMeta, _value: i64) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Int64))
    }
    fn read_utf8(self, _field: &FieldMeta, _value: &str) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Utf8))
    }

    fn read_binary(self, _field: &FieldMeta, _value: &[u8]) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Binary))
    }

    fn read_decimal128(
        self,
        _field: &FieldMeta,
        _value: i128,
        precision: u8,
        scale: i8,
    ) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Decimal128(
            precision, scale,
        )))
    }

    fn read_date32(self, _field: &FieldMeta, _value: i32) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Date32))
    }

    fn read_time64_nanosecond(self, _field: &FieldMeta, _value: i64) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Time64(
            TimeUnit::Nanosecond,
        )))
    }

    fn read_float32(self, _field: &FieldMeta, _value: f32) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Float32))
    }

    fn read_float64(self, _field: &FieldMeta, _value: f64) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Float64))
    }

    fn read_boolean(self, _field: &FieldMeta, _value: bool) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Boolean))
    }

    fn read_struct(
        self,
        _field: &FieldMeta,
        _array: &dyn Array,
        _row_idx: usize,
    ) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedArrowType(DataType::Struct(
            Vec::<arrow::datatypes::Field>::new().into(),
        )))
    }
}

impl<V: WriteValue<UBigInt>> ReadArrowValue<UBigInt> for V {
    fn read_int8(self, field: &FieldMeta, value: i8) -> Result<(), ExtractError> {
        read_u64(self, field, value as u64)
    }
    fn read_int16(self, field: &FieldMeta, value: i16) -> Result<(), ExtractError> {
        read_u64(self, field, value as u64)
    }
    fn read_int32(self, field: &FieldMeta, value: i32) -> Result<(), ExtractError> {
        read_u64(self, field, value as u64)
    }
    fn read_int64(self, field: &FieldMeta, value: i64) -> Result<(), ExtractError> {
        read_u64(self, field, value as u64)
    }
    fn read_utf8(self, _field: &FieldMeta, value: &str) -> Result<(), ExtractError> {
        // Parse string as u64 for large integers that don't fit in i64
        let parsed = value.parse::<u64>().map_err(|_| {
            ExtractError::ConversionError(format!("Failed to parse '{}' as u64", value))
        })?;
        self.write(parsed);
        Ok(())
    }
    fn read_decimal128(
        self,
        _field: &FieldMeta,
        value: i128,
        precision: u8,
        scale: i8,
    ) -> Result<(), ExtractError> {
        read_u128(self, value as u128, precision, scale)
    }
}

impl<V: WriteValue<Real>> ReadArrowValue<Real> for V {
    fn read_float32(self, _field: &FieldMeta, value: f32) -> Result<(), ExtractError> {
        self.write(value);
        Ok(())
    }
    fn read_float64(self, _field: &FieldMeta, value: f64) -> Result<(), ExtractError> {
        self.write(value as f32);
        Ok(())
    }
    fn read_int32(self, _field: &FieldMeta, value: i32) -> Result<(), ExtractError> {
        self.write(value as f32);
        Ok(())
    }
    fn read_int64(self, _field: &FieldMeta, value: i64) -> Result<(), ExtractError> {
        self.write(value as f32);
        Ok(())
    }
    fn read_boolean(self, _field: &FieldMeta, value: bool) -> Result<(), ExtractError> {
        self.write(if value { 1.0 } else { 0.0 });
        Ok(())
    }
}

pub trait WriteValue<T> {
    fn write(&self, value: T);
}

pub struct Contramap<V, T, U>
where
    V: WriteValue<T>,
{
    pub value: V,
    pub f: fn(U) -> T,
}

impl<V, T, U> Contramap<V, T, U>
where
    V: WriteValue<T>,
{
    pub fn new(value: V, f: fn(U) -> T) -> Self {
        Self { value, f }
    }
}

impl<V, T, U> WriteValue<U> for Contramap<V, T, U>
where
    V: WriteValue<T>,
{
    fn write(&self, value: U) {
        self.value.write((self.f)(value))
    }
}

pub struct Value<T> {
    pub value: *mut T,
}

impl<T: 'static> WriteValue<T> for Value<T> {
    fn write(&self, value: T) {
        unsafe { std::ptr::write(self.value, value) };
        #[cfg(debug_assertions)]
        if TypeId::of::<T>() == TypeId::of::<sql::Timestamp>() {
            let ts = unsafe { *(self.value as *const sql::Timestamp) };
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/rust_debug.log")
                .and_then(|mut f| {
                    writeln!(
                        f,
                        "DEBUG Value::write sql::Timestamp => {}-{}-{} {:02}:{:02}:{:02}.{:09}",
                        ts.year, ts.month, ts.day, ts.hour, ts.minute, ts.second, ts.fraction
                    )
                });
        }
    }
}

impl<T: 'static> Value<T> {
    pub fn new(value: *mut T) -> Self {
        Self { value }
    }
    pub fn contramap<U>(self, f: fn(U) -> T) -> Contramap<Self, T, U> {
        Contramap::new(self, f)
    }
}

pub struct Buffer<T> {
    pub data: *mut T,
    pub len: usize,
    pub str_len_or_ind: *mut sql::Len,
}

impl WriteValue<&str> for Buffer<sql::Char> {
    fn write(&self, value: &str) {
        if !self.str_len_or_ind.is_null() {
            unsafe { std::ptr::write(self.str_len_or_ind, value.len() as sql::Len) };
        }

        // Calculate how many bytes we can actually copy (reserve 1 byte for null terminator)
        let bytes_to_copy = if self.len > 0 {
            std::cmp::min(self.len - 1, value.len())
        } else {
            0
        };

        unsafe {
            // Copy the string data
            std::ptr::copy_nonoverlapping(
                value.as_ptr() as *const sql::Char,
                self.data,
                bytes_to_copy,
            );

            // Add null terminator if there's space
            if self.len > 0 {
                std::ptr::write(self.data.add(bytes_to_copy), 0);
            }
        };
    }
}

// Removed - implementation is below as ReadArrowValue<&str> for Buffer<sql::Char>

impl<T> Buffer<T> {
    pub fn new(data: *mut T, len: usize, str_len_or_ind: *mut sql::Len) -> Self {
        Self {
            data,
            len,
            str_len_or_ind,
        }
    }
}

trait TextBuffer {
    fn write_value(&self, value: &str);
    fn write_null(&self);
}

impl TextBuffer for Buffer<sql::Char> {
    fn write_value(&self, value: &str) {
        self.write(value);
    }

    fn write_null(&self) {
        if !self.str_len_or_ind.is_null() {
            unsafe { std::ptr::write(self.str_len_or_ind, sql::NULL_DATA) };
        }
        if self.len > 0 {
            unsafe { std::ptr::write(self.data, 0) };
        }
    }
}

impl TextBuffer for Buffer<sql::WChar> {
    fn write_value(&self, value: &str) {
        // On macOS, wchar_t is 4 bytes (UTF-32), but sql::WChar is u16
        // We need to write UTF-32 on macOS and UTF-16 on Windows
        #[cfg(target_os = "macos")]
        {
            // Write as UTF-32 (4 bytes per character)
            let utf32: Vec<u32> = value.chars().map(|c| c as u32).collect();
            let wchar_size = 4usize;

            if !self.str_len_or_ind.is_null() {
                unsafe {
                    std::ptr::write(self.str_len_or_ind, (utf32.len() * wchar_size) as sql::Len)
                };
            }

            if self.len == 0 {
                return;
            }

            // self.len is in bytes, convert to character count
            let max_chars = (self.len / 2).saturating_sub(1); // Divide by 2 because sql::WChar is u16
            let copy_len = utf32.len().min(max_chars);

            unsafe {
                let data_ptr = self.data as *mut u32;
                if copy_len > 0 {
                    std::ptr::copy_nonoverlapping(utf32.as_ptr(), data_ptr, copy_len);
                }
                std::ptr::write(data_ptr.add(copy_len), 0);
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            let wchar_size = mem::size_of::<sql::WChar>();
            let utf16: Vec<sql::WChar> = value.encode_utf16().map(|c| c as sql::WChar).collect();

            if !self.str_len_or_ind.is_null() {
                unsafe {
                    std::ptr::write(self.str_len_or_ind, (utf16.len() * wchar_size) as sql::Len)
                };
            }

            if self.len == 0 {
                return;
            }

            let max_chars = self.len.saturating_sub(1);
            let copy_len = utf16.len().min(max_chars);

            unsafe {
                if copy_len > 0 {
                    std::ptr::copy_nonoverlapping(utf16.as_ptr(), self.data, copy_len);
                }
                std::ptr::write(self.data.add(copy_len), 0);
            }
        }
    }

    fn write_null(&self) {
        if !self.str_len_or_ind.is_null() {
            unsafe { std::ptr::write(self.str_len_or_ind, sql::NULL_DATA) };
        }
        if self.len > 0 {
            #[cfg(target_os = "macos")]
            unsafe {
                std::ptr::write(self.data as *mut u32, 0)
            };
            #[cfg(not(target_os = "macos"))]
            unsafe {
                std::ptr::write(self.data, 0)
            };
        }
    }
}

fn decimal_to_string(value: i128, scale: u32) -> String {
    if scale == 0 {
        return value.to_string();
    }

    let scale_dec = 10_i128.pow(scale);
    let whole = value / scale_dec;
    let decimal = value % scale_dec;
    format!(
        "{}.{:0width$}",
        whole,
        decimal.abs(),
        width = scale as usize
    )
}

fn drop_decimal_digits_i64(value: i64, scale: u32) -> i64 {
    if scale == 0 {
        return value;
    }
    let scale_dec = 10_i64.pow(scale);
    value / scale_dec
}

fn drop_decimal_digits_u64(value: u64, scale: u32) -> u64 {
    if scale == 0 {
        return value;
    }
    let scale_dec = 10_u64.pow(scale);
    value / scale_dec
}

fn format_time_from_decimal(value: i128, scale: u32) -> String {
    // Convert decimal value to nanoseconds
    // If scale < 9, we need to multiply to get nanoseconds
    // If scale >= 9, we need to divide to get nanoseconds
    let nanos = if scale >= 9 {
        let divisor = 10_i128.pow(scale - 9);
        value / divisor
    } else {
        let factor = 10_i128.pow(9 - scale);
        value * factor
    };
    format_time_from_nanos(nanos)
}

fn format_time_from_nanos(nanos: i128) -> String {
    let day_ns = 86_400_000_000_000i128;
    let nanos = ((nanos % day_ns) + day_ns) % day_ns;
    let hour = (nanos / 3_600_000_000_000) as i64;
    let minute = ((nanos / 60_000_000_000) % 60) as i64;
    let second = ((nanos / 1_000_000_000) % 60) as i64;
    let fractional = (nanos % 1_000_000_000) as i64;

    let mut result = format!("{:02}:{:02}:{:02}", hour, minute, second);
    if fractional != 0 {
        let mut frac_str = format!("{:09}", fractional.abs());
        while frac_str.ends_with('0') {
            frac_str.pop();
        }
        result.push('.');
        result.push_str(&frac_str);
    }
    result
}

fn format_float_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value.is_sign_positive() {
            "INFINITY".to_string()
        } else {
            "-INFINITY".to_string()
        };
    }
    trim_numeric_string(value.to_string())
}

fn format_float32_string(value: f32) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value.is_sign_positive() {
            "INFINITY".to_string()
        } else {
            "-INFINITY".to_string()
        };
    }
    // 7 decimal places capture the full precision of a 32-bit float.
    let formatted = format!("{value:.7}");
    trim_numeric_string(formatted)
}

fn trim_numeric_string(mut value: String) -> String {
    if value.eq_ignore_ascii_case("-0") {
        return "0".to_string();
    }

    if let Some(exp_idx) = value.find(['e', 'E']) {
        let exponent = value.split_off(exp_idx);
        let mut mantissa = value;
        trim_fractional_part(&mut mantissa);
        mantissa.push_str(&exponent);
        mantissa
    } else {
        trim_fractional_part(&mut value);
        value
    }
}

fn trim_fractional_part(value: &mut String) {
    if let Some(dot_idx) = value.find('.') {
        let mut trim_idx = value.len();
        while trim_idx > dot_idx && value.as_bytes()[trim_idx - 1] == b'0' {
            trim_idx -= 1;
        }
        if trim_idx > dot_idx + 1 {
            value.truncate(trim_idx);
        } else {
            value.truncate(dot_idx);
        }
    }
    if value == "-0" {
        *value = "0".to_string();
    }
}

impl<T> ReadArrowValue<&str> for T
where
    T: TextBuffer,
{
    fn read_null(self) -> Result<(), ExtractError> {
        self.write_null();
        Ok(())
    }

    fn read_utf8(self, _field: &FieldMeta, value: &str) -> Result<(), ExtractError> {
        self.write_value(value);
        Ok(())
    }

    fn read_binary(self, _field: &FieldMeta, value: &[u8]) -> Result<(), ExtractError> {
        // Convert binary to hex string for display
        let hex: String = value.iter().map(|b| format!("{:02X}", b)).collect();
        self.write_value(&hex);
        Ok(())
    }

    fn read_time64_nanosecond(self, _field: &FieldMeta, value: i64) -> Result<(), ExtractError> {
        let s = format_time_from_nanos(value as i128);
        self.write_value(&s);
        Ok(())
    }

    fn read_int8(self, field: &FieldMeta, value: i8) -> Result<(), ExtractError> {
        self.read_int64(field, value as i64)
    }

    fn read_int16(self, field: &FieldMeta, value: i16) -> Result<(), ExtractError> {
        self.read_int64(field, value as i64)
    }

    fn read_int32(self, field: &FieldMeta, value: i32) -> Result<(), ExtractError> {
        if let FieldMeta::Other {
            logical_type: Some(logical_type),
            ..
        } = field
        {
            if logical_type.eq_ignore_ascii_case("DATE") {
                let date = date_from_days(value);
                let s = sql_date_to_string(&date);
                self.write_value(&s);
                return Ok(());
            }
        }
        self.read_int64(field, value as i64)
    }

    fn read_int64(self, field: &FieldMeta, value: i64) -> Result<(), ExtractError> {
        if let FieldMeta::Fixed {
            scale,
            logical_type,
            ..
        } = field
        {
            if matches!(logical_type.as_deref(), Some("TIME")) {
                let s = format_time_from_decimal(value as i128, *scale);
                self.write_value(&s);
                return Ok(());
            }
            let s = decimal_to_string(value as i128, *scale);
            self.write_value(&s);
            Ok(())
        } else {
            // For timestamps (logical types), format appropriately
            let (logical_type, scale_opt) = match field {
                FieldMeta::Other {
                    logical_type,
                    scale,
                } => (logical_type.as_deref(), *scale),
                FieldMeta::Text => (None, None),
                FieldMeta::Fixed { .. } => (None, None),
            };
            tracing::debug!(
                "read_int64 logical_type {:?} raw={} scale={:?}",
                logical_type,
                value,
                scale_opt
            );
            let (secs, nanos) = match logical_type {
                Some("TIMESTAMP_LTZ") | Some("TIMESTAMP_NTZ") | Some("TIMESTAMP_TZ") => {
                    let scale = scale_opt.unwrap_or(0);
                    let parts = scaled_seconds_to_parts(value as i128, scale);
                    if let Ok(mut f) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/rust_debug.log")
                    {
                        let _ = writeln!(
                            f,
                            "read_int64 {} raw={} declared_scale={} -> secs={} nanos={}",
                            logical_type.unwrap_or("TIMESTAMP"),
                            value,
                            scale,
                            parts.0,
                            parts.1
                        );
                    }
                    parts
                }
                _ => split_timestamp_nanos(value),
            };
            let s = match logical_type {
                Some("TIMESTAMP_LTZ") => format_timestamp_ltz_from_parts(secs, nanos),
                Some("TIMESTAMP_NTZ") => format_timestamp_ntz_from_parts(secs, nanos),
                Some("TIMESTAMP_TZ") => format_timestamp_ntz_from_parts(secs, nanos),
                _ => format_timestamp_ntz_from_parts(secs, nanos),
            };
            tracing::debug!("read_int64 formatted={s}");
            self.write_value(&s);
            Ok(())
        }
    }

    fn read_decimal128(
        self,
        field: &FieldMeta,
        value: i128,
        _precision: u8,
        scale: i8,
    ) -> Result<(), ExtractError> {
        match field {
            FieldMeta::Fixed {
                scale: field_scale,
                logical_type: Some(logical_type),
                ..
            } if logical_type == "TIMESTAMP_LTZ" => {
                let (secs, nanos) = scaled_seconds_to_parts(value, *field_scale);
                let s = format_timestamp_ltz_from_parts(secs, nanos);
                self.write_value(&s);
                Ok(())
            }
            FieldMeta::Fixed {
                scale: field_scale,
                logical_type: Some(logical_type),
                ..
            } if logical_type == "TIMESTAMP_NTZ" => {
                let (secs, nanos) = scaled_seconds_to_parts(value, *field_scale);
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/rust_debug.log")
                {
                    let _ = writeln!(
                        f,
                        "read_decimal128 TIMESTAMP_NTZ value={} scale={} -> secs={} nanos={}",
                        value, field_scale, secs, nanos
                    );
                }
                let s = format_timestamp_ntz_from_parts(secs, nanos);
                self.write_value(&s);
                Ok(())
            }
            FieldMeta::Other {
                logical_type: Some(logical_type),
                ..
            } if logical_type == "TIMESTAMP_LTZ" => {
                let (secs, nanos) = scaled_seconds_to_parts(value, scale as u32);
                let s = format_timestamp_ltz_from_parts(secs, nanos);
                self.write_value(&s);
                Ok(())
            }
            FieldMeta::Other {
                logical_type: Some(logical_type),
                ..
            } if logical_type == "TIMESTAMP_NTZ" => {
                let (secs, nanos) = scaled_seconds_to_parts(value, scale as u32);
                let s = format_timestamp_ntz_from_parts(secs, nanos);
                self.write_value(&s);
                Ok(())
            }
            _ => {
                let s = decimal_to_string(value, scale as u32);
                self.write_value(&s);
                Ok(())
            }
        }
    }

    fn read_date32(self, _field: &FieldMeta, value: i32) -> Result<(), ExtractError> {
        let date = date_from_days(value);
        let s = sql_date_to_string(&date);
        self.write_value(&s);
        Ok(())
    }

    fn read_float32(self, _field: &FieldMeta, value: f32) -> Result<(), ExtractError> {
        let float_str = format_float32_string(value);
        self.write_value(&float_str);
        Ok(())
    }

    fn read_float64(self, _field: &FieldMeta, value: f64) -> Result<(), ExtractError> {
        let float_str = format_float_string(value);
        self.write_value(&float_str);
        Ok(())
    }

    fn read_struct(
        self,
        field: &FieldMeta,
        array: &dyn Array,
        row_idx: usize,
    ) -> Result<(), ExtractError> {
        // Handle struct types - specifically TIMESTAMP_LTZ/TIMESTAMP_TZ with epoch/fraction/timezone fields
        use arrow::array::StructArray;

        let struct_array = array
            .as_any()
            .downcast_ref::<StructArray>()
            .ok_or(ExtractError::DowncastError)?;
        tracing::debug!("TIMESTAMP struct fields: {:?}", struct_array.data_type());

        let logical_type = match field {
            FieldMeta::Fixed { logical_type, .. } => {
                logical_type.as_ref().map(|s| s.to_ascii_uppercase())
            }
            FieldMeta::Other { logical_type, .. } => {
                logical_type.as_ref().map(|s| s.to_ascii_uppercase())
            }
            FieldMeta::Text => None,
        };

        // Check if this is a TIMESTAMP_LTZ struct (has "epoch" field)
        let epoch_col = struct_array.column_by_name("epoch");
        if let Some(epoch_array) = epoch_col {
            // Extract epoch value (seconds since Unix epoch)
            let epoch_seconds = if epoch_array.is_null(row_idx) {
                return self.read_null();
            } else {
                get_value::<Int64Type>(epoch_array, row_idx)?
            };

            // Determine if the struct has a fraction field and capture scale metadata
            let (has_fraction_field, epoch_scale) =
                if let arrow::datatypes::DataType::Struct(fields) = struct_array.data_type() {
                    let has_fraction = fields.iter().any(|f| f.name() == "fraction");
                    let scale = fields
                        .iter()
                        .find(|f| f.name() == "epoch")
                        .and_then(|f| f.metadata().get("scale"))
                        .and_then(|val| val.parse::<u32>().ok())
                        .unwrap_or(0);
                    tracing::debug!(
                        "struct timestamp field metadata: has_fraction={} scale={}",
                        has_fraction,
                        scale
                    );
                    (has_fraction, scale)
                } else {
                    (false, 0)
                };

            // Check for fraction field to get additional nanoseconds
            let fraction_ns = if let Some(fraction_array) = struct_array.column_by_name("fraction")
            {
                if !fraction_array.is_null(row_idx) {
                    get_value::<Int32Type>(fraction_array, row_idx)? as i64
                } else {
                    0
                }
            } else {
                0
            };

            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/rust_debug.log")
                .and_then(|mut f| {
                    writeln!(
                        f,
                        "DEBUG read_struct string logical={:?} epoch={} fraction={} scale={} has_fraction={}",
                        logical_type,
                        epoch_seconds,
                        fraction_ns,
                        epoch_scale,
                        has_fraction_field
                    )
                });

            // Check for timezone field (offset in minutes from UTC)
            // Snowflake uses "timezone" field name in Arrow structs
            let timezone_offset_minutes = if let Some(timezone_array) =
                struct_array.column_by_name("timezone")
            {
                if !timezone_array.is_null(row_idx) {
                    get_value::<Int32Type>(timezone_array, row_idx)? as i64
                } else {
                    0
                }
            } else {
                // Fallback: try "timezone_offset_minutes" if "timezone" doesn't exist
                if let Some(timezone_array) = struct_array.column_by_name("timezone_offset_minutes")
                {
                    if !timezone_array.is_null(row_idx) {
                        get_value::<Int32Type>(timezone_array, row_idx)? as i64
                    } else {
                        0
                    }
                } else {
                    0
                }
            };

            // WORKAROUND: Snowflake has a bug where it splits timestamps incorrectly
            // For example, 1512037025 seconds gets split as epoch=1, fraction=512037025
            // Handle Snowflake's "misplaced decimal" bug
            // When epoch is small (< 100) and fraction is large (> 100M), the epoch was split incorrectly
            // E.g., epoch=1, fraction=512065825 means 1512065825 seconds total
            // For precisions <= 3 Snowflake may encode fractional digits directly in epoch.
            // When there's no explicit fraction column, use the scale metadata to split seconds/fraction.
            let (mut total_seconds, mut actual_fraction_ns) = if has_fraction_field {
                // Fraction field already arrives in nanoseconds regardless of declared precision.
                (epoch_seconds, fraction_ns)
            } else if epoch_scale > 0 {
                let scale_pow = 10_i128.pow(epoch_scale.min(12) as u32);

                let frac_digits = if epoch_scale <= 9 {
                    let divisor = 10_i128.pow(9 - epoch_scale as u32);
                    (fraction_ns as i128) / divisor
                } else {
                    let multiplier = 10_i128.pow(epoch_scale as u32 - 9);
                    (fraction_ns as i128) * multiplier
                };

                let combined = (epoch_seconds as i128)
                    .checked_mul(scale_pow)
                    .and_then(|v| v.checked_add(frac_digits))
                    .ok_or_else(|| {
                        ExtractError::ConversionError(
                            "TIMESTAMP struct epoch/fraction overflow (scale combine)".to_string(),
                        )
                    })?;

                let micros = combined;
                let secs = micros.div_euclid(1_000_000);
                let rem_micros = micros.rem_euclid(1_000_000);

                let secs_i64 = i64::try_from(secs).map_err(|_| {
                    ExtractError::ConversionError(
                        "TIMESTAMP struct seconds exceed i64 range after scaling".to_string(),
                    )
                })?;
                let nanos_i64 = rem_micros
                    .checked_mul(1_000)
                    .and_then(|v| i64::try_from(v).ok())
                    .ok_or_else(|| {
                        ExtractError::ConversionError(
                            "TIMESTAMP struct fractional nanos overflow".to_string(),
                        )
                    })?;

                (secs_i64, nanos_i64)
            } else {
                // Standard path: epoch is already in seconds, fraction array carries sub-second part
                let snowflake_misplaced_decimal = (0..100).contains(&epoch_seconds)
                    && epoch_seconds > 0
                    && fraction_ns > 100_000_000;
                let adjusted_total = if snowflake_misplaced_decimal {
                    epoch_seconds * 1_000_000_000 + fraction_ns
                } else {
                    epoch_seconds
                };
                let adjusted_fraction = if snowflake_misplaced_decimal {
                    0i64
                } else {
                    fraction_ns
                };

                (adjusted_total, adjusted_fraction)
            };

            // Guard against overflow in the Snowflake misplaced-decimal workaround.
            // When epoch_seconds represents real seconds (|epoch_seconds| > 100),
            // multiplying by 1e9 should never happen. If it does (and overflows),
            // reset back to the original epoch/fraction pair.
            const OVERFLOW_GUARD_SECS: i64 = 1_000_000_000_000; // ~year 33658
            if has_fraction_field
                && epoch_scale == 0
                && epoch_seconds.abs() > 100
                && epoch_seconds.abs() < OVERFLOW_GUARD_SECS
                && total_seconds.abs() >= OVERFLOW_GUARD_SECS
            {
                tracing::warn!(
                    "Resetting TIMESTAMP struct split due to overflow: epoch={} fraction={} total_seconds={} actual_fraction_ns={}",
                    epoch_seconds,
                    fraction_ns,
                    total_seconds,
                    actual_fraction_ns
                );
                total_seconds = epoch_seconds;
                actual_fraction_ns = fraction_ns;
            }

            // Convert to nanoseconds since epoch
            // The epoch from Snowflake is in UTC, but we need to display in session timezone
            let utc_ns_total: i128 =
                (total_seconds as i128) * 1_000_000_000i128 + (actual_fraction_ns as i128);

            #[allow(unused)]
            {
                use std::io::Write;
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/rust_debug.log")
                    .and_then(|mut f| {
                        writeln!(
                            f,
                            "DEBUG read_struct epoch={} fraction={} tz_minutes={} total_secs={} actual_frac_ns={}",
                            epoch_seconds, fraction_ns, timezone_offset_minutes, total_seconds, actual_fraction_ns
                        )
                    });
            }

            // Format timestamp using logical type when available, otherwise fall back
            let has_timezone_field = struct_array.column_by_name("timezone").is_some()
                || struct_array
                    .column_by_name("timezone_offset_minutes")
                    .is_some();
            let normalized_offset = normalize_tz_offset(timezone_offset_minutes);
            let tz_format = get_read_timestamp_tz_format();
            let timestamp_str = match logical_type.as_deref() {
                Some("TIMESTAMP_NTZ") => {
                    format_timestamp_ntz_from_parts(total_seconds, actual_fraction_ns as u32)
                }
                Some("TIMESTAMP_TZ") => format_timestamp_tz_from_parts(
                    total_seconds,
                    actual_fraction_ns as u32,
                    normalized_offset,
                    tz_format,
                ),
                Some("TIMESTAMP_LTZ") => {
                    format_timestamp_ltz_from_parts(total_seconds, actual_fraction_ns as u32)
                }
                _ => {
                    if has_timezone_field {
                        format_timestamp_tz_from_parts(
                            total_seconds,
                            actual_fraction_ns as u32,
                            normalized_offset,
                            tz_format,
                        )
                    } else {
                        format_timestamp_ltz_from_parts(total_seconds, actual_fraction_ns as u32)
                    }
                }
            };
            tracing::debug!("TIMESTAMP struct rendered {}", timestamp_str);
            self.write_value(&timestamp_str);
            Ok(())
        } else {
            // Unknown struct type - try to format as JSON-like string
            Err(ExtractError::UnsupportedArrowType(
                array.data_type().clone(),
            ))
        }
    }

    fn read_boolean(self, _field: &FieldMeta, value: bool) -> Result<(), ExtractError> {
        let bool_str = if value { "1" } else { "0" };
        self.write_value(bool_str);
        Ok(())
    }
}

fn read_u64<V: WriteValue<UBigInt>>(
    sink: V,
    field: &FieldMeta,
    value: u64,
) -> Result<(), ExtractError> {
    if let FieldMeta::Fixed { scale, .. } = field {
        sink.write(drop_decimal_digits_u64(value, *scale) as UBigInt);
        Ok(())
    } else {
        Err(ExtractError::UnsupportedFieldMeta(
            field.clone(),
            DataType::Int64,
        ))
    }
}

fn read_u128<V: WriteValue<UBigInt>>(
    sink: V,
    value: u128,
    _precision: u8,
    scale: i8,
) -> Result<(), ExtractError> {
    let scale_dec = 10_u128.pow(scale as u32);
    let whole = value / scale_dec;
    sink.write(whole as UBigInt);
    Ok(())
}

fn read_i64<V: WriteValue<SBigInt>>(
    sink: V,
    field: &FieldMeta,
    value: i64,
) -> Result<(), ExtractError> {
    if let FieldMeta::Fixed { scale, .. } = field {
        sink.write(drop_decimal_digits_i64(value, *scale) as SBigInt);
        Ok(())
    } else {
        Err(ExtractError::UnsupportedFieldMeta(
            field.clone(),
            DataType::Int64,
        ))
    }
}

fn read_i128<V: WriteValue<SBigInt>>(
    sink: V,
    value: i128,
    _precision: u8,
    scale: i8,
) -> Result<(), ExtractError> {
    let scale_dec = 10_i128.pow(scale as u32);
    let whole = value / scale_dec;
    sink.write(whole as SBigInt);
    Ok(())
}

fn read_f64<V: WriteValue<Double>>(
    sink: V,
    field: &FieldMeta,
    value: i128,
) -> Result<(), ExtractError> {
    if let FieldMeta::Fixed { scale, .. } = field {
        // TODO: Don't parse to string, parse to f64 directly
        let value = decimal_to_string(value, *scale);
        sink.write(
            value
                .parse::<Double>()
                .map_err(|_| ExtractError::ConversionError("value not a valid f64".to_string()))?,
        );
        Ok(())
    } else {
        Err(ExtractError::UnsupportedFieldMeta(
            field.clone(),
            DataType::Int64,
        ))
    }
}

impl<V: WriteValue<SBigInt>> ReadArrowValue<SBigInt> for V {
    fn read_int8(self, _field: &FieldMeta, value: i8) -> Result<(), ExtractError> {
        read_i64(self, _field, value as i64)
    }
    fn read_int16(self, _field: &FieldMeta, value: i16) -> Result<(), ExtractError> {
        read_i64(self, _field, value as i64)
    }
    fn read_int32(self, _field: &FieldMeta, value: i32) -> Result<(), ExtractError> {
        read_i64(self, _field, value as i64)
    }
    fn read_int64(self, field: &FieldMeta, value: i64) -> Result<(), ExtractError> {
        read_i64(self, field, value)
    }
    fn read_utf8(self, _field: &FieldMeta, value: &str) -> Result<(), ExtractError> {
        // Parse string as i64 for large integers that don't fit in i64
        let parsed = value.parse::<i64>().map_err(|_| {
            ExtractError::ConversionError(format!("Failed to parse '{}' as i64", value))
        })?;
        self.write(parsed);
        Ok(())
    }
    fn read_decimal128(
        self,
        _field: &FieldMeta,
        value: i128,
        precision: u8,
        scale: i8,
    ) -> Result<(), ExtractError> {
        read_i128(self, value, precision, scale)
    }
    fn read_time64_nanosecond(self, field: &FieldMeta, value: i64) -> Result<(), ExtractError> {
        // Time64 nanoseconds can be read as i64
        read_i64(self, field, value)
    }
    fn read_float32(self, _field: &FieldMeta, value: f32) -> Result<(), ExtractError> {
        self.write(value as i64);
        Ok(())
    }
    fn read_float64(self, _field: &FieldMeta, value: f64) -> Result<(), ExtractError> {
        self.write(value as i64);
        Ok(())
    }
    fn read_boolean(self, _field: &FieldMeta, value: bool) -> Result<(), ExtractError> {
        self.write(if value { 1 } else { 0 });
        Ok(())
    }
}

impl<V: WriteValue<Double>> ReadArrowValue<Double> for V {
    fn read_int8(self, _field: &FieldMeta, value: i8) -> Result<(), ExtractError> {
        read_f64(self, _field, value as i128)?;
        Ok(())
    }
    fn read_int16(self, _field: &FieldMeta, value: i16) -> Result<(), ExtractError> {
        read_f64(self, _field, value as i128)?;
        Ok(())
    }
    fn read_int32(self, _field: &FieldMeta, value: i32) -> Result<(), ExtractError> {
        read_f64(self, _field, value as i128)?;
        Ok(())
    }
    fn read_int64(self, field: &FieldMeta, value: i64) -> Result<(), ExtractError> {
        read_f64(self, field, value as i128)?;
        Ok(())
    }
    fn read_decimal128(
        self,
        field: &FieldMeta,
        value: i128,
        _precision: u8,
        _scale: i8,
    ) -> Result<(), ExtractError> {
        read_f64(self, field, value)?;
        Ok(())
    }

    fn read_float32(self, _field: &FieldMeta, value: f32) -> Result<(), ExtractError> {
        self.write(value as f64);
        Ok(())
    }

    fn read_float64(self, _field: &FieldMeta, value: f64) -> Result<(), ExtractError> {
        self.write(value);
        Ok(())
    }
    fn read_boolean(self, _field: &FieldMeta, value: bool) -> Result<(), ExtractError> {
        self.write(if value { 1.0 } else { 0.0 });
        Ok(())
    }
}

impl<V: WriteValue<sql::Timestamp>> ReadArrowValue<sql::Timestamp> for V {
    fn read_int64(self, field: &FieldMeta, value: i64) -> Result<(), ExtractError> {
        let scale_opt = match field {
            FieldMeta::Fixed { scale, .. } => Some(*scale),
            FieldMeta::Other { scale, .. } => *scale,
            FieldMeta::Text => None,
        };
        let (secs, nanos) = if let Some(scale) = scale_opt {
            scaled_seconds_to_parts(value as i128, scale)
        } else {
            split_timestamp_nanos(value)
        };
        self.write(match field.logical_type() {
            Some(logical) if logical.eq_ignore_ascii_case("TIMESTAMP_NTZ") => {
                timestamp_ntz_to_sql_timestamp(secs, nanos)
            }
            _ => timestamp_from_local_parts(secs, nanos),
        });
        Ok(())
    }

    fn read_decimal128(
        self,
        field: &FieldMeta,
        value: i128,
        _precision: u8,
        scale: i8,
    ) -> Result<(), ExtractError> {
        let scale_u32 = match field {
            FieldMeta::Fixed { scale, .. } => *scale,
            _ => scale.max(0) as u32,
        };
        let (secs, nanos) = scaled_seconds_to_parts(value, scale_u32);
        self.write(match field.logical_type() {
            Some(logical) if logical.eq_ignore_ascii_case("TIMESTAMP_NTZ") => {
                timestamp_ntz_to_sql_timestamp(secs, nanos)
            }
            _ => timestamp_from_local_parts(secs, nanos),
        });
        Ok(())
    }

    fn read_null(self) -> Result<(), ExtractError> {
        self.write(sql::Timestamp {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            fraction: 0,
        });
        Ok(())
    }

    fn read_struct(
        self,
        field: &FieldMeta,
        array: &dyn Array,
        row_idx: usize,
    ) -> Result<(), ExtractError> {
        // Handle TIMESTAMP_LTZ struct (has "epoch" and "fraction" fields)
        use arrow::array::StructArray;

        let struct_array = array
            .as_any()
            .downcast_ref::<StructArray>()
            .ok_or(ExtractError::DowncastError)?;

        // Check if this is a TIMESTAMP_LTZ struct (has "epoch" field)
        let epoch_col = struct_array.column_by_name("epoch");
        if let Some(epoch_array) = epoch_col {
            // Extract epoch value (seconds since Unix epoch)
            let epoch_seconds = if epoch_array.is_null(row_idx) {
                return self.read_null();
            } else {
                get_value::<Int64Type>(epoch_array, row_idx)?
            };

            // Determine fraction field/scale metadata
            let (has_fraction_field, epoch_scale) =
                if let arrow::datatypes::DataType::Struct(fields) = struct_array.data_type() {
                    let has_fraction = fields.iter().any(|f| f.name() == "fraction");
                    let scale = fields
                        .iter()
                        .find(|f| f.name() == "epoch")
                        .and_then(|f| f.metadata().get("scale"))
                        .and_then(|val| val.parse::<u32>().ok())
                        .unwrap_or(0);
                    (has_fraction, scale)
                } else {
                    (false, 0)
                };

            // Fraction column (if present)
            let fraction_ns = if let Some(fraction_array) = struct_array.column_by_name("fraction")
            {
                if !fraction_array.is_null(row_idx) {
                    get_value::<Int32Type>(fraction_array, row_idx)? as i64
                } else {
                    0
                }
            } else {
                0
            };

            let (total_seconds, actual_fraction_ns) = if has_fraction_field {
                (epoch_seconds, fraction_ns)
            } else if epoch_scale > 0 {
                tracing::debug!(
                    "decode_scaled_epoch inputs epoch={} fraction={} scale={}",
                    epoch_seconds,
                    fraction_ns,
                    epoch_scale
                );
                decode_scaled_epoch(epoch_seconds, fraction_ns, epoch_scale)
            } else {
                (epoch_seconds, 0)
            };

            let secs = total_seconds;
            let nanos = actual_fraction_ns.rem_euclid(1_000_000_000) as u32;

            let ts = match field.logical_type() {
                Some(logical) if logical.eq_ignore_ascii_case("TIMESTAMP_NTZ") => {
                    let _ = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/rust_debug.log")
                        .and_then(|mut f| {
                            writeln!(
                                f,
                                "DEBUG read_struct logical_type=TIMESTAMP_NTZ secs={} nanos={}",
                                secs, nanos
                            )
                        });
                    timestamp_ntz_to_sql_timestamp(secs, nanos)
                }
                _ => timestamp_from_local_parts(secs, nanos),
            };
            self.write(ts);
            Ok(())
        } else {
            Err(ExtractError::UnsupportedArrowType(
                array.data_type().clone(),
            ))
        }
    }
}

impl<V: WriteValue<sql::Date>> ReadArrowValue<sql::Date> for V {
    fn read_date32(self, _field: &FieldMeta, value: i32) -> Result<(), ExtractError> {
        self.write(date_from_days(value));
        Ok(())
    }

    fn read_null(self) -> Result<(), ExtractError> {
        self.write(sql::Date {
            year: 0,
            month: 0,
            day: 0,
        });
        Ok(())
    }
}

impl<V: WriteValue<sql::Time>> ReadArrowValue<sql::Time> for V {
    fn read_time64_nanosecond(self, _field: &FieldMeta, value: i64) -> Result<(), ExtractError> {
        self.write(time_from_ns(value));
        Ok(())
    }

    fn read_int32(self, field: &FieldMeta, value: i32) -> Result<(), ExtractError> {
        // TIME values can come as Int32 with scale (e.g., scale=0 means seconds)
        if let FieldMeta::Fixed {
            scale,
            logical_type,
            ..
        } = field
        {
            if matches!(logical_type.as_deref(), Some("TIME")) {
                // Convert from scaled integer to nanoseconds
                let nanos = if *scale >= 9 {
                    let divisor = 10_i64.pow(*scale - 9);
                    (value as i64) / divisor
                } else {
                    let factor = 10_i64.pow(9 - *scale);
                    (value as i64) * factor
                };
                self.write(time_from_ns(nanos));
                return Ok(());
            }
        }
        Err(ExtractError::UnsupportedArrowType(DataType::Int32))
    }

    fn read_int64(self, field: &FieldMeta, value: i64) -> Result<(), ExtractError> {
        // TIME values can come as Int64 with scale
        if let FieldMeta::Fixed {
            scale,
            logical_type,
            ..
        } = field
        {
            if matches!(logical_type.as_deref(), Some("TIME")) {
                // Convert from scaled integer to nanoseconds
                let nanos = if *scale >= 9 {
                    let divisor = 10_i64.pow(*scale - 9);
                    value / divisor
                } else {
                    let factor = 10_i64.pow(9 - *scale);
                    value * factor
                };
                self.write(time_from_ns(nanos));
                return Ok(());
            }
        }
        Err(ExtractError::UnsupportedArrowType(DataType::Int64))
    }

    fn read_null(self) -> Result<(), ExtractError> {
        // Write a zero time - the indicator will be set separately by the caller
        self.write(sql::Time {
            hour: 0,
            minute: 0,
            second: 0,
        });
        Ok(())
    }
}

fn date_from_days(days: i32) -> sql::Date {
    const UNIX_EPOCH_DAYS_FROM_CE: i32 = 719_163;
    let days_from_ce = match (UNIX_EPOCH_DAYS_FROM_CE as i64).checked_add(days as i64) {
        Some(val) if val >= i32::MIN as i64 && val <= i32::MAX as i64 => val as i32,
        _ => {
            return sql::Date {
                year: 0,
                month: 0,
                day: 0,
            };
        }
    };

    if let Some(date) = NaiveDate::from_num_days_from_ce_opt(days_from_ce) {
        let year = snowflake_year(date.year());
        sql::Date {
            year: year as i16,
            month: date.month() as u16,
            day: date.day() as u16,
        }
    } else {
        sql::Date {
            year: 0,
            month: 0,
            day: 0,
        }
    }
}

fn sql_date_to_string(date: &sql::Date) -> String {
    if date.year == 0 && date.month == 0 && date.day == 0 {
        "0000-00-00".to_string()
    } else {
        let year_str = format_year_padded(date.year as i32);
        format!("{year_str}-{:02}-{:02}", date.month, date.day)
    }
}

fn snowflake_year(year: i32) -> i32 {
    if year >= 1 { year } else { year - 1 }
}

fn format_year_padded(year: i32) -> String {
    if year >= 0 {
        format!("{year:04}")
    } else {
        format!("-{:04}", (-year))
    }
}

fn time_from_ns(value: i64) -> sql::Time {
    let day_ns = 86_400_000_000_000i64;
    let nanos = ((value % day_ns) + day_ns) % day_ns;
    let hour = (nanos / 3_600_000_000_000) as u16;
    let minute = ((nanos / 60_000_000_000) % 60) as u16;
    let second = ((nanos / 1_000_000_000) % 60) as u16;
    sql::Time {
        hour,
        minute,
        second,
    }
}

fn scaled_seconds_to_parts(value: i128, scale: u32) -> (i64, u32) {
    if scale == 0 {
        return (value as i64, 0);
    }

    let scale_pow = 10_i128.pow(scale);
    let secs = value.div_euclid(scale_pow);
    let rem = value.rem_euclid(scale_pow);

    let nanos = if scale <= 9 {
        rem * 10_i128.pow(9 - scale)
    } else {
        rem / 10_i128.pow(scale - 9)
    };

    (secs as i64, nanos as u32)
}

#[derive(Clone, Copy, Debug)]
struct LocalDateTimeParts {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    nanosecond: u32,
    offset_seconds: i32,
}

#[cfg(not(unix))]
fn current_timezone_offset_seconds() -> i32 {
    if let Ok(value) = std::env::var("TZ_OFFSET_SECONDS") {
        if let Ok(parsed) = value.parse::<i32>() {
            return parsed;
        }
    }

    if let Ok(tz_name) = std::env::var("TZ") {
        if let Ok(tz) = tz_name.parse::<chrono_tz::Tz>() {
            let now = chrono::Utc::now();
            let local = now.with_timezone(&tz);
            return local.offset().fix().local_minus_utc();
        }
    }

    #[cfg(unix)]
    unsafe {
        let mut now: libc::time_t = 0;
        libc::time(&mut now);
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&now, &mut tm).is_null() {
            0
        } else {
            tm.tm_gmtoff as i32
        }
    }

    #[cfg(not(unix))]
    {
        chrono::Local::now().offset().local_minus_utc()
    }
}

fn session_timezone_datetime_parts(utc_secs: i64, utc_nanos: u32) -> Option<LocalDateTimeParts> {
    let session_tz = get_read_session_timezone()?;
    const EARLIEST_TZ_SECS: i64 = -2_208_988_800; // 1900-01-01
    if utc_secs < EARLIEST_TZ_SECS {
        return fallback_fixed_offset_parts(&session_tz, utc_secs, utc_nanos);
    }
    let naive_utc = NaiveDateTime::from_timestamp_opt(utc_secs, utc_nanos)?;
    let offset_seconds = match std::panic::catch_unwind(|| {
        session_tz
            .offset_from_utc_datetime(&naive_utc)
            .fix()
            .local_minus_utc()
    }) {
        Ok(offset) => offset,
        Err(_) => return fallback_fixed_offset_parts(&session_tz, utc_secs, utc_nanos),
    };
    const MAX_TZ_OFFSET_SECONDS: i32 = 24 * 3600;
    if offset_seconds.abs() > MAX_TZ_OFFSET_SECONDS {
        return fallback_fixed_offset_parts(&session_tz, utc_secs, utc_nanos);
    }
    let normalized_offset = normalize_offset_seconds(offset_seconds);
    let adjusted_secs = utc_secs.checked_add(normalized_offset as i64)?;
    let adjusted_dt = NaiveDateTime::from_timestamp_opt(adjusted_secs, utc_nanos)?;
    Some(LocalDateTimeParts {
        year: adjusted_dt.year(),
        month: adjusted_dt.month(),
        day: adjusted_dt.day(),
        hour: adjusted_dt.hour(),
        minute: adjusted_dt.minute(),
        second: adjusted_dt.second(),
        nanosecond: adjusted_dt.nanosecond(),
        offset_seconds: normalized_offset,
    })
}

fn fallback_fixed_offset_parts(
    session_tz: &Tz,
    utc_secs: i64,
    utc_nanos: u32,
) -> Option<LocalDateTimeParts> {
    let reference = NaiveDateTime::from_timestamp_opt(0, 0)?;
    let offset_seconds = session_tz
        .offset_from_utc_datetime(&reference)
        .fix()
        .local_minus_utc();
    let adjusted_secs = utc_secs.checked_add(offset_seconds as i64)?;
    let adjusted_dt = NaiveDateTime::from_timestamp_opt(adjusted_secs, utc_nanos)?;
    Some(LocalDateTimeParts {
        year: adjusted_dt.year(),
        month: adjusted_dt.month(),
        day: adjusted_dt.day(),
        hour: adjusted_dt.hour(),
        minute: adjusted_dt.minute(),
        second: adjusted_dt.second(),
        nanosecond: adjusted_dt.nanosecond(),
        offset_seconds,
    })
}

fn local_datetime_parts(utc_secs: i64, utc_nanos: u32) -> Option<LocalDateTimeParts> {
    #[cfg(unix)]
    {
        let mut tm = std::mem::MaybeUninit::<libc::tm>::uninit();
        let ts = utc_secs as libc::time_t;
        let result = unsafe { libc::localtime_r(&ts, tm.as_mut_ptr()) };
        if result.is_null() {
            return None;
        }
        let tm = unsafe { tm.assume_init() };
        let actual_offset = tm.tm_gmtoff as i32;
        let normalized_offset = normalize_offset_seconds(actual_offset);
        if normalized_offset == actual_offset {
            Some(LocalDateTimeParts {
                year: tm.tm_year + 1900,
                month: (tm.tm_mon + 1) as u32,
                day: tm.tm_mday as u32,
                hour: tm.tm_hour as u32,
                minute: tm.tm_min as u32,
                second: tm.tm_sec as u32,
                nanosecond: utc_nanos,
                offset_seconds: actual_offset,
            })
        } else if let Some(utc_dt) = chrono::DateTime::from_timestamp(utc_secs, utc_nanos) {
            let adjusted_dt = utc_dt + chrono::Duration::seconds(normalized_offset as i64);
            Some(LocalDateTimeParts {
                year: adjusted_dt.year(),
                month: adjusted_dt.month(),
                day: adjusted_dt.day(),
                hour: adjusted_dt.hour(),
                minute: adjusted_dt.minute(),
                second: adjusted_dt.second(),
                nanosecond: adjusted_dt.nanosecond(),
                offset_seconds: normalized_offset,
            })
        } else {
            None
        }
    }
    #[cfg(not(unix))]
    {
        if let Some(utc_dt) = chrono::DateTime::from_timestamp(utc_secs, utc_nanos) {
            let offset_seconds = current_timezone_offset_seconds();
            let local_dt = utc_dt + chrono::Duration::seconds(offset_seconds as i64);
            Some(LocalDateTimeParts {
                year: local_dt.year(),
                month: local_dt.month(),
                day: local_dt.day(),
                hour: local_dt.hour(),
                minute: local_dt.minute(),
                second: local_dt.second(),
                nanosecond: local_dt.nanosecond(),
                offset_seconds,
            })
        } else {
            None
        }
    }
}

fn normalize_offset_seconds(offset: i32) -> i32 {
    if offset >= 0 {
        (offset / 3600) * 3600
    } else {
        -(((offset.abs()) + 3599) / 3600) * 3600
    }
}

fn clamp_year_to_i16(year: i32) -> i16 {
    if year > i16::MAX as i32 {
        i16::MAX
    } else if year < i16::MIN as i32 {
        i16::MIN
    } else {
        year as i16
    }
}

fn sql_timestamp_from_parts(parts: LocalDateTimeParts) -> sql::Timestamp {
    sql::Timestamp {
        year: clamp_year_to_i16(snowflake_year(parts.year)),
        month: parts.month as u16,
        day: parts.day as u16,
        hour: parts.hour as u16,
        minute: parts.minute as u16,
        second: parts.second as u16,
        fraction: parts.nanosecond,
    }
}

fn timestamp_from_local_parts(secs: i64, nanos: u32) -> sql::Timestamp {
    if let Some(parts) = local_datetime_parts(secs, nanos) {
        return sql_timestamp_from_parts(parts);
    }

    if let Some(utc_dt) = chrono::DateTime::from_timestamp(secs, nanos) {
        let year = snowflake_year(utc_dt.year());
        return sql::Timestamp {
            year: year as i16,
            month: utc_dt.month() as u16,
            day: utc_dt.day() as u16,
            hour: utc_dt.hour() as u16,
            minute: utc_dt.minute() as u16,
            second: utc_dt.second() as u16,
            fraction: utc_dt.nanosecond(),
        };
    }

    sql::Timestamp {
        year: 0,
        month: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        fraction: 0,
    }
}

fn timestamp_ntz_to_sql_timestamp(secs: i64, nanos: u32) -> sql::Timestamp {
    if let Some(naive) = NaiveDateTime::from_timestamp_opt(secs, nanos) {
        let year = snowflake_year(naive.year());
        let ts = sql::Timestamp {
            year: year as i16,
            month: naive.month() as u16,
            day: naive.day() as u16,
            hour: naive.hour() as u16,
            minute: naive.minute() as u16,
            second: naive.second() as u16,
            fraction: naive.nanosecond(),
        };
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/rust_debug.log")
            .and_then(|mut f| {
                writeln!(
                    f,
                    "DEBUG timestamp_ntz_to_sql_timestamp result secs={} nanos={} => {}-{}-{} {:02}:{:02}:{:02}.{:09}",
                    secs,
                    nanos,
                    ts.year,
                    ts.month,
                    ts.day,
                    ts.hour,
                    ts.minute,
                    ts.second,
                    ts.fraction
                )
            });
        return ts;
    }

    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rust_debug.log")
        .and_then(|mut f| {
            writeln!(
                f,
                "WARN timestamp_ntz_to_sql_timestamp: NaiveDateTime::from_timestamp_opt failed secs={} nanos={}",
                secs, nanos
            )
        });
    sql::Timestamp {
        year: 0,
        month: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        fraction: 0,
    }
}

fn split_timestamp_nanos(value: i64) -> (i64, u32) {
    let secs = value.div_euclid(1_000_000_000);
    let nanos = value.rem_euclid(1_000_000_000) as u32;
    (secs, nanos)
}

fn decode_scaled_epoch(epoch_seconds: i64, fraction_ns: i64, scale: u32) -> (i64, i64) {
    const MICROS_PER_SEC: i128 = 1_000_000;
    const NANOS_PER_MICRO: i128 = 1_000;

    let scale = scale.min(9);
    let scale_pow = 10_i128.pow(scale);

    let base = i128::from(epoch_seconds)
        .checked_mul(scale_pow)
        .unwrap_or(0);
    let fraction_adjust =
        i128::from(fraction_ns).checked_mul(scale_pow).unwrap_or(0) / 1_000_000_000;
    let micros = base + fraction_adjust;

    let secs = micros.div_euclid(MICROS_PER_SEC);
    let rem_micros = micros.rem_euclid(MICROS_PER_SEC);
    let nanos = rem_micros * NANOS_PER_MICRO;
    (secs as i64, nanos as i64)
}

fn normalize_tz_offset(minutes: i64) -> i64 {
    if minutes > 840 {
        minutes - 1440
    } else if minutes < -840 {
        minutes + 1440
    } else {
        minutes
    }
}

fn format_timestamp_ntz_from_parts(secs: i64, nanos: u32) -> String {
    tracing::debug!(
        "format_timestamp_ntz_from_parts secs={} nanos={}",
        secs,
        nanos
    );
    if let Some(naive) = NaiveDateTime::from_timestamp_opt(secs, nanos) {
        return format_ntz_naive_datetime(naive, nanos);
    }

    #[allow(unused)]
    {
        use std::io::Write;
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/rust_debug.log")
            .and_then(|mut f| {
                writeln!(
                    f,
                    "WARN format_timestamp_ntz_from_parts out-of-range secs={secs} nanos={nanos}"
                )
            });
    }
    format!("{secs}")
}

fn format_ntz_naive_datetime(naive: NaiveDateTime, nanos: u32) -> String {
    let year_str = format_year_padded(snowflake_year(naive.year()));
    let base = format!(
        "{year_str}-{:02}-{:02} {:02}:{:02}:{:02}",
        naive.month(),
        naive.day(),
        naive.hour(),
        naive.minute(),
        naive.second()
    );
    let format_flags = get_read_timestamp_ntz_format();
    if let Some(frac_str) = fractional_component(nanos, &format_flags) {
        format!("{base}.{frac_str}")
    } else {
        base
    }
}

fn format_timestamp_ltz_from_parts(utc_secs: i64, utc_nanos: u32) -> String {
    let format_flags = get_read_timestamp_ltz_format();
    let parts = if format_flags.include_timezone {
        local_datetime_parts(utc_secs, utc_nanos)
    } else {
        session_timezone_datetime_parts(utc_secs, utc_nanos)
            .or_else(|| local_datetime_parts(utc_secs, utc_nanos))
    };

    if let Some(parts) = parts {
        let tz_suffix = if format_flags.include_timezone {
            if parts.offset_seconds == 0 {
                " Z".to_string()
            } else {
                format!(" {:+03}", parts.offset_seconds / 3600)
            }
        } else {
            String::new()
        };
        let year_str = format_year_padded(snowflake_year(parts.year));
        let base = format!(
            "{year_str}-{:02}-{:02} {:02}:{:02}:{:02}",
            parts.month, parts.day, parts.hour, parts.minute, parts.second
        );
        let formatted =
            if let Some(frac_str) = fractional_component(parts.nanosecond, &format_flags) {
                format!("{base}.{frac_str}{tz_suffix}")
            } else {
                format!("{base}{tz_suffix}")
            };
        return formatted;
    }

    format_timestamp_ntz_from_parts(utc_secs, utc_nanos)
}

fn format_timestamp_tz_from_parts(
    utc_secs: i64,
    nanos: u32,
    offset_minutes: i64,
    format: TimestampLtzFormat,
) -> String {
    if let Some(utc_dt) = chrono::DateTime::from_timestamp(utc_secs, nanos) {
        let local_dt = utc_dt + chrono::Duration::minutes(offset_minutes);
        let date = local_dt.date_naive();
        let time = local_dt.time();
        let frac_component = fractional_component(time.nanosecond(), &format);
        let abs_offset = offset_minutes.abs();
        let offset_hours = abs_offset / 60;
        let offset_mins = abs_offset % 60;
        let tz_sign = if offset_minutes >= 0 { '+' } else { '-' };
        let year_str = format_year_padded(snowflake_year(date.year()));
        let tz_suffix = if format.include_timezone {
            format!(" {}{:02}{:02}", tz_sign, offset_hours, offset_mins)
        } else {
            String::new()
        };
        let base = format!(
            "{year_str}-{:02}-{:02} {:02}:{:02}:{:02}",
            date.month(),
            date.day(),
            time.hour(),
            time.minute(),
            time.second()
        );
        if let Some(frac_str) = frac_component {
            format!("{base}.{frac_str}{tz_suffix}")
        } else {
            format!("{base}{tz_suffix}")
        }
    } else {
        format!("{utc_secs}")
    }
}

fn fractional_component(nanos: u32, format: &TimestampLtzFormat) -> Option<String> {
    if !format.fractional {
        return None;
    }
    match format.fractional_digits {
        Some(digits) => {
            let digits = digits.min(9);
            if digits == 0 {
                return None;
            }
            let frac_str = format!("{:09}", nanos);
            if nanos == 0 && !format.force_fractional {
                None
            } else {
                Some(frac_str[..digits as usize].to_string())
            }
        }
        None => {
            if nanos == 0 && !format.force_fractional {
                return None;
            }
            let mut frac_str = format!("{:09}", nanos);
            if !format.force_fractional {
                while frac_str.ends_with('0') {
                    frac_str.pop();
                }
            }
            if frac_str.is_empty() {
                None
            } else {
                Some(frac_str)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decimal_to_string() {
        assert_eq!(decimal_to_string(12345, 2), "123.45");
        assert_eq!(decimal_to_string(-12345, 2), "-123.45");
        assert_eq!(decimal_to_string(12345, 3), "12.345");
        assert_eq!(decimal_to_string(1000, 3), "1.000");
        assert_eq!(decimal_to_string(0, 2), "0.00");
        assert_eq!(decimal_to_string(-12304, 2), "-123.04");
    }

    #[test]
    fn test_date_from_days_epoch() {
        let date = date_from_days(0);
        assert_eq!(date.year, 1970);
        assert_eq!(date.month, 1);
        assert_eq!(date.day, 1);
    }

    #[test]
    fn test_date_from_days_before_ce() {
        // -0001-12-31 should be 719_163 days before 1970-01-01
        let date = date_from_days(-719_163);
        assert_eq!(date.year, -1);
        assert_eq!(date.month, 12);
        assert_eq!(date.day, 31);
    }
}
