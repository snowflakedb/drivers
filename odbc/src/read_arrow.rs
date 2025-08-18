use arrow::{
    array::{Array, ArrowPrimitiveType, GenericByteArray, PrimitiveArray},
    datatypes::{ByteArrayType, DataType, Int8Type, Int64Type, Utf8Type},
};
use odbc_sys as sql;

#[derive(Debug)]
pub enum ExtractError {
    UnsupportedType,
    DowncastError,
}

fn get_value<T: ArrowPrimitiveType>(
    array: &dyn Array,
    row_idx: usize,
) -> Result<T::Native, ExtractError> {
    Ok(array
        .as_any()
        .downcast_ref::<PrimitiveArray<T>>()
        .ok_or(ExtractError::DowncastError)?
        .value(row_idx))
}

fn get_byte_array_value<T: ByteArrayType>(
    array: &dyn Array,
    row_idx: usize,
) -> Result<&T::Native, ExtractError> {
    Ok(array
        .as_any()
        .downcast_ref::<GenericByteArray<T>>()
        .unwrap()
        .value(row_idx))
}

pub trait ReadArrowValue: Sized {
    fn read(self, array: &dyn Array, row_idx: usize) -> Result<(), ExtractError> {
        match array.data_type() {
            DataType::Int8 => self.read_int8(get_value::<Int8Type>(array, row_idx)?),
            DataType::Int64 => self.read_int64(get_value::<Int64Type>(array, row_idx)?),
            DataType::Utf8 => self.read_utf8(get_byte_array_value::<Utf8Type>(array, row_idx)?),
            _ => Err(ExtractError::UnsupportedType),
        }
    }
    fn read_int8(self, _value: i8) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedType)
    }
    fn read_int64(self, _value: i64) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedType)
    }
    fn read_utf8(self, _value: &str) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedType)
    }
}

impl ReadArrowValue for *mut sql::UInteger {
    fn read_int8(self, value: i8) -> Result<(), ExtractError> {
        unsafe { std::ptr::write(self, value as sql::UInteger) };
        Ok(())
    }
    fn read_int64(self, value: i64) -> Result<(), ExtractError> {
        unsafe { std::ptr::write(self, value as sql::UInteger) };
        Ok(())
    }
}

pub struct Buffer<T> {
    pub data: *mut T,
    pub len: usize,
    pub str_len_or_ind: *mut sql::Len,
}

impl<T> Buffer<T> {
    pub fn new(data: *mut T, len: usize, str_len_or_ind: *mut sql::Len) -> Self {
        Self {
            data,
            len,
            str_len_or_ind,
        }
    }
}

impl ReadArrowValue for Buffer<sql::Char> {
    fn read_utf8(self, value: &str) -> Result<(), ExtractError> {
        if !self.str_len_or_ind.is_null() {
            unsafe { std::ptr::write(self.str_len_or_ind, value.len() as sql::Len) };
        }
        unsafe {
            std::ptr::copy_nonoverlapping(
                value.as_ptr() as *const sql::Char,
                self.data,
                std::cmp::min(self.len, value.len()),
            )
        };
        Ok(())
    }
}
