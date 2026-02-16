use arrow::array::{Array, ArrowPrimitiveType, PrimitiveArray};
use odbc_sys as sql;
use odbc_sys::Len;

use crate::cdata_types::CDataType;
use crate::conversion::error::{
    InvalidValueSnafu, NumericValueOutOfRangeSnafu, ReadArrowError, UnsupportedOdbcTypeSnafu,
    WriteOdbcError,
};
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
    fn sql_type(&self) -> sql::SqlDataType {
        sql::SqlDataType::DECIMAL
    }

    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
        _get_data_offset: &mut Option<usize>,
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
            CDataType::Short | CDataType::SShort => {
                let scaled = (snowflake_value as i64) / 10i64.pow(self.scale);
                let value = i16::try_from(scaled).map_err(|_| {
                    NumericValueOutOfRangeSnafu {
                        reason: format!("{scaled} out of range for SQL_C_SHORT"),
                    }
                    .build()
                })?;
                binding.write_fixed(value);
                Ok(vec![])
            }
            CDataType::UShort => {
                let scaled = (snowflake_value as i64) / 10i64.pow(self.scale);
                let value = u16::try_from(scaled).map_err(|_| {
                    NumericValueOutOfRangeSnafu {
                        reason: format!("{scaled} out of range for SQL_C_USHORT"),
                    }
                    .build()
                })?;
                binding.write_fixed(value);
                Ok(vec![])
            }
            CDataType::TinyInt | CDataType::STinyInt => {
                let scaled = (snowflake_value as i64) / 10i64.pow(self.scale);
                let value = i8::try_from(scaled).map_err(|_| {
                    NumericValueOutOfRangeSnafu {
                        reason: format!("{scaled} out of range for SQL_C_TINYINT"),
                    }
                    .build()
                })?;
                binding.write_fixed(value);
                Ok(vec![])
            }
            CDataType::UTinyInt => {
                let scaled = (snowflake_value as i64) / 10i64.pow(self.scale);
                let value = u8::try_from(scaled).map_err(|_| {
                    NumericValueOutOfRangeSnafu {
                        reason: format!("{scaled} out of range for SQL_C_UTINYINT"),
                    }
                    .build()
                })?;
                binding.write_fixed(value);
                Ok(vec![])
            }
            CDataType::Long | CDataType::SLong => {
                let scaled = (snowflake_value as i64) / 10i64.pow(self.scale);
                let value = i32::try_from(scaled).map_err(|_| {
                    NumericValueOutOfRangeSnafu {
                        reason: format!("{scaled} out of range for SQL_C_LONG"),
                    }
                    .build()
                })?;
                binding.write_fixed(value);
                Ok(vec![])
            }
            CDataType::ULong => {
                let scaled = (snowflake_value as i64) / 10i64.pow(self.scale);
                let value = u32::try_from(scaled).map_err(|_| {
                    NumericValueOutOfRangeSnafu {
                        reason: format!("{scaled} out of range for SQL_C_ULONG"),
                    }
                    .build()
                })?;
                binding.write_fixed(value);
                Ok(vec![])
            }
            CDataType::SBigInt => {
                let value = (snowflake_value as i64) / 10i64.pow(self.scale);
                binding.write_fixed(value);
                Ok(vec![])
            }
            CDataType::UBigInt => {
                let scaled = (snowflake_value as u128) / 10u128.pow(self.scale);
                let value = u64::try_from(scaled).map_err(|_| {
                    NumericValueOutOfRangeSnafu {
                        reason: format!("{scaled} out of range for SQL_C_UBIGINT"),
                    }
                    .build()
                })?;
                binding.write_fixed(value);
                Ok(vec![])
            }
            CDataType::Numeric => {
                // The application must set SQL_DESC_PRECISION and SQL_DESC_SCALE on the ARD
                // via SQLSetDescField before fetching SQL_C_NUMERIC data.
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

                // Rescale: convert from Snowflake scale to target scale.
                // unscaled_target = abs_value * 10^(target_scale - snowflake_scale)
                let scale_diff = target_scale as i32 - self.scale as i32;
                let unscaled: u128 = if scale_diff >= 0 {
                    abs_value * 10u128.pow(scale_diff as u32)
                } else {
                    abs_value / 10u128.pow((-scale_diff) as u32)
                };

                // Build the SQL_NUMERIC_STRUCT (odbc_sys::Numeric)
                let numeric = sql::Numeric {
                    precision: target_precision as u8,
                    scale: target_scale as i8,
                    sign: if is_negative { 0 } else { 1 },
                    val: unscaled.to_le_bytes(),
                };

                binding.write_fixed(numeric);
                Ok(vec![])
            }
            CDataType::Default | CDataType::Char => {
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
