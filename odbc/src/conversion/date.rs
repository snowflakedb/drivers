use arrow::array::{Array, PrimitiveArray};
use arrow::datatypes::Date32Type;
use chrono::{Datelike, NaiveDate};
use odbc_sys as sql;
use serde_json::Value;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::error::{JsonBindingError, UnsupportedCDataTypeSnafu};
use crate::conversion::error::{
    NumericValueOutOfRangeSnafu, ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError,
};
use crate::conversion::param_binding::{read_char_str, read_unaligned, read_wchar_str};
use crate::conversion::traits::Binding;
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, WriteJson};
use crate::conversion::warning::Warnings;
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

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
                let formatted = format!(
                    "{:04}-{:02}-{:02}",
                    snowflake_value.year(),
                    snowflake_value.month(),
                    snowflake_value.day()
                );
                Ok(binding.write_char_string(&formatted, get_data_offset))
            }
            CDataType::WChar => {
                if binding.buffer_length > 0 && binding.buffer_length < 22 {
                    return NumericValueOutOfRangeSnafu {
                        reason: "Buffer too small for SQL_C_WCHAR date (minimum 22 bytes)"
                            .to_string(),
                    }
                    .fail();
                }
                let formatted = format!(
                    "{:04}-{:02}-{:02}",
                    snowflake_value.year(),
                    snowflake_value.month(),
                    snowflake_value.day()
                );
                Ok(binding.write_wchar_string(&formatted, get_data_offset))
            }
            CDataType::Binary => {
                let date = sql::Date {
                    year: snowflake_value.year() as i16,
                    month: snowflake_value.month() as u16,
                    day: snowflake_value.day() as u16,
                };
                let mut bytes = [0u8; std::mem::size_of::<sql::Date>()];
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        &date as *const sql::Date as *const u8,
                        bytes.as_mut_ptr(),
                        bytes.len(),
                    );
                }
                if binding.buffer_length > 0
                    && (binding.buffer_length as usize) < std::mem::size_of::<sql::Date>()
                {
                    return NumericValueOutOfRangeSnafu {
                        reason: "Buffer too small for SQL_C_BINARY date".to_string(),
                    }
                    .fail();
                }
                Ok(binding.write_binary(&bytes, get_data_offset))
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
