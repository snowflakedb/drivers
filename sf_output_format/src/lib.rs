//! Shared rendering of decoded Snowflake values into their canonical string
//! form, for front ends that string-render in Rust. INTERVAL is the value type
//! rendered here today.
//!
//! The Arrow decode — one signed integer per cell — lives in
//! `sf_types::SnowflakeIntervalYearMonth` (total months) and
//! `SnowflakeIntervalDayTime` (total nanoseconds). Composing that integer into
//! the canonical ANSI literal (`[-]Y-MM` and `[-]D HH:MM:SS[.f]`) stays here so
//! `nodejs_bridge` and `odbc` render the same bytes. The literal matches the
//! grammar the ODBC bind path (`interval_str.rs`) parses, so an ODBC
//! `SQL_C_INTERVAL_*` target re-reads this output through the existing parser.

const NANOS_PER_SECOND: u128 = 1_000_000_000;
const SECONDS_PER_MINUTE: u128 = 60;
const SECONDS_PER_HOUR: u128 = 3_600;
const SECONDS_PER_DAY: u128 = 86_400;
const MONTHS_PER_YEAR: u128 = 12;

pub fn format_year_month(total_months: i128) -> String {
    let sign = if total_months < 0 { "-" } else { "" };
    let abs = total_months.unsigned_abs();
    let years = abs / MONTHS_PER_YEAR;
    let months = abs % MONTHS_PER_YEAR;
    format!("{sign}{years}-{months:02}")
}

/// `scale` is the column's fractional-second precision (0..=9); the caller
/// validates that bound, so `10^(9 - scale)` cannot underflow here.
pub fn format_day_time(total_nanos: i128, scale: u32) -> String {
    let sign = if total_nanos < 0 { "-" } else { "" };
    let abs = total_nanos.unsigned_abs();

    let total_seconds = abs / NANOS_PER_SECOND;
    let frac_nanos = abs % NANOS_PER_SECOND;
    let days = total_seconds / SECONDS_PER_DAY;
    let rem = total_seconds % SECONDS_PER_DAY;
    let hours = rem / SECONDS_PER_HOUR;
    let minutes = (rem % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE;
    let seconds = rem % SECONDS_PER_MINUTE;

    let mut result = format!("{sign}{days} {hours:02}:{minutes:02}:{seconds:02}");
    if scale > 0 {
        let frac = frac_nanos / 10u128.pow(9 - scale);
        let width = scale as usize;
        result.push_str(&format!(".{frac:0width$}"));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn year_month_positive() {
        assert_eq!(format_year_month(27), "2-03");
    }

    #[test]
    fn year_month_negative() {
        assert_eq!(format_year_month(-27), "-2-03");
    }

    #[test]
    fn year_month_zero() {
        assert_eq!(format_year_month(0), "0-00");
    }

    #[test]
    fn year_month_whole_years() {
        assert_eq!(format_year_month(24), "2-00");
    }

    #[test]
    fn year_month_sub_year() {
        assert_eq!(format_year_month(5), "0-05");
    }

    #[test]
    fn day_time_scale_zero_omits_fraction() {
        let nanos = (SECONDS_PER_DAY + 2 * SECONDS_PER_HOUR + 3 * SECONDS_PER_MINUTE + 4) as i128
            * NANOS_PER_SECOND as i128;
        assert_eq!(format_day_time(nanos, 0), "1 02:03:04");
    }

    #[test]
    fn day_time_scale_six() {
        let nanos = (2 * SECONDS_PER_HOUR + 3 * SECONDS_PER_MINUTE + 4) as i128
            * NANOS_PER_SECOND as i128
            + 500_000_000;
        assert_eq!(format_day_time(nanos, 6), "0 02:03:04.500000");
    }

    #[test]
    fn day_time_scale_nine_full_precision() {
        let nanos = 123_456_789i128;
        assert_eq!(format_day_time(nanos, 9), "0 00:00:00.123456789");
    }

    #[test]
    fn day_time_negative() {
        let nanos = -(SECONDS_PER_DAY as i128 * NANOS_PER_SECOND as i128) - 1;
        assert_eq!(format_day_time(nanos, 9), "-1 00:00:00.000000001");
    }

    #[test]
    fn day_time_zero() {
        assert_eq!(format_day_time(0, 0), "0 00:00:00");
    }

    #[test]
    fn day_time_truncates_to_scale() {
        let nanos = 123_456_789i128;
        assert_eq!(format_day_time(nanos, 3), "0 00:00:00.123");
    }

    #[test]
    fn day_time_beyond_i64_nanoseconds() {
        let nanos = 9_223_372_036_854_775_808i128;
        assert_eq!(format_day_time(nanos, 9), "106751 23:47:16.854775808");
    }
}
