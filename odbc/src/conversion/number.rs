use arrow::array::{Array, ArrowPrimitiveType, PrimitiveArray};
use odbc_sys::Len;

use crate::cdata_types::CDataType;
use crate::conversion::error::{ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError};
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
    fn set_fixed_indicator(binding: &Binding, byte_size: Len) {
        if !binding.str_len_or_ind_ptr.is_null() {
            unsafe { std::ptr::write(binding.str_len_or_ind_ptr, byte_size) };
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
        match binding.target_type {
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
                let short_value = (snowflake_value as i64) / 10i64.pow(self.scale);
                binding.write_fixed(short_value as u16);
                Ok(vec![])
            }
            CDataType::TinyInt | CDataType::STinyInt | CDataType::UTinyInt => {
                let tinyint_value = (snowflake_value as i64) / 10i64.pow(self.scale);
                binding.write_fixed(tinyint_value as u8);
                Ok(vec![])
            }
            CDataType::Long | CDataType::SLong | CDataType::ULong => {
                let long_value = (snowflake_value as i32) / 10i32.pow(self.scale);
                binding.write_fixed(long_value);
                Ok(vec![])
            }
            CDataType::SBigInt | CDataType::UBigInt => {
                let int_value = (snowflake_value as i64) / 10i64.pow(self.scale);
                binding.write_fixed(int_value);
                Ok(vec![])
            }
            CDataType::Bit => {
                let int_value = (snowflake_value as i64) / 10i64.pow(self.scale);
                if !(0..=1).contains(&int_value) {
                    return Err(ConversionError::NumericValueOutOfRange {
                        target_type,
                        location: snafu::location!(),
                    });
                }
                unsafe {
                    std::ptr::write(binding.value as *mut u8, int_value as u8);
                }
                Self::set_fixed_indicator(binding, std::mem::size_of::<u8>() as Len);
                Ok(())
            }
            CDataType::Char => {
                let num_str = if self.scale > 0 {
                    let mut s = snowflake_value.to_string();
                    let is_negative = s.starts_with('-');
                    if is_negative {
                        s.remove(0);
                    }

                    // Pad with leading zeros if necessary
                    while s.len() <= self.scale as usize {
                        s.insert(0, '0');
                    }

                    // Insert decimal point
                    let decimal_pos = s.len() - self.scale as usize;
                    s.insert(decimal_pos, '.');

                    if is_negative {
                        s.insert(0, '-');
                    }
                    s
                } else {
                    snowflake_value.to_string()
                };
                let bytes = num_str.as_bytes();
                if !binding.str_len_or_ind_ptr.is_null() {
                    unsafe { std::ptr::write(binding.str_len_or_ind_ptr, bytes.len() as Len) };
                }
                if binding.buffer_length > 0 {
                    let copy_len = std::cmp::min((binding.buffer_length - 1) as usize, bytes.len());
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            bytes.as_ptr(),
                            binding.target_value_ptr as *mut u8,
                            copy_len,
                        );
                        // Null-terminate per ODBC spec
                        std::ptr::write((binding.value as *mut u8).add(copy_len), 0u8);
                    }
                }
                Ok(vec![])
            }
            _ => UnsupportedOdbcTypeSnafu {
                target_type: binding.target_type,
            }
            .fail(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdata_types::CDataType;
    use crate::conversion::traits::Binding;
    use odbc_sys as sql;

    fn binding_for_value<T>(
        target_type: CDataType,
        value: &mut T,
        str_len: &mut sql::Len,
    ) -> Binding {
        Binding {
            target_type,
            value: value as *mut T as sql::Pointer,
            buffer_length: 0,
            str_len_or_ind_ptr: str_len as *mut sql::Len,
        }
    }

    fn binding_for_char_buffer(
        target_type: CDataType,
        buffer: &mut [u8],
        str_len: &mut sql::Len,
    ) -> Binding {
        Binding {
            target_type,
            value: buffer.as_mut_ptr() as sql::Pointer,
            buffer_length: buffer.len() as sql::Len,
            str_len_or_ind_ptr: str_len as *mut sql::Len,
        }
    }

    fn make_decimal(scale: u32, precision: u32) -> SnowflakeNumber {
        SnowflakeNumber {
            scale,
            precision,
            sql_type: NumericSqlType::Decimal,
        }
    }

    #[test]
    fn decimal_default_c_type_is_char() {
        assert_eq!(NumericSqlType::Decimal.default_c_type(), CDataType::Char);
    }

    #[test]
    fn decimal_default_writes_integer_as_char() {
        let sn = make_decimal(0, 10);
        let mut buffer = vec![0u8; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Default, &mut buffer, &mut str_len);

        sn.write_odbc_type(42, &binding).unwrap();

        assert_eq!(str_len, 2);
        assert_eq!(&buffer[..2], b"42");
        assert_eq!(buffer[2], 0);
    }

    #[test]
    fn decimal_default_writes_scaled_value_as_char() {
        let sn = make_decimal(2, 10);
        let mut buffer = vec![0u8; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Default, &mut buffer, &mut str_len);

        sn.write_odbc_type(12345, &binding).unwrap();

        assert_eq!(str_len, 6);
        assert_eq!(&buffer[..6], b"123.45");
        assert_eq!(buffer[6], 0);
    }

    #[test]
    fn decimal_default_writes_negative_scaled_value_as_char() {
        let sn = make_decimal(3, 10);
        let mut buffer = vec![0u8; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Default, &mut buffer, &mut str_len);

        sn.write_odbc_type(-50, &binding).unwrap();

        assert_eq!(str_len, 6);
        assert_eq!(&buffer[..6], b"-0.050");
        assert_eq!(buffer[6], 0);
    }

    #[test]
    fn decimal_default_writes_zero_as_char() {
        let sn = make_decimal(0, 10);
        let mut buffer = vec![0u8; 32];
        let mut str_len: sql::Len = 0;
        let binding = binding_for_char_buffer(CDataType::Default, &mut buffer, &mut str_len);

        sn.write_odbc_type(0, &binding).unwrap();

        assert_eq!(str_len, 1);
        assert_eq!(&buffer[..1], b"0");
        assert_eq!(buffer[1], 0);
    }

    #[test]
    fn decimal_explicit_slong_writes_i32() {
        let sn = make_decimal(0, 10);
        let mut value: i32 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::SLong, &mut value, &mut str_len);

        sn.write_odbc_type(42, &binding).unwrap();

        assert_eq!(value, 42);
    }

    #[test]
    fn decimal_explicit_sbigint_writes_i64() {
        let sn = make_decimal(0, 10);
        let mut value: i64 = 0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::SBigInt, &mut value, &mut str_len);

        sn.write_odbc_type(123456789, &binding).unwrap();

        assert_eq!(value, 123456789i64);
    }

    #[test]
    fn decimal_explicit_double_writes_f64() {
        let sn = make_decimal(0, 10);
        let mut value: f64 = 0.0;
        let mut str_len: sql::Len = 0;
        let binding = binding_for_value(CDataType::Double, &mut value, &mut str_len);

        sn.write_odbc_type(42, &binding).unwrap();

        assert!((value - 42.0).abs() < f64::EPSILON);
    }
}
