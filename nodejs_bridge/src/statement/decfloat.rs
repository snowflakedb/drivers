//! DECFLOAT decoding.
//!
//! `format_decfloat` and `i128_from_big_endian_signed` started as copies of
//! `odbc/src/conversion/decfloat.rs` but this version differs:
//! ODBC switches to scientific notation on the rendered *length*, which emits
//! a stray `e0` for 38-digit values and picks scientific for `1e-37` where
//! the server sends plain. ODBC is likely affected by both.
//!
//! TODO: refactor into shared code.
//!

use arrow::array::{Array, StructArray};

pub(super) fn decfloat_field<T: Array + Clone + 'static>(
    array: &StructArray,
    name: &str,
) -> Result<T, String> {
    let child = array
        .column_by_name(name)
        .ok_or_else(|| format!("DECFLOAT struct is missing the {name:?} field"))?;
    child.as_any().downcast_ref::<T>().cloned().ok_or_else(|| {
        format!(
            "DECFLOAT {name:?} field could not be downcast; it is {}",
            child.data_type()
        )
    })
}

/// Converts a big-endian two's complement byte slice (1–16 bytes) into an i128.
/// The Arrow wire format trims leading bytes, so we sign-extend to 16 bytes
/// before calling `i128::from_be_bytes`. Empty input is treated as zero.
pub(super) fn i128_from_big_endian_signed(bytes: &[u8]) -> Result<i128, String> {
    if bytes.is_empty() {
        return Ok(0);
    }
    if bytes.len() > 16 {
        return Err(format!(
            "significand byte length {} exceeds maximum of 16",
            bytes.len()
        ));
    }
    let sign_bytes = if bytes[0] & 0x80 != 0 { 0xFF } else { 0x00 };
    let mut buf = [sign_bytes; 16];
    buf[16 - bytes.len()..].copy_from_slice(bytes);
    Ok(i128::from_be_bytes(buf))
}

/// Formats a DECFLOAT value as a string.
///
/// Uses plain decimal notation while the adjusted exponent stays within
/// `max_plain_digits`, and normalized scientific notation otherwise. Scientific
/// notation uses lowercase 'e' with no '+' on positive exponents.
///
/// The threshold is undocumented; it was derived by comparing ~50 values against
/// the old driver, which returns the server's text verbatim.
pub(super) fn format_decfloat(sig: i128, exp: i16, max_plain_digits: usize) -> String {
    if sig == 0 {
        return "0".to_string();
    }

    let is_negative = sig < 0;
    let mut abs_sig = sig.unsigned_abs();
    let mut exp = exp as i64;

    // Normalize: strip trailing zeros from significand
    while abs_sig.is_multiple_of(10) {
        abs_sig /= 10;
        exp += 1;
    }

    let digits = abs_sig.to_string();
    let n = digits.len();
    let adjusted_exp = exp + (n as i64) - 1;
    let exponent_magnitude = adjusted_exp.unsigned_abs() as usize;

    let mut result = if exponent_magnitude < max_plain_digits {
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
