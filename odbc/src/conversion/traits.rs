use odbc_sys as sql;

use crate::cdata_types::CDataType;
use crate::conversion::error::{ReadArrowError, WriteOdbcError};
use crate::conversion::warning::Warnings;

#[derive(Debug)]
pub struct Binding {
    pub target_type: CDataType,
    pub target_value_ptr: sql::Pointer,
    pub buffer_length: sql::Len,
    pub str_len_or_ind_ptr: *mut sql::Len,
}

impl Binding {
    pub fn write_fixed<T>(&self, value: T) {
        unsafe {
            if !self.target_value_ptr.is_null() {
                std::ptr::write(self.target_value_ptr as *mut T, value);
            }
            if !self.str_len_or_ind_ptr.is_null() {
                std::ptr::write(
                    self.str_len_or_ind_ptr,
                    std::mem::size_of::<T>() as sql::Len,
                );
            }
        }
    }
}

pub trait WriteODBCType: SnowflakeType {
    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
    ) -> Result<Warnings, WriteOdbcError>;
}

pub trait SnowflakeType {
    type Representation<'a>: Sized;
}

pub trait ReadArrowType<ArrowArrayType>: SnowflakeType {
    #[allow(clippy::wrong_self_convention)]
    fn read_arrow_type<'a>(
        &self,
        array: &'a ArrowArrayType,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError>;
}
