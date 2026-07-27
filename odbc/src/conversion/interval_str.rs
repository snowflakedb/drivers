//! Parser for VARCHAR → SQL_C_INTERVAL_* fetches (`SQLGetData`).
//!
//! Per ODBC Appendix D ("Converting Data from SQL to C Data Types",
//! section "Character to Interval"), a Snowflake VARCHAR fetched as a
//! SQL_C_INTERVAL_* value must be parsed as an SQL interval literal
//! whose qualifier matches the C target type. The literal grammar is
//! the bare value form (without the `INTERVAL '...' <qualifier>`
//! envelope):
//!
//! ```text
//!   YEAR              : [-]<years>
//!   MONTH             : [-]<months>
//!   DAY               : [-]<days>
//!   HOUR              : [-]<hours>
//!   MINUTE            : [-]<minutes>
//!   SECOND            : [-]<seconds>[.<fraction>]
//!   YEAR_TO_MONTH     : [-]<years>-<months>
//!   DAY_TO_HOUR       : [-]<days> <hours>
//!   DAY_TO_MINUTE     : [-]<days> <hours>:<minutes>
//!   DAY_TO_SECOND     : [-]<days> <hours>:<minutes>:<seconds>[.<fraction>]
//!   HOUR_TO_MINUTE    : [-]<hours>:<minutes>
//!   HOUR_TO_SECOND    : [-]<hours>:<minutes>:<seconds>[.<fraction>]
//!   MINUTE_TO_SECOND  : [-]<minutes>:<seconds>[.<fraction>]
//! ```
//!
//! The ODBC spec defines four outcomes for the conversion:
//!
//!   1. Valid value, no truncation. SQL_SUCCESS.
//!   2. Valid value, trailing-field truncation. SQL_SUCCESS_WITH_INFO,
//!      SQLSTATE 01S07.
//!   3. Valid value, leading-field precision lost. SQL_ERROR,
//!      SQLSTATE 22015.
//!   4. Not a valid interval value. SQL_ERROR, SQLSTATE 22018.
//!
//! Shape handling: the parser accepts the canonical ANSI literal
//! for the *target* qualifier, plus any literal that carries *more*
//! trailing fields than the target wants — the latter is the
//! truncation case (#2). Any other shape is a format error (#4).
//!
//! Range handling: trailing composite fields are validated against
//! their canonical ANSI SQL ranges and a violation surfaces 22015
//! ("Interval field overflow"). The enforced ranges for *trailing*
//! slots in a composite literal are:
//!
//!   * MONTH (in `YEAR_TO_MONTH`)        : 0..=11
//!   * HOUR  (in `*_TO_HOUR/MINUTE/SECOND`) : 0..=23
//!   * MINUTE (in `*_TO_MINUTE/SECOND`)  : 0..=59
//!   * SECOND (in `*_TO_SECOND`)         : 0..=59
//!
//! 22015 is the spec-mandated state for this case via two reinforcing
//! readings of the ODBC reference:
//!
//!   * The `SQLGetData` diagnostics table defines 22015 as fired when
//!     "there was no representation of the value of the SQL type in
//!     the interval C type" — a trailing field outside its Gregorian
//!     range has no representation in the interval C structure, whose
//!     trailing fields are constrained by Gregorian rules per the
//!     "Constraints of the Gregorian Calendar" appendix.
//!   * The driver already uses `IntervalFieldOverflow` (→ 22015) for
//!     analogous field-level overflows: `numeric_helpers::push_field_value`
//!     fires it for trailing-field `u32::MAX` overflow, and this file's
//!     own `field_overflow` helper fires it for day/hour/minute/second
//!     overflow when the parsed value won't fit `u32`. Using 22018 for
//!     a value of 61 while using 22015 for a value of 4_294_967_295
//!     would be internally inconsistent — 22015 covers both ends.
//!
//! The *leading* field of any single-field or composite target keeps
//! the precision-driven check (`SQL_DESC_DATETIME_INTERVAL_PRECISION`,
//! also 22015 on overflow) and is *not* range-checked, because per
//! ANSI a leading field can be arbitrarily large within its declared
//! precision. For `MINUTE_TO_SECOND`, the leading slot is the minute,
//! so only the trailing second is range-checked against 0..=59.

