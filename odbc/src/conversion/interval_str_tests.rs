//! Unit tests for the VARCHAR → SQL_C_INTERVAL_* parser.
//!
//! These tests exercise `varchar_to_interval` against an in-memory
//! `Binding` so we cover the lossy/truncation/format paths without
//! needing a live Snowflake connection. The e2e mirror is
//! `odbc_tests/tests/e2e/types/string_conversion_to_c_interval.cpp`.

use odbc_sys as sql;

use crate::api::CDataType;
use crate::conversion::error::WriteOdbcError;
use crate::conversion::interval_str::varchar_to_interval;
use crate::conversion::traits::Binding;
use crate::conversion::warning::Warning;

fn make_buffer() -> Box<sql::IntervalStruct> {
    Box::new(sql::IntervalStruct {
        interval_type: 0,
        interval_sign: 0,
        interval_value: sql::IntervalUnion {
            day_second: sql::DaySecond::default(),
        },
    })
}

fn make_binding(target: CDataType, buf: &mut sql::IntervalStruct) -> Binding {
    Binding {
        target_type: target,
        target_value_ptr: buf as *mut sql::IntervalStruct as sql::Pointer,
        buffer_length: std::mem::size_of::<sql::IntervalStruct>() as sql::Len,
        octet_length_ptr: std::ptr::null_mut(),
        indicator_ptr: std::ptr::null_mut(),
        precision: None,
        scale: None,
        datetime_interval_precision: None,
    }
}

fn make_binding_with_precision(
    target: CDataType,
    buf: &mut sql::IntervalStruct,
    leading_precision: i16,
) -> Binding {
    let mut b = make_binding(target, buf);
    b.datetime_interval_precision = Some(leading_precision);
    b
}

fn run(
    value: &str,
    target: CDataType,
) -> (sql::IntervalStruct, Result<Vec<Warning>, WriteOdbcError>) {
    let mut buf = make_buffer();
    let binding = make_binding(target, &mut buf);
    let r = varchar_to_interval(value, target, &binding);
    (*buf, r)
}

#[test]
fn year_single_field_round_trip() {
    let (iv, r) = run("5", CDataType::IntervalYear);
    let warnings = r.expect("parse should succeed");
    assert!(warnings.is_empty());
    assert_eq!(iv.interval_type, sql::Interval::Year as i32);
    assert_eq!(iv.interval_sign, 0);
    unsafe {
        assert_eq!(iv.interval_value.year_month.year, 5);
        assert_eq!(iv.interval_value.year_month.month, 0);
    }
}

#[test]
fn negative_year_sets_sign_bit() {
    let (iv, r) = run("-7", CDataType::IntervalYear);
    let warnings = r.expect("parse should succeed");
    assert!(warnings.is_empty());
    assert_eq!(iv.interval_sign, 1);
    unsafe {
        assert_eq!(iv.interval_value.year_month.year, 7);
    }
}

#[test]
fn negative_zero_keeps_sign_unset() {
    let (iv, _) = run("-0", CDataType::IntervalYear);
    assert_eq!(iv.interval_sign, 0);
}

#[test]
fn day_single_field() {
    let (iv, r) = run("31", CDataType::IntervalDay);
    let warnings = r.expect("parse should succeed");
    assert!(warnings.is_empty());
    assert_eq!(iv.interval_type, sql::Interval::Day as i32);
    unsafe {
        assert_eq!(iv.interval_value.day_second.day, 31);
    }
}

#[test]
fn second_with_fraction_round_trips_fraction() {
    let (iv, r) = run("12.500000", CDataType::IntervalSecond);
    let warnings = r.expect("parse should succeed");
    assert!(
        warnings.is_empty(),
        "fraction belongs to a SECOND target, no warning expected, got {warnings:?}"
    );
    unsafe {
        assert_eq!(iv.interval_value.day_second.second, 12);
        assert_eq!(iv.interval_value.day_second.fraction, 500_000);
    }
}

#[test]
fn second_truncates_extra_fraction_digits() {
    // ODBC normalises fractional seconds to 6 digits (microseconds).
    // When dropped digits are non-zero we owe the application a 01S07
    // (`NumericValueTruncated` → `FractionalTruncation`) warning,
    // mirroring `numeric_helpers::compute_interval_fraction`'s
    // `was_truncated`.
    let (iv, r) = run("0.1234567", CDataType::IntervalSecond);
    let warnings = r.expect("parse should succeed");
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, Warning::NumericValueTruncated)),
        "expected 01S07 truncation warning, got {warnings:?}"
    );
    unsafe {
        assert_eq!(iv.interval_value.day_second.fraction, 123_456);
    }
}

