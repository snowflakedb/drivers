use arrow::array::{Array, BooleanArray};
use odbc_sys as sql;
use serde_json::Value;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::error::JsonBindingError;
use crate::conversion::error::UnsupportedCDataTypeSnafu;
use crate::conversion::error::{ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError};
use crate::conversion::numeric_helpers::{
    reject_multi_field_interval, write_interval_second, write_single_field_interval,
};
use crate::conversion::param_binding::{
    buffer_data_len, read_char_str, read_unaligned, read_wchar_str,
};
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
            CDataType::IntervalYear
            | CDataType::IntervalMonth
            | CDataType::IntervalDay
            | CDataType::IntervalHour
            | CDataType::IntervalMinute => write_single_field_interval(
                binding.target_type,
                int_value as i128,
                false,
                false,
                binding,
            ),
            CDataType::IntervalSecond => {
                write_interval_second(int_value as i128, int_value as u128, 0, false, binding)
            }
            CDataType::IntervalYearToMonth
            | CDataType::IntervalDayToHour
            | CDataType::IntervalDayToMinute
            | CDataType::IntervalDayToSecond
            | CDataType::IntervalHourToMinute
            | CDataType::IntervalHourToSecond
            | CDataType::IntervalMinuteToSecond => reject_multi_field_interval(binding.target_type),
            _ => UnsupportedOdbcTypeSnafu {
                target_type: binding.target_type,
            }
            .fail(),
        }
    }
}

/// Parse a string value to a boolean per ODBC spec: the string is first
/// converted to a numeric value, then 0 → false, nonzero → true.
fn parse_str_to_bool(s: &str) -> Result<bool, JsonBindingError> {
    let trimmed = s.trim();
    if let Ok(i) = trimmed.parse::<i64>() {
        return Ok(i != 0);
    }
    if let Ok(f) = trimmed.parse::<f64>() {
        return Ok(f != 0.0);
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => UnsupportedCDataTypeSnafu {
            c_type: CDataType::Char,
        }
        .fail(),
    }
}

impl ReadODBC for SnowflakeBoolean {
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, JsonBindingError> {
        match binding.value_type {
            CDataType::Bit | CDataType::UTinyInt => Ok(read_unaligned::<u8>(binding) != 0),
            CDataType::TinyInt | CDataType::STinyInt => Ok(read_unaligned::<i8>(binding) != 0),
            CDataType::Long | CDataType::SLong => Ok(read_unaligned::<i32>(binding) != 0),
            CDataType::ULong => Ok(read_unaligned::<u32>(binding) != 0),
            CDataType::Short | CDataType::SShort => Ok(read_unaligned::<i16>(binding) != 0),
            CDataType::UShort => Ok(read_unaligned::<u16>(binding) != 0),
            CDataType::SBigInt => Ok(read_unaligned::<i64>(binding) != 0),
            CDataType::UBigInt => Ok(read_unaligned::<u64>(binding) != 0),
            CDataType::Float => Ok(read_unaligned::<f32>(binding) != 0.0),
            CDataType::Double => Ok(read_unaligned::<f64>(binding) != 0.0),
            CDataType::Char => {
                let s = read_char_str(binding)?;
                parse_str_to_bool(&s)
            }
            CDataType::WChar => {
                let s = read_wchar_str(binding)?;
                parse_str_to_bool(&s)
            }
            CDataType::Numeric => {
                let n = read_unaligned::<sql::Numeric>(binding);
                Ok(u128::from_le_bytes(n.val) != 0)
            }
            CDataType::Binary => {
                let len = buffer_data_len(binding);
                if len == 0 {
                    return Ok(false);
                }
                let first = unsafe { *(binding.parameter_value_ptr as *const u8) };
                Ok(first != 0)
            }
            _ => UnsupportedCDataTypeSnafu {
                c_type: binding.value_type,
            }
            .fail(),
        }
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
