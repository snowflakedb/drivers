use arrow::array::{ArrowPrimitiveType, PrimitiveArray};
use odbc_sys::Len;

use crate::cdata_types::CDataType;
use crate::conversion::traits::Binding;
use crate::conversion::{ConversionError, ReadArrowType, SnowflakeType, WriteODBCType};

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
    ) -> Result<Self::Representation<'a>, ConversionError> {
        let v: i128 = array.value(row_idx).into();
        Ok(v)
    }
}

impl WriteODBCType for SnowflakeNumber {
    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
    ) -> Result<(), ConversionError> {
        match binding.target_type {
            CDataType::Double => {
                let double_value: f64 = snowflake_value as f64 / 10f64.powi(self.scale as i32);
                unsafe {
                    std::ptr::write(binding.value as *mut f64, double_value);
                }
                Ok(())
            }
            CDataType::Float => {
                let float_value: f32 = snowflake_value as f32 / 10f32.powi(self.scale as i32);
                unsafe {
                    std::ptr::write(binding.value as *mut f32, float_value);
                }
                Ok(())
            }
            CDataType::Short | CDataType::SShort | CDataType::UShort => {
                let short_value = (snowflake_value as i64) / 10i64.pow(self.scale);
                unsafe {
                    std::ptr::write(binding.value as *mut i16, short_value as i16);
                }
                Ok(())
            }
            CDataType::TinyInt | CDataType::STinyInt | CDataType::UTinyInt => {
                let tinyint_value = (snowflake_value as i64) / 10i64.pow(self.scale);
                unsafe {
                    std::ptr::write(binding.value as *mut i8, tinyint_value as i8);
                }
                Ok(())
            }
            CDataType::Long | CDataType::SLong | CDataType::ULong => {
                let long_value = (snowflake_value as i32) / 10i32.pow(self.scale);
                unsafe {
                    std::ptr::write(binding.value as *mut i32, long_value);
                }
                Ok(())
            }
            CDataType::SBigInt | CDataType::UBigInt => {
                let int_value = (snowflake_value as i64) / 10i64.pow(self.scale);
                unsafe {
                    std::ptr::write(binding.value as *mut i64, int_value);
                }
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
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        binding.value as *mut u8,
                        std::cmp::min(binding.buffer_length as usize, bytes.len()),
                    );
                }
                Ok(())
            }
            _ => Err(ConversionError::UnsupportedOdbcType {
                target_type: binding.target_type,
                location: snafu::location!(),
            }),
        }
    }
}