use odbc_sys as sql;
use snafu::OptionExt;

use crate::api::CDataType;
use crate::conversion::error::{
    IntervalFieldOverflowSnafu, InvalidValueSnafu, UnsupportedOdbcTypeSnafu, WriteOdbcError,
};
use crate::conversion::traits::Binding;
use crate::conversion::warning::{Warning, Warnings};

/// Parsed components of an interval string. Any field the input
/// did not carry is `None`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct IntervalParts {
    is_negative: bool,
    year: Option<u128>,
    month: Option<u128>,
    day: Option<u128>,
    hour: Option<u128>,
    minute: Option<u128>,
    second: Option<u128>,
    /// Microseconds (0–999_999). Captured separately from `second` so
    /// integer-only targets can ignore it without touching `second`.
    fraction_micros: Option<u32>,
    /// `true` if the input had a fractional `.<digits>` component
    /// after the seconds field. Used to surface 01S07 truncation when
    /// the target type cannot carry fractional seconds.
    has_fraction: bool,
    /// `true` when the input fractional component carried more than
    /// 6 digits and at least one of the dropped digits was non-zero
    /// (i.e. data was actually lost). Surfaced as 01S07 even when
    /// the target *does* consume the fraction.
    fraction_was_truncated: bool,
    /// `true` if the input was a bare numeric like `"5"` or `"5.5"`
    /// with no day-or-time delimiters. The bare-number value is
    /// stored in *every* field so single-field targets (YEAR / MONTH
    /// / DAY / HOUR / MINUTE / SECOND) can read whichever field they
    /// want; this flag suppresses the "trailing field truncation"
    /// heuristic (none of the other fields really exist).
    is_single_int_input: bool,
}

/// Drop the leading sign (if any) and return `(is_negative, rest)`.
fn split_sign(s: &str) -> (bool, &str) {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix('-') {
        (true, rest.trim_start())
    } else if let Some(rest) = s.strip_prefix('+') {
        (false, rest.trim_start())
    } else {
        (false, s)
    }
}

fn parse_u128(s: &str) -> Result<u128, WriteOdbcError> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return InvalidValueSnafu {
            reason: format!("expected unsigned integer, got '{s}'"),
        }
        .fail();
    }
    s.parse::<u128>().map_err(|e| {
        // Per ODBC spec, "value too large for the leading-field precision"
        // is 22015 (interval field overflow), not 22018 (invalid format).
        // The string itself IS a valid unsigned integer literal here —
        // it just doesn't fit into u128 (and therefore certainly does
        // not fit into any interval field's u32 storage).
        IntervalFieldOverflowSnafu {
            reason: format!("integer overflow parsing '{s}': {e}"),
        }
        .build()
    })
}

/// Parsed seconds-with-fraction component.
#[derive(Debug, Clone, Copy)]
struct SecondParse {
    second: u128,
    fraction_micros: u32,
    has_fraction: bool,
    /// `true` when the source carried more than 6 fractional digits
    /// AND at least one of the dropped digits was non-zero. Mirrors
    /// `numeric_helpers::compute_interval_fraction`'s `was_truncated`
    /// flag and lets the caller surface 01S07 / `NumericValueTruncated`.
    fraction_was_truncated: bool,
}

