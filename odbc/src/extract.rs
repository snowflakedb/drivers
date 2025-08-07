use arrow::{
    array::{Array, ArrowPrimitiveType, PrimitiveArray},
    datatypes::{DataType, Int8Type, Int64Type},
};

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

pub trait ReadArrowValue: Sized {
    fn read(self, array: &dyn Array, row_idx: usize) -> Result<(), ExtractError> {
        match array.data_type() {
            DataType::Int8 => self.read_int8(get_value::<Int8Type>(array, row_idx)?),
            DataType::Int64 => self.read_int64(get_value::<Int64Type>(array, row_idx)?),
            _ => Err(ExtractError::UnsupportedType),
        }
    }
    fn read_int8(self, _value: i8) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedType)
    }
    fn read_int64(self, _value: i64) -> Result<(), ExtractError> {
        Err(ExtractError::UnsupportedType)
    }
}

impl ReadArrowValue for *mut i64 {
    fn read_int8(self, value: i8) -> Result<(), ExtractError> {
        unsafe { std::ptr::write(self, value as i64) };
        Ok(())
    }
    fn read_int64(self, value: i64) -> Result<(), ExtractError> {
        unsafe { std::ptr::write(self, value) };
        Ok(())
    }
}
