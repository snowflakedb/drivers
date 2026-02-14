use arrow::array::{Array, ArrowPrimitiveType, PrimitiveArray};
use odbc_sys::Len;

use crate::cdata_types::CDataType;
use crate::conversion::error::{ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError};
use crate::conversion::traits::Binding;
use crate::conversion::warning::Warnings;
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

pub(crate) struct SnowflakeNumber {
    pub(crate) scale: u32,
    #[allow(dead_code)]
    pub(crate) precision: u32,
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

impl WriteODBCType for SnowflakeNumber {
    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
    ) -> Result<Warnings, WriteOdbcError> {
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
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        binding.target_value_ptr as *mut u8,
                        std::cmp::min(binding.buffer_length as usize, bytes.len()),
                    );
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
