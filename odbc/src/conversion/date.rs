use std::io::{Cursor, Write as _};

use arrow::array::{Array, PrimitiveArray};
use arrow::datatypes::Date32Type;
use chrono::{Datelike, NaiveDate};
use odbc_sys as sql;
use serde_json::Value;

use snafu::ResultExt;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::batched::{BatchedWrite, write_odbc_segment_per_row};
use crate::conversion::error::{
    BindingNumericOutOfRangeSnafu, ConversionError, JsonBindingError, ReadArrowValueSnafu,
    UnsupportedCDataTypeSnafu, WriteOdbcValueSnafu,
};
use crate::conversion::error::{
    NumericValueOutOfRangeSnafu, ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError,
};
use crate::conversion::param_binding::{
    read_binary_struct, read_char_str, read_unaligned, read_wchar_str,
};
use crate::conversion::traits::{Binding, BindingStrides};
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, WriteJson};
use crate::conversion::warning::Warnings;
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

/// Format a `NaiveDate` as `YYYY-MM-DD` into a stack buffer without heap
/// allocation. 32 bytes is sufficient for any year chrono can represent.
fn format_date_ascii<'a>(date: &NaiveDate, buf: &'a mut [u8; 32]) -> &'a str {
    let mut cursor = Cursor::new(&mut buf[..]);
    // Infallible: the buffer is large enough for any year chrono produces.
    let _ = write!(
        cursor,
        "{:04}-{:02}-{:02}",
        date.year(),
        date.month(),
        date.day()
    );
    let len = cursor.position() as usize;
    // SAFETY: we only wrote ASCII digits and '-' above.
    unsafe { std::str::from_utf8_unchecked(&buf[..len]) }
}

pub(crate) struct SnowflakeDate;

const UNIX_EPOCH: NaiveDate = match NaiveDate::from_ymd_opt(1970, 1, 1) {
    Some(d) => d,
    None => unreachable!(),
};

impl SnowflakeType for SnowflakeDate {
    type Representation<'a> = NaiveDate;
}

impl ReadArrowType<PrimitiveArray<Date32Type>> for SnowflakeDate {
    fn read_arrow_type<'a>(
        &self,
        array: &'a PrimitiveArray<Date32Type>,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        if array.is_null(row_idx) {
            return Err(ReadArrowError::NullValue {
                location: snafu::location!(),
            });
        }
        let days_since_epoch = array.value(row_idx);
        let date = UNIX_EPOCH + chrono::Duration::days(days_since_epoch as i64);
        Ok(date)
    }
}

impl WriteODBCType for SnowflakeDate {
    fn sql_type(&self) -> sql::SqlDataType {
        sql::SqlDataType::DATE
    }

