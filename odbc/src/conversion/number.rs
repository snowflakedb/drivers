use arrow::array::{Array, ArrowPrimitiveType, PrimitiveArray};
use odbc_sys as sql;
use serde_json::Value;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::error::{
    BindingNumericOutOfRangeSnafu, JsonBindingError, NumericMagnitudeOverflowSnafu,
    UnsupportedCDataTypeSnafu,
};
use crate::conversion::error::{
    NumericValueOutOfRangeSnafu, ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError,
};
use crate::conversion::numeric_helpers::{
    check_integer_range, fractional_warning, reject_multi_field_interval, whole_digits_len,
    write_interval_second, write_numeric_as_binary, write_single_field_interval,
};
use crate::conversion::param_binding::{
    buffer_data_len, read_char_str, read_numeric_struct, read_unaligned, read_wchar_str,
};
use crate::conversion::traits::Binding;
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, WriteJson};
use crate::conversion::warning::{Warning, Warnings};
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

/// Controls how FIXED numeric columns are reported to ODBC applications.
/// These settings match the Snowflake server-side session parameters
/// `ODBC_TREAT_DECIMAL_AS_INT` and `ODBC_TREAT_BIG_NUMBER_AS_STRING`.
#[derive(Debug, Clone, Copy)]
pub struct NumericSettings {
    /// When true, FIXED columns with scale=0 are reported as SQL_BIGINT
    /// instead of SQL_DECIMAL. Default C type becomes SQL_C_SBIGINT.
    /// Can be overridden by `treat_big_number_as_string` for precision > 18.
    pub treat_decimal_as_int: bool,
    /// When true, FIXED columns with precision > 18 are reported as SQL_VARCHAR.
    /// Takes precedence over `treat_decimal_as_int` for high-precision columns.
    pub treat_big_number_as_string: bool,
    /// Server-reported maximum VARCHAR size (from session parameter
    /// `VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT`). Used as the default
    /// `column_size` in auto-populated IPD records for untyped `?` markers.
    pub max_varchar_size: u64,
}

/// Snowflake default max VARCHAR size (16 MB). Overridden by the server's
/// `VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT` session parameter after login.
pub const SF_DEFAULT_VARCHAR_MAX_LEN: u64 = 16_777_216;

impl Default for NumericSettings {
    fn default() -> Self {
        Self {
            treat_decimal_as_int: false,
            treat_big_number_as_string: false,
            max_varchar_size: SF_DEFAULT_VARCHAR_MAX_LEN,
        }
    }
}

/// Represents the SQL numeric data types as defined by the ODBC specification.
/// Each SQL type has a different default C type used when the application
/// specifies `SQL_C_DEFAULT`.
/// Reference: https://learn.microsoft.com/en-us/sql/odbc/reference/appendixes/sql-to-c-numeric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NumericSqlType {
    Decimal,
    BigInt,
    VarChar,
}

impl NumericSqlType {
    pub(crate) fn default_c_type(&self) -> CDataType {
        match self {
            Self::Decimal => CDataType::Char,
            Self::BigInt => CDataType::SBigInt,
            Self::VarChar => CDataType::Char,
        }
    }

    pub(crate) fn from_scale_and_precision(
        scale: u32,
        precision: u32,
        settings: &NumericSettings,
    ) -> Self {
        let mut result = Self::Decimal;

        if settings.treat_decimal_as_int && scale == 0 {
            result = Self::BigInt;
        }

        if precision > 18 && settings.treat_big_number_as_string {
            result = Self::VarChar;
        }

        result
    }
}

pub(crate) struct SnowflakeNumber {
    pub(crate) scale: u32,
    pub(crate) precision: u32,
    pub(crate) sql_type: NumericSqlType,
}

impl SnowflakeType for SnowflakeNumber {
    type Representation<'a> = i128;
}