#[test]
fn second_zero_padded_extra_fraction_digits_does_not_warn() {
    // `0.1234560000000` has nothing meaningful past the 6th digit, so
    // no warning is owed even though the source was longer than 6
    // characters.
    let (iv, r) = run("0.1234560000", CDataType::IntervalSecond);
    let warnings = r.expect("parse should succeed");
    assert!(
        warnings.is_empty(),
        "padded zeros should not trigger 01S07, got {warnings:?}"
    );
    unsafe {
        assert_eq!(iv.interval_value.day_second.fraction, 123_456);
    }
}

#[test]
fn year_to_month_two_components() {
    let (iv, r) = run("3-6", CDataType::IntervalYearToMonth);
    let warnings = r.expect("parse should succeed");
    assert!(warnings.is_empty());
    assert_eq!(iv.interval_type, sql::Interval::YearToMonth as i32);
    unsafe {
        assert_eq!(iv.interval_value.year_month.year, 3);
        assert_eq!(iv.interval_value.year_month.month, 6);
    }
}

#[test]
fn year_to_month_into_year_truncates_month() {
    let (iv, r) = run("3-6", CDataType::IntervalYear);
    let warnings = r.expect("parse should succeed");
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, Warning::NumericValueTruncated)),
        "expected 01S07 truncation warning, got {warnings:?}"
    );
    unsafe {
        assert_eq!(iv.interval_value.year_month.year, 3);
    }
}

#[test]
fn day_to_hour_two_fields() {
    let (iv, r) = run("5 10", CDataType::IntervalDayToHour);
    let warnings = r.expect("parse should succeed");
    assert!(warnings.is_empty());
    assert_eq!(iv.interval_type, sql::Interval::DayToHour as i32);
    unsafe {
        assert_eq!(iv.interval_value.day_second.day, 5);
        assert_eq!(iv.interval_value.day_second.hour, 10);
    }
}

#[test]
fn day_to_minute_three_fields() {
    let (iv, r) = run("3 14:30", CDataType::IntervalDayToMinute);
    let warnings = r.expect("parse should succeed");
    assert!(warnings.is_empty());
    unsafe {
        assert_eq!(iv.interval_value.day_second.day, 3);
        assert_eq!(iv.interval_value.day_second.hour, 14);
        assert_eq!(iv.interval_value.day_second.minute, 30);
    }
}

#[test]
fn day_to_second_with_fraction() {
    let (iv, r) = run("2 08:15:30.250000", CDataType::IntervalDayToSecond);
    let warnings = r.expect("parse should succeed");
    assert!(warnings.is_empty());
    unsafe {
        assert_eq!(iv.interval_value.day_second.day, 2);
        assert_eq!(iv.interval_value.day_second.hour, 8);
        assert_eq!(iv.interval_value.day_second.minute, 15);
        assert_eq!(iv.interval_value.day_second.second, 30);
        assert_eq!(iv.interval_value.day_second.fraction, 250_000);
    }
}

#[test]
fn hour_to_minute_two_fields() {
    let (iv, r) = run("10:45", CDataType::IntervalHourToMinute);
    let warnings = r.expect("parse should succeed");
    assert!(warnings.is_empty());
    unsafe {
        assert_eq!(iv.interval_value.day_second.hour, 10);
        assert_eq!(iv.interval_value.day_second.minute, 45);
    }
}

#[test]
fn hour_to_second_three_fields() {
    let (iv, r) = run("12:30:45", CDataType::IntervalHourToSecond);
    let warnings = r.expect("parse should succeed");
    assert!(warnings.is_empty());
    unsafe {
        assert_eq!(iv.interval_value.day_second.hour, 12);
        assert_eq!(iv.interval_value.day_second.minute, 30);
        assert_eq!(iv.interval_value.day_second.second, 45);
    }
}

#[test]
fn minute_to_second_two_fields() {
    let (iv, r) = run("45:30", CDataType::IntervalMinuteToSecond);
    let warnings = r.expect("parse should succeed");
    assert!(warnings.is_empty());
    unsafe {
        assert_eq!(iv.interval_value.day_second.minute, 45);
        assert_eq!(iv.interval_value.day_second.second, 30);
    }
}

