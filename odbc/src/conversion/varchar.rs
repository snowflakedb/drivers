use std::borrow::Cow;

use arrow::array::{Array, GenericByteArray};
use arrow::datatypes::Utf8Type;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use odbc_sys as sql;
use serde_json::Value;
use snafu::ResultExt;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::binary::hex_encode_lowercase;
use crate::conversion::error::JsonBindingError;
use crate::conversion::error::{
    InvalidValueSnafu, NumericLiteralParsingSnafu, NumericValueOutOfRangeSnafu, ReadArrowError,
    RustParsingSnafu, UnsupportedCDataTypeSnafu, UnsupportedOdbcTypeSnafu, WriteOdbcError,
};
use crate::conversion::param_binding::{
    buffer_data_len, read_char_str, read_unaligned, read_wchar_str,
};
use crate::conversion::parsers::numeric_literal_parser::{Sign, parse_numeric_literal};
use crate::conversion::traits::Binding;
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, WriteJson};
use crate::conversion::warning::{Warning, Warnings};
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

pub(crate) struct SnowflakeVarchar {
    #[allow(dead_code)]
    pub len: u32,
}

impl SnowflakeType for SnowflakeVarchar {
    type Representation<'a> = Cow<'a, str>;
}

impl ReadArrowType<GenericByteArray<Utf8Type>> for SnowflakeVarchar {
    fn read_arrow_type<'a>(
        &self,
        array: &'a GenericByteArray<Utf8Type>,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        if array.is_null(row_idx) {
            return Err(ReadArrowError::NullValue {
                location: snafu::location!(),
            });
        }
        let v = array.value(row_idx);
        Ok(Cow::Borrowed(v))
    }
}

macro_rules! parse_i_number {
    ($value:expr, $type:ty) => {{
        let value = parse_numeric_literal($value).context(NumericLiteralParsingSnafu {})?;
        let normalized_value = value.normalize().context(NumericLiteralParsingSnafu {})?;
        let whole_part = normalized_value.whole_part_with_sign();
        let value = whole_part.parse::<$type>().map_err(|_| {
            RustParsingSnafu {
                reason: format!(
                    "Failed to parse whole part of numeric literal: {}",
                    whole_part
                ),
            }
            .build()
        })?;
        let warnings: Warnings = if normalized_value.has_fractional_part() {
            vec![Warning::NumericValueTruncated]
        } else {
            vec![]
        };
        (value, warnings)
    }};
}

macro_rules! parse_u_number {
    ($value:expr, $type:ty) => {{
        let value = parse_numeric_literal($value).context(NumericLiteralParsingSnafu {})?;
        let normalized_value = value.normalize().context(NumericLiteralParsingSnafu {})?;
        let whole_part = normalized_value.whole_part_with_sign();
        let value = whole_part.parse::<$type>().map_err(|_| {
            RustParsingSnafu {
                reason: format!(
                    "Failed to parse whole part of numeric literal: {}",
                    whole_part
                ),
            }
            .build()
        })?;
        let warnings: Warnings = if normalized_value.has_fractional_part() {
            vec![Warning::NumericValueTruncated]
        } else {
            vec![]
        };
        (value, warnings)
    }};
}

macro_rules! write_i_number {
    ($value:expr, $type:ty, $binding:expr) => {{
        let (value, warnings) = parse_i_number!($value, $type);
        $binding.write_fixed(value);
        Ok(warnings)
    }};
}

macro_rules! write_u_number {
    ($value:expr, $type:ty, $binding:expr) => {{
        let (value, warnings) = parse_u_number!($value, $type);
        $binding.write_fixed(value);
        Ok(warnings)
    }};
}

macro_rules! parse_float {
    ($value:expr, $type:ty) => {{
        if $value.trim().to_lowercase() == "nan" {
            <$type>::NAN
        } else if $value.trim().to_lowercase() == "inf" {
            <$type>::INFINITY
        } else if $value.trim().to_lowercase() == "-inf" {
            <$type>::NEG_INFINITY
        } else {
            let value = parse_numeric_literal($value).context(NumericLiteralParsingSnafu {})?;
            let normalized_value = value.normalize().context(NumericLiteralParsingSnafu {})?;
            let float_value_str = normalized_value.float_with_sign();
            let float_value = float_value_str.parse::<$type>().map_err(|e| {
                RustParsingSnafu {
                    reason: e.to_string(),
                }
                .build()
            })?;
            if float_value.is_infinite() {
                return NumericValueOutOfRangeSnafu {
                    reason: format!("Overflow float value({:?}): nan or inf", float_value_str),
                }
                .fail();
            }
            float_value
        }
    }};
}