impl<T: ArrowPrimitiveType> ReadArrowType<PrimitiveArray<T>> for SnowflakeNumber
where
    T::Native: Into<i128>,
{
    fn read_arrow_type<'a>(
        &self,
        array: &'a PrimitiveArray<T>,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        if array.is_null(row_idx) {
            return Err(ReadArrowError::NullValue {
                location: snafu::location!(),
            });
        }
        let v: i128 = array.value(row_idx).into();
        Ok(v)
    }
}

/// Maximum DECIMAL scale supported by Snowflake (and addressable by `i128`).
/// `10i128.pow(38)` is the largest power of ten that fits in `i128`.
const MAX_DECIMAL_SCALE: u32 = 38;

/// Precomputed `10^n` for `0 ≤ n ≤ 38`, as `i128`. Used to avoid invoking
/// `i128::pow` (which is not a const lookup) on every row in hot conversion
/// paths.
const POW10_I128: [i128; (MAX_DECIMAL_SCALE + 1) as usize] = {
    let mut a = [1i128; (MAX_DECIMAL_SCALE + 1) as usize];
    let mut i = 1;
    while i < a.len() {
        a[i] = a[i - 1] * 10;
        i += 1;
    }
    a
};

/// Same table for `u128`. Needed for re-scaling in the SQL_C_NUMERIC arm.
const POW10_U128: [u128; (MAX_DECIMAL_SCALE + 1) as usize] = {
    let mut a = [1u128; (MAX_DECIMAL_SCALE + 1) as usize];
    let mut i = 1;
    while i < a.len() {
        a[i] = a[i - 1] * 10;
        i += 1;
    }
    a
};

impl SnowflakeNumber {
    /// Format a scaled `i128` as a decimal string into `buf` without any heap
    /// allocation, returning the filled slice as `&str`.
    ///
    /// `buf` must be large enough for the widest possible output:
    /// optional `-`, up to 39 digits of whole part, a `.`, and up to 38
    /// fractional digits. 48 bytes is sufficient.
    ///
    /// The output is byte-identical to the previous
    /// `format!`/`String::insert`-based implementation, including the shape
    /// `"0.000...digits"` when `scale > digits`.
    fn format_decimal_into(value: i128, scale: u32, buf: &mut [u8; 48]) -> &str {
        // Stage the absolute-value digits through `itoa`-equivalent formatting
        // (via the std library's `Display` for unsigned integers, which writes
        // directly into the provided buffer without heap allocation).
        let mut abs_tmp = [0u8; 40]; // 39 digits for u128::MAX + headroom
        let abs_len = {
            let mut cur = std::io::Cursor::new(&mut abs_tmp[..]);
            use std::io::Write as _;
            // Infallible: abs_tmp is large enough for any u128.
            let _ = write!(cur, "{}", value.unsigned_abs());
            cur.position() as usize
        };
        let digits = &abs_tmp[..abs_len];
        let is_negative = value < 0;
        let scale = scale as usize;

        let mut len = 0;
        if is_negative {
            buf[len] = b'-';
            len += 1;
        }
        if scale == 0 {
            buf[len..len + digits.len()].copy_from_slice(digits);
            len += digits.len();
        } else if digits.len() > scale {
            let int_part = digits.len() - scale;
            buf[len..len + int_part].copy_from_slice(&digits[..int_part]);
            len += int_part;
            buf[len] = b'.';
            len += 1;
            buf[len..len + scale].copy_from_slice(&digits[int_part..]);
            len += scale;
        } else {
            // "0." + (scale - digits.len()) zero-pad + digits.
            // Matches the original implementation which wrote a leading "0"
            // whenever `scale >= digits.len()` (the `while s.len() <= scale`
            // loop grew `s` until the decimal insert position was zero).
            buf[len] = b'0';
            len += 1;
            buf[len] = b'.';
            len += 1;
            let pad = scale - digits.len();
            for b in &mut buf[len..len + pad] {
                *b = b'0';
            }
            len += pad;
            buf[len..len + digits.len()].copy_from_slice(digits);
            len += digits.len();
        }
        // SAFETY: only ASCII digits, '-', and '.' were written above.
        unsafe { std::str::from_utf8_unchecked(&buf[..len]) }
    }
}

