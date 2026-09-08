use odbc_sys as sql;

use crate::api::CDataType;
use crate::conversion::error::{
    IntervalFieldOverflowSnafu, NumericValueOutOfRangeSnafu, UnsupportedOdbcTypeSnafu,
    WriteOdbcError,
};
use crate::conversion::traits::Binding;
use crate::conversion::warning::{Warning, Warnings};

pub fn check_integer_range(value: i128, min: i128, max: i128) -> Result<(), WriteOdbcError> {
    if value < min || value > max {
        NumericValueOutOfRangeSnafu {
            reason: format!("Value {value} is out of range ({min} to {max})"),
        }
        .fail()
    } else {
        Ok(())
    }
}

pub fn fractional_warning(has_fractional: bool) -> Warnings {
    if has_fractional {
        vec![Warning::NumericValueTruncated]
    } else {
        vec![]
    }
}

pub fn check_leading_precision(
    abs_int: u128,
    display_value: impl std::fmt::Display,
    binding: &Binding,
) -> Result<(), WriteOdbcError> {
    let leading_precision = binding.datetime_interval_precision.unwrap_or(2) as u32;
    let exceeds = if leading_precision >= 39 {
        // u128 values have at most 39 digits, so any u128 fits within
        // a precision of 39+ digits.
        false
    } else {
        abs_int >= 10u128.pow(leading_precision)
    };
    if exceeds {
        return IntervalFieldOverflowSnafu {
            reason: format!(
                "Value {display_value} exceeds leading field precision of {leading_precision} digits"
            ),
        }
        .fail();
    }
    Ok(())
}

pub fn checked_u32(
    abs_int: u128,
    display_value: impl std::fmt::Display,
) -> Result<u32, WriteOdbcError> {
    u32::try_from(abs_int).map_err(|_| {
        IntervalFieldOverflowSnafu {
            reason: format!(
                "Value {display_value} exceeds maximum interval field value ({})",
                u32::MAX
            ),
        }
        .build()
    })
}

pub fn whole_digits_len(num_str: &str) -> usize {
    match num_str.find('.') {
        Some(pos) => pos,
        None => num_str.len(),
    }
}

/// Writes an `sql::Numeric` struct into a binding's buffer as raw bytes (SQL_C_BINARY).
/// The caller is responsible for constructing the Numeric struct with the
/// appropriate precision, scale, sign, and value.
pub fn write_numeric_as_binary(
    numeric: &sql::Numeric,
    binding: &Binding,
) -> Result<(), WriteOdbcError> {
    let numeric_size = std::mem::size_of::<sql::Numeric>();
    if (binding.buffer_length as usize) < numeric_size {
        return NumericValueOutOfRangeSnafu {
            reason: format!(
                "Buffer size {} is too small for SQL_C_BINARY (need {numeric_size} bytes)",
                binding.buffer_length
            ),
        }
        .fail();
    }
    let numeric_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(numeric as *const sql::Numeric as *const u8, numeric_size)
    };
    unsafe {
        std::ptr::copy_nonoverlapping(
            numeric_bytes.as_ptr(),
            binding.target_value_ptr as *mut u8,
            numeric_size,
        );
    }
    let _ = binding.write_length_or_null(crate::conversion::traits::LengthOrNull::Length(
        numeric_size as sql::Len,
    ));
    Ok(())
}

