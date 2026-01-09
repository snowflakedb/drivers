use odbc_sys as sql;

use crate::cdata_types::CDataType;
use crate::conversion::ConversionError;

pub struct Binding {
    pub target_type: CDataType,
    pub value: sql::Pointer,
    pub buffer_length: sql::Len,
    pub str_len_or_ind_ptr: *mut sql::Len,
}

pub trait WriteODBCType: SnowflakeType {
    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
    ) -> Result<(), ConversionError>;
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
    ) -> Result<Self::Representation<'a>, ConversionError>;
}
