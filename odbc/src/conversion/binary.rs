use std::borrow::Cow;
use std::slice;

use arrow::array::GenericByteArray;
use arrow::datatypes::GenericBinaryType;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::batch::{CHAR_SCRATCH_LEN, CharKernel};
use crate::conversion::error::BindingError;
use crate::conversion::error::{
    ConversionError, InvalidHexLiteralSnafu, NumericValueOutOfRangeSnafu, ReadArrowError,
    UnsupportedCDataTypeSnafu, UnsupportedOdbcTypeSnafu, WriteOdbcError, WriteOdbcValueSnafu,
};
use crate::conversion::param_binding::{buffer_data_len, read_char_str, read_wchar_str};
use crate::conversion::traits::{Binding, LengthOrNull};
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, WriteWire};
use crate::conversion::warning::Warnings;
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};
use odbc_sys as sql;
use snafu::ResultExt;

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
        Ok(Cow::Borrowed(sf_types::ReadArrowType::read_arrow_type(
            &sf_types::SnowflakeBinary,
            array,
            row_idx,
        )?))
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

fn hex_decode_ascii(input: &str) -> Result<Vec<u8>, BindingError> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i + 1 < bytes.len() {
        let hi = hex_nibble(bytes[i])?;
        let lo = hex_nibble(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    if i < bytes.len() {
        hex_nibble(bytes[i])?;
    }
    Ok(out)
}

fn hex_nibble(c: u8) -> Result<u8, BindingError> {
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
        odbc_sys::SqlDataType::EXT_BINARY
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
                    Some(hex_byte as crate::api::encoding::WideChar)
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

/// Batched `SQL_C_CHAR` kernel for BINARY, reused by the generic
/// [`convert_char_range`](crate::conversion::batch::convert_char_range) loop.
/// Hex output has no fixed bound, so `write_non_null` writes hex nibbles
/// directly into the bind buffer instead of going through `format_into` +
/// the fixed-size shared scratch.
pub(crate) struct BinaryCharKernel;

impl CharKernel for BinaryCharKernel {
    type Array = GenericByteArray<GenericBinaryType<i32>>;
    type Value = Vec<u8>;

    fn read_validate(
        &self,
        array: &GenericByteArray<GenericBinaryType<i32>>,
        idx: usize,
    ) -> Result<Vec<u8>, ConversionError> {
        Ok(array.value(idx).to_vec())
    }

    fn format_into<'s>(
        &self,
        value: &Vec<u8>,
        _binding: &Binding,
        scratch: &'s mut [u8; CHAR_SCRATCH_LEN],
    ) -> Result<&'s str, WriteOdbcError> {
        let copy_len = value.len() * 2;
        if copy_len > CHAR_SCRATCH_LEN {
            return NumericValueOutOfRangeSnafu {
                reason: format!(
                    "BINARY value needs {copy_len} hex chars, exceeds {CHAR_SCRATCH_LEN}-byte scratch buffer"
                ),
            }
            .fail();
        }
        let complete_bytes = copy_len / 2;
        for (src, dst) in value[..complete_bytes]
            .iter()
            .zip(scratch[..copy_len].chunks_exact_mut(2))
        {
            dst[0] = hex_digit_to_ascii(src >> 4);
            dst[1] = hex_digit_to_ascii(*src);
        }
        if !copy_len.is_multiple_of(2) {
            scratch[copy_len - 1] = hex_digit_to_ascii(value[complete_bytes] >> 4);
        }
        // SAFETY: only ASCII hex digits were written into `scratch[..copy_len]` above.
        Ok(unsafe { std::str::from_utf8_unchecked(&scratch[..copy_len]) })
    }

    fn write_non_null(
        &self,
        array: &GenericByteArray<GenericBinaryType<i32>>,
        idx: usize,
        binding: &Binding,
        _scratch: &mut [u8; CHAR_SCRATCH_LEN],
    ) -> Result<bool, ConversionError> {
        let value = array.value(idx);
        let total_len = value.len() * 2;

        if binding.target_value_ptr.is_null() || binding.buffer_length <= 0 {
            binding
                .write_length_or_null(LengthOrNull::Length(total_len as sql::Len))
                .context(WriteOdbcValueSnafu)?;
            return Ok(total_len > 0);
        }

        let max_payload = binding.buffer_length as usize - 1;
        let copy_len = total_len.min(max_payload);
        unsafe {
            let target =
                std::slice::from_raw_parts_mut(binding.target_value_ptr as *mut u8, copy_len + 1);
            let complete_bytes = copy_len / 2;
            for (src, dst) in value[..complete_bytes]
                .iter()
                .zip(target[..complete_bytes * 2].chunks_exact_mut(2))
            {
                dst[0] = hex_digit_to_ascii(src >> 4);
                dst[1] = hex_digit_to_ascii(*src);
            }
            if !copy_len.is_multiple_of(2) {
                target[copy_len - 1] = hex_digit_to_ascii(value[complete_bytes] >> 4);
            }
            target[copy_len] = 0;
        }

        binding
            .write_length_or_null(LengthOrNull::Length(total_len as sql::Len))
            .context(WriteOdbcValueSnafu)?;
        Ok(copy_len < total_len)
    }
}

impl ReadODBC for SnowflakeBinary {
    /// Read a `SQLBindParameter` value bound against a `SQL_BINARY` /
    /// `SQL_VARBINARY` / `SQL_LONGVARBINARY` target.
    ///
    /// Legal C source types for a binary SQL target:
    ///
    /// - `SQL_C_BINARY` (and `SQL_C_DEFAULT`, which the driver maps to
    ///   `SQL_C_BINARY` for binary targets) — bytes are taken verbatim
    ///   from the application's buffer.
    /// - `SQL_C_CHAR` — the buffer is an ASCII hex literal (e.g.
    ///   `"DEADBEEF"`); the driver decodes pairs of hex digits into
    ///   raw bytes.
    /// - `SQL_C_WCHAR` — same as `SQL_C_CHAR` after wide → UTF-8
    ///   transcode (`read_wchar_str`).
    ///
    /// Every other C type — numerics, dates, intervals, GUID, …  — is
    /// rejected with SQLSTATE 07006 ("restricted data type attribute
    /// violation").
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, BindingError> {
        match binding.value_type {
            CDataType::Default | CDataType::Binary => {
                let len = buffer_data_len(binding);
                let bytes =
                    unsafe { slice::from_raw_parts(binding.parameter_value_ptr as *const u8, len) };
                Ok(Cow::Borrowed(bytes))
            }
            CDataType::Char => {
                let s = read_char_str(binding)?;
                Ok(Cow::Owned(hex_decode_ascii(&s)?))
            }
            CDataType::WChar => {
                let s = read_wchar_str(binding)?;
                Ok(Cow::Owned(hex_decode_ascii(&s)?))
            }
            other => UnsupportedCDataTypeSnafu { c_type: other }.fail(),
        }
    }
}

/// Hex-encode a byte slice as a lowercase string (e.g. `[0xDE, 0xAD]` → `"dead"`).
pub(crate) fn hex_encode_lowercase(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

impl WriteWire for SnowflakeBinary {
    fn write_wire(&self, value: Self::Representation<'_>) -> Result<String, BindingError> {
        Ok(hex_encode_lowercase(value.as_ref()))
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::Binary
    }
}
