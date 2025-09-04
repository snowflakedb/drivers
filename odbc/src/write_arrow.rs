use std::{
    collections::HashMap,
    ffi::{CStr, c_char},
    marker::PhantomData,
    slice,
    sync::Arc,
};

use arrow::{
    array::{Array, Int8Array, Int32Array, StringArray},
    datatypes::{DataType, Int32Type, Utf8Type},
    ffi::{FFI_ArrowArray, FFI_ArrowSchema},
};

use crate::{api::ParameterBinding, cdata_types::CDataType};
use odbc_sys as sql;

#[derive(Debug)]
#[allow(dead_code)]
pub enum ArrowBindingError {
    InvalidParameterIndices,
    UnsupportedParameterType(sql::SqlDataType),
    UnsupportedCDataType(CDataType),
}

impl std::error::Error for ArrowBindingError {}

impl std::fmt::Display for ArrowBindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

struct Writer<T> {
    marker: PhantomData<T>,
}

impl<T> Writer<T> {
    fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

trait ArrowWriter {
    fn arrow_type(&self) -> DataType;
    fn write(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        match binding.value_type {
            CDataType::Long => self.write_long(binding),
            CDataType::Char => self.write_char(binding),
            _ => Err(ArrowBindingError::UnsupportedCDataType(binding.value_type)),
        }
    }

    fn write_long(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        Err(ArrowBindingError::UnsupportedCDataType(binding.value_type))
    }

    fn write_char(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        Err(ArrowBindingError::UnsupportedCDataType(binding.value_type))
    }
}

impl ArrowWriter for Writer<Int8Array> {
    fn arrow_type(&self) -> DataType {
        DataType::Int8
    }
}

impl ArrowWriter for Writer<Int32Type> {
    fn arrow_type(&self) -> DataType {
        DataType::Int32
    }

    fn write_long(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        Ok(Arc::new(Int32Array::from(vec![unsafe {
            std::ptr::read(binding.parameter_value_ptr as *const i32)
        }])))
    }
}

impl ArrowWriter for Writer<Utf8Type> {
    fn arrow_type(&self) -> DataType {
        DataType::Utf8
    }

    fn write_char(&self, binding: &ParameterBinding) -> Result<Arc<dyn Array>, ArrowBindingError> {
        let value = if binding.buffer_length == sql::NTS {
            unsafe {
                CStr::from_ptr(binding.parameter_value_ptr as *const c_char)
                    .to_string_lossy()
                    .to_string()
            }
        } else {
            unsafe {
                str::from_utf8(slice::from_raw_parts(
                    binding.parameter_value_ptr as *const u8,
                    binding.buffer_length as usize,
                ))
                .unwrap()
                .to_string()
            }
        };
        unsafe {
            std::ptr::write(binding.str_len_or_ind_ptr, value.len() as sql::Len);
        }
        Ok(Arc::new(StringArray::from(vec![value])))
    }
}

fn arrow_writer_from_sql_type(
    parameter_type: &sql::SqlDataType,
) -> Result<Box<dyn ArrowWriter + Send + Sync>, ArrowBindingError> {
    match *parameter_type {
        sql::SqlDataType::INTEGER => Ok(Box::new(Writer::<Int32Type>::new())),
        sql::SqlDataType::VARCHAR => Ok(Box::new(Writer::<Utf8Type>::new())),
        _ => Err(ArrowBindingError::UnsupportedParameterType(*parameter_type)),
    }
}

pub fn odbc_bindings_to_arrow_bindings(
    bindings: &HashMap<u16, ParameterBinding>,
) -> Result<(Box<FFI_ArrowSchema>, Box<FFI_ArrowArray>), ArrowBindingError> {
    let mut schema_fields = Vec::new();
    let mut arrays = Vec::new();
    let max_key = *bindings.keys().max().unwrap_or(&0);
    let min_key = *bindings.keys().min().unwrap_or(&1);
    for param_num in min_key..=max_key {
        let binding = bindings.get(&param_num);
        if binding.is_none() {
            tracing::error!(
                "SQLExecute: parameter #{param_num} not found. Make sure parameter bindings are contiguous and start at 1.",
            );
            return Err(ArrowBindingError::InvalidParameterIndices);
        }
        let binding = binding.unwrap();
        let writer = arrow_writer_from_sql_type(&binding.parameter_type)?;
        schema_fields.push(arrow::datatypes::Field::new(
            format!("param_{param_num}"),
            writer.arrow_type(),
            false,
        ));
        arrays.push((
            Arc::new(arrow::datatypes::Field::new(
                format!("param_{param_num}"),
                writer.arrow_type(),
                false,
            )),
            writer.write(binding)?,
        ));
    }
    let schema = arrow::datatypes::Schema::new(schema_fields);
    let schema = Box::new(arrow::ffi::FFI_ArrowSchema::try_from(&schema).unwrap());
    let array = arrow::array::StructArray::from(arrays);
    let array = Box::new(arrow::ffi::FFI_ArrowArray::new(&array.into_data()));
    Ok((schema, array))
}