#[test]
fn minute_to_second_with_fraction() {
    let (iv, r) = run("45:30.125", CDataType::IntervalMinuteToSecond);
    let warnings = r.expect("parse should succeed");
    assert!(warnings.is_empty());
    unsafe {
        assert_eq!(iv.interval_value.day_second.minute, 45);
        assert_eq!(iv.interval_value.day_second.second, 30);
        assert_eq!(iv.interval_value.day_second.fraction, 125_000);
    }
}

#[test]
fn day_to_second_truncates_into_day_target() {
    // "5 10:30:45" routed to IntervalDay → only the day field
    // survives, hour/minute/second are dropped → 01S07.
    let (iv, r) = run("5 10:30:45", CDataType::IntervalDay);
    let warnings = r.expect("parse should succeed");
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, Warning::NumericValueTruncated)),
        "expected 01S07 truncation warning, got {warnings:?}"
    );
    unsafe {
        assert_eq!(iv.interval_value.day_second.day, 5);
    }
}

#[test]
fn invalid_format_returns_22018() {
    let (_, r) = run("abc", CDataType::IntervalYear);
    let err = r.expect_err("non-numeric input should fail");
    assert!(
        matches!(err, WriteOdbcError::InvalidValue { .. }),
        "{err:?}"
    );
}

#[test]
fn empty_string_returns_22018() {
    let (_, r) = run("", CDataType::IntervalYear);
    let err = r.expect_err("empty string should fail");
    assert!(
        matches!(err, WriteOdbcError::InvalidValue { .. }),
        "{err:?}"
    );
}

#[test]
fn minute_to_second_rejects_three_component_input() {
    // "1 12:30:45" is the DAY_TO_SECOND shape; for MINUTE_TO_SECOND
    // the spec mandates the bare "M:S[.fraction]" literal form.
    let (_, r) = run("1 12:30:45", CDataType::IntervalMinuteToSecond);
    let err = r.expect_err("wrong shape must fail");
    assert!(
        matches!(err, WriteOdbcError::InvalidValue { .. }),
        "{err:?}"
    );
}

#[test]
fn missing_required_component_returns_22018() {
    // YEAR_TO_MONTH requires the "Y-M" form; passing a bare integer
    // is a 22018 (not just a missing-month default).
    let (_, r) = run("5", CDataType::IntervalYearToMonth);
    let err = r.expect_err("year-to-month requires the 'Y-M' form");
    assert!(
        matches!(err, WriteOdbcError::InvalidValue { .. }),
        "{err:?}"
    );
}

// ============================================================================
// Sign-bit edge cases
// ============================================================================

#[test]
fn negative_fractional_only_second_sets_sign_bit() {
    // `-0.5` must write `interval_sign = 1` for SQL_C_INTERVAL_SECOND;
    // the integer-second `field` is zero but the magnitude carried by
    // `fraction_micros` is non-zero.
    let (iv, r) = run("-0.5", CDataType::IntervalSecond);
    let warnings = r.expect("parse should succeed");
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        iv.interval_sign, 1,
        "sign must be set for non-zero magnitude"
    );
    unsafe {
        assert_eq!(iv.interval_value.day_second.second, 0);
        assert_eq!(iv.interval_value.day_second.fraction, 500_000);
    }
}

#[test]
fn negative_zero_with_explicit_zero_fraction_keeps_sign_unset() {
    // `-0.0` has zero magnitude — sign bit must NOT be set.
    let (iv, _) = run("-0.0", CDataType::IntervalSecond);
    assert_eq!(iv.interval_sign, 0);
    unsafe {
        assert_eq!(iv.interval_value.day_second.second, 0);
        assert_eq!(iv.interval_value.day_second.fraction, 0);
    }
}

#[test]
fn negative_year_to_month_sets_sign_bit() {
    let (iv, r) = run("-3-6", CDataType::IntervalYearToMonth);
    r.expect("parse should succeed");
    assert_eq!(iv.interval_sign, 1);
    unsafe {
        assert_eq!(iv.interval_value.year_month.year, 3);
        assert_eq!(iv.interval_value.year_month.month, 6);
    }
}

#[test]
fn negative_day_to_second_sets_sign_bit() {
    let (iv, r) = run("-2 08:15:30.250000", CDataType::IntervalDayToSecond);
    r.expect("parse should succeed");
    assert_eq!(iv.interval_sign, 1);
    unsafe {
        assert_eq!(iv.interval_value.day_second.day, 2);
        assert_eq!(iv.interval_value.day_second.hour, 8);
        assert_eq!(iv.interval_value.day_second.minute, 15);
        assert_eq!(iv.interval_value.day_second.second, 30);
        assert_eq!(iv.interval_value.day_second.fraction, 250_000);
    }
}