    fn column_size(&self) -> sql::ULen {
        10
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
        match binding.target_type {
            CDataType::Default | CDataType::Date | CDataType::TypeDate => {
                let date = sql::Date {
                    year: snowflake_value.year() as i16,
                    month: snowflake_value.month() as u16,
                    day: snowflake_value.day() as u16,
                };
                binding.write_fixed(date);
                Ok(vec![])
            }
            CDataType::Char => {
                if binding.buffer_length > 0 && binding.buffer_length < 11 {
                    return NumericValueOutOfRangeSnafu {
                        reason: "Buffer too small for SQL_C_CHAR date (minimum 11 bytes)"
                            .to_string(),
                    }
                    .fail();
                }
                let mut buf = [0u8; 32];
                let s = format_date_ascii(&snowflake_value, &mut buf);
                Ok(binding.write_char_string(s, get_data_offset))
            }
            CDataType::WChar => {
                if binding.buffer_length > 0 && binding.buffer_length < 22 {
                    return NumericValueOutOfRangeSnafu {
                        reason: "Buffer too small for SQL_C_WCHAR date (minimum 22 bytes)"
                            .to_string(),
                    }
                    .fail();
                }
                let mut buf = [0u8; 32];
                let s = format_date_ascii(&snowflake_value, &mut buf);
                Ok(binding.write_wchar_string(s, get_data_offset))
            }
            CDataType::Binary => {
                if binding.buffer_length > 0
                    && (binding.buffer_length as usize) < std::mem::size_of::<sql::Date>()
                {
                    return NumericValueOutOfRangeSnafu {
                        reason: "Buffer too small for SQL_C_BINARY date".to_string(),
                    }
                    .fail();
                }
                let date = sql::Date {
                    year: snowflake_value.year() as i16,
                    month: snowflake_value.month() as u16,
                    day: snowflake_value.day() as u16,
                };
                // SAFETY: `sql::Date` is a POD struct (repr(C), no padding
                // beyond the u16/i16 layout) defined by odbc_sys. Borrowing
                // its bytes for the duration of this call is sound and lets
                // us avoid an intermediate stack copy per row.
                let bytes = unsafe {
                    std::slice::from_raw_parts(
                        &date as *const sql::Date as *const u8,
                        std::mem::size_of::<sql::Date>(),
                    )
                };
                Ok(binding.write_binary(bytes, get_data_offset))
            }
            CDataType::TimeStamp | CDataType::TypeTimestamp => {
                let ts = sql::Timestamp {
                    year: snowflake_value.year() as i16,
                    month: snowflake_value.month() as u16,
                    day: snowflake_value.day() as u16,
                    hour: 0,
                    minute: 0,
                    second: 0,
                    fraction: 0,
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

/// Hoist the `target_type` match out of the per-cell loop and skip the
/// `chrono::Duration` round-trip on the `SQL_C_CHAR` hot path. Other
/// targets keep the existing per-row dispatch.
impl BatchedWrite<PrimitiveArray<Date32Type>> for SnowflakeDate {
    fn write_odbc_segment(
        &self,
        array: &PrimitiveArray<Date32Type>,
        arrow_row_range: std::ops::Range<usize>,
        base_binding: &Binding,
        out_row_start: usize,
        strides: BindingStrides,
        outputs: &mut [Result<Warnings, ConversionError>],
    ) {
        if !matches!(base_binding.target_type, CDataType::Char) {
            write_odbc_segment_per_row(
                self,
                array,
                arrow_row_range,
                base_binding,
                out_row_start,
                strides,
                outputs,
            );
            return;
        }

        if base_binding.buffer_length > 0 && base_binding.buffer_length < 11 {
            // Single check up-front, then mark every row as failed.
            for slot in outputs.iter_mut().take(arrow_row_range.len()) {
                if slot.is_ok() {
                    *slot = Err(WriteOdbcError::NumericValueOutOfRange {
                        reason: "Buffer too small for SQL_C_CHAR date (minimum 11 bytes)"
                            .to_string(),
                        location: snafu::location!(),
                    })
                    .context(WriteOdbcValueSnafu);
                }
            }
            return;
        }

        let values = array.values();
        let validity = array.nulls();
        let mut buf = [0u8; 32];

        for (i, batch_idx) in arrow_row_range.enumerate() {
            if outputs[i].is_err() {
                continue;
            }

            if let Some(nulls) = validity
                && !nulls.is_valid(batch_idx)
            {
                outputs[i] = Err(ReadArrowError::NullValue {
                    location: snafu::location!(),
                })
                .context(ReadArrowValueSnafu);
                continue;
            }

            let days = values[batch_idx];
            let date = UNIX_EPOCH + chrono::Duration::days(days as i64);
            let s = format_date_ascii(&date, &mut buf);

            let binding = match strides.for_row(base_binding, out_row_start + i) {
                Ok(b) => b,
                Err(e) => {
                    outputs[i] = Err(e);
                    continue;
                }
            };
            let warnings = binding.write_char_string(s, &mut None);
            if let Ok(existing) = &mut outputs[i] {
                existing.extend(warnings);
            }
        }
    }
}

impl ReadODBC for SnowflakeDate {
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, JsonBindingError> {
        match binding.value_type {
            CDataType::Date | CDataType::TypeDate => {
                let date = read_unaligned::<sql::Date>(binding);
                NaiveDate::from_ymd_opt(date.year as i32, date.month as u32, date.day as u32)
                    .ok_or_else(|| {
                        UnsupportedCDataTypeSnafu {
                            c_type: binding.value_type,
                        }
                        .build()
                    })
            }
            CDataType::Char => {
                let s = read_char_str(binding)?;
                NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").map_err(|_| {
                    UnsupportedCDataTypeSnafu {
                        c_type: binding.value_type,
                    }
                    .build()
                })
            }
            CDataType::WChar => {
                let s = read_wchar_str(binding)?;
                NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").map_err(|_| {
                    UnsupportedCDataTypeSnafu {
                        c_type: binding.value_type,
                    }
                    .build()
                })
            }
            CDataType::Binary => {
                let date = read_binary_struct::<sql::Date>(binding, "SQL_DATE_STRUCT")?;
                NaiveDate::from_ymd_opt(date.year as i32, date.month as u32, date.day as u32)
                    .ok_or_else(|| {
                        BindingNumericOutOfRangeSnafu {
                            reason: format!(
                                "invalid date from SQL_C_BINARY: year={}, month={}, day={}",
                                date.year, date.month, date.day
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

impl WriteJson for SnowflakeDate {
    fn write_json(&self, value: Self::Representation<'_>) -> Result<Value, JsonBindingError> {
        let millis = (value - UNIX_EPOCH).num_days() * 86_400_000;
        Ok(Value::String(millis.to_string()))
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::Date
    }
}

#[cfg(test)]
mod format_date_ascii_tests {
    use super::*;

    #[test]
    fn formats_typical_date() {
        let mut buf = [0u8; 32];
        let d = NaiveDate::from_ymd_opt(2026, 4, 12).unwrap();
        assert_eq!(format_date_ascii(&d, &mut buf), "2026-04-12");
    }

    #[test]
    fn pads_single_digit_components() {
        let mut buf = [0u8; 32];
        let d = NaiveDate::from_ymd_opt(1, 1, 1).unwrap();
        assert_eq!(format_date_ascii(&d, &mut buf), "0001-01-01");
    }

    #[test]
    fn formats_large_year() {
        let mut buf = [0u8; 32];
        let d = NaiveDate::from_ymd_opt(9999, 12, 31).unwrap();
        assert_eq!(format_date_ascii(&d, &mut buf), "9999-12-31");
    }

    #[test]
    fn formats_negative_year() {
        let mut buf = [0u8; 32];
        let d = NaiveDate::from_ymd_opt(-44, 3, 15).unwrap();
        assert_eq!(format_date_ascii(&d, &mut buf), "-044-03-15");
    }
}