/// Builds and writes an `IntervalStruct` for single-field interval types
/// (Year, Month, Day, Hour, Minute). Checks leading precision overflow.
pub fn write_single_field_interval(
    target_type: CDataType,
    int_value: i128,
    is_source_negative: bool,
    has_fractional: bool,
    binding: &Binding,
) -> Result<Warnings, WriteOdbcError> {
    let abs_int = int_value.unsigned_abs();
    check_leading_precision(abs_int, int_value, binding)?;
    let field_val = checked_u32(abs_int, int_value)?;
    let is_negative = is_source_negative && field_val > 0;
    let mut interval = sql::IntervalStruct {
        interval_type: 0,
        interval_sign: if is_negative { 1 } else { 0 },
        interval_value: sql::IntervalUnion {
            day_second: sql::DaySecond::default(),
        },
    };
    match target_type {
        CDataType::IntervalYear => {
            interval.interval_type = sql::Interval::Year as i32;
            interval.interval_value = sql::IntervalUnion {
                year_month: sql::YearMonth {
                    year: field_val,
                    month: 0,
                },
            };
        }
        CDataType::IntervalMonth => {
            interval.interval_type = sql::Interval::Month as i32;
            interval.interval_value = sql::IntervalUnion {
                year_month: sql::YearMonth {
                    year: 0,
                    month: field_val,
                },
            };
        }
        #[allow(unused_unsafe)]
        CDataType::IntervalDay => {
            interval.interval_type = sql::Interval::Day as i32;
            unsafe { interval.interval_value.day_second.day = field_val };
        }
        #[allow(unused_unsafe)]
        CDataType::IntervalHour => {
            interval.interval_type = sql::Interval::Hour as i32;
            unsafe { interval.interval_value.day_second.hour = field_val };
        }
        #[allow(unused_unsafe)]
        CDataType::IntervalMinute => {
            interval.interval_type = sql::Interval::Minute as i32;
            unsafe { interval.interval_value.day_second.minute = field_val };
        }
        _ => unreachable!("write_single_field_interval called with {target_type:?}"),
    }
    binding.write_fixed(interval);
    Ok(fractional_warning(has_fractional))
}

/// Computes interval-second fraction (microseconds) from the raw absolute value
/// and its decimal scale. Returns `(fraction_microseconds, was_truncated)`.
pub fn compute_interval_fraction(abs_value: u128, scale: u32) -> (u32, bool) {
    if scale == 0 {
        return (0, false);
    }
    match 10u128.checked_pow(scale) {
        Some(divisor) => {
            let remainder = abs_value % divisor;
            if scale > 6 {
                let frac_divisor = 10u128.pow(scale - 6);
                (
                    (remainder / frac_divisor) as u32,
                    !remainder.is_multiple_of(frac_divisor),
                )
            } else {
                let multiplier = 10u128.pow(6 - scale);
                ((remainder * multiplier) as u32, false)
            }
        }
        None => (0, abs_value != 0),
    }
}

/// Writes a fully-computed IntervalSecond struct to the binding.
/// Shared by both FLOAT (f64-based fraction) and DECFLOAT/NUMBER (integer-based fraction).
pub fn build_and_write_interval_second(
    second_val: u32,
    frac_value: u32,
    frac_truncated: bool,
    is_source_negative: bool,
    binding: &Binding,
) -> Warnings {
    let is_negative = is_source_negative && (second_val > 0 || frac_value > 0);
    let interval = sql::IntervalStruct {
        interval_type: sql::Interval::Second as i32,
        interval_sign: if is_negative { 1 } else { 0 },
        interval_value: sql::IntervalUnion {
            day_second: sql::DaySecond {
                day: 0,
                hour: 0,
                minute: 0,
                second: second_val,
                fraction: frac_value,
            },
        },
    };
    binding.write_fixed(interval);
    if frac_truncated {
        vec![Warning::NumericValueTruncated]
    } else {
        vec![]
    }
}

/// Builds and writes an `IntervalStruct` for IntervalSecond, including the
/// fractional microseconds component. Checks leading precision overflow.
pub fn write_interval_second(
    int_value: i128,
    abs_raw_value: u128,
    scale: u32,
    is_source_negative: bool,
    binding: &Binding,
) -> Result<Warnings, WriteOdbcError> {
    let abs_int = int_value.unsigned_abs();
    check_leading_precision(abs_int, int_value, binding)?;
    let second_val = checked_u32(abs_int, int_value)?;
    let (frac_value, frac_truncated) = compute_interval_fraction(abs_raw_value, scale);
    Ok(build_and_write_interval_second(
        second_val,
        frac_value,
        frac_truncated,
        is_source_negative,
        binding,
    ))
}

