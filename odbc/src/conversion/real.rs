use arrow::array::{Array, Float64Array};
use odbc_sys as sql;
use serde_json::Value;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::error::{JsonBindingError, UnsupportedCDataTypeSnafu};
use crate::conversion::error::{
    NumericValueOutOfRangeSnafu, ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError,
};
use crate::conversion::numeric_helpers::{
    build_and_write_interval_second, check_leading_precision, checked_u32,
    reject_multi_field_interval, whole_digits_len, write_numeric_as_binary,
    write_single_field_interval,
};
use crate::conversion::param_binding::{read_char_str, read_unaligned, read_wchar_str};
use crate::conversion::traits::Binding;
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, WriteJson};
use crate::conversion::warning::{Warning, Warnings};
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

/// Handles Snowflake's "REAL" logical type (FLOAT, DOUBLE, REAL).
/// The old driver maps "real" → SQL_DOUBLE; the default C type is SQL_C_DOUBLE.
pub(crate) struct SnowflakeReal;

impl SnowflakeType for SnowflakeReal {
    type Representation<'a> = f64;
}

impl ReadArrowType<Float64Array> for SnowflakeReal {
    fn read_arrow_type<'a>(
        &self,
        array: &'a Float64Array,
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

fn check_float_range(value: f64, min: f64, max: f64) -> Result<(), WriteOdbcError> {
    if value.is_nan() {
        return NumericValueOutOfRangeSnafu {
            reason: "NaN cannot be converted to an integer type".to_string(),
        }
        .fail();
    }
    let truncated = value.trunc();
    if truncated < min || truncated > max {
        NumericValueOutOfRangeSnafu {
            reason: format!("Value {value} is out of range ({min} to {max})"),
        }
        .fail()
    } else {
        Ok(())
    }
}

fn write_float_interval_second(value: f64, binding: &Binding) -> Result<Warnings, WriteOdbcError> {
    let abs = value.abs();
    let abs_int = (abs.trunc() as i128).unsigned_abs();

    let frac_micros_f64 = abs.fract() * 1_000_000.0;
    let mut frac_microseconds = frac_micros_f64.trunc() as u32;
    let mut frac_truncated = frac_micros_f64.fract() != 0.0;

    // Guard against floating-point rounding producing >= 1_000_000 microseconds.
    // If that happens, carry into the seconds field and normalize the fraction.
    let carry = if frac_microseconds >= 1_000_000 {
        frac_microseconds = 0;
        frac_truncated = true;
        1u128
    } else {
        0u128
    };

    let final_abs_int = abs_int + carry;
    check_leading_precision(final_abs_int, value, binding)?;
    let second_val = checked_u32(final_abs_int, value)?;

    Ok(build_and_write_interval_second(
        second_val,
        frac_microseconds,
        frac_truncated,
        value.is_sign_negative(),
        binding,
    ))
}

fn fractional_warning(value: f64) -> Warnings {
    if value.fract() != 0.0 {
        vec![Warning::NumericValueTruncated]
    } else {
        vec![]
    }
}

impl WriteODBCType for SnowflakeReal {
    fn sql_type(&self) -> sql::SqlDataType {
        sql::SqlDataType::DOUBLE
    }

