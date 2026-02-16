use arrow::array::{Array, StructArray};
use arrow::datatypes::{Int32Type, Int64Type};
use chrono::{DateTime, Datelike, NaiveDateTime, Timelike};
use odbc_sys as sql;

use crate::cdata_types::CDataType;
use crate::conversion::error::{ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError};
use crate::conversion::traits::Binding;
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
