use arrow::array::{Array, GenericByteArray, StructArray};
use arrow::datatypes::{GenericBinaryType, Int16Type};
use odbc_sys as sql;

use crate::api::CDataType;
use crate::conversion::error::{
    InvalidArrowValueSnafu, NumericValueOutOfRangeSnafu, ReadArrowError, UnsupportedOdbcTypeSnafu,
    WriteOdbcError,
};
use crate::conversion::numeric_helpers::{
    check_integer_range, fractional_warning, reject_multi_field_interval, write_interval_second,
    write_numeric_as_binary, write_single_field_interval,
};
use crate::conversion::traits::Binding;
use crate::conversion::warning::Warnings;
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

pub(crate) struct SnowflakeDecfloat {
    pub precision: u32,
}

impl SnowflakeType for SnowflakeDecfloat {
    type Representation<'a> = (i128, i16);
}

/// Converts a big-endian two's complement byte slice (1–16 bytes) into an i128.
/// The Arrow wire format trims leading bytes, so we sign-extend to 16 bytes
/// before calling `i128::from_be_bytes`. Empty input is treated as zero.
pub(crate) fn i128_from_big_endian_signed(bytes: &[u8]) -> Result<i128, ReadArrowError> {
    if bytes.is_empty() {
        return Ok(0);
    }
    if bytes.len() > 16 {
        return InvalidArrowValueSnafu {
            reason: format!(
                "significand byte length {} exceeds maximum of 16",
                bytes.len()
            ),
        }
        .fail();
    }
    let sign_bytes = if bytes[0] & 0x80 != 0 { 0xFF } else { 0x00 };
    let mut buf = [sign_bytes; 16];
    buf[16 - bytes.len()..].copy_from_slice(bytes);
    Ok(i128::from_be_bytes(buf))
}

impl ReadArrowType<StructArray> for SnowflakeDecfloat {
    fn read_arrow_type<'a>(
        &self,
        array: &'a StructArray,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        if array.is_null(row_idx) {
            return Err(ReadArrowError::NullValue {
                location: snafu::location!(),
            });
        }

        let exponent_array = array
            .column_by_name("exponent")
            .ok_or_else(|| ReadArrowError::InvalidArrowValue {
                reason: "DECFLOAT struct missing 'exponent' field".to_string(),
                location: snafu::location!(),
            })?
            .as_any()
            .downcast_ref::<arrow::array::PrimitiveArray<Int16Type>>()
            .ok_or_else(|| ReadArrowError::InvalidArrowValue {
                reason: "DECFLOAT 'exponent' field is not Int16".to_string(),
                location: snafu::location!(),
            })?;

        let significand_array = array
            .column_by_name("significand")
            .ok_or_else(|| ReadArrowError::InvalidArrowValue {
                reason: "DECFLOAT struct missing 'significand' field".to_string(),
                location: snafu::location!(),
            })?
            .as_any()
            .downcast_ref::<GenericByteArray<GenericBinaryType<i32>>>()
            .ok_or_else(|| ReadArrowError::InvalidArrowValue {
                reason: "DECFLOAT 'significand' field is not Binary".to_string(),
                location: snafu::location!(),
            })?;

        let exponent = exponent_array.value(row_idx);
        let significand = i128_from_big_endian_signed(significand_array.value(row_idx))?;

        Ok((significand, exponent))
    }
}

/// Formats a DECFLOAT value as a string.
///
/// Uses plain decimal notation when the result fits within 38 characters
/// (unsigned), and normalized scientific notation otherwise. Scientific
/// notation uses lowercase 'e' with no '+' on positive exponents.
pub(crate) fn format_decfloat(sig: i128, exp: i16, max_plain_digits: usize) -> String {
    if sig == 0 {
        return "0".to_string();
    }

    let is_negative = sig < 0;
    let mut abs_sig = sig.unsigned_abs();
    let mut exp = exp as i64;

    // Normalize: strip trailing zeros from significand
    while abs_sig.is_multiple_of(10) {
        abs_sig /= 10;
        exp += 1;
    }

    let digits = abs_sig.to_string();
    let n = digits.len();

    let plain_len = if exp >= 0 {
        n + exp as usize
    } else {
        let abs_exp = (-exp) as usize;
        if abs_exp < n {
            n + 1 // decimal point within digits
        } else {
            2 + abs_exp // "0." prefix + leading zeros + digits
        }
    };

    let mut result = if plain_len <= max_plain_digits {
        if exp >= 0 {
            let mut s = digits;
            for _ in 0..exp {
                s.push('0');
            }
            s
        } else {
            let abs_exp = (-exp) as usize;
            if abs_exp < n {
                let decimal_pos = n - abs_exp;
                let mut s = String::with_capacity(n + 1);
                s.push_str(&digits[..decimal_pos]);
                s.push('.');
                s.push_str(&digits[decimal_pos..]);
                s
            } else {
                let leading_zeros = abs_exp - n;
                let mut s = String::with_capacity(2 + abs_exp);
                s.push_str("0.");
                for _ in 0..leading_zeros {
                    s.push('0');
                }
                s.push_str(&digits);
                s
            }
        }
    } else {
        // Scientific notation
        let adjusted_exp = exp + (n as i64) - 1;
        let mut s = String::new();
        s.push_str(&digits[0..1]);
        if n > 1 {
            s.push('.');
            s.push_str(&digits[1..]);
        }
        s.push('e');
        s.push_str(&adjusted_exp.to_string());
        s
    };

    if is_negative {
        result.insert(0, '-');
    }
    result
}

