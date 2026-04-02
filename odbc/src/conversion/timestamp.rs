use arrow::array::{Array, PrimitiveArray, StructArray};
use arrow::datatypes::{Int32Type, Int64Type};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use odbc_sys as sql;
use serde_json::Value;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::error::{
    InvalidArrowValueSnafu, ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError,
};
use crate::conversion::error::{
    JsonBindingError, NumericValueOutOfRangeSnafu, UnsupportedCDataTypeSnafu,
};
use crate::conversion::param_binding::{read_char_str, read_unaligned, read_wchar_str};
use crate::conversion::traits::Binding;
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, WriteJson};
use crate::conversion::warning::{Warning, Warnings};
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

// =============================================================================
// Arrow reading helpers
// =============================================================================

/// Split a raw scaled epoch value into (epoch_seconds, nanoseconds).
///
/// Snowflake sends timestamp epoch values at varying scales:
///   scale 0 → seconds, scale 3 → milliseconds, scale 6 → microseconds, etc.
fn split_scaled_epoch(raw: i64, scale: u32) -> Result<(i64, u32), ReadArrowError> {
    if scale > 9 {
        return InvalidArrowValueSnafu {
            reason: format!("timestamp scale {scale} exceeds maximum of 9"),
        }
        .fail();
    }
    Ok(match scale {
        0 => (raw, 0u32),
        3 => {
            let secs = raw.div_euclid(1_000);
            let millis = raw.rem_euclid(1_000) as u32;
            (secs, millis * 1_000_000)
        }
        6 => {
            let secs = raw.div_euclid(1_000_000);
            let micros = raw.rem_euclid(1_000_000) as u32;
            (secs, micros * 1_000)
        }
        _ => {
            // Handles scales 1,2,4,5,7,8,9. The division 10^9 / 10^scale is
            // exact for all integer scales 0–9 because 10^scale always divides
            // evenly into 10^9. The guard `scale > 9` above ensures this.
            let divisor = 10i64.pow(scale);
            let secs = raw.div_euclid(divisor);
            let frac = raw.rem_euclid(divisor) as u32;
            let nanos = frac * (1_000_000_000u32 / divisor as u32);
            (secs, nanos)
        }
    })
}

fn read_struct_timestamp(
    array: &StructArray,
    row_idx: usize,
) -> Result<NaiveDateTime, ReadArrowError> {
    if array.is_null(row_idx) {
        return Err(ReadArrowError::NullValue {
            location: snafu::location!(),
        });
    }

    if array.num_columns() < 2 {
        return InvalidArrowValueSnafu {
            reason: format!(
                "timestamp struct has {} column(s), expected at least 2",
                array.num_columns()
            ),
        }
        .fail();
    }

    let epoch_array = array
        .column(0)
        .as_any()
        .downcast_ref::<PrimitiveArray<Int64Type>>()
        .ok_or_else(|| {
            InvalidArrowValueSnafu {
                reason: "timestamp struct column 0 is not Int64".to_string(),
            }
            .build()
        })?;
    let fraction_array = array
        .column(1)
        .as_any()
        .downcast_ref::<PrimitiveArray<Int32Type>>()
        .ok_or_else(|| {
            InvalidArrowValueSnafu {
                reason: "timestamp struct column 1 is not Int32".to_string(),
            }
            .build()
        })?;

    let epoch_seconds = epoch_array.value(row_idx);
    let fraction_nanos = fraction_array.value(row_idx);

    if !(0..1_000_000_000).contains(&fraction_nanos) {
        return InvalidArrowValueSnafu {
            reason: format!(
                "fraction_nanos={fraction_nanos} is out of valid range [0, 1_000_000_000)"
            ),
        }
        .fail();
    }

    DateTime::from_timestamp(epoch_seconds, fraction_nanos as u32)
        .map(|dt| dt.naive_utc())
        .ok_or_else(|| {
            InvalidArrowValueSnafu {
                reason: format!(
                    "epoch_seconds={epoch_seconds}, fraction_nanos={fraction_nanos} is out of range"
                ),
            }
            .build()
        })
}

/// Read a timestamp from a flat Int64 array where the value is an epoch in
/// units determined by `scale` (0 = seconds, 3 = millis, 6 = micros, etc.).
fn read_scaled_timestamp(
    array: &PrimitiveArray<Int64Type>,
    row_idx: usize,
    scale: u32,
) -> Result<NaiveDateTime, ReadArrowError> {
    if array.is_null(row_idx) {
        return Err(ReadArrowError::NullValue {
            location: snafu::location!(),
        });
    }

    let raw = array.value(row_idx);
    let (epoch_seconds, nanos) = split_scaled_epoch(raw, scale)?;

    DateTime::from_timestamp(epoch_seconds, nanos)
        .map(|dt| dt.naive_utc())
        .ok_or_else(|| {
            InvalidArrowValueSnafu {
                reason: format!(
                    "scaled epoch raw={raw}, scale={scale} produced out-of-range timestamp"
                ),
            }
            .build()
        })
}