#[test]
fn negative_hour_to_second_sets_sign_bit() {
    let (iv, r) = run("-12:30:45", CDataType::IntervalHourToSecond);
    r.expect("parse should succeed");
    assert_eq!(iv.interval_sign, 1);
    unsafe {
        assert_eq!(iv.interval_value.day_second.hour, 12);
        assert_eq!(iv.interval_value.day_second.minute, 30);
        assert_eq!(iv.interval_value.day_second.second, 45);
    }
}

#[test]
fn negative_hour_to_minute_sets_sign_bit() {
    let (iv, r) = run("-10:45", CDataType::IntervalHourToMinute);
    r.expect("parse should succeed");
    assert_eq!(iv.interval_sign, 1);
    unsafe {
        assert_eq!(iv.interval_value.day_second.hour, 10);
        assert_eq!(iv.interval_value.day_second.minute, 45);
    }
}

// ============================================================================
// 5:10.0 → H:M composites
// ============================================================================

#[test]
fn explicit_zero_fraction_two_component_routes_to_hour_to_minute() {
    // Snowflake's INTERVAL HOUR TO MINUTE textual rendering can
    // include a trailing `.0` — make sure we accept it instead of
    // rejecting with 22018 ("missing required 'hour' component").
    let (iv, r) = run("5:10.0", CDataType::IntervalHourToMinute);
    let warnings = r.expect("explicit-zero-fraction H:M must succeed");
    assert!(warnings.is_empty(), "{warnings:?}");
    unsafe {
        assert_eq!(iv.interval_value.day_second.hour, 5);
        assert_eq!(iv.interval_value.day_second.minute, 10);
    }
}

#[test]
fn explicit_zero_fraction_three_component_routes_to_day_to_minute() {
    // `D H:M.0` should populate (day, hour, minute) on a DAY_TO_MINUTE target.
    let (iv, r) = run("3 5:10.0", CDataType::IntervalDayToMinute);
    let warnings = r.expect("explicit-zero-fraction D H:M must succeed");
    assert!(warnings.is_empty(), "{warnings:?}");
    unsafe {
        assert_eq!(iv.interval_value.day_second.day, 3);
        assert_eq!(iv.interval_value.day_second.hour, 5);
        assert_eq!(iv.interval_value.day_second.minute, 10);
    }
}

#[test]
fn explicit_zero_fraction_two_component_still_works_for_minute_to_second() {
    // The MinuteToSecond target should still re-interpret `5:10.0` as
    // `(minute=5, second=10, fraction=0)` — the ambiguity is resolved
    // by the target qualifier.
    let (iv, r) = run("5:10.0", CDataType::IntervalMinuteToSecond);
    let warnings = r.expect("explicit-zero-fraction M:S must succeed");
    assert!(warnings.is_empty(), "{warnings:?}");
    unsafe {
        assert_eq!(iv.interval_value.day_second.minute, 5);
        assert_eq!(iv.interval_value.day_second.second, 10);
        assert_eq!(iv.interval_value.day_second.fraction, 0);
    }
}

#[test]
fn nonzero_fraction_two_component_still_rejects_hour_to_minute() {
    // `5:10.125` is unambiguously M:S.fraction (a minute field cannot
    // carry a fractional component) — HOUR_TO_MINUTE must fail.
    let (_, r) = run("5:10.125", CDataType::IntervalHourToMinute);
    r.expect_err("non-zero fraction with H:M target must fail");
}

// ============================================================================
// >6 fraction digits via composite paths
// ============================================================================

#[test]
fn day_to_second_truncated_fraction_warns() {
    // 7-digit fraction with non-zero trailing digit — the parser
    // truncates to 6 and we owe the application a 01S07.
    let (iv, r) = run("1 02:03:04.1234567", CDataType::IntervalDayToSecond);
    let warnings = r.expect("parse should succeed");
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, Warning::NumericValueTruncated)),
        "expected 01S07 truncation warning for sub-microsecond loss in D-S, got {warnings:?}"
    );
    unsafe {
        assert_eq!(iv.interval_value.day_second.fraction, 123_456);
    }
}