macro_rules! write_float {
    ($value:expr, $type:ty, $binding:expr) => {{
        let value = parse_float!($value, $type);
        $binding.write_fixed(value);
        Ok(vec![])
    }};
}

/// Validates that a date string is in strict YYYY-MM-DD format.
/// Returns false for formats like "24-01-15" (2-digit year) or "2024-1-5" (single-digit month/day).
fn is_valid_date_format(s: &str) -> bool {
    // Must be exactly 10 characters: YYYY-MM-DD
    if s.len() != 10 {
        return false;
    }
    let bytes = s.as_bytes();
    // Check format: DDDD-DD-DD where D is digit
    bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[4] == b'-'
        && bytes[5].is_ascii_digit()
        && bytes[6].is_ascii_digit()
        && bytes[7] == b'-'
        && bytes[8].is_ascii_digit()
        && bytes[9].is_ascii_digit()
}

/// Validates that a time string is in strict HH:MM:SS format.
/// Returns false for formats like "9:5:3" (single-digit components).
fn is_valid_time_format(s: &str) -> bool {
    // Must be exactly 8 characters: HH:MM:SS
    if s.len() != 8 {
        return false;
    }
    let bytes = s.as_bytes();
    // Check format: DD:DD:DD where D is digit
    bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2] == b':'
        && bytes[3].is_ascii_digit()
        && bytes[4].is_ascii_digit()
        && bytes[5] == b':'
        && bytes[6].is_ascii_digit()
        && bytes[7].is_ascii_digit()
}

impl WriteODBCType for SnowflakeVarchar {
    fn sql_type(&self) -> sql::SqlDataType {
        sql::SqlDataType::VARCHAR
    }

    fn column_size(&self) -> sql::ULen {
        self.len as sql::ULen
    }

