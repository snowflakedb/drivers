use arrow::array::{Array, StructArray};
use arrow::datatypes::{Int32Type, Int64Type};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use odbc_sys as sql;
use serde_json::Value;

use crate::api::ParameterBinding;
use crate::cdata_types::CDataType;
use crate::conversion::error::{JsonBindingError, UnsupportedCDataTypeSnafu};
use crate::conversion::error::{ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError};
use crate::conversion::param_binding::{read_char_str, read_unaligned, read_wchar_str};
use crate::conversion::traits::Binding;
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, WriteJson};
use crate::conversion::warning::Warnings;
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

pub(crate) struct SnowflakeTimestampNtz;

impl SnowflakeType for SnowflakeTimestampNtz {
    type Representation<'a> = NaiveDateTime;
}

impl ReadArrowType<StructArray> for SnowflakeTimestampNtz {
    fn read_arrow_type<'a>(
        &self,
        array: &'a StructArray,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        if array.is_null(row_idx) {
            return Err(ReadArrowError::NullValue {
                location: snafu::location!(),
            });
        }

        let epoch_array = array
            .column(0)
            .as_any()
            .downcast_ref::<arrow::array::PrimitiveArray<Int64Type>>()
            .expect("epoch field should be Int64");
        let fraction_array = array
            .column(1)
            .as_any()
            .downcast_ref::<arrow::array::PrimitiveArray<Int32Type>>()
            .expect("fraction field should be Int32");

        let epoch_seconds = epoch_array.value(row_idx);
        let fraction_nanos = fraction_array.value(row_idx);

        let dt = DateTime::from_timestamp(epoch_seconds, fraction_nanos as u32)
            .expect("valid timestamp")
            .naive_utc();
        Ok(dt)
    }
}

impl WriteODBCType for SnowflakeTimestampNtz {
    fn sql_type(&self) -> sql::SqlDataType {
        sql::SqlDataType::TIMESTAMP
    }

    fn column_size(&self) -> sql::ULen {
        29
    }

    fn decimal_digits(&self) -> sql::SmallInt {
        9
    }

    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
        _get_data_offset: &mut Option<usize>,
    ) -> Result<Warnings, WriteOdbcError> {
        match binding.target_type {
            CDataType::Default | CDataType::TimeStamp | CDataType::TypeTimestamp => {
                let ts = sql::Timestamp {
                    year: snowflake_value.year() as i16,
                    month: snowflake_value.month() as u16,
                    day: snowflake_value.day() as u16,
                    hour: snowflake_value.hour() as u16,
                    minute: snowflake_value.minute() as u16,
                    second: snowflake_value.second() as u16,
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

impl ReadODBC for SnowflakeTimestampNtz {
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, JsonBindingError> {
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
}

impl WriteJson for SnowflakeTimestampNtz {
    fn write_json(&self, value: Self::Representation<'_>) -> Result<Value, JsonBindingError> {
        let epoch_nanos = value.and_utc().timestamp_nanos_opt().ok_or_else(|| {
            UnsupportedCDataTypeSnafu {
                c_type: CDataType::TypeTimestamp,
            }
            .build()
        })?;
        Ok(Value::String(epoch_nanos.to_string()))
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::TimestampNtz
    }
}