/// Parse `<seconds>[.<fraction>]`. The fraction is normalised to
/// microseconds (always 6 digits internally): shorter fractions are
/// zero-padded, longer ones are truncated and `fraction_was_truncated`
/// is set when any of the dropped digits was non-zero.
fn parse_seconds_with_fraction(s: &str) -> Result<SecondParse, WriteOdbcError> {
    if let Some((whole, frac)) = s.split_once('.') {
        let second = parse_u128(whole)?;
        if frac.is_empty() || !frac.bytes().all(|b| b.is_ascii_digit()) {
            return InvalidValueSnafu {
                reason: format!("invalid fraction component '{frac}'"),
            }
            .fail();
        }
        // Zero-pad / truncate to exactly 6 digits (microseconds).
        let mut micro_buf = [b'0'; 6];
        for (i, b) in frac.bytes().take(6).enumerate() {
            micro_buf[i] = b;
        }
        let micros: u32 = std::str::from_utf8(&micro_buf)
            .expect("ascii-only digit buffer")
            .parse()
            .expect("six ascii digits always fit in u32");
        // Anything past the 6th fractional digit is silently dropped
        // ONLY when those digits are all `0`; otherwise we owe the
        // application a 01S07 warning.
        let fraction_was_truncated = frac.len() > 6 && frac.bytes().skip(6).any(|b| b != b'0');
        Ok(SecondParse {
            second,
            fraction_micros: micros,
            has_fraction: true,
            fraction_was_truncated,
        })
    } else {
        Ok(SecondParse {
            second: parse_u128(s)?,
            fraction_micros: 0,
            has_fraction: false,
            fraction_was_truncated: false,
        })
    }
}

/// Detect the input's *shape* and parse every field present, regardless
/// of the target type. The caller then projects to the target qualifier
/// and decides whether unused trailing fields cause 01S07 truncation
/// or whether the input shape is incompatible with the target (22018).
fn parse_any_shape(s: &str) -> Result<IntervalParts, WriteOdbcError> {
    let (is_negative, body) = split_sign(s);
    if body.is_empty() {
        return InvalidValueSnafu {
            reason: "empty interval value".to_string(),
        }
        .fail();
    }
    let mut p = IntervalParts {
        is_negative,
        ..Default::default()
    };

    // YEAR_TO_MONTH form: "<year>-<month>" (and only this form has '-' as
    // an internal separator after the optional leading sign was stripped).
    if let Some((year_str, month_str)) = body.split_once('-') {
        p.year = Some(parse_u128(year_str.trim())?);
        p.month = Some(parse_u128(month_str.trim())?);
        return Ok(p);
    }

    // DAY-bearing day-time forms always carry a single space between the
    // day field and the time-of-day fields.
    let (day_part, time_part) = if let Some((d, t)) = body.split_once(' ') {
        p.day = Some(parse_u128(d.trim())?);
        (Some(d.trim()), Some(t.trim()))
    } else {
        (None, Some(body))
    };

    // The remaining segment is either:
    //   - a single integer (single-field DAY/HOUR/MINUTE/SECOND or just
    //     YEAR/MONTH if the caller chose that interpretation), or
    //   - colon-separated time fields.
    if let Some(time_str) = time_part {
        let time_str = time_str.trim();
        if time_str.is_empty() {
            return InvalidValueSnafu {
                reason: "missing time component after day".to_string(),
            }
            .fail();
        }
        let parts: Vec<&str> = time_str.split(':').collect();
        match parts.len() {
            1 => {
                // Single integer-or-decimal value. If the input also
                // carried a day field, this is hours; otherwise the
                // caller picks the interpretation (year/month/day/
                // hour/minute/second).
                let parsed = parse_seconds_with_fraction(parts[0].trim())?;
                if day_part.is_some() {
                    // "D H" → no fraction allowed on the hour field.
                    if parsed.has_fraction {
                        return InvalidValueSnafu {
                            reason: "hour field does not accept a fractional component".to_string(),
                        }
                        .fail();
                    }
                    p.hour = Some(parsed.second);
                } else {
                    // Single-component value. Stash it in every field
                    // `single_component_value()` may want to read.
                    p.year = Some(parsed.second);
                    p.month = Some(parsed.second);
                    p.day = Some(parsed.second);
                    p.hour = Some(parsed.second);
                    p.minute = Some(parsed.second);
                    p.second = Some(parsed.second);
                    p.fraction_micros = Some(parsed.fraction_micros);
                    p.has_fraction = parsed.has_fraction;
                    p.fraction_was_truncated = parsed.fraction_was_truncated;
                    p.is_single_int_input = true;
                }
            }
            2 => {
                // 2-component time. Two valid readings: "H:M" (no
                // fraction allowed on minute) or "M:S[.fraction]"
                // (fraction sticks to seconds only).
                //
                // When the trailing component carries a *non-zero*
                // fraction the input can ONLY be M:S form (a minute
                // field must be an integer), so we store it that
                // way and the H:M composites correctly fail with
                // 22018.
                //
                // When the trailing component has no fraction OR an
                // explicit zero fraction (e.g. `5:10.0`, which
                // Snowflake's own textual rendering of an INTERVAL
                // HOUR TO MINUTE value can emit), the input is
                // compatible with the H:M reading too, so we store
                // it as H:M and let the `IntervalMinuteToSecond` arm
                // in `build_composite` re-interpret as M:S when the
                // target asks for it.
                let leading = parse_u128(parts[0].trim())?;
                let parsed = parse_seconds_with_fraction(parts[1].trim())?;
                if parsed.has_fraction && parsed.fraction_micros > 0 {
                    p.minute = Some(leading);
                    p.second = Some(parsed.second);
                    p.fraction_micros = Some(parsed.fraction_micros);
                    p.has_fraction = true;
                    p.fraction_was_truncated = parsed.fraction_was_truncated;
                } else {
                    // Either no fraction or an explicit `.0` zero
                    // fraction — keep the unambiguous H:M reading.
                    // Note: a fractional `.0` with extra trailing
                    // digits ("10.000000007") cannot be silently
                    // dropped on a minute field; flag it so the
                    // caller surfaces 01S07 even though the H:M
                    // reading is otherwise lossless.
                    p.hour = Some(leading);
                    p.minute = Some(parsed.second);
                    p.fraction_was_truncated = parsed.fraction_was_truncated;
                }
            }
            3 => {
                // "H:M:S[.F]"
                p.hour = Some(parse_u128(parts[0].trim())?);
                p.minute = Some(parse_u128(parts[1].trim())?);
                let parsed = parse_seconds_with_fraction(parts[2].trim())?;
                p.second = Some(parsed.second);
                p.fraction_micros = Some(parsed.fraction_micros);
                p.has_fraction = parsed.has_fraction;
                p.fraction_was_truncated = parsed.fraction_was_truncated;
            }
            _ => {
                return InvalidValueSnafu {
                    reason: format!("too many ':' separators in time component '{time_str}'"),
                }
                .fail();
            }
        }
    }
    Ok(p)
}

