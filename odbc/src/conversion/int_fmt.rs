//! Fast fixed-width decimal formatting for the SQL_C_CHAR conversion hot path.
//!
//! `core::fmt` (`write!(buf, "{:04}-{:02}…")`) was the dominant per-cell cost
//! when rendering temporal values to `SQL_C_CHAR`. These helpers write digits
//! directly into a caller-provided byte buffer with no `Formatter`, no
//! `Arguments`, and no padding machinery, and are unit-proven byte-identical
//! to the `{:0N}` specs they replace.

/// Number of bytes [`put_year`] will write for `year` — an optional `-` plus
/// the magnitude zero-padded to at least 4 digits (matching `"{:04}"`).
#[inline]
pub(crate) fn year_width(year: i32) -> usize {
    let mag = year.unsigned_abs();
    let digits = if mag == 0 {
        1
    } else {
        mag.ilog10() as usize + 1
    };
    (digits + (year < 0) as usize).max(4)
}

/// Write `value` as exactly `width` zero-padded ASCII decimal digits into
/// `buf[pos..pos + width]`; returns `pos + width`.
///
/// The caller guarantees `value < 10^width` and that the slice has room. Both
/// hold for the bounded calendar fields it is used for (month/day/hour/minute/
/// second → width 2; sub-second fraction → width 9; tz offset h/m → width 2).
#[inline]
pub(crate) fn put_padded(buf: &mut [u8], pos: usize, mut value: u32, width: usize) -> usize {
    let end = pos + width;
    let mut i = end;
    while i > pos {
        i -= 1;
        buf[i] = b'0' + (value % 10) as u8;
        value /= 10;
    }
    end
}

/// Write a calendar `year`, byte-identical to `core::fmt`'s `"{:04}"` for any
/// `i32`: an optional leading `-`, then the magnitude zero-padded so the total
/// width (sign included) is at least 4. Returns the new position.
#[inline]
pub(crate) fn put_year(buf: &mut [u8], pos: usize, year: i32) -> usize {
    let neg = year < 0;
    let mut m = year.unsigned_abs();
    let sign = neg as usize;
    let end = pos + year_width(year);
    // Write digits right-to-left into the `sign..` region. Once `m` is
    // exhausted it stays 0, so the remaining leading positions are naturally
    // zero-padded to the minimum width — byte-identical to `{:04}`.
    for i in 1..=(end - pos - sign) {
        buf[end - i] = b'0' + (m % 10) as u8;
        m /= 10;
    }
    if neg {
        buf[pos] = b'-';
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_padded_matches_core_fmt_width2() {
        for v in 0u32..=99 {
            let mut buf = [0u8; 2];
            let end = put_padded(&mut buf, 0, v, 2);
            assert_eq!(end, 2);
            assert_eq!(std::str::from_utf8(&buf).unwrap(), format!("{v:02}"));
        }
    }

    #[test]
    fn put_padded_matches_core_fmt_width9() {
        for v in [0u32, 1, 9, 123, 999_999_999, 100_000_000, 123_456_789] {
            let mut buf = [0u8; 9];
            put_padded(&mut buf, 0, v, 9);
            assert_eq!(std::str::from_utf8(&buf).unwrap(), format!("{v:09}"));
        }
    }

    #[test]
    fn put_year_matches_core_fmt() {
        // Cover the SQL range, the chrono extremes, zero, and negatives —
        // put_year must be byte-identical to "{:04}" across all of them.
        let cases = [
            0,
            1,
            9,
            99,
            999,
            1000,
            1970,
            9999,
            10_000,
            262_143,
            -1,
            -44,
            -9999,
            -262_144,
            // i32 extremes: `unsigned_abs` avoids the `abs()` panic at MIN.
            i32::MIN,
            i32::MAX,
        ];
        for y in cases {
            let mut buf = [0u8; 16];
            let end = put_year(&mut buf, 0, y);
            assert_eq!(year_width(y), end, "year_width disagrees for {y}");
            assert_eq!(std::str::from_utf8(&buf[..end]).unwrap(), format!("{y:04}"));
        }
    }

    #[test]
    fn put_padded_writes_at_offset() {
        let mut buf = [b'_'; 6];
        let end = put_padded(&mut buf, 2, 7, 2);
        assert_eq!(end, 4);
        assert_eq!(&buf, b"__07__");
    }
}
