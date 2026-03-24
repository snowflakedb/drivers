use arrow::array::{Array, BooleanArray};
use odbc_sys as sql;
use serde_json::Value;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::error::JsonBindingError;
use crate::conversion::error::{ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError};
use crate::conversion::param_binding::read_unaligned;
use crate::conversion::traits::Binding;
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, WriteJson};
use crate::conversion::warning::Warnings;
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

pub(crate) struct SnowflakeBoolean;

impl SnowflakeType for SnowflakeBoolean {
    type Representation<'a> = bool;
}

impl ReadArrowType<BooleanArray> for SnowflakeBoolean {
    fn read_arrow_type<'a>(
        &self,
        array: &'a BooleanArray,
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

impl WriteODBCType for SnowflakeBoolean {
    fn sql_type(&self) -> sql::SqlDataType {
        sql::SqlDataType::EXT_BIT
    }

    fn column_size(&self) -> sql::ULen {
        1
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
        let int_value = snowflake_value as u8;

        match binding.target_type {
            CDataType::Default | CDataType::Bit => {
                binding.write_fixed(int_value);
                Ok(vec![])
            }
            CDataType::TinyInt | CDataType::STinyInt => {
                binding.write_fixed(int_value as i8);
                Ok(vec![])
            }
            CDataType::UTinyInt => {
                binding.write_fixed(int_value);
                Ok(vec![])
            }
            CDataType::Short | CDataType::SShort => {
                binding.write_fixed(int_value as i16);
                Ok(vec![])
            }
            CDataType::UShort => {
                binding.write_fixed(int_value as u16);
                Ok(vec![])
            }
            CDataType::Long | CDataType::SLong => {
                binding.write_fixed(int_value as i32);
                Ok(vec![])
            }
            CDataType::ULong => {
                binding.write_fixed(int_value as u32);
                Ok(vec![])
            }
            CDataType::SBigInt => {
                binding.write_fixed(int_value as i64);
                Ok(vec![])
            }
            CDataType::UBigInt => {
                binding.write_fixed(int_value as u64);
                Ok(vec![])
            }
            CDataType::Float => {
                binding.write_fixed(int_value as f32);
                Ok(vec![])
            }
            CDataType::Double => {
                binding.write_fixed(int_value as f64);
                Ok(vec![])
            }
            CDataType::Char => {
                let s = if snowflake_value { "1" } else { "0" };
                Ok(binding.write_char_string(s, get_data_offset))
            }
            CDataType::WChar => {
                let s = if snowflake_value { "1" } else { "0" };
                Ok(binding.write_wchar_string(s, get_data_offset))
            }
            CDataType::Numeric => {
                let precision = binding.precision.unwrap_or(1);
                let scale = binding.scale.unwrap_or(0);
                let numeric = sql::Numeric {
                    precision: precision as u8,
                    scale: scale as i8,
                    sign: 1,
                    val: (int_value as u128).to_le_bytes(),
                };
                binding.write_fixed(numeric);
                Ok(vec![])
            }
            CDataType::Binary => Ok(binding.write_binary(&[int_value], get_data_offset)),
            _ => UnsupportedOdbcTypeSnafu {
                target_type: binding.target_type,
            }
            .fail(),
        }
    }
}

impl ReadODBC for SnowflakeBoolean {
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, JsonBindingError> {
        Ok(read_unaligned::<u8>(binding) != 0)
    }
}

impl WriteJson for SnowflakeBoolean {
    fn write_json(&self, value: Self::Representation<'_>) -> Result<Value, JsonBindingError> {
        Ok(Value::String(value.to_string()))
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::Boolean
    }
}
