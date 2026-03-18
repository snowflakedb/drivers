use chrono::{NaiveTime, Timelike};
use odbc_sys as sql;
use serde_json::Value;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::error::{JsonBindingError, UnsupportedCDataTypeSnafu};
use crate::conversion::param_binding::{read_char_str, read_unaligned, read_wchar_str};
use crate::conversion::traits::SnowflakeType;
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, WriteJson};

pub(crate) struct SnowflakeTime;

impl SnowflakeType for SnowflakeTime {
    type Representation<'a> = NaiveTime;
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