#[test]
fn hour_to_second_truncated_fraction_warns() {
    let (iv, r) = run("01:02:03.7654321", CDataType::IntervalHourToSecond);
    let warnings = r.expect("parse should succeed");
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, Warning::NumericValueTruncated)),
        "{warnings:?}"
    );
    unsafe {
        assert_eq!(iv.interval_value.day_second.fraction, 765_432);
    }
}

#[test]
fn minute_to_second_truncated_fraction_warns() {
    let (iv, r) = run("45:30.1234567", CDataType::IntervalMinuteToSecond);
    let warnings = r.expect("parse should succeed");
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, Warning::NumericValueTruncated)),
        "{warnings:?}"
    );
    unsafe {
        assert_eq!(iv.interval_value.day_second.fraction, 123_456);
    }
}

// ============================================================================
// 22015 leading-precision overflow
// ============================================================================

#[test]
fn single_field_year_overflow_returns_22015_at_default_precision() {
    // Default `datetime_interval_precision = 2` rejects values >= 100.
    let (_, r) = run("999", CDataType::IntervalYear);
    let err = r.expect_err("999 exceeds default precision of 2 digits");
    assert!(
        matches!(err, WriteOdbcError::IntervalFieldOverflow { .. }),
        "expected 22015 (IntervalFieldOverflow), got {err:?}"
    );
}

#[test]
fn composite_year_to_month_overflow_returns_22015_at_default_precision() {
    let (_, r) = run("999-3", CDataType::IntervalYearToMonth);
    let err = r.expect_err("year=999 exceeds default precision of 2 digits");
    assert!(
        matches!(err, WriteOdbcError::IntervalFieldOverflow { .. }),
        "expected 22015 (IntervalFieldOverflow), got {err:?}"
    );
}

#[test]
fn u128_overflow_returns_22015_not_22018() {
    // Value too large to fit u128 — the literal IS a valid integer
    // but exceeds storage; spec mandates 22015, not 22018.
    let huge = "1".repeat(40); // ~10^40, well past u128 (10^38).
    let (_, r) = run(&huge, CDataType::IntervalYear);
    let err = r.expect_err("u128-overflowing input must fail");
    assert!(
        matches!(err, WriteOdbcError::IntervalFieldOverflow { .. }),
        "expected 22015 (IntervalFieldOverflow), got {err:?}"
    );
}

// ============================================================================
// Explicit datetime_interval_precision honored
// ============================================================================

#[test]
fn explicit_precision_allows_value_within_range() {
    let mut buf = make_buffer();
    let binding = make_binding_with_precision(CDataType::IntervalYear, &mut buf, 5);
    let r = crate::conversion::interval_str::varchar_to_interval(
        "12345",
        CDataType::IntervalYear,
        &binding,
    );
    let warnings = r.expect("12345 fits within 5 leading-precision digits");
    assert!(warnings.is_empty(), "{warnings:?}");
    unsafe {
        assert_eq!(buf.interval_value.year_month.year, 12345);
    }
}

#[test]
fn explicit_precision_rejects_value_exceeding_range() {
    let mut buf = make_buffer();
    let binding = make_binding_with_precision(CDataType::IntervalYear, &mut buf, 3);
    let r = crate::conversion::interval_str::varchar_to_interval(
        "9999",
        CDataType::IntervalYear,
        &binding,
    );
    let err = r.expect_err("9999 exceeds 3 leading-precision digits");
    assert!(
        matches!(err, WriteOdbcError::IntervalFieldOverflow { .. }),
        "{err:?}"
    );
}

// ============================================================================
// Trailing-field truncation into composite targets
// ============================================================================

#[test]
fn day_to_second_truncates_into_day_to_hour() {
    // `5 10:30:45` routed to IntervalDayToHour — the minute/second
    // fields must be dropped with 01S07.
    let (iv, r) = run("5 10:30:45", CDataType::IntervalDayToHour);
    let warnings = r.expect("parse should succeed");
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, Warning::NumericValueTruncated)),
        "expected 01S07 truncation warning, got {warnings:?}"
    );
    unsafe {
        assert_eq!(iv.interval_value.day_second.day, 5);
        assert_eq!(iv.interval_value.day_second.hour, 10);
    }
}

