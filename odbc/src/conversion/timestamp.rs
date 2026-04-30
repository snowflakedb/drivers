use std::io::{Cursor, Write as _};

use arrow::array::{Array, PrimitiveArray, StructArray};
use arrow::datatypes::{Int32Type, Int64Type};
use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use odbc_sys as sql;
use serde_json::Value;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::api::types::{SQL_SF_TIMESTAMP_TZ_COLUMN_SIZE, SqlType};
use crate::conversion::error::{
    BindingNumericOutOfRangeSnafu, JsonBindingError, NumericValueOutOfRangeSnafu,
    UnsupportedCDataTypeSnafu,
};
use crate::conversion::error::{
    InvalidArrowValueSnafu, ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError,
};
use crate::conversion::param_binding::{
    read_binary_struct, read_char_str, read_unaligned, read_wchar_str,
};
use crate::conversion::traits::Binding;
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, WriteJson};
use crate::conversion::warning::{Warning, Warnings};
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

/// Wire-protocol bias for TIMESTAMP_TZ offset minutes.
///
/// The Snowflake server expects the JSON `value` for a TIMESTAMP_TZ binding
/// in the form `"<epoch_nanoseconds> <offset_minutes_plus_1440>"`. The 1440
/// (= 24 * 60) bias keeps the second token non-negative for any legal
/// timezone offset and matches the legacy 3.16.0 ODBC and Python connectors.
const TZ_OFFSET_BIAS_MINUTES: i32 = 1440;

/// A TIMESTAMP_TZ value carrying both the UTC instant and its original
/// timezone offset in minutes.
///
/// `utc` is the wall-clock representation **at UTC** (`offset_minutes == 0`
/// means "this naive datetime is already UTC"). `offset_minutes` records the
/// offset that produced the *local* wall-clock the user originally saw, so
/// CHAR/WCHAR formatting and JSON binding can reconstruct it without losing
/// information. `SQL_C_TYPE_TIMESTAMP` reads ignore `offset_minutes` because
/// the ODBC struct has no field to store it (matches the spec rule that
/// "datetime with timezone -> datetime without timezone" converts to UTC).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TzInstant {
    pub utc: NaiveDateTime,
    pub offset_minutes: i32,
}

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

/// Read a TIMESTAMP_TZ value (UTC instant + original offset) from a StructArray.
///
/// Snowflake uses different StructArray layouts depending on the declared scale:
///   - Scale 6-9: 3 columns `{epoch_sec: Int64, fraction_nanos: Int32, tz_offset_min: Int32}`
///   - Scale 0-5: 2 columns `{epoch_scaled: Int64, tz_offset_min: Int32}`
///
/// In both cases the epoch value already represents the UTC instant; the
/// `tz_offset_min` column carries the original timezone offset (Snowflake
/// pre-biases by +1440 on the wire, so we subtract here to recover the signed
/// offset in minutes). The offset is preserved alongside the UTC datetime so
/// downstream conversions can reconstruct the local wall-clock for
/// `SQL_C_CHAR`/`SQL_C_WCHAR` (with `+/-HH:MM` suffix) without losing
/// information. `SQL_C_TYPE_TIMESTAMP` ignores the offset because the ODBC
/// struct has no field to store it.
fn read_struct_timestamp_tz(
    array: &StructArray,
    row_idx: usize,
    scale: u32,
) -> Result<TzInstant, ReadArrowError> {
    if array.is_null(row_idx) {
        return Err(ReadArrowError::NullValue {
            location: snafu::location!(),
        });
    }

    let num_columns = array.num_columns();
    let utc = if num_columns == 3 {
        read_struct_timestamp(array, row_idx)?
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
            })?
    } else {
        return InvalidArrowValueSnafu {
            reason: format!("TIMESTAMP_TZ struct has {num_columns} columns, expected 2 or 3"),
        }
        .fail();
    };

    // Offset always lives in the last column. The wire value is the
    // signed offset in minutes plus 1440 (Snowflake's positive-only
    // encoding); subtract the bias here so callers see the true offset.
    let offset_col_idx = num_columns - 1;
    let offset_array = array
        .column(offset_col_idx)
        .as_any()
        .downcast_ref::<PrimitiveArray<Int32Type>>()
        .ok_or_else(|| {
            InvalidArrowValueSnafu {
                reason: format!(
                    "TIMESTAMP_TZ struct column {offset_col_idx} is not Int32 (expected tz_offset_min)"
                ),
            }
            .build()
        })?;
    let offset_minutes = offset_array.value(row_idx) - TZ_OFFSET_BIAS_MINUTES;

    Ok(TzInstant {
        utc,
        offset_minutes,
    })
}

