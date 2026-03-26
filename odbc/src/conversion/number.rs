use arrow::array::{Array, ArrowPrimitiveType, PrimitiveArray};
use odbc_sys as sql;
use serde_json::Value;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::error::{
    IntervalFieldOverflowSnafu, NumericValueOutOfRangeSnafu, ReadArrowError,
    UnsupportedOdbcTypeSnafu, WriteOdbcError,
};
use crate::conversion::error::{JsonBindingError, UnsupportedCDataTypeSnafu};
use crate::conversion::param_binding::{read_char_str, read_unaligned, read_wchar_str};
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

impl SnowflakeNumber {
    fn format_decimal(value: i128, scale: u32) -> String {
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

    fn check_integer_range(value: i128, min: i128, max: i128) -> Result<(), WriteOdbcError> {
        if value < min || value > max {
            NumericValueOutOfRangeSnafu {
                reason: format!("Value {value} is out of range ({min} to {max})"),
            }
            .fail()
        } else {
            Ok(())
        }
    }

    fn fractional_warning(has_fractional: bool) -> Warnings {
        if has_fractional {
            vec![Warning::NumericValueTruncated]
        } else {
            vec![]
        }
    }

    fn whole_digits_len(num_str: &str) -> usize {
        match num_str.find('.') {
            Some(pos) => pos,
            None => num_str.len(),
        }
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

        let scale_factor = 10i128.pow(self.scale);
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
                Self::check_integer_range(int_value, i16::MIN as i128, i16::MAX as i128)?;
                binding.write_fixed(int_value as i16);
                Ok(Self::fractional_warning(has_fractional))
            }
            CDataType::UShort => {
                Self::check_integer_range(int_value, 0, u16::MAX as i128)?;
                binding.write_fixed(int_value as u16);
                Ok(Self::fractional_warning(has_fractional))
            }
            CDataType::TinyInt | CDataType::STinyInt => {
                Self::check_integer_range(int_value, i8::MIN as i128, i8::MAX as i128)?;
                binding.write_fixed(int_value as i8);
                Ok(Self::fractional_warning(has_fractional))
            }
            CDataType::UTinyInt => {
                Self::check_integer_range(int_value, 0, u8::MAX as i128)?;
                binding.write_fixed(int_value as u8);
                Ok(Self::fractional_warning(has_fractional))
            }
            CDataType::Long | CDataType::SLong => {
                Self::check_integer_range(int_value, i32::MIN as i128, i32::MAX as i128)?;
                binding.write_fixed(int_value as i32);
                Ok(Self::fractional_warning(has_fractional))
            }
            CDataType::ULong => {
                Self::check_integer_range(int_value, 0, u32::MAX as i128)?;
                binding.write_fixed(int_value as u32);
                Ok(Self::fractional_warning(has_fractional))
            }
            CDataType::SBigInt => {
                Self::check_integer_range(int_value, i64::MIN as i128, i64::MAX as i128)?;
                binding.write_fixed(int_value as i64);
                Ok(Self::fractional_warning(has_fractional))
            }
            CDataType::UBigInt => {
                Self::check_integer_range(int_value, 0, u64::MAX as i128)?;
                binding.write_fixed(int_value as u64);
                Ok(Self::fractional_warning(has_fractional))
            }
            CDataType::Bit => {
                if snowflake_value < 0 || int_value >= 2 {
                    return NumericValueOutOfRangeSnafu {
                        reason: format!(
                            "Value out of range for SQL_C_BIT (must be 0 or 1, got {})",
                            int_value
                        ),
                    }
                    .fail();
                }
                binding.write_fixed(int_value as u8);
                Ok(Self::fractional_warning(has_fractional))
            }
            CDataType::Char => {
                let num_str = Self::format_decimal(snowflake_value, self.scale);
                let warnings = binding.write_char_string(&num_str, get_data_offset);
                if warnings
                    .iter()
                    .any(|w| matches!(w, Warning::StringDataTruncated))
                {
                    let whole_len = Self::whole_digits_len(&num_str);
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
                let num_str = Self::format_decimal(snowflake_value, self.scale);
                let warnings = binding.write_wchar_string(&num_str, get_data_offset);
                if warnings
                    .iter()
                    .any(|w| matches!(w, Warning::StringDataTruncated))
                {
                    let whole_len = Self::whole_digits_len(&num_str);
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
            CDataType::Numeric => {
                let target_precision = binding.precision.unwrap_or(self.precision as i16);
                let target_scale = binding.scale.unwrap_or(0);

                let is_negative = snowflake_value < 0;
                let abs_value = snowflake_value.unsigned_abs();

                let scale_diff = target_scale as i32 - self.scale as i32;
                let truncated = if scale_diff < 0 {
                    let divisor = 10u128.pow((-scale_diff) as u32);
                    abs_value % divisor != 0
                } else {
                    false
                };
                let unscaled: u128 = if scale_diff >= 0 {
                    abs_value * 10u128.pow(scale_diff as u32)
                } else {
                    abs_value / 10u128.pow((-scale_diff) as u32)
                };

                let numeric = sql::Numeric {
                    precision: target_precision as u8,
                    scale: target_scale as i8,
                    sign: if is_negative { 0 } else { 1 },
                    val: unscaled.to_le_bytes(),
                };

                binding.write_fixed(numeric);
                Ok(Self::fractional_warning(truncated))
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
                    std::slice::from_raw_parts(
                        &numeric as *const sql::Numeric as *const u8,
                        numeric_size,
                    )
                };
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        numeric_bytes.as_ptr(),
                        binding.target_value_ptr as *mut u8,
                        numeric_size,
                    );
                }
                let _ = binding.write_length_or_null(
                    crate::conversion::traits::LengthOrNull::Length(numeric_size as sql::Len),
                );
                Ok(vec![])
            }
            CDataType::IntervalYear
            | CDataType::IntervalMonth
            | CDataType::IntervalDay
            | CDataType::IntervalHour
            | CDataType::IntervalMinute => {
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
                let is_negative = snowflake_value < 0 && field_val > 0;
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
                    CDataType::IntervalDay => {
                        interval.interval_type = sql::Interval::Day as i32;
                        interval.interval_value.day_second.day = field_val;
                    }
                    CDataType::IntervalHour => {
                        interval.interval_type = sql::Interval::Hour as i32;
                        interval.interval_value.day_second.hour = field_val;
                    }
                    CDataType::IntervalMinute => {
                        interval.interval_type = sql::Interval::Minute as i32;
                        interval.interval_value.day_second.minute = field_val;
                    }
                    _ => return UnsupportedOdbcTypeSnafu { target_type }.fail(),
                }
                binding.write_fixed(interval);
                Ok(Self::fractional_warning(has_fractional))
            }
            CDataType::IntervalSecond => {
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
                let (frac_value, frac_truncated) = if self.scale > 0 {
                    let remainder = snowflake_value.unsigned_abs() % (scale_factor as u128);
                    if self.scale > 6 {
                        let divisor = 10u128.pow(self.scale - 6);
                        (
                            (remainder / divisor) as u32,
                            !remainder.is_multiple_of(divisor),
                        )
                    } else {
                        let multiplier = 10u128.pow(6 - self.scale);
                        ((remainder * multiplier) as u32, false)
                    }
                } else {
                    (0, false)
                };
                let is_negative = snowflake_value < 0 && (second_val > 0 || frac_value > 0);
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
            CDataType::IntervalYearToMonth
            | CDataType::IntervalDayToHour
            | CDataType::IntervalDayToMinute
            | CDataType::IntervalDayToSecond
            | CDataType::IntervalHourToMinute
            | CDataType::IntervalHourToSecond
            | CDataType::IntervalMinuteToSecond => IntervalFieldOverflowSnafu {
                reason: format!(
                    "Cannot convert numeric value to multi-field interval type {target_type:?}"
                ),
            }
            .fail(),
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
