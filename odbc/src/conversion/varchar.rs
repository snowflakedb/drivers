use arrow::array::GenericByteArray;
use arrow::datatypes::Utf8Type;
use odbc_sys::Len;

use crate::cdata_types::CDataType;
use crate::conversion::traits::Binding;
use crate::conversion::{ConversionError, ReadArrowType, SnowflakeType, WriteODBCType};

pub(crate) struct SnowflakeVarchar {
    pub len: u32,
}

impl SnowflakeType for SnowflakeVarchar {
    type Representation<'a> = &'a str;
}

impl ReadArrowType<GenericByteArray<Utf8Type>> for SnowflakeVarchar {
    fn read_arrow_type<'a>(
        &self,
        array: &'a GenericByteArray<Utf8Type>,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ConversionError> {
        let v = array.value(row_idx);
        Ok(v)
    }
}

impl WriteODBCType for SnowflakeVarchar {
    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
    ) -> Result<(), ConversionError> {
        match binding.target_type {
            CDataType::Char => {
                if !binding.str_len_or_ind_ptr.is_null() {
                    unsafe {
                        std::ptr::write(binding.str_len_or_ind_ptr, snowflake_value.len() as Len)
                    };
                }
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        snowflake_value.as_ptr(),
                        binding.value as *mut u8,
                        std::cmp::min(
                            binding.buffer_length as usize,
                            std::cmp::min(snowflake_value.len(), self.len as usize),
                        ),
                    );
                }
                Ok(())
            }
            _ => Err(ConversionError::UnsupportedOdbcType {
                target_type: binding.target_type,
                location: snafu::location!(),
            }),
        }
    }
}
