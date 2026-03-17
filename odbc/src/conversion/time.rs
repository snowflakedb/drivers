use arrow::array::{Array, PrimitiveArray};
use arrow::datatypes::Int64Type;
use chrono::{NaiveTime, Timelike};
use odbc_sys as sql;
use serde_json::Value;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::error::{JsonBindingError, UnsupportedCDataTypeSnafu};
use crate::conversion::error::{ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError};
use crate::conversion::param_binding::{read_char_str, read_unaligned, read_wchar_str};
use crate::conversion::traits::Binding;
use crate::conversion::traits::SnowflakeType;
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, WriteJson};
use crate::conversion::warning::Warnings;
use crate::conversion::{ReadArrowType, WriteODBCType};

pub(crate) struct SnowflakeTime;

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
        let nanos_since_midnight = array.value(row_idx);
        let secs = (nanos_since_midnight / 1_000_000_000) as u32;
        let nanos = (nanos_since_midnight % 1_000_000_000) as u32;
        NaiveTime::from_num_seconds_from_midnight_opt(secs, nanos).ok_or(
            ReadArrowError::NullValue {
                location: snafu::location!(),
            },
        )
    }
}

impl WriteODBCType for SnowflakeTime {
    fn sql_type(&self) -> sql::SqlDataType {
        sql::SqlDataType::TIME
    }

    fn column_size(&self) -> sql::ULen {
        8
    }

    fn decimal_digits(&self) -> sql::SmallInt {
        9
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
                Ok(vec![])
            }
            CDataType::Char => {
                let formatted = snowflake_value.format("%H:%M:%S").to_string();
                Ok(binding.write_char_string(&formatted, get_data_offset))
            }
            CDataType::WChar => {
                let formatted = snowflake_value.format("%H:%M:%S").to_string();
                Ok(binding.write_wchar_string(&formatted, get_data_offset))
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