// =============================================================================
// ODBC write/read helpers (shared by all three timestamp types)
// =============================================================================

/// Format a `NaiveDateTime` as `YYYY-MM-DD HH:MM:SS[.fffffffff]` into a stack
/// buffer without any heap allocation, returning the filled slice as `&str`.
///
/// 48 bytes is sufficient: `YYYY-MM-DD HH:MM:SS.` = 20 bytes + up to 9 fractional
/// digits + signed/4-digit year headroom. If a future chrono release ever
/// widens this beyond the buffer, the caller receives a typed
/// `NumericValueOutOfRange` error instead of a silent truncation through
/// the unsafe `from_utf8_unchecked` below.
fn format_timestamp_string_into<'a>(
    dt: &NaiveDateTime,
    buf: &'a mut [u8; 48],
) -> Result<&'a str, WriteOdbcError> {
    let nanos = dt.nanosecond();
    let len = {
        let mut cur = Cursor::new(&mut buf[..]);
        let write_result = write!(
            cur,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            dt.year(),
            dt.month(),
            dt.day(),
            dt.hour(),
            dt.minute(),
            dt.second()
        )
        .and_then(|()| {
            if nanos != 0 {
                write!(cur, ".{nanos:09}")
            } else {
                Ok(())
            }
        });
        if write_result.is_err() {
            return NumericValueOutOfRangeSnafu {
                reason: format!(
                    "timestamp value does not fit in the {}-byte format buffer",
                    buf.len()
                ),
            }
            .fail();
        }
        cur.position() as usize
    };
    // Trim trailing zeros from the fractional part (matching the previous
    // `.trim_end_matches('0')` behavior). Skip the scan entirely when we
    // know no fractional part was written.
    let mut end = len;
    if nanos != 0 {
        while end > 0 && buf[end - 1] == b'0' {
            end -= 1;
        }
    }
    // SAFETY: only ASCII digits, '-', ':', ' ', and '.' were written above.
    Ok(unsafe { std::str::from_utf8_unchecked(&buf[..end]) })
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
            if binding.buffer_length > 0 && binding.buffer_length < 20 {
                return NumericValueOutOfRangeSnafu {
                    reason: "Buffer too small for SQL_C_CHAR timestamp (minimum 20 bytes)"
                        .to_string(),
                }
                .fail();
            }
            let mut buf = [0u8; 48];
            let s = format_timestamp_string_into(dt, &mut buf)?;
            Ok(binding.write_char_string(s, get_data_offset))
        }
        CDataType::WChar => {
            if binding.buffer_length > 0 && binding.buffer_length < 40 {
                return NumericValueOutOfRangeSnafu {
                    reason: "Buffer too small for SQL_C_WCHAR timestamp (minimum 40 bytes)"
                        .to_string(),
                }
                .fail();
            }
            let mut buf = [0u8; 48];
            let s = format_timestamp_string_into(dt, &mut buf)?;
            Ok(binding.write_wchar_string(s, get_data_offset))
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
        CDataType::Binary => {
            let ts = read_binary_struct::<sql::Timestamp>(binding, "SQL_TIMESTAMP_STRUCT")?;
            let date = NaiveDate::from_ymd_opt(ts.year as i32, ts.month as u32, ts.day as u32)
                .ok_or_else(|| {
                    BindingNumericOutOfRangeSnafu {
                        reason: format!(
                            "invalid date from SQL_C_BINARY: year={}, month={}, day={}",
                            ts.year, ts.month, ts.day
                        ),
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
                BindingNumericOutOfRangeSnafu {
                    reason: format!(
                        "invalid time from SQL_C_BINARY: hour={}, minute={}, second={}, fraction={}",
                        ts.hour, ts.minute, ts.second, ts.fraction
                    ),
                }
                .build()
            })?;
            Ok(NaiveDateTime::new(date, time))
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
// TIMESTAMP_TZ-specific helpers
//
// TZ differs from NTZ/LTZ on two axes:
//   1. CHAR/WCHAR formatting must include a `+/-HH:MM` offset suffix so the
//      value is round-trippable as text (NTZ/LTZ have no offset to emit).
//   2. JSON binding must emit `<epoch_nanos> <offset_minutes + 1440>` so the
//      server stores the original instant *and* its offset (NTZ/LTZ only need
//      the epoch).
// `SQL_C_TYPE_TIMESTAMP` reads/writes intentionally drop the offset because
// the ODBC struct can't carry it -- the spec rule is "datetime with timezone
// -> datetime without timezone converts to UTC".
// =============================================================================

/// Apply an offset (in minutes) to a UTC `NaiveDateTime`, producing the local
/// wall-clock the user originally observed. Returns `None` if the resulting
/// year falls outside chrono's representable range.
fn local_wall_clock(utc: &NaiveDateTime, offset_minutes: i32) -> Option<NaiveDateTime> {
    let offset = FixedOffset::east_opt(offset_minutes * 60)?;
    Some(DateTime::<FixedOffset>::from_naive_utc_and_offset(*utc, offset).naive_local())
}

/// Format a `TzInstant` as `YYYY-MM-DD HH:MM:SS[.fffffffff] +/-HH:MM` into a
/// stack buffer with no heap allocation, returning the filled slice as `&str`.
///
/// 64 bytes is sufficient: 48 for the bare timestamp portion (see
/// `format_timestamp_string_into`) plus ` +HH:MM` (7 bytes) and headroom.
fn format_timestamp_tz_string_into<'a>(
    value: &TzInstant,
    buf: &'a mut [u8; 64],
) -> Result<&'a str, WriteOdbcError> {
    let local = local_wall_clock(&value.utc, value.offset_minutes).ok_or_else(|| {
        NumericValueOutOfRangeSnafu {
            reason: format!(
                "TIMESTAMP_TZ offset {} minutes pushes datetime out of representable range",
                value.offset_minutes
            ),
        }
        .build()
    })?;

    // Reuse the bare-timestamp formatter into a 48-byte scratch then copy the
    // result into our wider buffer alongside the offset suffix.
    let mut bare = [0u8; 48];
    let bare_str = format_timestamp_string_into(&local, &mut bare)?;
    let bare_bytes = bare_str.as_bytes();

    let offset_total = value.offset_minutes;
    let sign = if offset_total < 0 { '-' } else { '+' };
    let abs_minutes = offset_total.unsigned_abs();
    let off_hours = abs_minutes / 60;
    let off_mins = abs_minutes % 60;

    let mut cur = Cursor::new(&mut buf[..]);
    let res = cur
        .write_all(bare_bytes)
        .and_then(|()| write!(cur, " {sign}{off_hours:02}:{off_mins:02}"));
    if res.is_err() {
        return NumericValueOutOfRangeSnafu {
            reason: format!(
                "TIMESTAMP_TZ value does not fit in the {}-byte format buffer",
                buf.len()
            ),
        }
        .fail();
    }
    let len = cur.position() as usize;
    // SAFETY: only ASCII digits, '-', '+', ':', ' ', and '.' were written.
    Ok(unsafe { std::str::from_utf8_unchecked(&buf[..len]) })
}

fn write_timestamp_tz_to_odbc(
    value: &TzInstant,
    binding: &Binding,
    get_data_offset: &mut Option<usize>,
) -> Result<Warnings, WriteOdbcError> {
    match binding.target_type {
        // CHAR/WCHAR: render with `+/-HH:MM` suffix (offset preserved).
        CDataType::Char => {
            // Minimum buffer: bare timestamp (>=20) + space + 6-byte suffix = 27.
            if binding.buffer_length > 0 && binding.buffer_length < 27 {
                return NumericValueOutOfRangeSnafu {
                    reason: "Buffer too small for SQL_C_CHAR TIMESTAMP_TZ (minimum 27 bytes)"
                        .to_string(),
                }
                .fail();
            }
            let mut buf = [0u8; 64];
            let s = format_timestamp_tz_string_into(value, &mut buf)?;
            Ok(binding.write_char_string(s, get_data_offset))
        }
        CDataType::WChar => {
            if binding.buffer_length > 0 && binding.buffer_length < 54 {
                return NumericValueOutOfRangeSnafu {
                    reason: "Buffer too small for SQL_C_WCHAR TIMESTAMP_TZ (minimum 54 bytes)"
                        .to_string(),
                }
                .fail();
            }
            let mut buf = [0u8; 64];
            let s = format_timestamp_tz_string_into(value, &mut buf)?;
            Ok(binding.write_wchar_string(s, get_data_offset))
        }
        // For every other target type (TYPE_TIMESTAMP, BINARY, DATE, TIME),
        // delegate to the offset-less writer with the UTC wall-clock. This
        // preserves the established TZ -> SQL_C_TYPE_TIMESTAMP behavior
        // (return UTC, drop offset) which matches the ODBC spec rule for
        // "datetime with timezone -> datetime without timezone".
        _ => write_timestamp_to_odbc(&value.utc, binding, get_data_offset),
    }
}

/// Read a TIMESTAMP_TZ value from a parameter binding. Captures both the UTC
/// instant and the offset so `write_timestamp_tz_json` can emit the legacy
/// two-token wire format.
///
/// Bind paths:
/// - `SQL_C_TYPE_TIMESTAMP` / `SQL_C_BINARY`: the struct has no offset field,
///   so we treat the wall-clock as UTC (offset = 0). Matches the legacy
///   Python connector's treatment of a naive `datetime` bound to TIMESTAMP_TZ.
/// - `SQL_C_CHAR` / `SQL_C_WCHAR`: parse `YYYY-MM-DD HH:MM:SS[.fff] +/-HH:MM`;
///   if no offset suffix is present, fall back to the offset-less parser and
///   treat as UTC (offset = 0). A genuinely unparseable string surfaces as
///   `UnsupportedCDataType` (mapped to SQLSTATE HY000) so the caller learns
///   the bind failed instead of silently storing the Unix epoch.
fn read_timestamp_tz_odbc(binding: &ParameterBinding) -> Result<TzInstant, JsonBindingError> {
    match binding.value_type {
        CDataType::Char => {
            let s = read_char_str(binding)?;
            parse_tz_string_with_fallback(s.trim(), binding.value_type)
        }
        CDataType::WChar => {
            let s = read_wchar_str(binding)?;
            parse_tz_string_with_fallback(s.trim(), binding.value_type)
        }
        _ => {
            // Reuse the existing offset-less reader (handles
            // SQL_C_TYPE_TIMESTAMP, SQL_C_BINARY, etc.) and treat the
            // result as UTC + offset 0.
            let utc = read_timestamp_odbc(binding)?;
            Ok(TzInstant {
                utc,
                offset_minutes: 0,
            })
        }
    }
}

/// Try `YYYY-MM-DD HH:MM:SS[.fff] +/-HH:MM` first, then fall back to the
/// offset-less formats (treated as UTC). Returns `UnsupportedCDataType` if
/// neither parse succeeds.
fn parse_tz_string_with_fallback(
    s: &str,
    c_type: CDataType,
) -> Result<TzInstant, JsonBindingError> {
    for fmt in &["%Y-%m-%d %H:%M:%S%.f %:z", "%Y-%m-%d %H:%M:%S%.f%:z"] {
        if let Ok(dt) = DateTime::<FixedOffset>::parse_from_str(s, fmt) {
            return Ok(TzInstant {
                utc: dt.naive_utc(),
                offset_minutes: dt.offset().local_minus_utc() / 60,
            });
        }
    }
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f"))
        .map(|utc| TzInstant {
            utc,
            offset_minutes: 0,
        })
        .map_err(|_| UnsupportedCDataTypeSnafu { c_type }.build())
}

fn write_timestamp_tz_json(value: TzInstant) -> Result<Value, JsonBindingError> {
    let epoch_nanos = value.utc.and_utc().timestamp_nanos_opt().ok_or_else(|| {
        UnsupportedCDataTypeSnafu {
            c_type: CDataType::TypeTimestamp,
        }
        .build()
    })?;
    let biased_offset = value.offset_minutes + TZ_OFFSET_BIAS_MINUTES;
    Ok(Value::String(format!("{epoch_nanos} {biased_offset}")))
}

// =============================================================================
// Macro for the NTZ/LTZ trait impls (shared offset-less representation).
//
// TZ is implemented manually below because its `Representation` is `TzInstant`
// (UTC datetime + offset minutes) rather than the bare `NaiveDateTime` used
// by NTZ/LTZ.
// =============================================================================

macro_rules! impl_snowflake_timestamp_standard {
    ($name:ident, $logical_type:expr, $sql_type:expr) => {
        impl SnowflakeType for $name {
            type Representation<'a> = NaiveDateTime;
        }

        impl ReadArrowType<StructArray> for $name {
            fn read_arrow_type<'a>(
                &self,
                array: &'a StructArray,
                row_idx: usize,
            ) -> Result<Self::Representation<'a>, ReadArrowError> {
                read_struct_timestamp(array, row_idx)
            }
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
                $sql_type.into()
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

impl_snowflake_timestamp_standard!(
    SnowflakeTimestampNtz,
    SnowflakeLogicalType::TimestampNtz,
    SqlType::SqlSfTimestampNtz
);

pub(crate) struct SnowflakeTimestampLtz {
    pub(crate) scale: u32,
}

impl_snowflake_timestamp_standard!(
    SnowflakeTimestampLtz,
    SnowflakeLogicalType::TimestampLtz,
    SqlType::SqlSfTimestampLtz
);

pub(crate) struct SnowflakeTimestampTz {
    pub(crate) scale: u32,
}

impl SnowflakeType for SnowflakeTimestampTz {
    type Representation<'a> = TzInstant;
}

impl ReadArrowType<StructArray> for SnowflakeTimestampTz {
    fn read_arrow_type<'a>(
        &self,
        array: &'a StructArray,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        read_struct_timestamp_tz(array, row_idx, self.scale)
    }
}

// TZ never actually arrives as a flat Int64 array in practice -- Snowflake
// always sends it as a Struct with the offset column. The trait impl exists
// only to satisfy `make_timestamp_converter!`, which compiles a flat-Int64
// branch for every timestamp type. If TZ ever did show up flat, treating the
// epoch as UTC + offset 0 is the safe fallback (matches what NTZ does).
impl ReadArrowType<PrimitiveArray<Int64Type>> for SnowflakeTimestampTz {
    fn read_arrow_type<'a>(
        &self,
        array: &'a PrimitiveArray<Int64Type>,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        let utc = read_scaled_timestamp(array, row_idx, self.scale)?;
        Ok(TzInstant {
            utc,
            offset_minutes: 0,
        })
    }
}

