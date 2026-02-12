use arrow::array::{Array, GenericByteArray};
use arrow::datatypes::GenericBinaryType;
use odbc_sys as sql;

use crate::cdata_types::CDataType;
use crate::conversion::error::{ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError};
use crate::conversion::traits::Binding;
use crate::conversion::warning::Warnings;
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

pub(crate) struct SnowflakeBinary;

impl SnowflakeType for SnowflakeBinary {
    type Representation<'a> = &'a [u8];
}

impl ReadArrowType<GenericByteArray<GenericBinaryType<i32>>> for SnowflakeBinary {
    fn read_arrow_type<'a>(
        &self,
        array: &'a GenericByteArray<GenericBinaryType<i32>>,
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

impl WriteODBCType for SnowflakeBinary {
    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
    ) -> Result<Warnings, WriteOdbcError> {
        match binding.target_type {
            CDataType::Binary => {
                let copy_len = std::cmp::min(snowflake_value.len(), binding.buffer_length as usize);
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        snowflake_value.as_ptr(),
                        binding.target_value_ptr as *mut u8,
                        copy_len,
                    );
                }
                if !binding.str_len_or_ind_ptr.is_null() {
                    unsafe {
                        std::ptr::write(
                            binding.str_len_or_ind_ptr,
                            snowflake_value.len() as sql::Len,
                        )
                    };
                }
                Ok(vec![])
            }
            _ => UnsupportedOdbcTypeSnafu {
                target_type: binding.target_type,
            }
            .fail(),
        }
    }
}
