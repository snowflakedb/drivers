use arrow::array::Float64Array;
use odbc_sys as sql;

use crate::cdata_types::CDataType;
use crate::conversion::error::{ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError};
use crate::conversion::traits::Binding;
use crate::conversion::warning::Warnings;
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

/// Handles Snowflake's "REAL" logical type (FLOAT, DOUBLE, REAL).
/// The old driver maps "real" → SQL_DOUBLE; the default C type is SQL_C_DOUBLE.
pub(crate) struct SnowflakeReal;

impl SnowflakeType for SnowflakeReal {
    type Representation<'a> = f64;
}

impl ReadArrowType<Float64Array> for SnowflakeReal {
    fn read_arrow_type<'a>(
        &self,
        array: &'a Float64Array,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        Ok(array.value(row_idx))
    }
}

impl WriteODBCType for SnowflakeReal {
    fn sql_type(&self) -> sql::SqlDataType {
        sql::SqlDataType::DOUBLE
    }

    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
        _get_data_offset: &mut Option<usize>,
    ) -> Result<Warnings, WriteOdbcError> {
        let target_type = match binding.target_type {
            CDataType::Default => CDataType::Double,
            other => other,
        };
        match target_type {
            CDataType::Double => {
                binding.write_fixed(snowflake_value);
                Ok(vec![])
            }
            _ => UnsupportedOdbcTypeSnafu { target_type }.fail(),
        }
    }
}
