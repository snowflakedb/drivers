use odbc_sys as sql;

use crate::api::CDataType;
use crate::conversion::error::{
    IntervalFieldOverflowSnafu, NumericValueOutOfRangeSnafu, WriteOdbcError,
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
    let leading_precision = binding.datetime_interval_precision.unwrap_or(2) as u32;
    let max_leading = 10u128.pow(leading_precision);
    if abs_int >= max_leading {
        return IntervalFieldOverflowSnafu {
            reason: format!(
                "Value {int_value} exceeds leading field precision of {leading_precision} digits"
            ),
        }
        .fail();
    }
    let field_val = abs_int as u32;
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
    let leading_precision = binding.datetime_interval_precision.unwrap_or(2) as u32;
    let max_leading = 10u128.pow(leading_precision);
    if abs_int >= max_leading {
        return IntervalFieldOverflowSnafu {
            reason: format!(
                "Value {int_value} exceeds leading field precision of {leading_precision} digits"
            ),
        }
        .fail();
    }
    let second_val = abs_int as u32;
    let (frac_value, frac_truncated) = compute_interval_fraction(abs_raw_value, scale);
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
    Ok(if frac_truncated {
        vec![Warning::NumericValueTruncated]
    } else {
        vec![]
    })
}

pub fn reject_multi_field_interval(target_type: CDataType) -> Result<Warnings, WriteOdbcError> {
    IntervalFieldOverflowSnafu {
        reason: format!(
            "Cannot convert numeric value to multi-field interval type {target_type:?}"
        ),
    }
    .fail()
}
