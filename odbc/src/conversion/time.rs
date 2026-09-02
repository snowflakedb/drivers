use arrow::array::PrimitiveArray;
use arrow::datatypes::ArrowPrimitiveType;
use chrono::{Datelike, NaiveDate, NaiveTime, Timelike};
use odbc_sys as sql;
use snafu::OptionExt;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::error::{
    BindingError, BindingNumericOutOfRangeSnafu, DatetimeFieldOverflowSnafu,
    InvalidDatetimeValueSnafu, NumericValueOutOfRangeSnafu, ReadArrowError,
    UnsupportedCDataTypeSnafu, UnsupportedOdbcTypeSnafu, WriteOdbcError,
};
use crate::conversion::int_fmt;
use crate::conversion::param_binding::{
    parse_temporal_char_input, read_binary_struct, read_unaligned,
};
use crate::conversion::traits::{Binding, ReadODBC, SnowflakeLogicalType, WriteWire};
use crate::conversion::warning::{Warning, Warnings};
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

/// Expected literal shape for a `SQL_C_CHAR` / `SQL_C_WCHAR` source bound to a
/// TIME target, surfaced in the 22018 diagnostic when parsing fails.
const TIME_CHAR_EXPECTED_FORMAT: &str = "HH:MM:SS[.fffffffff]";

/// Format a `NaiveTime` as `HH:MM:SS[.fffffffff]` into a stack buffer without
/// heap allocation. 32 bytes is ample for the widest output (`HH:MM:SS.` + 9
/// fractional digits = 18 bytes).
fn format_time_ascii<'a>(time: &NaiveTime, buf: &'a mut [u8; 32]) -> &'a str {
    // Hand-rolled digit writes rather than `write!`/`core::fmt`, the dominant
    // per-cell cost for temporal SQL_C_CHAR rendering.
    let mut p = int_fmt::put_padded(buf, 0, time.hour(), 2);
    buf[p] = b':';
    p = int_fmt::put_padded(buf, p + 1, time.minute(), 2);
    buf[p] = b':';
    p = int_fmt::put_padded(buf, p + 1, time.second(), 2);
    // Snowflake TIME fractions are < 1e9 (scale ≤ 9, no leap seconds), so the
    // fraction is exactly 9 digits — matching the old `{:09}` — before
    // trailing zeros are trimmed.
    let nanos = time.nanosecond();
    if nanos != 0 {
        buf[p] = b'.';
        p = int_fmt::put_padded(buf, p + 1, nanos, 9);
        while buf[p - 1] == b'0' {
            p -= 1;
        }
    }
    // SAFETY: only ASCII digits, `:`, and `.` written above.
    unsafe { std::str::from_utf8_unchecked(&buf[..p]) }
}

pub(crate) use sf_types::SnowflakeTime;

impl SnowflakeType for SnowflakeTime {
    type Representation<'a> = NaiveTime;
}

impl<T: ArrowPrimitiveType> ReadArrowType<PrimitiveArray<T>> for SnowflakeTime
where
    T::Native: Into<i64>,
{
    fn read_arrow_type<'a>(
        &self,
        array: &'a PrimitiveArray<T>,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        Ok(sf_types::ReadArrowType::read_arrow_type(
            self, array, row_idx,
        )?)
    }
}

impl WriteODBCType for SnowflakeTime {
    fn sql_type(&self) -> sql::SqlDataType {
        sql::SqlDataType::TIME
    }