    fn column_size(&self) -> sql::ULen {
        15
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
        let target_type = match binding.target_type {
            CDataType::Default => CDataType::Double,
            other => other,
        };
        match target_type {
            CDataType::Double => {
                binding.write_fixed(snowflake_value);
                Ok(vec![])
            }
            CDataType::Float => {
                if snowflake_value.abs() > f32::MAX as f64 {
                    return NumericValueOutOfRangeSnafu {
                        reason: format!("Value {snowflake_value} is out of range for SQL_C_FLOAT"),
                    }
                    .fail();
                }
                binding.write_fixed(snowflake_value as f32);
                Ok(vec![])
            }
            CDataType::Short | CDataType::SShort => {
                check_float_range(snowflake_value, i16::MIN as f64, i16::MAX as f64)?;
                binding.write_fixed(snowflake_value as i16);
                Ok(fractional_warning(snowflake_value))
            }
            CDataType::UShort => {
                check_float_range(snowflake_value, 0.0, u16::MAX as f64)?;
                binding.write_fixed(snowflake_value as u16);
                Ok(fractional_warning(snowflake_value))
            }
            CDataType::TinyInt | CDataType::STinyInt => {
                check_float_range(snowflake_value, i8::MIN as f64, i8::MAX as f64)?;
                binding.write_fixed(snowflake_value as i8);
                Ok(fractional_warning(snowflake_value))
            }
            CDataType::UTinyInt => {
                check_float_range(snowflake_value, 0.0, u8::MAX as f64)?;
                binding.write_fixed(snowflake_value as u8);
                Ok(fractional_warning(snowflake_value))
            }
            CDataType::Long | CDataType::SLong => {
                check_float_range(snowflake_value, i32::MIN as f64, i32::MAX as f64)?;
                binding.write_fixed(snowflake_value as i32);
                Ok(fractional_warning(snowflake_value))
            }
            CDataType::ULong => {
                check_float_range(snowflake_value, 0.0, u32::MAX as f64)?;
                binding.write_fixed(snowflake_value as u32);
                Ok(fractional_warning(snowflake_value))
            }
            CDataType::SBigInt => {
                check_float_range(snowflake_value, i64::MIN as f64, i64::MAX as f64)?;
                binding.write_fixed(snowflake_value as i64);
                Ok(fractional_warning(snowflake_value))
            }
            CDataType::UBigInt => {
                check_float_range(snowflake_value, 0.0, u64::MAX as f64)?;
                binding.write_fixed(snowflake_value as u64);
                Ok(fractional_warning(snowflake_value))
            }
            CDataType::Bit => {
                if !snowflake_value.is_finite()
                    || snowflake_value < 0.0
                    || snowflake_value.trunc() >= 2.0
                {
                    return NumericValueOutOfRangeSnafu {
                        reason: format!(
                            "Value out of range for SQL_C_BIT (must be 0 or 1, got {snowflake_value})"
                        ),
                    }
                    .fail();
                }
                binding.write_fixed(snowflake_value.trunc() as u8);
                Ok(fractional_warning(snowflake_value))
            }
            CDataType::Numeric => {
                if snowflake_value.is_nan() {
                    return NumericValueOutOfRangeSnafu {
                        reason: "NaN cannot be converted to SQL_C_NUMERIC".to_string(),
                    }
                    .fail();
                }
                let target_scale = binding.scale.unwrap_or(0);
                let abs_value = snowflake_value.abs();

                let scale_factor = 10f64.powi(target_scale as i32);
                let scaled = abs_value * scale_factor;
                let int_scaled = scaled.trunc();
                let was_truncated = scaled != int_scaled;

                if int_scaled > u128::MAX as f64 {
                    return NumericValueOutOfRangeSnafu {
                        reason: format!(
                            "Value {snowflake_value} is out of range for SQL_C_NUMERIC"
                        ),
                    }
                    .fail();
                }

                let magnitude = int_scaled as u128;
                let numeric = sql::Numeric {
                    precision: binding.precision.unwrap_or(38) as u8,
                    scale: target_scale as i8,
                    sign: if snowflake_value.is_sign_negative() {
                        0
                    } else {
                        1
                    },
                    val: magnitude.to_le_bytes(),
                };

                binding.write_fixed(numeric);
                if was_truncated {
                    Ok(vec![Warning::NumericValueTruncated])
                } else {
                    Ok(vec![])
                }
            }
            CDataType::Char => {
                let num_str = snowflake_value.to_string();
                let warnings = binding.write_char_string(&num_str, get_data_offset);
                if warnings
                    .iter()
                    .any(|w| matches!(w, Warning::StringDataTruncated))
                {
                    let whole_len = whole_digits_len(&num_str);
                    if whole_len >= binding.buffer_length as usize {
                        *get_data_offset = None;
                        return NumericValueOutOfRangeSnafu {
                            reason: format!(
                                "Whole digits of '{num_str}' do not fit in buffer of {} bytes",
                                binding.buffer_length
                            ),
                        }
                        .fail();
                    }
                }
                Ok(warnings)
            }
            CDataType::WChar => {
                let num_str = snowflake_value.to_string();
                let warnings = binding.write_wchar_string(&num_str, get_data_offset);
                if warnings
                    .iter()
                    .any(|w| matches!(w, Warning::StringDataTruncated))
                {
                    let whole_len = whole_digits_len(&num_str);
                    let wchar_capacity = (binding.buffer_length / 2) as usize;
                    if whole_len >= wchar_capacity {
                        *get_data_offset = None;
                        return NumericValueOutOfRangeSnafu {
                            reason: format!(
                                "Whole digits of '{num_str}' do not fit in wchar buffer of {wchar_capacity} chars",
                            ),
                        }
                        .fail();
                    }
                }
                Ok(warnings)
            }
            CDataType::Binary => {
                if snowflake_value.is_nan() {
                    return NumericValueOutOfRangeSnafu {
                        reason: "NaN cannot be converted to SQL_C_BINARY".to_string(),
                    }
                    .fail();
                }
                let truncated = snowflake_value.trunc();
                let abs_val = truncated.abs();
                if abs_val > u128::MAX as f64 {
                    return NumericValueOutOfRangeSnafu {
                        reason: format!("Value {snowflake_value} is out of range for SQL_C_BINARY"),
                    }
                    .fail();
                }
                let magnitude = abs_val as u128;
                let numeric = sql::Numeric {
                    precision: 38,
                    scale: 0,
                    sign: if snowflake_value.is_sign_negative() {
                        0
                    } else {
                        1
                    },
                    val: magnitude.to_le_bytes(),
                };
                write_numeric_as_binary(&numeric, binding)?;
                Ok(fractional_warning(snowflake_value))
            }
            CDataType::IntervalYear
            | CDataType::IntervalMonth
            | CDataType::IntervalDay
            | CDataType::IntervalHour
            | CDataType::IntervalMinute
            | CDataType::IntervalSecond => {
                if !snowflake_value.is_finite() {
                    return NumericValueOutOfRangeSnafu {
                        reason: format!(
                            "Value {snowflake_value} cannot be converted to an interval type"
                        ),
                    }
                    .fail();
                }
                if target_type == CDataType::IntervalSecond {
                    write_float_interval_second(snowflake_value, binding)
                } else {
                    let int_value = snowflake_value.trunc() as i128;
                    let has_fractional = snowflake_value.fract() != 0.0;
                    write_single_field_interval(
                        target_type,
                        int_value,
                        snowflake_value.is_sign_negative(),
                        has_fractional,
                        binding,
                    )
                }
            }
            CDataType::IntervalYearToMonth
            | CDataType::IntervalDayToHour
            | CDataType::IntervalDayToMinute
            | CDataType::IntervalDayToSecond
            | CDataType::IntervalHourToMinute
            | CDataType::IntervalHourToSecond
            | CDataType::IntervalMinuteToSecond => reject_multi_field_interval(target_type),
            _ => UnsupportedOdbcTypeSnafu { target_type }.fail(),
        }
    }
}

impl ReadODBC for SnowflakeReal {
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, JsonBindingError> {
        let value = match binding.value_type {
            CDataType::Float => read_unaligned::<f32>(binding) as f64,
            CDataType::Double => read_unaligned::<f64>(binding),
            CDataType::Char => {
                let s = read_char_str(binding)?;
                s.trim().parse::<f64>().map_err(|_| {
                    UnsupportedCDataTypeSnafu {
                        c_type: binding.value_type,
                    }
                    .build()
                })?
            }
            CDataType::WChar => {
                let s = read_wchar_str(binding)?;
                s.trim().parse::<f64>().map_err(|_| {
                    UnsupportedCDataTypeSnafu {
                        c_type: binding.value_type,
                    }
                    .build()
                })?
            }
            _ => {
                return UnsupportedCDataTypeSnafu {
                    c_type: binding.value_type,
                }
                .fail();
            }
        };
        Ok(value)
    }
}

impl WriteJson for SnowflakeReal {
    fn write_json(&self, value: Self::Representation<'_>) -> Result<Value, JsonBindingError> {
        Ok(Value::String(value.to_string()))
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::Real
    }
}
