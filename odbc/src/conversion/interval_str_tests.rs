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
    // ODBC normalises fractional seconds to 6 digits (microseconds);
    // anything past the 6th digit is silently truncated.
    let (iv, r) = run("0.1234567", CDataType::IntervalSecond);
    r.expect("parse should succeed");
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
            .any(|w| matches!(w, Warning::StringDataTruncated)),
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
            .any(|w| matches!(w, Warning::StringDataTruncated)),
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
