use std::io::{Cursor, Write as _};

use arrow::array::{Array, PrimitiveArray};
use arrow::datatypes::Int64Type;
use chrono::{Datelike, NaiveTime, Timelike};
use odbc_sys as sql;
use serde_json::Value;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::error::{
    BindingNumericOutOfRangeSnafu, InvalidArrowValueSnafu, JsonBindingError,
    NumericValueOutOfRangeSnafu, ReadArrowError, UnsupportedCDataTypeSnafu,
    UnsupportedOdbcTypeSnafu, WriteOdbcError,
};
use crate::conversion::param_binding::{
    read_binary_struct, read_char_str, read_unaligned, read_wchar_str,
};
use crate::conversion::traits::{Binding, ReadODBC, SnowflakeLogicalType, WriteJson};
use crate::conversion::warning::{Warning, Warnings};
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

/// Format a `NaiveTime` as `HH:MM:SS[.fffffffff]` into a stack buffer without
/// heap allocation. 32 bytes is ample for the widest output (`HH:MM:SS.` + 9
/// fractional digits = 18 bytes).
fn format_time_ascii<'a>(time: &NaiveTime, buf: &'a mut [u8; 32]) -> &'a str {
    let len = {
        let mut cursor = Cursor::new(&mut buf[..]);
        let _ = write!(
            cursor,
            "{:02}:{:02}:{:02}",
            time.hour(),
            time.minute(),
            time.second()
        );
        let nanos = time.nanosecond();
        if nanos != 0 {
            let _ = write!(cursor, ".{nanos:09}");
        }
        cursor.position() as usize
    };
    // Trim trailing zeros from the optional fractional part.
    let mut end = len;
    if buf[..end].contains(&b'.') {
        while end > 0 && buf[end - 1] == b'0' {
            end -= 1;
        }
    }
    // SAFETY: only ASCII digits, `:`, and `.` written above.
    unsafe { std::str::from_utf8_unchecked(&buf[..end]) }
}

pub(crate) struct SnowflakeTime {
    pub(crate) scale: u32,
}

impl SnowflakeType for SnowflakeTime {
    type Representation<'a> = NaiveTime;
}

impl ReadArrowType<PrimitiveArray<Int64Type>> for SnowflakeTime {
    fn read_arrow_type<'a>(
        &self,
        array: &'a PrimitiveArray<Int64Type>,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        if array.is_null(row_idx) {
            return Err(ReadArrowError::NullValue {
                location: snafu::location!(),
            });
        }
        if self.scale > 9 {
            return InvalidArrowValueSnafu {
                reason: format!("TIME scale {} exceeds maximum of 9", self.scale),
            }
            .fail();
        }
        let raw = array.value(row_idx);
        if raw < 0 {
            return InvalidArrowValueSnafu {
                reason: format!("negative TIME value: {raw}"),
            }
            .fail();
        }
        let divisor = 10i64.pow(self.scale);
        let secs_i64 = raw / divisor;
        if !(0..86_400).contains(&secs_i64) {
            return InvalidArrowValueSnafu {
                reason: format!("TIME seconds {secs_i64} out of range 0..86399"),
            }
            .fail();
        }
        let secs = secs_i64 as u32;
        let frac = (raw % divisor) as u32;
        let nanos = frac * 10u32.pow(9 - self.scale);
        NaiveTime::from_num_seconds_from_midnight_opt(secs, nanos).ok_or_else(|| {
            InvalidArrowValueSnafu {
                reason: format!("out-of-range TIME: secs={secs}, nanos={nanos}"),
            }
            .build()
        })
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
                    fraction: 0,
                };
                binding.write_fixed(ts);
                if snowflake_value.nanosecond() != 0 {
                    Ok(vec![Warning::NumericValueTruncated])
                } else {
                    Ok(vec![])
                }
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
    ) -> Result<Self::Representation<'a>, JsonBindingError> {
        match binding.value_type {
            CDataType::Time | CDataType::TypeTime => {
                let time = read_unaligned::<sql::Time>(binding);
                NaiveTime::from_hms_opt(time.hour as u32, time.minute as u32, time.second as u32)
                    .ok_or_else(|| {
                        UnsupportedCDataTypeSnafu {
                            c_type: binding.value_type,
                        }
                        .build()
                    })
            }
            CDataType::Char => {
                let s = read_char_str(binding)?;
                NaiveTime::parse_from_str(s.trim(), "%H:%M:%S")
                    .or_else(|_| NaiveTime::parse_from_str(s.trim(), "%H:%M:%S%.f"))
                    .map_err(|_| {
                        UnsupportedCDataTypeSnafu {
                            c_type: binding.value_type,
                        }
                        .build()
                    })
            }
            CDataType::WChar => {
                let s = read_wchar_str(binding)?;
                NaiveTime::parse_from_str(s.trim(), "%H:%M:%S")
                    .or_else(|_| NaiveTime::parse_from_str(s.trim(), "%H:%M:%S%.f"))
                    .map_err(|_| {
                        UnsupportedCDataTypeSnafu {
                            c_type: binding.value_type,
                        }
                        .build()
                    })
            }
            CDataType::Binary => {
                let time = read_binary_struct::<sql::Time>(binding, "SQL_TIME_STRUCT")?;
                NaiveTime::from_hms_opt(time.hour as u32, time.minute as u32, time.second as u32)
                    .ok_or_else(|| {
                        BindingNumericOutOfRangeSnafu {
                            reason: format!(
                                "invalid time from SQL_C_BINARY: hour={}, minute={}, second={}",
                                time.hour, time.minute, time.second
                            ),
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
}

impl WriteJson for SnowflakeTime {
    fn write_json(&self, value: Self::Representation<'_>) -> Result<Value, JsonBindingError> {
        let secs = value.num_seconds_from_midnight() as i64;
        let nanos = value.nanosecond() as i64;
        let total_nanos = secs * 1_000_000_000 + nanos;
        Ok(Value::String(total_nanos.to_string()))
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::Time
    }
}