/// Read a TIMESTAMP_TZ value from a StructArray.
///
/// Snowflake uses different StructArray layouts depending on the declared scale:
///   - Scale 6-9: 3 columns `{epoch_sec: Int64, fraction_nanos: Int32, tz_offset_min: Int32}`
///   - Scale 0-5: 2 columns `{epoch_scaled: Int64, tz_offset_min: Int32}`
///
/// In both cases, the epoch value already represents the UTC instant. The
/// `tz_offset_min` column carries the original timezone offset in minutes but
/// is intentionally **not** applied to the returned `NaiveDateTime`. This
/// matches the old driver behavior: `SQL_TIMESTAMP_STRUCT` has no field for
/// timezone, so callers always receive the UTC wall-clock time. When values
/// are fetched as `SQL_C_CHAR`/`SQL_C_WCHAR` through this Arrow-based
/// conversion path, the formatted string likewise reflects the UTC instant and
/// does not include the original timezone offset. Applications that need to
/// preserve or reconstruct the original offset must obtain it by other means
/// (for example, by reading the offset column explicitly or using an API that
/// exposes the server-formatted string with offset).
fn read_struct_timestamp_tz(
    array: &StructArray,
    row_idx: usize,
    scale: u32,
) -> Result<NaiveDateTime, ReadArrowError> {
    if array.is_null(row_idx) {
        return Err(ReadArrowError::NullValue {
            location: snafu::location!(),
        });
    }

    let num_columns = array.num_columns();

    if num_columns == 3 {
        read_struct_timestamp(array, row_idx)
    } else if num_columns == 2 {
        let epoch_array = array
            .column(0)
            .as_any()
            .downcast_ref::<PrimitiveArray<Int64Type>>()
            .ok_or_else(|| {
                InvalidArrowValueSnafu {
                    reason: "TIMESTAMP_TZ struct column 0 is not Int64".to_string(),
                }
                .build()
            })?;

        let raw = epoch_array.value(row_idx);
        let (epoch_seconds, nanos) = split_scaled_epoch(raw, scale)?;

        DateTime::from_timestamp(epoch_seconds, nanos)
            .map(|dt| dt.naive_utc())
            .ok_or_else(|| {
                InvalidArrowValueSnafu {
                    reason: format!(
                        "TZ scaled epoch raw={raw}, scale={scale} produced out-of-range timestamp"
                    ),
                }
                .build()
            })
    } else {
        InvalidArrowValueSnafu {
            reason: format!("TIMESTAMP_TZ struct has {num_columns} columns, expected 2 or 3"),
        }
        .fail()
    }
}

// =============================================================================
// ODBC write/read helpers (shared by all three timestamp types)
// =============================================================================

fn format_timestamp_string(dt: &NaiveDateTime) -> String {
    let nanos = dt.nanosecond();
    if nanos == 0 {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            dt.year(),
            dt.month(),
            dt.day(),
            dt.hour(),
            dt.minute(),
            dt.second()
        )
    } else {
        let frac = format!("{:09}", nanos);
        let trimmed = frac.trim_end_matches('0');
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{}",
            dt.year(),
            dt.month(),
            dt.day(),
            dt.hour(),
            dt.minute(),
            dt.second(),
            trimmed
        )
    }
}

fn to_sql_timestamp(dt: &NaiveDateTime) -> sql::Timestamp {
    sql::Timestamp {
        year: dt.year() as i16,
        month: dt.month() as u16,
        day: dt.day() as u16,
        hour: dt.hour() as u16,
        minute: dt.minute() as u16,
        second: dt.second() as u16,
        fraction: dt.nanosecond(),
    }
}

