use arrow::array::{Array, Float64Array, Int8Array, Int64Array, StringArray};

#[derive(Debug)]
pub enum ArrowExtractError {
    UnsupportedType,
}

pub trait ArrowExtractValue: Sized {
    fn extract_int8(_value: i8) -> Result<Self, ArrowExtractError> {
        Err(ArrowExtractError::UnsupportedType)
    }
    fn extract_int64(_value: i64) -> Result<Self, ArrowExtractError> {
        Err(ArrowExtractError::UnsupportedType)
    }
    fn extract_float64(_value: f64) -> Result<Self, ArrowExtractError> {
        Err(ArrowExtractError::UnsupportedType)
    }
    fn extract_string(_value: &str) -> Result<Self, ArrowExtractError> {
        Err(ArrowExtractError::UnsupportedType)
    }
}

impl ArrowExtractValue for String {
    fn extract_int8(value: i8) -> Result<String, ArrowExtractError> {
        Ok(value.to_string())
    }

    fn extract_int64(value: i64) -> Result<String, ArrowExtractError> {
        Ok(value.to_string())
    }

    fn extract_float64(value: f64) -> Result<String, ArrowExtractError> {
        Ok(value.to_string())
    }

    fn extract_string(value: &str) -> Result<String, ArrowExtractError> {
        Ok(value.to_string())
    }
}

impl ArrowExtractValue for i64 {
    fn extract_int8(value: i8) -> Result<i64, ArrowExtractError> {
        Ok(value as i64)
    }
    fn extract_int64(value: i64) -> Result<i64, ArrowExtractError> {
        Ok(value)
    }
}

impl ArrowExtractValue for i32 {
    fn extract_int8(value: i8) -> Result<i32, ArrowExtractError> {
        Ok(value as i32)
    }
    fn extract_int64(value: i64) -> Result<i32, ArrowExtractError> {
        Ok(value as i32)
    }
}

pub fn extract_arrow_value<T: ArrowExtractValue>(
    column: &dyn Array,
    row_idx: usize,
) -> Result<T, ArrowExtractError> {
    use arrow::datatypes::DataType;
    match column.data_type() {
        DataType::Int8 => {
            let int_array = column
                .as_any()
                .downcast_ref::<Int8Array>()
                .expect("Expected int8 array");
            T::extract_int8(int_array.value(row_idx))
        }
        DataType::Int64 => {
            let int_array = column
                .as_any()
                .downcast_ref::<Int64Array>()
                .expect("Expected int64 array");
            T::extract_int64(int_array.value(row_idx))
        }
        DataType::Float64 => {
            let float_array = column
                .as_any()
                .downcast_ref::<Float64Array>()
                .expect("Expected float64 array");
            T::extract_float64(float_array.value(row_idx))
        }
        DataType::Utf8 => {
            let string_array = column
                .as_any()
                .downcast_ref::<StringArray>()
                .expect("Expected string array");
            T::extract_string(string_array.value(row_idx))
        }
        _ => Err(ArrowExtractError::UnsupportedType),
    }
}