/// Build an `IntervalStruct` for a single-field interval target,
/// reading the relevant field from `parts` and reporting truncation
/// for any *other* field that was present in the input.
fn build_single_field(
    parts: &IntervalParts,
    target: CDataType,
    binding: &Binding,
) -> Result<Warnings, WriteOdbcError> {
    use CDataType::*;
    let value = match target {
        IntervalYear => parts.year,
        IntervalMonth => parts.month,
        IntervalDay => parts.day,
        IntervalHour => parts.hour,
        IntervalMinute => parts.minute,
        IntervalSecond => parts.second,
        _ => unreachable!("build_single_field called with {target:?}"),
    }
    .with_context(|| InvalidValueSnafu {
        reason: format!("interval input does not carry a {target:?} component"),
    })?;

    crate::conversion::numeric_helpers::check_leading_precision(value, value, binding)?;
    let field = crate::conversion::numeric_helpers::checked_u32(value, value)?;

    // `interval_sign = 0` for a zero magnitude regardless of the source
    // sign; this matches the canonical IntervalStruct construction in
    // `numeric_helpers::write_single_field_interval`. For
    // `IntervalSecond` the magnitude also includes any fractional
    // microseconds — `"-0.5"` must surface `interval_sign = 1` even
    // though the integer-second `field` is zero.
    let fraction_for_sign = if matches!(target, IntervalSecond) {
        parts.fraction_micros.unwrap_or(0)
    } else {
        0
    };
    let is_negative = parts.is_negative && (field > 0 || fraction_for_sign > 0);
    let mut iv = sql::IntervalStruct {
        interval_type: 0,
        interval_sign: if is_negative { 1 } else { 0 },
        interval_value: sql::IntervalUnion {
            day_second: sql::DaySecond::default(),
        },
    };

    let warnings = trailing_field_warnings(parts, target);
    match target {
        IntervalYear => {
            iv.interval_type = sql::Interval::Year as i32;
            iv.interval_value = sql::IntervalUnion {
                year_month: sql::YearMonth {
                    year: field,
                    month: 0,
                },
            };
        }
        IntervalMonth => {
            iv.interval_type = sql::Interval::Month as i32;
            iv.interval_value = sql::IntervalUnion {
                year_month: sql::YearMonth {
                    year: 0,
                    month: field,
                },
            };
        }
        #[allow(unused_unsafe)]
        IntervalDay => {
            iv.interval_type = sql::Interval::Day as i32;
            unsafe {
                iv.interval_value.day_second.day = field;
            }
        }
        #[allow(unused_unsafe)]
        IntervalHour => {
            iv.interval_type = sql::Interval::Hour as i32;
            unsafe {
                iv.interval_value.day_second.hour = field;
            }
        }
        #[allow(unused_unsafe)]
        IntervalMinute => {
            iv.interval_type = sql::Interval::Minute as i32;
            unsafe {
                iv.interval_value.day_second.minute = field;
            }
        }
        #[allow(unused_unsafe)]
        IntervalSecond => {
            iv.interval_type = sql::Interval::Second as i32;
            unsafe {
                iv.interval_value.day_second.second = field;
                iv.interval_value.day_second.fraction = parts.fraction_micros.unwrap_or(0);
            }
        }
        _ => unreachable!(),
    }
    binding.write_fixed(iv);
    Ok(warnings)
}