fn write_timestamp_to_odbc(
    dt: &NaiveDateTime,
    binding: &Binding,
    get_data_offset: &mut Option<usize>,
) -> Result<Warnings, WriteOdbcError> {
    match binding.target_type {
        CDataType::Default | CDataType::TimeStamp | CDataType::TypeTimestamp => {
            let ts = to_sql_timestamp(dt);
            binding.write_fixed(ts);
            Ok(vec![])
        }
        CDataType::Char => {
            let s = format_timestamp_string(dt);
            if binding.buffer_length > 0 && binding.buffer_length < 20 {
                return NumericValueOutOfRangeSnafu {
                    reason: "Buffer too small for SQL_C_CHAR timestamp (minimum 20 bytes)"
                        .to_string(),
                }
                .fail();
            }
            Ok(binding.write_char_string(&s, get_data_offset))
        }
        CDataType::WChar => {
            let s = format_timestamp_string(dt);
            if binding.buffer_length > 0 && binding.buffer_length < 40 {
                return NumericValueOutOfRangeSnafu {
                    reason: "Buffer too small for SQL_C_WCHAR timestamp (minimum 40 bytes)"
                        .to_string(),
                }
                .fail();
            }
            Ok(binding.write_wchar_string(&s, get_data_offset))
        }
        CDataType::Date | CDataType::TypeDate => {
            let date = sql::Date {
                year: dt.year() as i16,
                month: dt.month() as u16,
                day: dt.day() as u16,
            };
            binding.write_fixed(date);
            let has_time =
                dt.hour() != 0 || dt.minute() != 0 || dt.second() != 0 || dt.nanosecond() != 0;
            if has_time {
                Ok(vec![Warning::NumericValueTruncated])
            } else {
                Ok(vec![])
            }
        }
        CDataType::Time | CDataType::TypeTime => {
            let time = sql::Time {
                hour: dt.hour() as u16,
                minute: dt.minute() as u16,
                second: dt.second() as u16,
            };
            binding.write_fixed(time);
            if dt.nanosecond() != 0 {
                Ok(vec![Warning::NumericValueTruncated])
            } else {
                Ok(vec![])
            }
        }
        CDataType::Binary => {
            let mut bytes = [0u8; std::mem::size_of::<sql::Timestamp>()];
            let ts = to_sql_timestamp(dt);
            // SAFETY: sql::Timestamp is a repr(C) POD struct. Writing into a
            // pre-zeroed buffer ensures any padding bytes are deterministic.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    &ts as *const sql::Timestamp as *const u8,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                );
            }
            let ts_bytes: &[u8] = &bytes;
            if binding.buffer_length > 0
                && (binding.buffer_length as usize) < std::mem::size_of::<sql::Timestamp>()
            {
                return NumericValueOutOfRangeSnafu {
                    reason: "Buffer too small for SQL_C_BINARY timestamp".to_string(),
                }
                .fail();
            }
            Ok(binding.write_binary(ts_bytes, get_data_offset))
        }
        _ => UnsupportedOdbcTypeSnafu {
            target_type: binding.target_type,
        }
        .fail(),
    }
}

fn read_timestamp_odbc(binding: &ParameterBinding) -> Result<NaiveDateTime, JsonBindingError> {
    match binding.value_type {
        CDataType::TimeStamp | CDataType::TypeTimestamp => {
            let ts = read_unaligned::<sql::Timestamp>(binding);
            let date = NaiveDate::from_ymd_opt(ts.year as i32, ts.month as u32, ts.day as u32)
                .ok_or_else(|| {
                    UnsupportedCDataTypeSnafu {
                        c_type: binding.value_type,
                    }
                    .build()
                })?;
            let time = NaiveTime::from_hms_nano_opt(
                ts.hour as u32,
                ts.minute as u32,
                ts.second as u32,
                ts.fraction,
            )
            .ok_or_else(|| {
                UnsupportedCDataTypeSnafu {
                    c_type: binding.value_type,
                }
                .build()
            })?;
            Ok(NaiveDateTime::new(date, time))
        }
        CDataType::Char => {
            let s = read_char_str(binding)?;
            NaiveDateTime::parse_from_str(s.trim(), "%Y-%m-%d %H:%M:%S")
                .or_else(|_| NaiveDateTime::parse_from_str(s.trim(), "%Y-%m-%d %H:%M:%S%.f"))
                .map_err(|_| {
                    UnsupportedCDataTypeSnafu {
                        c_type: binding.value_type,
                    }
                    .build()
                })
        }
        CDataType::WChar => {
            let s = read_wchar_str(binding)?;
            NaiveDateTime::parse_from_str(s.trim(), "%Y-%m-%d %H:%M:%S")
                .or_else(|_| NaiveDateTime::parse_from_str(s.trim(), "%Y-%m-%d %H:%M:%S%.f"))
                .map_err(|_| {
                    UnsupportedCDataTypeSnafu {
                        c_type: binding.value_type,
                    }
                    .build()
                })
        }
        _ => UnsupportedCDataTypeSnafu {
            c_type: binding.value_type,
        }
        .fail(),
    }
}

