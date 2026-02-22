use odbc_sys as sql;

use crate::conversion::{
    Binding, LengthOrNull, ReadArrowType, SnowflakeType, WriteODBCType,
    error::{ReadArrowError, WriteOdbcError},
    warning::Warnings,
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
    fn sql_type(&self) -> sql::SqlDataType {
        self.value.sql_type()
    }

    fn column_size(&self) -> sql::ULen {
        self.value.column_size()
    }

    fn decimal_digits(&self) -> sql::SmallInt {
        self.value.decimal_digits()
    }

    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
        get_data_offset: &mut Option<usize>,
    ) -> Result<Warnings, WriteOdbcError> {
        match snowflake_value {
            Some(value) => self.value.write_odbc_type(value, binding, get_data_offset),
            None => {
                binding.write_length_or_null(LengthOrNull::Null)?;
                Ok(vec![])
            }
        }
    }
}