/// Build an `IntervalStruct` for a composite interval target.
fn build_composite(
    parts: &IntervalParts,
    target: CDataType,
    binding: &Binding,
) -> Result<Warnings, WriteOdbcError> {
    use CDataType::*;

    // Composite targets demand a literal whose shape matches the
    // qualifier — a bare integer input ("5") is not a valid
    // YEAR_TO_MONTH / DAY_TO_HOUR / etc. literal even though
    // `parse_any_shape` populated every field for the convenience
    // of single-field targets. Reject 22018 here to match the spec.
    if parts.is_single_int_input {
        return InvalidValueSnafu {
            reason: format!(
                "interval target {target:?} requires a multi-field literal, not a bare integer"
            ),
        }
        .fail();
    }

    let read = |field: Option<u128>, name: &str| -> Result<u128, WriteOdbcError> {
        field.with_context(|| InvalidValueSnafu {
            reason: format!("interval input is missing required '{name}' component for {target:?}"),
        })
    };

    let (interval_type, leading_value, day_second_fields, year_month_fields) = match target {
        IntervalYearToMonth => {
            let y = read(parts.year, "year")?;
            let m = read(parts.month, "month")?;
            check_trailing_field_range("month", m, 11)?;
            (sql::Interval::YearToMonth as i32, y, None, Some((y, m)))
        }
        IntervalDayToHour => {
            let d = read(parts.day, "day")?;
            let h = read(parts.hour, "hour")?;
            check_trailing_field_range("hour", h, 23)?;
            (
                sql::Interval::DayToHour as i32,
                d,
                Some((d, h, 0u128, 0u128, 0u32)),
                None,
            )
        }
        IntervalDayToMinute => {
            let d = read(parts.day, "day")?;
            let h = read(parts.hour, "hour")?;
            let m = read(parts.minute, "minute")?;
            check_trailing_field_range("hour", h, 23)?;
            check_trailing_field_range("minute", m, 59)?;
            (
                sql::Interval::DayToMinute as i32,
                d,
                Some((d, h, m, 0u128, 0u32)),
                None,
            )
        }
        IntervalDayToSecond => {
            let d = read(parts.day, "day")?;
            let h = read(parts.hour, "hour")?;
            let m = read(parts.minute, "minute")?;
            let s = read(parts.second, "second")?;
            check_trailing_field_range("hour", h, 23)?;
            check_trailing_field_range("minute", m, 59)?;
            check_trailing_field_range("second", s, 59)?;
            (
                sql::Interval::DayToSecond as i32,
                d,
                Some((d, h, m, s, parts.fraction_micros.unwrap_or(0))),
                None,
            )
        }
        IntervalHourToMinute => {
            let h = read(parts.hour, "hour")?;
            let m = read(parts.minute, "minute")?;
            check_trailing_field_range("minute", m, 59)?;
            (
                sql::Interval::HourToMinute as i32,
                h,
                Some((0, h, m, 0, 0u32)),
                None,
            )
        }
        IntervalHourToSecond => {
            let h = read(parts.hour, "hour")?;
            let m = read(parts.minute, "minute")?;
            let s = read(parts.second, "second")?;
            check_trailing_field_range("minute", m, 59)?;
            check_trailing_field_range("second", s, 59)?;
            (
                sql::Interval::HourToSecond as i32,
                h,
                Some((0, h, m, s, parts.fraction_micros.unwrap_or(0))),
                None,
            )
        }
        IntervalMinuteToSecond => {
            // For MINUTE_TO_SECOND, the canonical input shape is
            // "M:S" or "M:S.fraction". `parse_any_shape` stores
            // fraction-bearing 2-component input directly as
            // (minute=M, second=S, fraction=micros), and stores the
            // ambiguous "M:S" form as (hour=M, minute=S) — we
            // re-interpret the latter here. Anything else (day
            // field, 3-component time, bare integer) is a
            // 22018-grade format error.
            if parts.day.is_some() || parts.is_single_int_input {
                return InvalidValueSnafu {
                    reason: "minute-to-second interval requires 'M:S[.fraction]' input shape"
                        .to_string(),
                }
                .fail();
            }
            let (m, s, micros) = match (parts.hour, parts.minute, parts.second) {
                // Fraction-bearing path: "M:S.fraction" → minute,second already set.
                (None, Some(min), Some(sec)) => (min, sec, parts.fraction_micros.unwrap_or(0)),
                // Ambiguous "M:S" path: parse_any_shape stored as (hour=M, minute=S).
                (Some(h_as_min), Some(m_as_sec), None) => {
                    (h_as_min, m_as_sec, parts.fraction_micros.unwrap_or(0))
                }
                _ => {
                    return InvalidValueSnafu {
                        reason: "interval input is missing required 'minute' or 'second' component for IntervalMinuteToSecond".to_string(),
                    }
                    .fail();
                }
            };
            // Range check the trailing SECOND slot only. The leading
            // MINUTE slot (`m`) is governed by the precision-based
            // 22015 check below, not the canonical 0..=59 range.
            check_trailing_field_range("second", s, 59)?;
            (
                sql::Interval::MinuteToSecond as i32,
                m,
                Some((0, 0, m, s, micros)),
                None,
            )
        }
        _ => unreachable!("build_composite called with {target:?}"),
    };

    crate::conversion::numeric_helpers::check_leading_precision(
        leading_value,
        leading_value,
        binding,
    )?;

    let warnings = trailing_field_warnings(parts, target);
    let mut iv = sql::IntervalStruct {
        interval_type,
        interval_sign: 0,
        interval_value: sql::IntervalUnion {
            day_second: sql::DaySecond::default(),
        },
    };

    if let Some((y, m)) = year_month_fields {
        let year = u32::try_from(y).map_err(|_| field_overflow("year", y))?;
        let month = u32::try_from(m).map_err(|_| field_overflow("month", m))?;
        iv.interval_value = sql::IntervalUnion {
            year_month: sql::YearMonth { year, month },
        };
        // Sign: zero magnitude (both fields zero) stays unsigned.
        if parts.is_negative && (year > 0 || month > 0) {
            iv.interval_sign = 1;
        }
    }
    if let Some((d, h, m, s, frac)) = day_second_fields {
        let day = u32::try_from(d).map_err(|_| field_overflow("day", d))?;
        let hour = u32::try_from(h).map_err(|_| field_overflow("hour", h))?;
        let minute = u32::try_from(m).map_err(|_| field_overflow("minute", m))?;
        let second = u32::try_from(s).map_err(|_| field_overflow("second", s))?;
        iv.interval_value = sql::IntervalUnion {
            day_second: sql::DaySecond {
                day,
                hour,
                minute,
                second,
                fraction: frac,
            },
        };
        if parts.is_negative && (day > 0 || hour > 0 || minute > 0 || second > 0 || frac > 0) {
            iv.interval_sign = 1;
        }
    }
    binding.write_fixed(iv);
    Ok(warnings)
}

