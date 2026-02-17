use arrow::array::{Array, ArrowPrimitiveType, PrimitiveArray};
use odbc_sys as sql;

use crate::cdata_types::CDataType;
use crate::conversion::error::{
    InvalidValueSnafu, NumericValueOutOfRangeSnafu, ReadArrowError, UnsupportedOdbcTypeSnafu,
    WriteOdbcError,
};
use crate::conversion::traits::Binding;
use crate::conversion::warning::{Warning, Warnings};
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

/// Controls how FIXED numeric columns are reported to ODBC applications.
/// These settings match the Snowflake server-side session parameters
/// `ODBC_TREAT_DECIMAL_AS_INT` and `ODBC_TREAT_BIG_NUMBER_AS_STRING`.
#[derive(Debug, Clone, Copy, Default)]
pub struct NumericSettings {
    /// When true, FIXED columns with scale=0 are reported as SQL_BIGINT
    /// instead of SQL_DECIMAL. Default C type becomes SQL_C_SBIGINT.
    /// Can be overridden by `treat_big_number_as_string` for precision > 18.
    pub treat_decimal_as_int: bool,
    /// When true, FIXED columns with precision > 18 are reported as SQL_VARCHAR.
    /// Takes precedence over `treat_decimal_as_int` for high-precision columns.
    pub treat_big_number_as_string: bool,
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
}

impl WriteODBCType for SnowflakeNumber {
    fn sql_type(&self) -> sql::SqlDataType {
        sql::SqlDataType::DECIMAL
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
                Ok(warnings)
            }
            CDataType::WChar => {
                let num_str = Self::format_decimal(snowflake_value, self.scale);
                let warnings = binding.write_wchar_string(&num_str, get_data_offset);
                Ok(warnings)
            }
            CDataType::Numeric => {
                let target_precision = binding.precision.ok_or_else(|| {
                    InvalidValueSnafu {
                        reason:
                            "SQL_DESC_PRECISION must be set on the ARD for SQL_C_NUMERIC bindings"
                                .to_string(),
                    }
                    .build()
                })?;
                let target_scale = binding.scale.ok_or_else(|| {
                    InvalidValueSnafu {
                        reason: "SQL_DESC_SCALE must be set on the ARD for SQL_C_NUMERIC bindings"
                            .to_string(),
                    }
                    .build()
                })?;

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
                let numeric_bytes: &[u8] = unsafe {
                    std::slice::from_raw_parts(
                        &numeric as *const sql::Numeric as *const u8,
                        std::mem::size_of::<sql::Numeric>(),
                    )
                };
                let copy_len = std::cmp::min(numeric_bytes.len(), binding.buffer_length as usize);
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        numeric_bytes.as_ptr(),
                        binding.target_value_ptr as *mut u8,
                        copy_len,
                    );
                }
                if !binding.str_len_or_ind_ptr.is_null() {
                    unsafe {
                        std::ptr::write(
                            binding.str_len_or_ind_ptr,
                            std::mem::size_of::<sql::Numeric>() as sql::Len,
                        )
                    };
                }
                Ok(vec![])
            }
            _ => UnsupportedOdbcTypeSnafu { target_type }.fail(),
        }
    }
}