#[test]
fn hour_to_second_truncates_into_hour_to_minute() {
    // `12:30:45` routed to IntervalHourToMinute — the second field
    // is dropped with 01S07.
    let (iv, r) = run("12:30:45", CDataType::IntervalHourToMinute);
    let warnings = r.expect("parse should succeed");
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, Warning::NumericValueTruncated)),
        "expected 01S07 truncation warning, got {warnings:?}"
    );
    unsafe {
        assert_eq!(iv.interval_value.day_second.hour, 12);
        assert_eq!(iv.interval_value.day_second.minute, 30);
    }
}

#[test]
fn year_to_month_truncates_into_month_target() {
    // Symmetric to `year_to_month_into_year_truncates_month`: this
    // time the YEAR field is dropped with 01S07 and only the month
    // survives.
    let (iv, r) = run("3-6", CDataType::IntervalMonth);
    let warnings = r.expect("parse should succeed");
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, Warning::NumericValueTruncated)),
        "expected 01S07 truncation warning, got {warnings:?}"
    );
    unsafe {
        assert_eq!(iv.interval_value.year_month.month, 6);
    }
}

// ============================================================================
// Defensive arm in varchar_to_interval
// ============================================================================

#[test]
fn non_interval_target_returns_07006() {
    // Direct call into `varchar_to_interval` with a non-interval
    // target should map to 07006 (UnsupportedOdbcType), not 22003.
    let mut buf = make_buffer();
    let binding = make_binding(CDataType::SLong, &mut buf);
    let r = crate::conversion::interval_str::varchar_to_interval("5", CDataType::SLong, &binding);
    let err = r.expect_err("non-interval target must fail");
    assert!(
        matches!(err, WriteOdbcError::UnsupportedOdbcType { .. }),
        "expected 07006 (UnsupportedOdbcType), got {err:?}"
    );
}

// ============================================================================
// Whitespace handling
//
// `split_sign` calls `s.trim()` before extracting the sign — these tests
// pin that down at the unit level so any future regression that breaks the
// outer trim is caught here, not only in the e2e suite.
// ============================================================================

#[test]
fn outer_whitespace_is_trimmed_for_single_field() {
    // Plain integer with leading + trailing ASCII whitespace.
    let (iv, r) = run("  5  ", CDataType::IntervalYear);
    let warnings = r.expect("parse should succeed after trim");
    assert!(warnings.is_empty(), "{warnings:?}");
    unsafe {
        assert_eq!(iv.interval_value.year_month.year, 5);
    }
    assert_eq!(iv.interval_sign, 0);
}

#[test]
fn outer_whitespace_is_trimmed_for_signed_value() {
    // Whitespace must be stripped on both sides of the sign.
    let (iv, r) = run("  -7\t", CDataType::IntervalDay);
    let warnings = r.expect("parse should succeed after trim");
    assert!(warnings.is_empty(), "{warnings:?}");
    unsafe {
        assert_eq!(iv.interval_value.day_second.day, 7);
    }
    assert_eq!(iv.interval_sign, 1);
}

#[test]
fn outer_whitespace_is_trimmed_for_composite() {
    // Outer trim works on composite literals too; inner spaces in
    // `<years>-<months>` remain rejected (covered separately).
    let (iv, r) = run("\n 3-6 \n", CDataType::IntervalYearToMonth);
    let warnings = r.expect("parse should succeed after trim");
    assert!(warnings.is_empty(), "{warnings:?}");
    unsafe {
        assert_eq!(iv.interval_value.year_month.year, 3);
        assert_eq!(iv.interval_value.year_month.month, 6);
    }
}

// ============================================================================
// Strict ANSI range enforcement on TRAILING composite-interval fields.
//
// Per ODBC Appendix D outcome #4, a trailing-field magnitude outside the
// canonical ANSI range is "not a valid interval value" → SQLSTATE 22018.
// Leading fields keep the precision-based 22015 check (covered above) since
// ANSI permits a leading field to be arbitrarily large within its declared
// `SQL_DESC_DATETIME_INTERVAL_PRECISION`.
//
// Enforced trailing ranges:
//   YEAR_TO_MONTH   trailing MONTH  : 0..=11
//   *_TO_HOUR       trailing HOUR   : 0..=23
//   *_TO_MINUTE     trailing MINUTE : 0..=59
//   *_TO_SECOND     trailing SECOND : 0..=59
// ============================================================================

