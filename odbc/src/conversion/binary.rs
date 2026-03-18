use std::slice;

use arrow::array::{Array, GenericByteArray};
use arrow::datatypes::GenericBinaryType;
use serde_json::Value;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::error::JsonBindingError;
use crate::conversion::error::{ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError};
use crate::conversion::param_binding::buffer_data_len;
use crate::conversion::traits::Binding;
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, WriteJson};
use crate::conversion::warning::Warnings;
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};
use odbc_sys as sql;

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
    fn sql_type(&self) -> sql::SqlDataType {
        odbc_sys::SqlDataType::EXT_VAR_BINARY
    }

    fn column_size(&self) -> sql::ULen {
        8_388_608
    }

    fn decimal_digits(&self) -> sql::SmallInt {
        0
    }

    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
        get_data_offset: &mut Option<usize>,
    ) -> Result<Warnings, WriteOdbcError> {
        match binding.target_type {
            CDataType::Default | CDataType::Binary => {
                Ok(binding.write_binary(snowflake_value, get_data_offset))
            }
            _ => UnsupportedOdbcTypeSnafu {
                target_type: binding.target_type,
            }
            .fail(),
        }
    }
}

impl ReadODBC for SnowflakeBinary {
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, JsonBindingError> {
        let len = buffer_data_len(binding);
        let bytes = unsafe { slice::from_raw_parts(binding.parameter_value_ptr as *const u8, len) };
        Ok(bytes)
    }
}

impl WriteJson for SnowflakeBinary {
    fn write_json(&self, value: Self::Representation<'_>) -> Result<Value, JsonBindingError> {
        let hex: String = value.iter().map(|b| format!("{:02x}", b)).collect();
        Ok(Value::String(hex))
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::Binary
    }
}
