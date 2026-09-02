use arrow::array::BooleanArray;
use odbc_sys as sql;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::error::BindingError;
use crate::conversion::error::{
    InvalidBooleanValueSnafu, NumericMagnitudeOverflowSnafu, UnsupportedCDataTypeSnafu,
};
use crate::conversion::error::{ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError};
use crate::conversion::numeric_helpers::{
    reject_multi_field_interval, write_interval_second, write_single_field_interval,
};
use crate::conversion::param_binding::{
    buffer_data_len, read_char_str, read_numeric_struct, read_unaligned, read_wchar_str,
};
use crate::conversion::traits::Binding;
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, WriteWire};
use crate::conversion::warning::{Warning, Warnings};
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

pub(crate) use sf_types::SnowflakeBoolean;

impl SnowflakeType for SnowflakeBoolean {
    type Representation<'a> = bool;
}

impl ReadArrowType<BooleanArray> for SnowflakeBoolean {
    fn read_arrow_type<'a>(
        &self,
        array: &'a BooleanArray,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        Ok(sf_types::ReadArrowType::read_arrow_type(
            self, array, row_idx,
        )?)
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

fn bit_from_i128(value: i128) -> Result<bool, BindingError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => NumericMagnitudeOverflowSnafu {
            reason: format!("SQL_BIT accepts only 0 or 1, got {value}"),
        }
        .fail(),
    }
}

fn bit_from_f64(value: f64) -> Result<(bool, Warnings), BindingError> {
    if value.is_nan() {
        return Ok((true, vec![]));
    }
    if value.is_infinite() || !(0.0..2.0).contains(&value) {
        return NumericMagnitudeOverflowSnafu {
            reason: format!("SQL_BIT accepts only 0 or 1, got {value}"),
        }
        .fail();
    }
    if value == 0.0 {
        return Ok((false, vec![]));
    }
    if value == 1.0 {
        return Ok((true, vec![]));
    }
    Ok((value.trunc() != 0.0, vec![Warning::NumericValueTruncated]))
}

fn bit_from_numeric(binding: &ParameterBinding) -> Result<bool, BindingError> {
    let (signed, scale) = read_numeric_struct(binding)?;
    if signed == 0 {
        return Ok(false);
    }
    let is_one = if scale == 0 {
        signed == 1
    } else if scale > 0 {
        10i128
            .checked_pow(scale as u32)
            .is_some_and(|factor| signed == factor)
    } else {
        false
    };
    if is_one {
        Ok(true)
    } else {
        NumericMagnitudeOverflowSnafu {
            reason: format!(
                "SQL_BIT accepts only 0 or 1, got numeric mantissa {signed} scale {scale}"
            ),
        }
        .fail()
    }
}

fn parse_str_to_bool(s: &str) -> Result<(bool, Warnings), BindingError> {
    let trimmed = s.trim();
    if let Ok(i) = trimmed.parse::<i128>() {
        return bit_from_i128(i).map(|b| (b, vec![]));
    }
    if let Ok(f) = trimmed.parse::<f64>()
        && f.is_finite()
    {
        return bit_from_f64(f);
    }
    InvalidBooleanValueSnafu {
        value: trimmed.to_string(),
    }
    .fail()
}

pub(crate) fn read_boolean_param(
    binding: &ParameterBinding,
) -> Result<(bool, Warnings), BindingError> {
    match binding.value_type {
        CDataType::Default | CDataType::Bit => Ok((read_unaligned::<u8>(binding) != 0, vec![])),
        CDataType::UTinyInt => {
            bit_from_i128(i128::from(read_unaligned::<u8>(binding))).map(|b| (b, vec![]))
        }
        CDataType::TinyInt | CDataType::STinyInt => {
            bit_from_i128(i128::from(read_unaligned::<i8>(binding))).map(|b| (b, vec![]))
        }
        CDataType::Long | CDataType::SLong => {
            bit_from_i128(i128::from(read_unaligned::<i32>(binding))).map(|b| (b, vec![]))
        }
        CDataType::ULong => {
            bit_from_i128(i128::from(read_unaligned::<u32>(binding))).map(|b| (b, vec![]))
        }
        CDataType::Short | CDataType::SShort => {
            bit_from_i128(i128::from(read_unaligned::<i16>(binding))).map(|b| (b, vec![]))
        }
        CDataType::UShort => {
            bit_from_i128(i128::from(read_unaligned::<u16>(binding))).map(|b| (b, vec![]))
        }
        CDataType::SBigInt => {
            bit_from_i128(i128::from(read_unaligned::<i64>(binding))).map(|b| (b, vec![]))
        }
        CDataType::UBigInt => {
            bit_from_i128(i128::from(read_unaligned::<u64>(binding))).map(|b| (b, vec![]))
        }
        CDataType::Float => bit_from_f64(f64::from(read_unaligned::<f32>(binding))),
        CDataType::Double => bit_from_f64(read_unaligned::<f64>(binding)),
        CDataType::Char => {
            let s = read_char_str(binding)?;
            parse_str_to_bool(&s)
        }
        CDataType::WChar => {
            let s = read_wchar_str(binding)?;
            parse_str_to_bool(&s)
        }
        CDataType::Numeric => bit_from_numeric(binding).map(|b| (b, vec![])),
        CDataType::Binary => {
            let len = buffer_data_len(binding);
            if len != 1 {
                return NumericMagnitudeOverflowSnafu {
                    reason: format!("SQL_C_BINARY to SQL_BIT requires exactly 1 byte, got {len}"),
                }
                .fail();
            }
            // SAFETY: `len == 1` was checked and the bind buffer is a single byte.
            let byte = unsafe { *(binding.parameter_value_ptr as *const u8) };
            Ok((byte != 0, vec![]))
        }
        other => UnsupportedCDataTypeSnafu { c_type: other }.fail(),
    }
}

impl ReadODBC for SnowflakeBoolean {
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, BindingError> {
        let (value, _) = read_boolean_param(binding)?;
        Ok(value)
    }
}

impl WriteWire for SnowflakeBoolean {
    fn write_wire(&self, value: Self::Representation<'_>) -> Result<String, BindingError> {
        Ok(value.to_string())
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::Boolean
    }
}