fn field_overflow(name: &str, value: u128) -> WriteOdbcError {
    IntervalFieldOverflowSnafu {
        reason: format!("{name} value {value} exceeds u32 range"),
    }
    .build()
}

/// Validate a *trailing* composite-interval field against its
/// canonical ANSI SQL range. Returns `IntervalFieldOverflow` (mapped
/// to SQLSTATE 22015, "Interval field overflow") when the magnitude
/// exceeds `max_inclusive`. Per the `SQLGetData` diagnostics table
/// 22015 fires when "there was no representation of the value of the
/// SQL type in the interval C type", and an interval C structure has
/// no representation for a trailing field outside its Gregorian range
/// (HOUR > 23, MINUTE/SECOND > 59, MONTH > 11). This also matches the
/// existing `field_overflow` helper below, which uses 22015 for the
/// catastrophic `u32::MAX`-overflow form of the same problem. Leading
/// fields use a precision-based 22015 check elsewhere and must NOT be
/// passed through this helper.
fn check_trailing_field_range(
    name: &'static str,
    value: u128,
    max_inclusive: u32,
) -> Result<(), WriteOdbcError> {
    if value > u128::from(max_inclusive) {
        IntervalFieldOverflowSnafu {
            reason: format!(
                "trailing '{name}' field magnitude {value} is outside the canonical ANSI range 0..={max_inclusive}"
            ),
        }
        .fail()
    } else {
        Ok(())
    }
}