/// Variant of `run` that lets the test set
/// `SQL_DESC_DATETIME_INTERVAL_PRECISION` on the binding. The default in
/// `make_binding` is `None`, which `check_leading_precision` treats as
/// precision = 2 — too narrow to exercise the leading vs. trailing
/// asymmetry on its own.
fn run_with_precision(
    value: &str,
    target: CDataType,
    precision: i16,
) -> (sql::IntervalStruct, Result<Vec<Warning>, WriteOdbcError>) {
    let mut buf = make_buffer();
    let mut binding = make_binding(target, &mut buf);
    binding.datetime_interval_precision = Some(precision);
    let r = varchar_to_interval(value, target, &binding);
    (*buf, r)
}

fn assert_invalid_value(err: WriteOdbcError, msg: &str) {
    assert!(
        matches!(err, WriteOdbcError::InvalidValue { .. }),
        "{msg}: expected InvalidValue (22018), got {err:?}"
    );
}

#[test]
fn hour_to_minute_trailing_minute_above_59_returns_22018() {
    let (_, r) = run("5:60", CDataType::IntervalHourToMinute);
    assert_invalid_value(
        r.expect_err("minute=60 is out of range 0..=59"),
        "'5:60' -> IntervalHourToMinute",
    );
}

#[test]
fn hour_to_minute_minute_99_returns_22018() {
    let (_, r) = run("5:99", CDataType::IntervalHourToMinute);
    assert_invalid_value(
        r.expect_err("minute=99 is out of range 0..=59"),
        "'5:99' -> IntervalHourToMinute",
    );
}

#[test]
fn hour_to_second_trailing_second_above_59_returns_22018() {
    let (_, r) = run("5:59:60", CDataType::IntervalHourToSecond);
    assert_invalid_value(
        r.expect_err("second=60 is out of range 0..=59"),
        "'5:59:60' -> IntervalHourToSecond",
    );
}

#[test]
fn hour_to_second_trailing_minute_above_59_returns_22018() {
    let (_, r) = run("5:60:0", CDataType::IntervalHourToSecond);
    assert_invalid_value(
        r.expect_err("minute=60 is out of range 0..=59"),
        "'5:60:0' -> IntervalHourToSecond",
    );
}

#[test]
fn day_to_hour_trailing_hour_above_23_returns_22018() {
    // "5 24" → day=5 (leading), hour=24 (trailing). The leading day
    // field is unconstrained by the ANSI range; the trailing hour
    // must satisfy 0..=23.
    let (_, r) = run("5 24", CDataType::IntervalDayToHour);
    assert_invalid_value(
        r.expect_err("hour=24 is out of range 0..=23"),
        "'5 24' -> IntervalDayToHour",
    );
}

#[test]
fn day_to_hour_via_three_component_time_rejects_hour_above_23() {
    // Wider input ('5 24:0:0') routed to a narrower target still
    // applies the trailing-field range check before any 01S07
    // truncation handling.
    let (_, r) = run("5 24:0:0", CDataType::IntervalDayToHour);
    assert_invalid_value(
        r.expect_err("hour=24 is out of range 0..=23"),
        "'5 24:0:0' -> IntervalDayToHour",
    );
}

#[test]
fn day_to_minute_trailing_minute_above_59_returns_22018() {
    let (_, r) = run("5 0:60:0", CDataType::IntervalDayToMinute);
    assert_invalid_value(
        r.expect_err("minute=60 is out of range 0..=59"),
        "'5 0:60:0' -> IntervalDayToMinute",
    );
}

#[test]
fn day_to_second_trailing_second_above_59_returns_22018() {
    let (_, r) = run("5 0:0:60", CDataType::IntervalDayToSecond);
    assert_invalid_value(
        r.expect_err("second=60 is out of range 0..=59"),
        "'5 0:0:60' -> IntervalDayToSecond",
    );
}

#[test]
fn day_to_second_trailing_hour_above_23_returns_22018() {
    let (_, r) = run("5 25:0:0", CDataType::IntervalDayToSecond);
    assert_invalid_value(
        r.expect_err("hour=25 is out of range 0..=23"),
        "'5 25:0:0' -> IntervalDayToSecond",
    );
}

#[test]
fn year_to_month_trailing_month_12_returns_22018() {
    let (_, r) = run("3-12", CDataType::IntervalYearToMonth);
    assert_invalid_value(
        r.expect_err("month=12 is out of range 0..=11"),
        "'3-12' -> IntervalYearToMonth",
    );
}

#[test]
fn year_to_month_trailing_month_99_returns_22018() {
    let (_, r) = run("3-99", CDataType::IntervalYearToMonth);
    assert_invalid_value(
        r.expect_err("month=99 is out of range 0..=11"),
        "'3-99' -> IntervalYearToMonth",
    );
}