impl WriteODBCType for SnowflakeNumber {
    fn sql_type(&self) -> sql::SqlDataType {
        sql::SqlDataType::DECIMAL
    }

    fn column_size(&self) -> sql::ULen {
        self.precision as sql::ULen
    }

    fn decimal_digits(&self) -> sql::SmallInt {
        self.scale as sql::SmallInt
    }

    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
        get_data_offset: &mut Option<usize>,
    ) -> Result<Warnings, WriteOdbcError> {
        let target_type = match binding.target_type {
            CDataType::Default => self.sql_type.default_c_type(),
            other => other,
        };

        let scale_factor = POW10_I128[self.scale as usize];
        let int_value = snowflake_value / scale_factor;
        let has_fractional = self.scale > 0 && snowflake_value % scale_factor != 0;

        match target_type {
            CDataType::Double => {
                let double_value: f64 = snowflake_value as f64 / 10f64.powi(self.scale as i32);
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
                let float_value: f32 = snowflake_value as f32 / 10f32.powi(self.scale as i32);
                if float_value.is_infinite() {
                    return NumericValueOutOfRangeSnafu {
                        reason: "Value out of range for SQL_C_FLOAT".to_string(),
                    }
                    .fail();
                }
                binding.write_fixed(float_value);
                Ok(vec![])
            }
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
                if snowflake_value < 0 || int_value >= 2 {
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
            CDataType::Char => {
                let mut num_buf = [0u8; 48];
                let num_str = Self::format_decimal_into(snowflake_value, self.scale, &mut num_buf);
                let warnings = binding.write_char_string(num_str, get_data_offset);
                if warnings
                    .iter()
                    .any(|w| matches!(w, Warning::StringDataTruncated))
                    && whole_digits_len(num_str) >= binding.buffer_length as usize
                {
                    *get_data_offset = None;
                    return NumericValueOutOfRangeSnafu {
                        reason: format!(
                            "Whole digits of '{num_str}' do not fit in buffer of {} bytes",
                            binding.buffer_length
                        ),
                    }
                    .fail();
                }
                Ok(warnings)
            }
            CDataType::WChar => {
                let mut num_buf = [0u8; 48];
                let num_str = Self::format_decimal_into(snowflake_value, self.scale, &mut num_buf);
                let warnings = binding.write_wchar_string(num_str, get_data_offset);
                let wchar_capacity = (binding.buffer_length / 2) as usize;
                if warnings
                    .iter()
                    .any(|w| matches!(w, Warning::StringDataTruncated))
                    && whole_digits_len(num_str) >= wchar_capacity
                {
                    *get_data_offset = None;
                    return NumericValueOutOfRangeSnafu {
                        reason: format!(
                            "Whole digits of '{num_str}' do not fit in wchar buffer of {wchar_capacity} chars",
                        ),
                    }
                    .fail();
                }
                Ok(warnings)
            }
            CDataType::Numeric => {
                let target_precision = binding.precision.unwrap_or(self.precision as i16);
                let target_scale = binding.scale.unwrap_or(0);

                let is_negative = snowflake_value < 0;
                let abs_value = snowflake_value.unsigned_abs();

                let scale_diff = target_scale as i32 - self.scale as i32;
                let truncated = if scale_diff < 0 {
                    let divisor = POW10_U128[(-scale_diff) as usize];
                    abs_value % divisor != 0
                } else {
                    false
                };
                let unscaled: u128 = if scale_diff >= 0 {
                    abs_value * POW10_U128[scale_diff as usize]
                } else {
                    abs_value / POW10_U128[(-scale_diff) as usize]
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
            CDataType::Binary => {
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
            CDataType::IntervalYear
            | CDataType::IntervalMonth
            | CDataType::IntervalDay
            | CDataType::IntervalHour
            | CDataType::IntervalMinute => write_single_field_interval(
                target_type,
                int_value,
                snowflake_value < 0,
                has_fractional,
                binding,
            ),
            CDataType::IntervalSecond => write_interval_second(
                int_value,
                snowflake_value.unsigned_abs(),
                self.scale,
                snowflake_value < 0,
                binding,
            ),
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

impl ReadODBC for SnowflakeNumber {
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, JsonBindingError> {
        let value = match binding.value_type {
            CDataType::Long | CDataType::SLong => read_unaligned::<i32>(binding) as i128,
            CDataType::Short | CDataType::SShort => read_unaligned::<i16>(binding) as i128,
            CDataType::SBigInt => read_unaligned::<i64>(binding) as i128,
            CDataType::ULong => read_unaligned::<u32>(binding) as i128,
            CDataType::UShort => read_unaligned::<u16>(binding) as i128,
            CDataType::UBigInt => read_unaligned::<u64>(binding) as i128,
            CDataType::TinyInt | CDataType::STinyInt => read_unaligned::<i8>(binding) as i128,
            CDataType::UTinyInt => read_unaligned::<u8>(binding) as i128,
            CDataType::Float => {
                let v = read_unaligned::<f32>(binding) as f64;
                if !v.is_finite() {
                    return NumericMagnitudeOverflowSnafu {
                        reason: format!("non-finite f32 value {v} cannot be converted to integer"),
                    }
                    .fail();
                }
                let truncated = v.trunc();
                if truncated < (i128::MIN as f64) || truncated > (i128::MAX as f64) {
                    return NumericMagnitudeOverflowSnafu {
                        reason: format!("f32 value {v} out of i128 range"),
                    }
                    .fail();
                }
                truncated as i128
            }
            CDataType::Double => {
                let v = read_unaligned::<f64>(binding);
                if !v.is_finite() {
                    return NumericMagnitudeOverflowSnafu {
                        reason: format!("non-finite f64 value {v} cannot be converted to integer"),
                    }
                    .fail();
                }
                let truncated = v.trunc();
                if truncated < (i128::MIN as f64) || truncated > (i128::MAX as f64) {
                    return NumericMagnitudeOverflowSnafu {
                        reason: format!("f64 value {v} out of i128 range"),
                    }
                    .fail();
                }
                truncated as i128
            }
            CDataType::Bit => read_unaligned::<u8>(binding) as i128,
            CDataType::Numeric => {
                let (mantissa, scale) = read_numeric_struct(binding)?;
                if scale > 0 {
                    let divisor = 10i128.checked_pow(scale as u32).ok_or_else(|| {
                        NumericMagnitudeOverflowSnafu {
                            reason: format!("10^{scale} overflows i128 (positive scale too large)"),
                        }
                        .build()
                    })?;
                    mantissa / divisor
                } else if scale < 0 {
                    let abs_scale = if scale == i8::MIN {
                        (i8::MAX as u32) + 1
                    } else {
                        (-scale) as u32
                    };
                    let multiplier = 10i128.checked_pow(abs_scale).ok_or_else(|| {
                        NumericMagnitudeOverflowSnafu {
                            reason: format!(
                                "10^{abs_scale} overflows i128 (negative scale too large)"
                            ),
                        }
                        .build()
                    })?;
                    mantissa.checked_mul(multiplier).ok_or_else(|| {
                        NumericMagnitudeOverflowSnafu {
                            reason: format!("mantissa * 10^{abs_scale} overflows i128"),
                        }
                        .build()
                    })?
                } else {
                    mantissa
                }
            }
            CDataType::Char => {
                let s = read_char_str(binding)?;
                s.trim().parse::<i128>().map_err(|_| {
                    UnsupportedCDataTypeSnafu {
                        c_type: binding.value_type,
                    }
                    .build()
                })?
            }
            CDataType::WChar => {
                let s = read_wchar_str(binding)?;
                s.trim().parse::<i128>().map_err(|_| {
                    UnsupportedCDataTypeSnafu {
                        c_type: binding.value_type,
                    }
                    .build()
                })?
            }
            CDataType::Binary => {
                let len = buffer_data_len(binding);
                let expected = match binding.sql_data_type {
                    sql::SqlDataType::EXT_BIG_INT => 8usize,
                    sql::SqlDataType::INTEGER => 4,
                    sql::SqlDataType::SMALLINT => 2,
                    sql::SqlDataType::EXT_TINY_INT => 1,
                    _ => {
                        return BindingNumericOutOfRangeSnafu {
                            reason: format!(
                                "SQL_C_BINARY is not supported for {:?} in integer context",
                                binding.sql_data_type
                            ),
                        }
                        .fail();
                    }
                };
                if len != expected {
                    return BindingNumericOutOfRangeSnafu {
                        reason: format!(
                            "SQL_C_BINARY buffer length {len} does not match expected size {expected} for {:?}",
                            binding.sql_data_type
                        ),
                    }
                    .fail();
                }
                match expected {
                    8 => read_unaligned::<i64>(binding) as i128,
                    4 => read_unaligned::<i32>(binding) as i128,
                    2 => read_unaligned::<i16>(binding) as i128,
                    1 => read_unaligned::<i8>(binding) as i128,
                    _ => unreachable!(),
                }
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

impl WriteJson for SnowflakeNumber {
    fn write_json(&self, value: Self::Representation<'_>) -> Result<Value, JsonBindingError> {
        Ok(Value::String(value.to_string()))
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::Fixed
    }
}

#[cfg(test)]
mod format_decimal_into_tests {
    use super::{MAX_DECIMAL_SCALE, SnowflakeNumber};

    /// Reference implementation of the previous `format_decimal` — kept
    /// inside the test module so that we can assert byte-identical output
    /// for the new stack-buffer implementation.
    fn format_decimal_reference(value: i128, scale: u32) -> String {
        if scale > 0 {
            let mut s = value.to_string();
            let is_negative = s.starts_with('-');
            if is_negative {
                s.remove(0);
            }
            while s.len() <= scale as usize {
                s.insert(0, '0');
            }
            let decimal_pos = s.len() - scale as usize;
            s.insert(decimal_pos, '.');
            if is_negative {
                s.insert(0, '-');
            }
            s
        } else {
            value.to_string()
        }
    }

    fn assert_matches(value: i128, scale: u32) {
        let mut buf = [0u8; 48];
        let actual = SnowflakeNumber::format_decimal_into(value, scale, &mut buf);
        let expected = format_decimal_reference(value, scale);
        assert_eq!(
            actual, expected,
            "format_decimal_into({value}, {scale}) mismatch"
        );
    }

    #[test]
    fn scale_zero_integer() {
        assert_matches(0, 0);
        assert_matches(1, 0);
        assert_matches(-1, 0);
        assert_matches(123456789, 0);
        assert_matches(-123456789, 0);
    }

    #[test]
    fn scale_greater_than_digit_count_pads_with_zeros() {
        // Previously hit the O(n^2) insert(0) loop.
        assert_matches(1, 30);
        assert_matches(-1, 30);
        assert_matches(9, 10);
        assert_matches(-9, 10);
    }

    #[test]
    fn scale_within_digit_count() {
        assert_matches(12345, 2);
        assert_matches(-12345, 2);
        assert_matches(100, 3);
        assert_matches(-100, 3);
    }

    #[test]
    fn value_zero_at_various_scales() {
        for scale in 0..=MAX_DECIMAL_SCALE {
            assert_matches(0, scale);
        }
    }

    #[test]
    fn i128_extremes() {
        assert_matches(i128::MAX, 0);
        assert_matches(i128::MIN, 0);
        assert_matches(i128::MAX, 10);
        assert_matches(i128::MIN, 10);
    }

    #[test]
    fn large_values_with_fractional_scale() {
        assert_matches(123_456_789_012_345_678_901_234_567_890i128, 12);
        assert_matches(-123_456_789_012_345_678_901_234_567_890i128, 12);
    }

    #[test]
    fn every_scale_zero_to_max() {
        for scale in 0..=MAX_DECIMAL_SCALE {
            assert_matches(1, scale);
            assert_matches(-1, scale);
            assert_matches(1234567890, scale);
            assert_matches(-1234567890, scale);
        }
    }
}