    fn column_size(&self) -> sql::ULen {
        if self.scale == 0 {
            8
        } else {
            9 + self.scale as sql::ULen
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
        match binding.target_type {
            CDataType::Default | CDataType::Time | CDataType::TypeTime => {
                let time = sql::Time {
                    hour: snowflake_value.hour() as u16,
                    minute: snowflake_value.minute() as u16,
                    second: snowflake_value.second() as u16,
                };
                binding.write_fixed(time);
                // SQL_TIME_STRUCT has no fraction field, so any sub-second
                // component of the source TIME is genuinely dropped here. Mirror
                // the reference driver: flag the data loss with 01S07
                // (NumericValueTruncated) when a fraction is present, and emit
                // nothing when there is none to lose.
                if snowflake_value.nanosecond() != 0 {
                    Ok(vec![Warning::NumericValueTruncated])
                } else {
                    Ok(vec![])
                }
            }
            CDataType::Char => {
                if binding.buffer_length > 0 && binding.buffer_length < 9 {
                    return NumericValueOutOfRangeSnafu {
                        reason: "Buffer too small for SQL_C_CHAR time (minimum 9 bytes)"
                            .to_string(),
                    }
                    .fail();
                }
                let mut buf = [0u8; 32];
                let s = format_time_ascii(&snowflake_value, &mut buf);
                Ok(binding.write_char_string(s, get_data_offset))
            }
            CDataType::WChar => {
                if binding.buffer_length > 0 && binding.buffer_length < 18 {
                    return NumericValueOutOfRangeSnafu {
                        reason: "Buffer too small for SQL_C_WCHAR time (minimum 18 bytes)"
                            .to_string(),
                    }
                    .fail();
                }
                let mut buf = [0u8; 32];
                let s = format_time_ascii(&snowflake_value, &mut buf);
                Ok(binding.write_wchar_string(s, get_data_offset))
            }
            CDataType::Binary => {
                if binding.buffer_length > 0
                    && (binding.buffer_length as usize) < std::mem::size_of::<sql::Time>()
                {
                    return NumericValueOutOfRangeSnafu {
                        reason: "Buffer too small for SQL_C_BINARY time".to_string(),
                    }
                    .fail();
                }
                let time = sql::Time {
                    hour: snowflake_value.hour() as u16,
                    minute: snowflake_value.minute() as u16,
                    second: snowflake_value.second() as u16,
                };
                // SAFETY: `sql::Time` is a POD struct; borrowing its bytes
                // for the duration of this call avoids an intermediate stack
                // copy per row.
                let bytes = unsafe {
                    std::slice::from_raw_parts(
                        &time as *const sql::Time as *const u8,
                        std::mem::size_of::<sql::Time>(),
                    )
                };
                Ok(binding.write_binary(bytes, get_data_offset))
            }
            CDataType::TimeStamp | CDataType::TypeTimestamp => {
                let today = chrono::Local::now().date_naive();
                let ts = sql::Timestamp {
                    year: today.year() as i16,
                    month: today.month() as u16,
                    day: today.day() as u16,
                    hour: snowflake_value.hour() as u16,
                    minute: snowflake_value.minute() as u16,
                    second: snowflake_value.second() as u16,
                    // SQL_TIMESTAMP_STRUCT.fraction holds nanoseconds, so the
                    // sub-second component survives the conversion. Preserve it
                    // (mirroring the reference driver) rather than zeroing it —
                    // no data is lost, so no truncation warning is raised.
                    fraction: snowflake_value.nanosecond(),
                };
                binding.write_fixed(ts);
                Ok(vec![])
            }
            _ => UnsupportedOdbcTypeSnafu {
                target_type: binding.target_type,
            }
            .fail(),
        }
    }
}

impl ReadODBC for SnowflakeTime {
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, BindingError> {
        match binding.value_type {
            CDataType::Time | CDataType::TypeTime => {
                let time = read_unaligned::<sql::Time>(binding);
                NaiveTime::from_hms_opt(time.hour as u32, time.minute as u32, time.second as u32)
                    .with_context(|| InvalidDatetimeValueSnafu {
                        reason: format!(
                            "invalid time in SQL_C_TYPE_TIME for TIME target: \
                                 hour={}, minute={}, second={}",
                            time.hour, time.minute, time.second
                        ),
                    })
            }
            CDataType::Char | CDataType::WChar => {
                parse_temporal_char_input(binding, TIME_CHAR_EXPECTED_FORMAT, |s| {
                    NaiveTime::parse_from_str(s, "%H:%M:%S")
                        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M:%S%.f"))
                        .map_err(|_| ())
                })
            }
            CDataType::Binary => {
                let time = read_binary_struct::<sql::Time>(binding, "SQL_TIME_STRUCT")?;
                NaiveTime::from_hms_opt(time.hour as u32, time.minute as u32, time.second as u32)
                    .with_context(|| BindingNumericOutOfRangeSnafu {
                        reason: format!(
                            "invalid time from SQL_C_BINARY: hour={}, minute={}, second={}",
                            time.hour, time.minute, time.second
                        ),
                    })
            }
            // Bind SQL_C_TYPE_TIMESTAMP into a TIME column by extracting the
            // time portion of the timestamp. Per ODBC Appendix D, the
            // conversion only succeeds when the discarded fractional-seconds
            // portion is exactly zero; otherwise SQLSTATE 22008 ("Datetime
            // field overflow") is returned. The whole-second time fields are
            // always preserved; the date portion is silently discarded.
            //
            // Error precedence: validate the *whole struct* first (22007 —
            // also catches fraction > 999_999_999, hour > 23, month=13, …)
            // and only then enforce the 22008 fractional-seconds rule. Even
            // though the date portion is silently dropped, it must still be
            // a syntactically valid Y/M/D — otherwise an input like
            // {year=2024, month=13, day=1, hour=14, ...} would silently
            // succeed despite the struct being malformed.
            CDataType::TimeStamp | CDataType::TypeTimestamp => {
                let ts = read_unaligned::<sql::Timestamp>(binding);
                NaiveDate::from_ymd_opt(ts.year as i32, ts.month as u32, ts.day as u32)
                    .with_context(|| InvalidDatetimeValueSnafu {
                        reason: format!(
                            "invalid date in SQL_C_TYPE_TIMESTAMP for TIME target: \
                                 year={}, month={}, day={}",
                            ts.year, ts.month, ts.day
                        ),
                    })?;
                let time = NaiveTime::from_hms_nano_opt(
                    ts.hour as u32,
                    ts.minute as u32,
                    ts.second as u32,
                    ts.fraction,
                )
                .with_context(|| InvalidDatetimeValueSnafu {
                    reason: format!(
                        "invalid time in SQL_C_TYPE_TIMESTAMP for TIME target: \
                             hour={}, minute={}, second={}, fraction={}",
                        ts.hour, ts.minute, ts.second, ts.fraction
                    ),
                })?;
                if ts.fraction != 0 {
                    return DatetimeFieldOverflowSnafu {
                        reason: format!(
                            "SQL_C_TYPE_TIMESTAMP → SQL_TYPE_TIME: fractional seconds \
                             must be zero (got fraction={})",
                            ts.fraction
                        ),
                    }
                    .fail();
                }
                Ok(time)
            }
            _ => UnsupportedCDataTypeSnafu {
                c_type: binding.value_type,
            }
            .fail(),
        }
    }
}

impl WriteWire for SnowflakeTime {
    fn write_wire(&self, value: Self::Representation<'_>) -> Result<String, BindingError> {
        let secs = value.num_seconds_from_midnight() as i64;
        let nanos = value.nanosecond() as i64;
        let total_nanos = secs * 1_000_000_000 + nanos;
        Ok(total_nanos.to_string())
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::Time
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    fn fmt(h: u32, m: u32, s: u32, nano: u32) -> String {
        let t = NaiveTime::from_hms_nano_opt(h, m, s, nano).unwrap();
        let mut buf = [0u8; 32];
        format_time_ascii(&t, &mut buf).to_string()
    }

    #[test]
    fn format_time_ascii_matches_expected() {
        // No fraction — bounded fields are always two digits.
        assert_eq!(fmt(0, 0, 0, 0), "00:00:00");
        assert_eq!(fmt(1, 2, 3, 0), "01:02:03");
        assert_eq!(fmt(23, 59, 59, 0), "23:59:59");
        // Fractions render as up to 9 digits (matching the old `{:09}`) with
        // trailing zeros trimmed.
        assert_eq!(fmt(12, 34, 56, 1), "12:34:56.000000001");
        assert_eq!(fmt(12, 34, 56, 123_000_000), "12:34:56.123");
        assert_eq!(fmt(12, 34, 56, 123_456_789), "12:34:56.123456789");
        assert_eq!(fmt(12, 34, 56, 900_000_000), "12:34:56.9");
    }
}