impl WriteODBCType for SnowflakeTimestampTz {
    fn sql_type(&self) -> sql::SqlDataType {
        SqlType::SqlSfTimestampTz.into()
    }

    fn column_size(&self) -> sql::ULen {
        SQL_SF_TIMESTAMP_TZ_COLUMN_SIZE
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
        write_timestamp_tz_to_odbc(&snowflake_value, binding, get_data_offset)
    }
}

impl ReadODBC for SnowflakeTimestampTz {
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, JsonBindingError> {
        read_timestamp_tz_odbc(binding)
    }
}

impl WriteJson for SnowflakeTimestampTz {
    fn write_json(&self, value: Self::Representation<'_>) -> Result<Value, JsonBindingError> {
        write_timestamp_tz_json(value)
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::TimestampTz
    }
}

#[cfg(test)]
mod format_timestamp_string_into_tests {
    use super::format_timestamp_string_into;
    use chrono::{DateTime, NaiveDate};

    fn fmt(secs: i64, nanos: u32) -> String {
        let dt = DateTime::from_timestamp(secs, nanos)
            .expect("DateTime::from_timestamp with in-range inputs")
            .naive_utc();
        let mut buf = [0u8; 48];
        format_timestamp_string_into(&dt, &mut buf)
            .expect("format_timestamp_string_into")
            .to_string()
    }