    fn decimal_digits(&self) -> sql::SmallInt {
        0
    }

    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
        get_data_offset: &mut Option<usize>,
    ) -> Result<Warnings, WriteOdbcError> {
        let snowflake_value: &str = &snowflake_value;
        match binding.target_type {
            CDataType::Default | CDataType::Char => {
                Ok(binding.write_char_string(snowflake_value, get_data_offset))
            }
            CDataType::WChar => Ok(binding.write_wchar_string(snowflake_value, get_data_offset)),
            CDataType::SBigInt => write_i_number!(snowflake_value, i64, binding),
            CDataType::UBigInt => write_u_number!(snowflake_value, u64, binding),
            CDataType::Long | CDataType::SLong => write_i_number!(snowflake_value, i32, binding),
            CDataType::ULong => write_u_number!(snowflake_value, u32, binding),
            CDataType::Short | CDataType::SShort => write_i_number!(snowflake_value, i16, binding),
            CDataType::UShort => write_u_number!(snowflake_value, u16, binding),
            CDataType::TinyInt | CDataType::STinyInt => {
                write_i_number!(snowflake_value, i8, binding)
            }
            CDataType::UTinyInt => write_u_number!(snowflake_value, u8, binding),
            CDataType::Double => write_float!(snowflake_value, f64, binding),
            CDataType::Float => write_float!(snowflake_value, f32, binding),
            CDataType::Bit => {
                let (value, warnings) = parse_u_number!(snowflake_value, u8);
                match value {
                    0 | 1 => {
                        binding.write_fixed(value);
                        Ok(warnings)
                    }
                    _ => NumericValueOutOfRangeSnafu {
                        reason: "Trying to convert non-binary integer to BIT".to_string(),
                    }
                    .fail(),
                }
            }
            CDataType::Date | CDataType::TypeDate => {
                // Strict format validation: YYYY-MM-DD (exactly 10 chars)
                let value = snowflake_value.trim();
                if !is_valid_date_format(value) {
                    return InvalidValueSnafu {
                        reason: "Date must be in YYYY-MM-DD format".to_string(),
                    }
                    .fail();
                }
                let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|e| {
                    InvalidValueSnafu {
                        reason: e.to_string(),
                    }
                    .build()
                })?;
                let date = sql::Date {
                    year: Datelike::year(&date) as i16,
                    month: Datelike::month(&date) as u16,
                    day: Datelike::day(&date) as u16,
                };
                binding.write_fixed(date);
                Ok(vec![])
            }
            CDataType::Time | CDataType::TypeTime => {
                // Strict format validation: HH:MM:SS (exactly 8 chars)
                let value = snowflake_value.trim();
                if !is_valid_time_format(value) {
                    return InvalidValueSnafu {
                        reason: "Time must be in HH:MM:SS format".to_string(),
                    }
                    .fail();
                }
                let time = NaiveTime::parse_from_str(value, "%H:%M:%S").map_err(|e| {
                    InvalidValueSnafu {
                        reason: e.to_string(),
                    }
                    .build()
                })?;
                let time = sql::Time {
                    hour: Timelike::hour(&time) as u16,
                    minute: Timelike::minute(&time) as u16,
                    second: Timelike::second(&time) as u16,
                };
                binding.write_fixed(time);
                Ok(vec![])
            }
            CDataType::TimeStamp | CDataType::TypeTimestamp => {
                // Try parsing as full timestamp first, then as date-only with midnight default,
                // then as time-only with today's date
                let value = snowflake_value.trim();
                let timestamp = if let Ok(ts) =
                    NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                {
                    ts
                } else if is_valid_date_format(value) {
                    // Date-only string: default time to midnight
                    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|e| {
                        InvalidValueSnafu {
                            reason: e.to_string(),
                        }
                        .build()
                    })?;
                    date.and_hms_opt(0, 0, 0).ok_or_else(|| {
                        InvalidValueSnafu {
                            reason: "Failed to create midnight timestamp".to_string(),
                        }
                        .build()
                    })?
                } else if is_valid_time_format(value) {
                    // Time-only string: default date to today
                    let time = NaiveTime::parse_from_str(value, "%H:%M:%S").map_err(|e| {
                        InvalidValueSnafu {
                            reason: e.to_string(),
                        }
                        .build()
                    })?;
                    let today = chrono::Local::now().date_naive();
                    today.and_time(time)
                } else {
                    return InvalidValueSnafu {
                        reason: "Timestamp must be in YYYY-MM-DD HH:MM:SS, YYYY-MM-DD, or HH:MM:SS format".to_string(),
                    }.fail();
                };
                let timestamp = sql::Timestamp {
                    year: Datelike::year(&timestamp) as i16,
                    month: Datelike::month(&timestamp) as u16,
                    day: Datelike::day(&timestamp) as u16,
                    hour: Timelike::hour(&timestamp) as u16,
                    minute: Timelike::minute(&timestamp) as u16,
                    second: Timelike::second(&timestamp) as u16,
                    fraction: 0,
                };
                binding.write_fixed(timestamp);
                Ok(vec![])
            }
            CDataType::Numeric => {
                let value = parse_numeric_literal(snowflake_value)
                    .context(NumericLiteralParsingSnafu {})?;
                let normalized_value = value.normalize().context(NumericLiteralParsingSnafu {})?;
                let whole_part = normalized_value.whole_part.clone();
                let value = whole_part.parse::<u128>().map_err(|_| {
                    RustParsingSnafu {
                        reason: format!(
                            "Failed to parse whole part of numeric literal: {}",
                            whole_part
                        ),
                    }
                    .build()
                })?;
                let warnings: Warnings = if normalized_value.has_fractional_part() {
                    vec![Warning::NumericValueTruncated]
                } else {
                    vec![]
                };
                let sign = if normalized_value.sign == Sign::Positive {
                    1
                } else {
                    0
                };
                let numeric = sql::Numeric {
                    precision: 38,
                    scale: 0,
                    sign,
                    val: value.to_le_bytes(),
                };
                binding.write_fixed(numeric);
                Ok(warnings)
            }
            CDataType::Binary => {
                Ok(binding.write_binary(snowflake_value.as_bytes(), get_data_offset))
            }
            _ => UnsupportedOdbcTypeSnafu {
                target_type: binding.target_type,
            }
            .fail(),
        }
    }
}