fn write_timestamp_json(value: NaiveDateTime) -> Result<Value, JsonBindingError> {
    let epoch_nanos = value.and_utc().timestamp_nanos_opt().ok_or_else(|| {
        UnsupportedCDataTypeSnafu {
            c_type: CDataType::TypeTimestamp,
        }
        .build()
    })?;
    Ok(Value::String(epoch_nanos.to_string()))
}

// =============================================================================
// Macro to generate the five trait impls shared by NTZ, LTZ, and TZ.
//
// The only variation is:
//   - The struct reader for StructArray (NTZ/LTZ use `read_struct_timestamp`;
//     TZ uses `read_struct_timestamp_tz` which needs `self.scale`).
//   - The `SnowflakeLogicalType` returned by `sf_type()`.
// =============================================================================

macro_rules! impl_snowflake_timestamp {
    // NTZ/LTZ path: StructArray reader ignores scale.
    ($name:ident, standard, $logical_type:expr) => {
        impl ReadArrowType<StructArray> for $name {
            fn read_arrow_type<'a>(
                &self,
                array: &'a StructArray,
                row_idx: usize,
            ) -> Result<Self::Representation<'a>, ReadArrowError> {
                read_struct_timestamp(array, row_idx)
            }
        }

        impl_snowflake_timestamp!(@common $name, $logical_type);
    };

    // TZ path: StructArray reader uses scale to handle 2- vs 3-column layouts.
    ($name:ident, tz, $logical_type:expr) => {
        impl ReadArrowType<StructArray> for $name {
            fn read_arrow_type<'a>(
                &self,
                array: &'a StructArray,
                row_idx: usize,
            ) -> Result<Self::Representation<'a>, ReadArrowError> {
                read_struct_timestamp_tz(array, row_idx, self.scale)
            }
        }

        impl_snowflake_timestamp!(@common $name, $logical_type);
    };

    (@common $name:ident, $logical_type:expr) => {
        impl SnowflakeType for $name {
            type Representation<'a> = NaiveDateTime;
        }

        impl ReadArrowType<PrimitiveArray<Int64Type>> for $name {
            fn read_arrow_type<'a>(
                &self,
                array: &'a PrimitiveArray<Int64Type>,
                row_idx: usize,
            ) -> Result<Self::Representation<'a>, ReadArrowError> {
                read_scaled_timestamp(array, row_idx, self.scale)
            }
        }

        impl WriteODBCType for $name {
            fn sql_type(&self) -> sql::SqlDataType {
                sql::SqlDataType::TIMESTAMP
            }

            fn column_size(&self) -> sql::ULen {
                if self.scale == 0 {
                    19
                } else {
                    20 + self.scale as sql::ULen
                }
            }

            fn decimal_digits(&self) -> sql::SmallInt {
                self.scale as sql::SmallInt
            }

            fn write_odbc_type(
                &self,
                snowflake_value: Self::Representation<'_>,
                binding: &Binding,
                get_data_offset: &mut Option<usize>,
            ) -> Result<Warnings, WriteOdbcError> {
                write_timestamp_to_odbc(&snowflake_value, binding, get_data_offset)
            }
        }

        impl ReadODBC for $name {
            fn read_odbc<'a>(
                &self,
                binding: &'a ParameterBinding,
            ) -> Result<Self::Representation<'a>, JsonBindingError> {
                read_timestamp_odbc(binding)
            }
        }

        impl WriteJson for $name {
            fn write_json(
                &self,
                value: Self::Representation<'_>,
            ) -> Result<Value, JsonBindingError> {
                write_timestamp_json(value)
            }

            fn sf_type(&self) -> SnowflakeLogicalType {
                $logical_type
            }
        }
    };
}

// =============================================================================
// TIMESTAMP_NTZ / TIMESTAMP_LTZ / TIMESTAMP_TZ
// =============================================================================

pub(crate) struct SnowflakeTimestampNtz {
    pub(crate) scale: u32,
}

impl_snowflake_timestamp!(
    SnowflakeTimestampNtz,
    standard,
    SnowflakeLogicalType::TimestampNtz
);

pub(crate) struct SnowflakeTimestampLtz {
    pub(crate) scale: u32,
}

impl_snowflake_timestamp!(
    SnowflakeTimestampLtz,
    standard,
    SnowflakeLogicalType::TimestampLtz
);

pub(crate) struct SnowflakeTimestampTz {
    pub(crate) scale: u32,
}

impl_snowflake_timestamp!(SnowflakeTimestampTz, tz, SnowflakeLogicalType::TimestampTz);
