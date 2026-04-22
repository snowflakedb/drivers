use std::slice;

use arrow::array::{Array, GenericByteArray};
use arrow::datatypes::GenericBinaryType;
use serde_json::Value;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::error::JsonBindingError;
use crate::conversion::error::{ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError};
use crate::conversion::param_binding::buffer_data_len;
use crate::conversion::traits::Binding;
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, WriteJson};
use crate::conversion::warning::Warnings;
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};
use odbc_sys as sql;

pub(crate) struct SnowflakeBinary {
    pub len: u32,
}

impl SnowflakeType for SnowflakeBinary {
    type Representation<'a> = &'a [u8];
}

impl ReadArrowType<GenericByteArray<GenericBinaryType<i32>>> for SnowflakeBinary {
    fn read_arrow_type<'a>(
        &self,
        array: &'a GenericByteArray<GenericBinaryType<i32>>,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        if array.is_null(row_idx) {
            return Err(ReadArrowError::NullValue {
                location: snafu::location!(),
            });
        }
        Ok(array.value(row_idx))
    }
}

/// Convert a nibble (4-bit value) to its uppercase ASCII hex character
fn hex_digit_to_ascii(nibble: u8) -> u8 {
    let masked = nibble & 0xF;
    match masked {
        0..=9 => b'0' + masked,
        10..=15 => b'A' + (masked - 10),
        _ => unreachable!(),
    }
}

impl WriteODBCType for SnowflakeBinary {
    fn sql_type(&self) -> sql::SqlDataType {
        odbc_sys::SqlDataType::EXT_VAR_BINARY
    }

    fn column_size(&self) -> sql::ULen {
        self.len as sql::ULen
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
        match binding.target_type {
            CDataType::Default | CDataType::Binary => {
                Ok(binding.write_binary(snowflake_value, get_data_offset))
            }
            CDataType::Char => {
                let total_hex_len = (snowflake_value.len() * 2) as sql::Len;
                let converter = |pos: usize| {
                    let byte_idx = pos / 2;
                    let nibble_offset = pos % 2;

                    if byte_idx >= snowflake_value.len() {
                        return None;
                    }

                    let b = snowflake_value[byte_idx];
                    let hex_byte = if nibble_offset == 0 {
                        hex_digit_to_ascii(b >> 4)
                    } else {
                        hex_digit_to_ascii(b & 0x0F)
                    };
                    Some(hex_byte)
                };

                Ok(binding.write_char_from_fn(converter, total_hex_len, get_data_offset))
            }
            CDataType::WChar => {
                let total_hex_len = (snowflake_value.len() * 2) as sql::Len;
                let converter = |pos: usize| {
                    let byte_idx = pos / 2;
                    let nibble_offset = pos % 2;

                    if byte_idx >= snowflake_value.len() {
                        return None;
                    }

                    let b = snowflake_value[byte_idx];
                    let hex_byte = if nibble_offset == 0 {
                        hex_digit_to_ascii(b >> 4)
                    } else {
                        hex_digit_to_ascii(b & 0x0F)
                    };
                    Some(hex_byte as u16)
                };

                Ok(binding.write_wchar_from_fn(converter, total_hex_len, get_data_offset))
            }
            _ => UnsupportedOdbcTypeSnafu {
                target_type: binding.target_type,
            }
            .fail(),
        }
    }
}

impl ReadODBC for SnowflakeBinary {
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, JsonBindingError> {
        let len = buffer_data_len(binding);
        let bytes = unsafe { slice::from_raw_parts(binding.parameter_value_ptr as *const u8, len) };
        Ok(bytes)
    }
}

/// Hex-encode a byte slice as a lowercase string (e.g. `[0xDE, 0xAD]` → `"dead"`).
pub(crate) fn hex_encode_lowercase(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

impl WriteJson for SnowflakeBinary {
    fn write_json(&self, value: Self::Representation<'_>) -> Result<Value, JsonBindingError> {
        Ok(Value::String(hex_encode_lowercase(value)))
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::Binary
    }
}
