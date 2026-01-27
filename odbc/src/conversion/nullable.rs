use crate::{
    cdata_types::SQL_NULL_DATA,
    conversion::{
        Binding, ReadArrowType, SnowflakeType, WriteODBCType,
        error::{ReadArrowError, WriteOdbcError},
        warning::Warnings,
    },
};

pub(crate) struct Nullable<T> {
    pub value: T,
}

impl<T: SnowflakeType> SnowflakeType for Nullable<T> {
    type Representation<'a> = Option<T::Representation<'a>>;
}

impl<R, T: SnowflakeType + ReadArrowType<R>> ReadArrowType<R> for Nullable<T> {
    fn read_arrow_type<'a>(
        &self,
        array: &'a R,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        self.value
            .read_arrow_type(array, row_idx)
            .map(Some)
            .or_else(|e| match e {
                ReadArrowError::NullValue { .. } => Ok(None),
            })
    }
}

impl<T: WriteODBCType> WriteODBCType for Nullable<T> {
    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
    ) -> Result<Warnings, WriteOdbcError> {
        match snowflake_value {
            Some(value) => self.value.write_odbc_type(value, binding),
            None => {
                unsafe {
                    std::ptr::write(binding.str_len_or_ind_ptr, SQL_NULL_DATA);
                }
                Ok(vec![])
            }
        }
    }
}