/// Returns 01S07 (`Warning::NumericValueTruncated` →
/// `SqlState::FractionalTruncation`) when either:
///
///   * the parsed input carried a non-zero field that the target
///     qualifier cannot represent (classic trailing-field truncation —
///     Appendix D "Character to Interval" outcome #2), or
///   * the parser already discarded non-zero fractional digits past
///     the 6-digit microsecond cap (sub-microsecond loss; independent
///     of which qualifier the target asks for).
///
/// Per ODBC Appendix D outcome #2 the spec-mandated SQLSTATE for
/// interval-conversion truncation is 01S07, *not* 01004
/// (`StringDataRightTruncated`). `Warning::StringDataTruncated` maps
/// to 01004 in `odbc/src/api/diagnostic.rs`, which would mis-classify
/// the diagnostic, so this function uses `Warning::NumericValueTruncated`
/// throughout.
fn trailing_field_warnings(parts: &IntervalParts, target: CDataType) -> Warnings {
    use CDataType::*;

    // Sub-microsecond truncation is independent of the target's
    // qualifier coverage: even SECOND-bearing targets cannot carry
    // the dropped digits, so we always surface 01S07 when the
    // parser saw non-zero data past the 6-digit microsecond cap.
    if parts.fraction_was_truncated {
        return vec![Warning::NumericValueTruncated];
    }

    // Bare-numeric inputs ("5" or "5.5") populate every field with
    // the same value as a convenience for single-field targets; do
    // NOT treat that as truncation. The only meaningful loss is a
    // non-zero fraction sent to an integer-only target — a zero
    // fraction (e.g. "5.0") loses no information and must not warn.
    if parts.is_single_int_input {
        if parts.has_fraction
            && parts.fraction_micros.unwrap_or(0) > 0
            && !matches!(
                target,
                IntervalSecond
                    | IntervalDayToSecond
                    | IntervalHourToSecond
                    | IntervalMinuteToSecond
            )
        {
            return vec![Warning::NumericValueTruncated];
        }
        return vec![];
    }

    // Determine which fields the target *consumes*; any other
    // populated, non-zero field counts as a trailing truncation.
    //
    // MINUTE_TO_SECOND is special: `parse_any_shape` stores its
    // canonical "M:S" input as (hour=M, minute=S), so we treat both
    // hour and minute as consumed by this target.
    let consumes_year = matches!(target, IntervalYear | IntervalYearToMonth);
    let consumes_month = matches!(target, IntervalMonth | IntervalYearToMonth);
    let consumes_day = matches!(
        target,
        IntervalDay | IntervalDayToHour | IntervalDayToMinute | IntervalDayToSecond
    );
    let consumes_hour = matches!(
        target,
        IntervalHour
            | IntervalDayToHour
            | IntervalDayToMinute
            | IntervalDayToSecond
            | IntervalHourToMinute
            | IntervalHourToSecond
            | IntervalMinuteToSecond
    );
    let consumes_minute = matches!(
        target,
        IntervalMinute
            | IntervalDayToMinute
            | IntervalDayToSecond
            | IntervalHourToMinute
            | IntervalHourToSecond
            | IntervalMinuteToSecond
    );
    let consumes_second = matches!(
        target,
        IntervalSecond | IntervalDayToSecond | IntervalHourToSecond | IntervalMinuteToSecond
    );
    let consumes_fraction = consumes_second; // fraction always rides along with seconds

    let lost = (!consumes_year && parts.year.unwrap_or(0) > 0)
        || (!consumes_month && parts.month.unwrap_or(0) > 0)
        || (!consumes_day && parts.day.unwrap_or(0) > 0)
        || (!consumes_hour && parts.hour.unwrap_or(0) > 0)
        || (!consumes_minute && parts.minute.unwrap_or(0) > 0)
        || (!consumes_second && parts.second.unwrap_or(0) > 0)
        || (!consumes_fraction && parts.has_fraction && parts.fraction_micros.unwrap_or(0) > 0);

    if lost {
        vec![Warning::NumericValueTruncated]
    } else {
        vec![]
    }
}