impl ReadODBC for SnowflakeVarchar {
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, JsonBindingError> {
        let s = match binding.value_type {
            CDataType::Default | CDataType::Char => read_char_str(binding)?,
            CDataType::WChar => read_wchar_str(binding)?,
            CDataType::Long | CDataType::SLong => read_unaligned::<i32>(binding).to_string(),
            CDataType::Short | CDataType::SShort => read_unaligned::<i16>(binding).to_string(),
            CDataType::SBigInt => read_unaligned::<i64>(binding).to_string(),
            CDataType::ULong => read_unaligned::<u32>(binding).to_string(),
            CDataType::UShort => read_unaligned::<u16>(binding).to_string(),
            CDataType::UBigInt => read_unaligned::<u64>(binding).to_string(),
            CDataType::TinyInt | CDataType::STinyInt => read_unaligned::<i8>(binding).to_string(),
            CDataType::UTinyInt => read_unaligned::<u8>(binding).to_string(),
            CDataType::Double => {
                let v = read_unaligned::<f64>(binding);
                if v == 0.0 {
                    0.0_f64.to_string()
                } else {
                    v.to_string()
                }
            }
            CDataType::Float => {
                let v = read_unaligned::<f32>(binding);
                if v == 0.0 {
                    0.0_f32.to_string()
                } else {
                    v.to_string()
                }
            }
            CDataType::Bit => {
                if read_unaligned::<u8>(binding) != 0 {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            CDataType::TypeTimestamp | CDataType::TimeStamp => {
                let ts = read_unaligned::<sql::Timestamp>(binding);
                if ts.fraction == 0 {
                    format!(
                        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                        ts.year, ts.month, ts.day, ts.hour, ts.minute, ts.second
                    )
                } else {
                    format!(
                        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:09}",
                        ts.year, ts.month, ts.day, ts.hour, ts.minute, ts.second, ts.fraction
                    )
                }
            }
            CDataType::TypeDate | CDataType::Date => {
                let d = read_unaligned::<sql::Date>(binding);
                format!("{:04}-{:02}-{:02}", d.year, d.month, d.day)
            }
            CDataType::TypeTime | CDataType::Time => {
                let t = read_unaligned::<sql::Time>(binding);
                format!("{:02}:{:02}:{:02}", t.hour, t.minute, t.second)
            }
            CDataType::Numeric => {
                let n = read_unaligned::<sql::Numeric>(binding);
                let magnitude = u128::from_le_bytes(n.val);
                let abs_str = magnitude.to_string();
                let scaled = if n.scale > 0 {
                    let s = n.scale as usize;
                    if abs_str.len() <= s {
                        let padded = format!("{:0>width$}", abs_str, width = s + 1);
                        let (whole, frac) = padded.split_at(padded.len() - s);
                        format!("{}.{}", whole, frac)
                    } else {
                        let (whole, frac) = abs_str.split_at(abs_str.len() - s);
                        format!("{}.{}", whole, frac)
                    }
                } else if n.scale < 0 {
                    let zeros = (-(i16::from(n.scale))) as usize;
                    format!("{}{}", abs_str, "0".repeat(zeros))
                } else {
                    abs_str
                };
                if n.sign == 0 && magnitude != 0 {
                    format!("-{}", scaled)
                } else {
                    scaled
                }
            }
            CDataType::Binary => {
                let len = buffer_data_len(binding);
                let bytes = unsafe {
                    std::slice::from_raw_parts(binding.parameter_value_ptr as *const u8, len)
                };
                hex_encode_lowercase(bytes)
            }
            _ => {
                return UnsupportedCDataTypeSnafu {
                    c_type: binding.value_type,
                }
                .fail();
            }
        };
        Ok(Cow::Owned(s))
    }
}

impl WriteJson for SnowflakeVarchar {
    fn write_json(&self, value: Self::Representation<'_>) -> Result<Value, JsonBindingError> {
        Ok(Value::String(value.into_owned()))
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::Text
    }
}
