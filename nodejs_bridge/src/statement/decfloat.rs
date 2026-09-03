//! DECFLOAT presentation.
//!
//! The Arrow decode — struct → `(significand, exponent)` — lives in
//! `sf_types::SnowflakeDecfloat`. `format_decfloat` stays here because the
//! scientific-notation threshold differs from ODBC's: this version switches on
//! the adjusted exponent's magnitude, matching the text the old driver returns,
//! whereas ODBC switches on the rendered plain-form *length*.

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
