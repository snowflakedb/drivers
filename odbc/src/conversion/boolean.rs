use arrow::array::{Array, BooleanArray};

use crate::cdata_types::CDataType;
use crate::conversion::error::{ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError};
use crate::conversion::traits::Binding;
use crate::conversion::warning::Warnings;
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

pub(crate) struct SnowflakeBoolean;

impl SnowflakeType for SnowflakeBoolean {
    type Representation<'a> = bool;
}

impl ReadArrowType<BooleanArray> for SnowflakeBoolean {
    fn read_arrow_type<'a>(
        &self,
        array: &'a BooleanArray,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        if array.is_null(row_idx) {
            return Err(ReadArrowError::NullValue {
                location: snafu::location!(),
            });
        }
        Ok(array.value(row_idx))
    }
}

impl WriteODBCType for SnowflakeBoolean {
    fn sql_type(&self) -> odbc_sys::SqlDataType {
        odbc_sys::SqlDataType::EXT_BIT
    }

    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
        _get_data_offset: &mut Option<usize>,
    ) -> Result<Warnings, WriteOdbcError> {
        match binding.target_type {
            CDataType::Default | CDataType::Bit => {
                binding.write_fixed(snowflake_value as u8);
                Ok(vec![])
            }
            _ => UnsupportedOdbcTypeSnafu {
                target_type: binding.target_type,
            }
            .fail(),
        }
    }
}
