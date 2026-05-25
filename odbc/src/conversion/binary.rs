use std::borrow::Cow;
use std::slice;

use arrow::array::{Array, GenericByteArray};
use arrow::datatypes::GenericBinaryType;
use serde_json::Value;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::error::JsonBindingError;
use crate::conversion::error::{
    InvalidHexLiteralSnafu, ReadArrowError, UnsupportedCDataTypeSnafu, UnsupportedOdbcTypeSnafu,
    WriteOdbcError,
};
use crate::conversion::param_binding::{buffer_data_len, read_char_str, read_wchar_str};
use crate::conversion::traits::Binding;
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, WriteJson};
use crate::conversion::warning::Warnings;
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};
use odbc_sys as sql;

pub(crate) struct SnowflakeBinary {
    pub len: u32,
}

impl SnowflakeType for SnowflakeBinary {
    /// `Cow` so that the fetch / Arrow path returns a borrowed slice into
    /// the Arrow buffer (zero-copy), while the bind path can return an
    /// owned `Vec<u8>` for the `SQL_C_CHAR` / `SQL_C_WCHAR` cases that
    /// require hex-decoding (where the result is shorter than the input
    /// buffer and therefore cannot be borrowed from it).
    type Representation<'a> = Cow<'a, [u8]>;
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
        Ok(Cow::Borrowed(array.value(row_idx)))
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

/// Decode an ASCII hex literal (e.g. `"DEADBEEF"`) into the raw bytes
/// it represents. Per ODBC Appendix D ("Converting Data from C to SQL
/// Data Types"), a `SQL_C_CHAR` / `SQL_C_WCHAR` source bound to
/// `SQL_BINARY` / `SQL_VARBINARY` / `SQL_LONGVARBINARY` must be a
/// hex string with each pair of characters representing one byte.
/// Whitespace is *not* tolerated (the spec grammar admits no
/// separators), and odd-length / non-hex input must surface as
/// SQLSTATE 22018.
fn hex_decode_ascii(input: &str) -> Result<Vec<u8>, JsonBindingError> {
    if !input.len().is_multiple_of(2) {
        return InvalidHexLiteralSnafu {
            reason: format!(
                "hex literal must contain an even number of digits (got {} chars)",
                input.len()
            ),
        }
        .fail();
    }
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn hex_nibble(c: u8) -> Result<u8, JsonBindingError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => InvalidHexLiteralSnafu {
            reason: format!("'{}' is not a valid hex digit", c as char),
        }
        .fail(),
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
        let snowflake_value: &[u8] = snowflake_value.as_ref();
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
    /// Read a `SQLBindParameter` value bound against a `SQL_BINARY` /
    /// `SQL_VARBINARY` / `SQL_LONGVARBINARY` target.
    ///
    /// Per ODBC Appendix D ("Converting Data from C to SQL Data Types",
    /// section "Binary"), the only legal C source types here are:
    ///
    /// - `SQL_C_BINARY` (and `SQL_C_DEFAULT`, which the driver maps to
    ///   `SQL_C_BINARY` for binary targets) — bytes are taken verbatim
    ///   from the application's buffer.
    /// - `SQL_C_CHAR` — the buffer is an ASCII hex literal (e.g.
    ///   `"DEADBEEF"`); the driver decodes pairs of hex digits into
    ///   raw bytes.
    /// - `SQL_C_WCHAR` — same as `SQL_C_CHAR`, but the buffer is
    ///   UTF-16. The driver transcodes to UTF-8 first (`read_wchar_str`)
    ///   and then hex-decodes, so input like `"DEADBEEF"` produces the
    ///   same 4 bytes regardless of source encoding.
    ///
    /// Every other C type — numerics, dates, intervals, GUID, …  — must
    /// be rejected with SQLSTATE 07006 ("restricted data type
    /// attribute violation"). The legacy 3.16.0 driver and all
    /// well-behaved spec-conforming drivers do this; without it the
    /// driver would silently mangle (e.g.) a `SQL_C_LONG` source's
    /// little-endian representation into the BINARY column rather than
    /// raise an error the application can react to.
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, JsonBindingError> {
        match binding.value_type {
            CDataType::Default | CDataType::Binary => {
                let len = buffer_data_len(binding);
                let bytes =
                    unsafe { slice::from_raw_parts(binding.parameter_value_ptr as *const u8, len) };
                Ok(Cow::Borrowed(bytes))
            }
            CDataType::Char => {
                let s = read_char_str(binding)?;
                let bytes = hex_decode_ascii(&s)?;
                Ok(Cow::Owned(bytes))
            }
            CDataType::WChar => {
                let s = read_wchar_str(binding)?;
                let bytes = hex_decode_ascii(&s)?;
                Ok(Cow::Owned(bytes))
            }
            other => UnsupportedCDataTypeSnafu { c_type: other }.fail(),
        }
    }
}

/// Hex-encode a byte slice as a lowercase string (e.g. `[0xDE, 0xAD]` → `"dead"`).
pub(crate) fn hex_encode_lowercase(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

impl WriteJson for SnowflakeBinary {
    fn write_json(&self, value: Self::Representation<'_>) -> Result<Value, JsonBindingError> {
        Ok(Value::String(hex_encode_lowercase(value.as_ref())))
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::Binary
    }
}