#[test]
fn minute_to_second_trailing_second_60_returns_22018() {
    // Without fraction: parse_any_shape stores "M:S" as (hour=M,
    // minute=S); build_composite re-interprets for MinuteToSecond.
    // The leading minute (45) is precision-driven; the trailing
    // second (60) must satisfy 0..=59.
    let (_, r) = run("45:60", CDataType::IntervalMinuteToSecond);
    assert_invalid_value(
        r.expect_err("second=60 is out of range 0..=59"),
        "'45:60' -> IntervalMinuteToSecond",
    );
}

#[test]
fn minute_to_second_with_fraction_rejects_second_60() {
    // Fraction-bearing path: parse_any_shape stores directly as
    // (minute=M, second=S, frac). The trailing second still gets
    // range-checked.
    let (_, r) = run("45:60.5", CDataType::IntervalMinuteToSecond);
    assert_invalid_value(
        r.expect_err("second=60 is out of range 0..=59"),
        "'45:60.5' -> IntervalMinuteToSecond",
    );
}

#[test]
fn minute_to_second_leading_minute_unchecked_against_ansi_range() {
    // The LEADING minute slot in MINUTE_TO_SECOND is precision-driven,
    // not range-checked: with a wide enough precision a value above 59
    // is accepted as long as the trailing second stays canonical. This
    // pins the asymmetry between leading and trailing slots.
    let (iv, r) = run_with_precision("99:30", CDataType::IntervalMinuteToSecond, 3);
    r.expect("leading minute=99 fits precision=3 and is not range-checked");
    unsafe {
        assert_eq!(iv.interval_value.day_second.minute, 99);
        assert_eq!(iv.interval_value.day_second.second, 30);
    }
}

// ----------------------------------------------------------------------------
// Boundary cases: the ANSI ceiling is exclusive (24/60/12 reject) and the
// max value (23/59/11) is accepted.
// ----------------------------------------------------------------------------

#[test]
fn hour_to_minute_minute_59_accepted_at_boundary() {
    let (iv, r) = run("5:59", CDataType::IntervalHourToMinute);
    r.expect("minute=59 is the inclusive max");
    unsafe {
        assert_eq!(iv.interval_value.day_second.hour, 5);
        assert_eq!(iv.interval_value.day_second.minute, 59);
    }
}

#[test]
fn hour_to_second_minute_and_second_59_accepted_at_boundary() {
    let (iv, r) = run("5:59:59", CDataType::IntervalHourToSecond);
    r.expect("minute=59 and second=59 are the inclusive max");
    unsafe {
        assert_eq!(iv.interval_value.day_second.hour, 5);
        assert_eq!(iv.interval_value.day_second.minute, 59);
        assert_eq!(iv.interval_value.day_second.second, 59);
    }
}

#[test]
fn year_to_month_month_11_accepted_at_boundary() {
    let (iv, r) = run("3-11", CDataType::IntervalYearToMonth);
    r.expect("month=11 is the inclusive max");
    unsafe {
        assert_eq!(iv.interval_value.year_month.year, 3);
        assert_eq!(iv.interval_value.year_month.month, 11);
    }
}

#[test]
fn day_to_hour_hour_23_accepted_at_boundary() {
    let (iv, r) = run("5 23", CDataType::IntervalDayToHour);
    r.expect("hour=23 is the inclusive max");
    unsafe {
        assert_eq!(iv.interval_value.day_second.day, 5);
        assert_eq!(iv.interval_value.day_second.hour, 23);
    }
}

#[test]
fn day_to_second_full_boundary_accepted() {
    let (iv, r) = run("5 23:59:59", CDataType::IntervalDayToSecond);
    r.expect("hour=23, minute=59, second=59 are the inclusive max");
    unsafe {
        assert_eq!(iv.interval_value.day_second.day, 5);
        assert_eq!(iv.interval_value.day_second.hour, 23);
        assert_eq!(iv.interval_value.day_second.minute, 59);
        assert_eq!(iv.interval_value.day_second.second, 59);
    }
}

#[test]
fn minute_to_second_second_59_accepted_at_boundary() {
    let (iv, r) = run("45:59", CDataType::IntervalMinuteToSecond);
    r.expect("second=59 is the inclusive max");
    unsafe {
        assert_eq!(iv.interval_value.day_second.minute, 45);
        assert_eq!(iv.interval_value.day_second.second, 59);
    }
}