    // 2023-11-14 22:13:20 UTC, an arbitrary mid-range instant used to exercise
    // the fractional-seconds trimming paths.
    const REF_EPOCH: i64 = 1_700_000_000;

    #[test]
    fn no_fractional_seconds() {
        assert_eq!(fmt(0, 0), "1970-01-01 00:00:00");
        assert_eq!(fmt(REF_EPOCH, 0), "2023-11-14 22:13:20");
    }

    #[test]
    fn with_fractional_seconds_various_trailing_zero_counts() {
        // Trailing-zero trimming is the interesting behavior to preserve.
        assert_eq!(fmt(REF_EPOCH, 1), "2023-11-14 22:13:20.000000001");
        assert_eq!(fmt(REF_EPOCH, 10), "2023-11-14 22:13:20.00000001");
        assert_eq!(fmt(REF_EPOCH, 123_000_000), "2023-11-14 22:13:20.123");
        assert_eq!(fmt(REF_EPOCH, 123_456_789), "2023-11-14 22:13:20.123456789");
        assert_eq!(fmt(REF_EPOCH, 999_999_999), "2023-11-14 22:13:20.999999999");
    }

    #[test]
    fn pre_epoch_timestamp() {
        assert_eq!(fmt(-1_000, 0), "1969-12-31 23:43:20");
        assert_eq!(fmt(-1_000, 500_000), "1969-12-31 23:43:20.0005");
    }

    #[test]
    fn year_padding() {
        let dt = NaiveDate::from_ymd_opt(1, 1, 1)
            .expect("NaiveDate::from_ymd_opt with in-range inputs")
            .and_hms_opt(0, 0, 0)
            .expect("NaiveDate::and_hms_opt with in-range inputs");
        let mut buf = [0u8; 48];
        assert_eq!(
            format_timestamp_string_into(&dt, &mut buf).expect("format_timestamp_string_into"),
            "0001-01-01 00:00:00"
        );
    }
}