/// Coerces an already-integral value into the scalar C numeric targets shared
/// by integer-valued converters: the fixed-width integer types, `SQL_C_BIT`,
/// `SQL_C_NUMERIC`, and `SQL_C_BINARY`. `has_fractional` carries a truncation
/// that happened while reducing the source to `int_value` and surfaces as
/// 01S07. Targets outside this set — including any interval C type — return
/// 07006, so callers handle string and interval targets before delegating.
pub fn write_integer_as_numeric(
    target_type: CDataType,
    int_value: i128,
    has_fractional: bool,
    binding: &Binding,
) -> Result<Warnings, WriteOdbcError> {
    match target_type {
        CDataType::Short | CDataType::SShort => {
            check_integer_range(int_value, i16::MIN as i128, i16::MAX as i128)?;
            binding.write_fixed(int_value as i16);
            Ok(fractional_warning(has_fractional))
        }
        CDataType::UShort => {
            check_integer_range(int_value, 0, u16::MAX as i128)?;
            binding.write_fixed(int_value as u16);
            Ok(fractional_warning(has_fractional))
        }
        CDataType::TinyInt | CDataType::STinyInt => {
            check_integer_range(int_value, i8::MIN as i128, i8::MAX as i128)?;
            binding.write_fixed(int_value as i8);
            Ok(fractional_warning(has_fractional))
        }
        CDataType::UTinyInt => {
            check_integer_range(int_value, 0, u8::MAX as i128)?;
            binding.write_fixed(int_value as u8);
            Ok(fractional_warning(has_fractional))
        }
        CDataType::Long | CDataType::SLong => {
            check_integer_range(int_value, i32::MIN as i128, i32::MAX as i128)?;
            binding.write_fixed(int_value as i32);
            Ok(fractional_warning(has_fractional))
        }
        CDataType::ULong => {
            check_integer_range(int_value, 0, u32::MAX as i128)?;
            binding.write_fixed(int_value as u32);
            Ok(fractional_warning(has_fractional))
        }
        CDataType::SBigInt => {
            check_integer_range(int_value, i64::MIN as i128, i64::MAX as i128)?;
            binding.write_fixed(int_value as i64);
            Ok(fractional_warning(has_fractional))
        }
        CDataType::UBigInt => {
            check_integer_range(int_value, 0, u64::MAX as i128)?;
            binding.write_fixed(int_value as u64);
            Ok(fractional_warning(has_fractional))
        }
        CDataType::Bit => {
            if !(0..=1).contains(&int_value) {
                return NumericValueOutOfRangeSnafu {
                    reason: format!(
                        "Value out of range for SQL_C_BIT (must be 0 or 1, got {int_value})"
                    ),
                }
                .fail();
            }
            binding.write_fixed(int_value as u8);
            Ok(fractional_warning(has_fractional))
        }
        CDataType::Numeric => {
            let digits = int_value.unsigned_abs().to_string().len().max(1) as i16;
            let target_precision = binding.precision.unwrap_or(digits);
            let target_scale = binding.scale.unwrap_or(0);
            let abs = int_value.unsigned_abs();

            let (unscaled, truncated) = if target_scale >= 0 {
                match 10u128
                    .checked_pow(target_scale as u32)
                    .and_then(|f| abs.checked_mul(f))
                {
                    Some(v) => (v, false),
                    None => {
                        return NumericValueOutOfRangeSnafu {
                            reason: "Value out of range for SQL_C_NUMERIC".to_string(),
                        }
                        .fail();
                    }
                }
            } else {
                match 10u128.checked_pow((-target_scale) as u32) {
                    Some(divisor) => (abs / divisor, !abs.is_multiple_of(divisor)),
                    None => (0, abs != 0),
                }
            };

            let numeric = sql::Numeric {
                precision: target_precision as u8,
                scale: target_scale as i8,
                sign: if int_value < 0 { 0 } else { 1 },
                val: unscaled.to_le_bytes(),
            };
            binding.write_fixed(numeric);
            Ok(fractional_warning(has_fractional || truncated))
        }
        CDataType::Binary => {
            let digits = int_value.unsigned_abs().to_string().len().max(1) as u8;
            let numeric = sql::Numeric {
                precision: digits,
                scale: 0,
                sign: if int_value < 0 { 0 } else { 1 },
                val: int_value.unsigned_abs().to_le_bytes(),
            };
            write_numeric_as_binary(&numeric, binding)?;
            Ok(fractional_warning(has_fractional))
        }
        _ => UnsupportedOdbcTypeSnafu { target_type }.fail(),
    }
}

pub fn reject_multi_field_interval(target_type: CDataType) -> Result<Warnings, WriteOdbcError> {
    IntervalFieldOverflowSnafu {
        reason: format!(
            "Cannot convert numeric value to multi-field interval type {target_type:?}"
        ),
    }
    .fail()
}