/// Public entry point: convert the VARCHAR `value` into the
/// `SQL_C_INTERVAL_*` shape requested by `target` and write it into
/// `binding`. Returns warnings (e.g. 01S07 truncation) on success.
///
/// Mapped error → SQLSTATE:
///
///   * `InvalidValue`          → 22018 (invalid character value for cast)
///   * `IntervalFieldOverflow` → 22015 (interval field overflow)
///   * `UnsupportedOdbcType`   → 07006 (restricted data type attribute)
pub(crate) fn varchar_to_interval(
    value: &str,
    target: CDataType,
    binding: &Binding,
) -> Result<Warnings, WriteOdbcError> {
    use CDataType::*;
    let parts = parse_any_shape(value)?;
    match target {
        IntervalYear | IntervalMonth | IntervalDay | IntervalHour | IntervalMinute
        | IntervalSecond => build_single_field(&parts, target, binding),
        IntervalYearToMonth
        | IntervalDayToHour
        | IntervalDayToMinute
        | IntervalDayToSecond
        | IntervalHourToMinute
        | IntervalHourToSecond
        | IntervalMinuteToSecond => build_composite(&parts, target, binding),
        // Defensive: callers in `varchar.rs` only dispatch interval
        // targets here, but if a future caller forgets the filter the
        // spec-correct response is 07006 ("restricted data type
        // attribute violation"), not a numeric range error.
        _ => UnsupportedOdbcTypeSnafu {
            target_type: target,
        }
        .fail(),
    }
}