/// Computes the integer part and whether there is a fractional remainder
/// for a DECFLOAT value represented as `significand * 10^exponent`.
/// Returns `(int_value, has_fractional, overflowed)` where `overflowed`
/// is true when the scaled integer exceeds i128 range.
fn compute_int_and_fractional(sig: i128, exp: i16) -> (i128, bool, bool) {
    if exp >= 0 {
        let factor = 10i128.checked_pow(exp as u32);
        match factor {
            Some(f) => match sig.checked_mul(f) {
                Some(v) => (v, false, false),
                None => (if sig >= 0 { i128::MAX } else { i128::MIN }, false, true),
            },
            None => (if sig >= 0 { i128::MAX } else { i128::MIN }, false, true),
        }
    } else {
        let scale = (-exp) as u32;
        match 10i128.checked_pow(scale) {
            Some(divisor) => {
                let int_val = sig / divisor;
                let has_frac = sig % divisor != 0;
                (int_val, has_frac, false)
            }
            None => (0, sig != 0, false),
        }
    }
}

fn decfloat_to_f64(sig: i128, exp: i16) -> f64 {
    (sig as f64) * 10f64.powi(exp as i32)
}

impl WriteODBCType for SnowflakeDecfloat {
    fn sql_type(&self) -> sql::SqlDataType {
        sql::SqlDataType::NUMERIC
    }

    fn column_size(&self) -> sql::ULen {
        self.precision as sql::ULen
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
        let (significand, exponent) = snowflake_value;
        let target_type = match binding.target_type {
            CDataType::Default => CDataType::Char,
            other => other,
        };

        match target_type {
            CDataType::Char => {
                let decfloat = format_decfloat(significand, exponent, self.precision as usize);
                Ok(binding.write_char_string(&decfloat, get_data_offset))
            }
            CDataType::WChar => {
                let decfloat = format_decfloat(significand, exponent, self.precision as usize);
                Ok(binding.write_wchar_string(&decfloat, get_data_offset))
            }
            CDataType::Double => {
                let double_value = decfloat_to_f64(significand, exponent);
                if double_value.is_infinite() {
                    return NumericValueOutOfRangeSnafu {
                        reason: "Value out of range for SQL_C_DOUBLE".to_string(),
                    }
                    .fail();
                }
                binding.write_fixed(double_value);
                Ok(vec![])
            }
            CDataType::Float => {
                let float_value = decfloat_to_f64(significand, exponent) as f32;
                if float_value.is_infinite() {
                    return NumericValueOutOfRangeSnafu {
                        reason: "Value out of range for SQL_C_FLOAT".to_string(),
                    }
                    .fail();
                }
                binding.write_fixed(float_value);
                Ok(vec![])
            }
            _ => {
                let (int_value, has_fractional, overflowed) =
                    compute_int_and_fractional(significand, exponent);
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
                        if significand < 0 || int_value >= 2 {
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
                        let target_precision = binding.precision.unwrap_or(self.precision as i16);
                        let target_scale = binding.scale.unwrap_or(0);

                        let is_negative = significand < 0;
                        let abs_sig = significand.unsigned_abs();

                        let adjusted_exp = exponent as i32 + target_scale as i32;
                        let (unscaled, truncated) = if adjusted_exp >= 0 {
                            let factor = 10u128.checked_pow(adjusted_exp as u32);
                            match factor.and_then(|f| abs_sig.checked_mul(f)) {
                                Some(v) => (v, false),
                                None => {
                                    return NumericValueOutOfRangeSnafu {
                                        reason: "Value out of range for SQL_C_NUMERIC".to_string(),
                                    }
                                    .fail();
                                }
                            }
                        } else {
                            match 10u128.checked_pow((-adjusted_exp) as u32) {
                                Some(divisor) => (abs_sig / divisor, abs_sig % divisor != 0),
                                None => (0, abs_sig != 0),
                            }
                        };

                        let numeric = sql::Numeric {
                            precision: target_precision as u8,
                            scale: target_scale as i8,
                            sign: if is_negative { 0 } else { 1 },
                            val: unscaled.to_le_bytes(),
                        };

                        binding.write_fixed(numeric);
                        Ok(fractional_warning(truncated))
                    }
                    CDataType::IntervalYear
                    | CDataType::IntervalMonth
                    | CDataType::IntervalDay
                    | CDataType::IntervalHour
                    | CDataType::IntervalMinute => write_single_field_interval(
                        target_type,
                        int_value,
                        significand < 0,
                        has_fractional,
                        binding,
                    ),
                    CDataType::IntervalSecond => {
                        let scale = if exponent < 0 { (-exponent) as u32 } else { 0 };
                        write_interval_second(
                            int_value,
                            significand.unsigned_abs(),
                            scale,
                            significand < 0,
                            binding,
                        )
                    }
                    CDataType::IntervalYearToMonth
                    | CDataType::IntervalDayToHour
                    | CDataType::IntervalDayToMinute
                    | CDataType::IntervalDayToSecond
                    | CDataType::IntervalHourToMinute
                    | CDataType::IntervalHourToSecond
                    | CDataType::IntervalMinuteToSecond => reject_multi_field_interval(target_type),
                    CDataType::Binary => {
                        if overflowed {
                            return NumericValueOutOfRangeSnafu {
                                reason: "Value out of range for SQL_C_BINARY".to_string(),
                            }
                            .fail();
                        }
                        let abs_value = int_value.unsigned_abs();
                        let sign: u8 = if int_value >= 0 { 1 } else { 0 };
                        let numeric = sql::Numeric {
                            precision: self.precision as u8,
                            scale: 0,
                            sign,
                            val: abs_value.to_le_bytes(),
                        };
                        write_numeric_as_binary(&numeric, binding)?;
                        Ok(vec![])
                    }
                    _ => UnsupportedOdbcTypeSnafu { target_type }.fail(),
                }
            }
        }
    }
}
