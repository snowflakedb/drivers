//! Clock primitives shared across front ends.
//!
//! Like [`crate::civil`] for the calendar, this is the pure, allocation-free
//! integer kernel that sits *below* the [`crate::ReadArrowType`] TIME
//! materializer. A materializer turns an Arrow cell into a chrono `NaiveTime`
//! (the ergonomic, checked value most callers want); a front end with a
//! materialization-free hot path — ODBC's bulk `SQL_C_CHAR` fetch — instead
//! wants the broken-down `(seconds, nanoseconds)` fields directly, without ever
//! building a chrono value. Both sit on the same primitive here, so the clock
//! math has one home rather than one copy per front end.

/// Split a Snowflake TIME integer into `(seconds_of_day, nanoseconds)`.
///
/// The server encodes TIME as `seconds_since_midnight * 10^scale + fraction`.
/// `scale` (0..=9) is column metadata, so the raw integer is not a time of day
/// by itself. The fraction is widened to nanoseconds (`fraction * 10^(9-scale)`).
///
/// Returns `None` when `scale > 9`, `raw` is negative, or the second-of-day is
/// ≥ 86_400, so this stays total over `i64 × u32` (no panic, no `u32` overflow)
/// instead of assuming the server never sends garbage.
#[inline]
pub fn split_time_raw(raw: i64, scale: u32) -> Option<(u32, u32)> {
    if scale > 9 || raw < 0 {
        return None;
    }
    let divisor = 10i64.pow(scale);
    let secs = raw / divisor;
    if secs >= 86_400 {
        return None;
    }
    // `frac < divisor = 10^scale` and the multiplier is `10^(9-scale)`, so the
    // product is `< 10^9` and fits a `u32` for every scale in 0..=9.
    let frac = (raw % divisor) as u32;
    let nanos = frac * 10u32.pow(9 - scale);
    Some((secs as u32, nanos))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_split_whole_seconds_at_scale_0() {
        // 12:34:56 = 45_296 seconds, no fraction.
        assert_eq!(split_time_raw(45_296, 0), Some((45_296, 0)));
    }

    #[test]
    fn should_widen_fraction_to_nanoseconds() {
        // scale 3 → milliseconds; .789 widens to 789_000_000 ns.
        assert_eq!(split_time_raw(45_296_789, 3), Some((45_296, 789_000_000)));
        // scale 9 → the fraction is already nanoseconds.
        assert_eq!(
            split_time_raw(45_296_123_456_789, 9),
            Some((45_296, 123_456_789))
        );
    }

    #[test]
    fn should_split_midnight_and_last_second_of_day() {
        assert_eq!(split_time_raw(0, 9), Some((0, 0)));
        // 23:59:59 at scale 0 is the largest whole second of a day.
        assert_eq!(split_time_raw(86_399, 0), Some((86_399, 0)));
    }

    #[test]
    fn should_split_largest_int32_backed_value() {
        // The largest value Snowflake stores in an Int32 TIME column: scale=4,
        // time = 23:59:59.9999 → 86_399 * 10_000 + 9_999 = 863_999_999.
        assert_eq!(split_time_raw(863_999_999, 4), Some((86_399, 999_900_000)));
    }

    #[test]
    fn should_reject_scale_above_9() {
        assert_eq!(split_time_raw(0, 10), None);
    }

    #[test]
    fn should_reject_negative_raw() {
        assert_eq!(split_time_raw(-1, 9), None);
    }

    #[test]
    fn should_reject_second_of_day_at_or_past_86400() {
        // 24:00:00 exactly, and a value far past the end of the day.
        assert_eq!(split_time_raw(86_400, 0), None);
        assert_eq!(split_time_raw(100_000_000_000_000_000, 9), None);
    }

    /// The load-bearing guarantee: for every scale the server can send, the
    /// integer split agrees exactly with chrono's clock arithmetic. Both the
    /// materializer and any parts-only front end share this one contract.
    #[test]
    fn should_match_chrono_across_all_scales() {
        use chrono::{NaiveTime, Timelike};

        // A spread of times of day, including both day boundaries.
        let secs_samples = [0u32, 1, 3_661, 45_296, 86_398, 86_399];
        for scale in 0..=9u32 {
            let divisor = 10i64.pow(scale);
            for &secs in &secs_samples {
                // Largest in-scale fraction, e.g. 999 at scale 3.
                let frac = divisor - 1;
                let raw = secs as i64 * divisor + frac;

                let (got_secs, got_nanos) = split_time_raw(raw, scale).unwrap();
                let expected_nanos = frac as u32 * 10u32.pow(9 - scale);
                assert_eq!(
                    (got_secs, got_nanos),
                    (secs, expected_nanos),
                    "scale {scale}, secs {secs}"
                );

                // And the parts must build the very time chrono would.
                let time = NaiveTime::from_num_seconds_from_midnight_opt(got_secs, got_nanos)
                    .unwrap_or_else(|| panic!("scale {scale}, secs {secs}"));
                assert_eq!(time.num_seconds_from_midnight(), secs);
                assert_eq!(time.nanosecond(), expected_nanos);
            }
        }
    }
}
