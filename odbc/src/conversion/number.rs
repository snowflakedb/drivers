use arrow::array::{Array, ArrowPrimitiveType, PrimitiveArray};
use odbc_sys as sql;

use crate::cdata_types::CDataType;
use crate::conversion::error::{
    NumericValueOutOfRangeSnafu, ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError,
};
use crate::conversion::traits::Binding;
use crate::conversion::warning::Warnings;
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

/// Represents the SQL numeric data types as defined by the ODBC specification.
/// Each SQL type has a different default C type used when the application
/// specifies `SQL_C_DEFAULT`.
/// Reference: https://learn.microsoft.com/en-us/sql/odbc/reference/appendixes/sql-to-c-numeric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NumericSqlType {
    Decimal,
}

impl NumericSqlType {
    pub(crate) fn default_c_type(&self) -> CDataType {
        match self {
            Self::Decimal => CDataType::Char,
        }
    }
}

pub(crate) struct SnowflakeNumber {
    pub(crate) scale: u32,
    #[allow(dead_code)]
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
}

impl WriteODBCType for SnowflakeNumber {
    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
    ) -> Result<Warnings, WriteOdbcError> {
        let target_type = match binding.target_type {
            CDataType::Default => self.sql_type.default_c_type(),
            other => other,
        };
        match target_type {
            CDataType::Double => {
                let double_value: f64 = snowflake_value as f64 / 10f64.powi(self.scale as i32);
                binding.write_fixed(double_value);
                Ok(vec![])
            }
            CDataType::Float => {
                let float_value: f32 = snowflake_value as f32 / 10f32.powi(self.scale as i32);
                binding.write_fixed(float_value);
                Ok(vec![])
            }
            CDataType::Short | CDataType::SShort | CDataType::UShort => {
                let short_value = snowflake_value / 10i128.pow(self.scale);
                binding.write_fixed(short_value as u16);
                Ok(vec![])
            }
            CDataType::TinyInt | CDataType::STinyInt | CDataType::UTinyInt => {
                let tinyint_value = snowflake_value / 10i128.pow(self.scale);
                binding.write_fixed(tinyint_value as u8);
                Ok(vec![])
            }
            CDataType::Long | CDataType::SLong | CDataType::ULong => {
                let long_value = snowflake_value / 10i128.pow(self.scale);
                binding.write_fixed(long_value as i32);
                Ok(vec![])
            }
            CDataType::SBigInt | CDataType::UBigInt => {
                let int_value = snowflake_value / 10i128.pow(self.scale);
                binding.write_fixed(int_value as i64);
                Ok(vec![])
            }
            CDataType::Bit => {
                let int_value = snowflake_value / 10i128.pow(self.scale);
                if !(0..=1).contains(&int_value) {
                    return NumericValueOutOfRangeSnafu {
                        reason: format!(
                            "Value {} out of range for SQL_C_BIT (must be 0 or 1)",
                            int_value
                        ),
                    }
                    .fail();
                }
                binding.write_fixed(int_value as u8);
                Ok(vec![])
            }
            CDataType::Char => {
                let num_str = Self::format_decimal(snowflake_value, self.scale);
                let warnings = binding.write_char_string(&num_str);
                Ok(warnings)
            }
            CDataType::WChar => {
                let num_str = Self::format_decimal(snowflake_value, self.scale);
                let warnings = binding.write_wchar_string(&num_str);
                Ok(warnings)
            }
            CDataType::Numeric => {
                let int_value = snowflake_value / 10i128.pow(self.scale);
                let abs_value = int_value.unsigned_abs();
                let sign: u8 = if int_value >= 0 { 1 } else { 0 };
                let numeric = sql::Numeric {
                    precision: self.precision as u8,
                    scale: 0,
                    sign,
                    val: abs_value.to_le_bytes(),
                };
                binding.write_fixed(numeric);
                Ok(vec![])
            }
            CDataType::Binary => {
                let int_value = snowflake_value / 10i128.pow(self.scale);
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
