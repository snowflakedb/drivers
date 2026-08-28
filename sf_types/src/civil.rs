//! Calendar primitives shared across front ends.
//!
//! These are the pure, allocation-free integer kernels that sit *below* the
//! [`crate::ReadArrowType`] materializers. A materializer turns an Arrow cell
//! into a chrono value (the ergonomic, checked representation most callers
//! want); a front end with a materialization-free hot path — ODBC's bulk
//! `SQL_C_CHAR` fetch — instead wants the broken-down integer fields directly,
//! without ever building a chrono value. Both sit on the same primitive here,
//! so the calendar math has one home rather than one copy per front end.

/// Convert a day offset from the Unix epoch (an Arrow `Date32`) to a Gregorian
/// `(year, month, day)` using integer arithmetic only.
///
/// This is Howard Hinnant's `civil_from_days` algorithm, shifted so day zero is
/// 1970-01-01. It is total over `i32` (no panics, no allocation) and, across
/// the SQL `0001-01-01..=9999-12-31` range, byte-identical to
/// `NaiveDate(1970,1,1) + Duration::days(n)` — see the exhaustive test below.
///
/// The result is *unchecked* calendar fields: for day offsets whose year falls
/// outside what a given front end can represent, it is the caller's job to
/// reject or clamp (e.g. [`crate::SnowflakeDate`] feeds these into
/// `NaiveDate::from_ymd_opt`, which returns `None` beyond chrono's range).
#[inline]
pub fn civil_from_unix_days(days: i32) -> (i32, u32, u32) {
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year as i32, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The load-bearing guarantee: across the full SQL calendar range the
    /// server can send, the integer kernel agrees exactly with chrono's
    /// epoch-relative arithmetic. This is the same range and oracle the ODBC
    /// hot path proves itself against, so both front ends share one contract.
    #[test]
    fn should_match_chrono_across_the_sql_calendar_range() {
        use chrono::{Datelike, NaiveDate};

        let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        // -719_162 = 0001-01-01, 2_932_896 = 9999-12-31, as day offsets.
        for days in -719_162..=2_932_896 {
            let expected = epoch + chrono::Duration::days(days as i64);
            assert_eq!(
                civil_from_unix_days(days),
                (expected.year(), expected.month(), expected.day()),
                "day offset {days}"
            );
        }
    }

    #[test]
    fn should_decode_the_unix_epoch_at_day_zero() {
        assert_eq!(civil_from_unix_days(0), (1970, 1, 1));
    }

    #[test]
    fn should_decode_the_sql_range_boundaries() {
        assert_eq!(civil_from_unix_days(-719_162), (1, 1, 1));
        assert_eq!(civil_from_unix_days(2_932_896), (9999, 12, 31));
    }
}
