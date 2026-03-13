use arrow::array::{Array, GenericByteArray, StructArray};
use arrow::datatypes::{GenericBinaryType, Int16Type};
use odbc_sys as sql;

use crate::api::CDataType;
use crate::conversion::error::{
    InvalidArrowValueSnafu, ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError,
};
use crate::conversion::traits::Binding;
use crate::conversion::warning::Warnings;
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

pub(crate) struct SnowflakeDecfloat {
    pub precision: u32,
}

impl SnowflakeType for SnowflakeDecfloat {
    type Representation<'a> = (i128, i16);
}

/// Converts a big-endian two's complement byte slice (1–16 bytes) into an i128.
/// The Arrow wire format trims leading bytes, so we sign-extend to 16 bytes
/// before calling `i128::from_be_bytes`. Empty input is treated as zero.
pub(crate) fn i128_from_big_endian_signed(bytes: &[u8]) -> Result<i128, ReadArrowError> {
    if bytes.is_empty() {
        return Ok(0);
    }
    if bytes.len() > 16 {
        return InvalidArrowValueSnafu {
            reason: format!(
                "significand byte length {} exceeds maximum of 16",
                bytes.len()
            ),
        }
        .fail();
    }
    let sign_bytes = if bytes[0] & 0x80 != 0 { 0xFF } else { 0x00 };
    let mut buf = [sign_bytes; 16];
    buf[16 - bytes.len()..].copy_from_slice(bytes);
    Ok(i128::from_be_bytes(buf))
}

impl ReadArrowType<StructArray> for SnowflakeDecfloat {
    fn read_arrow_type<'a>(
        &self,
        array: &'a StructArray,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        if array.is_null(row_idx) {
            return Err(ReadArrowError::NullValue {
                location: snafu::location!(),
            });
        }

        let exponent_array = array
            .column_by_name("exponent")
            .ok_or_else(|| ReadArrowError::InvalidArrowValue {
                reason: "DECFLOAT struct missing 'exponent' field".to_string(),
                location: snafu::location!(),
            })?
            .as_any()
            .downcast_ref::<arrow::array::PrimitiveArray<Int16Type>>()
            .ok_or_else(|| ReadArrowError::InvalidArrowValue {
                reason: "DECFLOAT 'exponent' field is not Int16".to_string(),
                location: snafu::location!(),
            })?;

        let significand_array = array
            .column_by_name("significand")
            .ok_or_else(|| ReadArrowError::InvalidArrowValue {
                reason: "DECFLOAT struct missing 'significand' field".to_string(),
                location: snafu::location!(),
            })?
            .as_any()
            .downcast_ref::<GenericByteArray<GenericBinaryType<i32>>>()
            .ok_or_else(|| ReadArrowError::InvalidArrowValue {
                reason: "DECFLOAT 'significand' field is not Binary".to_string(),
                location: snafu::location!(),
            })?;

        let exponent = exponent_array.value(row_idx);
        let significand = i128_from_big_endian_signed(significand_array.value(row_idx))?;

        Ok((significand, exponent))
    }
}

/// Formats a DECFLOAT value as a string.
///
/// Uses plain decimal notation when the result fits within 38 characters
/// (unsigned), and normalized scientific notation otherwise. Scientific
/// notation uses lowercase 'e' with no '+' on positive exponents.
pub(crate) fn format_decfloat(sig: i128, exp: i16, max_plain_digits: usize) -> String {
    if sig == 0 {
        return "0".to_string();
    }

    let is_negative = sig < 0;
    let mut abs_sig = sig.unsigned_abs();
    let mut exp = exp as i64;

    // Normalize: strip trailing zeros from significand
    while abs_sig % 10 == 0 {
        abs_sig /= 10;
        exp += 1;
    }

    let digits = abs_sig.to_string();
    let n = digits.len();

    let plain_len = if exp >= 0 {
        n + exp as usize
    } else {
        let abs_exp = (-exp) as usize;
        if abs_exp < n {
            n + 1 // decimal point within digits
        } else {
            2 + abs_exp // "0." prefix + leading zeros + digits
        }
    };

    let mut result = if plain_len <= max_plain_digits {
        if exp >= 0 {
            let mut s = digits;
            for _ in 0..exp {
                s.push('0');
            }
            s
        } else {
            let abs_exp = (-exp) as usize;
            if abs_exp < n {
                let decimal_pos = n - abs_exp;
                let mut s = String::with_capacity(n + 1);
                s.push_str(&digits[..decimal_pos]);
                s.push('.');
                s.push_str(&digits[decimal_pos..]);
                s
            } else {
                let leading_zeros = abs_exp - n;
                let mut s = String::with_capacity(2 + abs_exp);
                s.push_str("0.");
                for _ in 0..leading_zeros {
                    s.push('0');
                }
                s.push_str(&digits);
                s
            }
        }
    } else {
        // Scientific notation
        let adjusted_exp = exp + (n as i64) - 1;
        let mut s = String::new();
        s.push_str(&digits[0..1]);
        if n > 1 {
            s.push('.');
            s.push_str(&digits[1..]);
        }
        s.push('e');
        s.push_str(&adjusted_exp.to_string());
        s
    };

    if is_negative {
        result.insert(0, '-');
    }
    result
}

impl WriteODBCType for SnowflakeDecfloat {
    fn sql_type(&self) -> sql::SqlDataType {
        sql::SqlDataType::NUMERIC
    }

    fn column_size(&self) -> sql::ULen {
        self.precision as sql::ULen
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
        let (significand, exponent) = snowflake_value;
        let target_type = match binding.target_type {
            CDataType::Default => CDataType::Char,
            other => other,
        };

        match target_type {
            CDataType::Char => {
                let decfloat = format_decfloat(significand, exponent, self.precision as usize);
                Ok(binding.write_char_string(&decfloat, get_data_offset))
            }
            CDataType::WChar => {
                let decfloat = format_decfloat(significand, exponent, self.precision as usize);
                Ok(binding.write_wchar_string(&decfloat, get_data_offset))
            }
            _ => UnsupportedOdbcTypeSnafu { target_type }.fail(),
        }
    }
}
