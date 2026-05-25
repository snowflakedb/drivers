//! Unit tests for the dedicated SQL_INTERVAL_* bind-parameter converters.
//!
//! Each test goes through `make_converter` (the factory used by
//! `odbc_bindings_to_json`) so the routing from SQL type → converter is
//! exercised alongside the per-source conversion path. The tests are
//! organised into three groups:
//!
//!   * Positive paths — every legal C source for both single-field and
//!     compound interval targets, covering character types, all exact
//!     numeric C types (single-field only), `SQL_C_NUMERIC` (with both
//!     integer and fractional input for SECOND), and same-family
//!     SQL_C_INTERVAL_* sources.
//!   * Negative paths — illegal C sources (FLOAT/DOUBLE/BINARY/GUID,
//!     date/time C types) and family mismatches (e.g. day-time C interval
//!     into a year-month SQL target). All must fail with SQLSTATE 07006
//!     (`UnsupportedCDataType`).
//!   * `SnowflakeLogicalType` propagation — confirms the JSON `type`
//!     field reads `INTERVAL_YEAR_MONTH` / `INTERVAL_DAY_TIME` rather than
//!     the legacy `TEXT`.

use odbc_sys as sql;
use serde_json::Value;

use crate::api::{ApdRecord, CDataType, IpdRecord, ParameterBinding};
use crate::conversion::error::JsonBindingError;
use crate::conversion::param_binding;
use crate::conversion::traits::SnowflakeLogicalType;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn make_binding(
    value_type: CDataType,
    parameter_type: sql::SqlDataType,
    ptr: sql::Pointer,
    buffer_length: sql::Len,
    ind_ptr: *mut sql::Len,
) -> ParameterBinding {
    let apd = ApdRecord {
        value_type,
        data_ptr: ptr,
        buffer_length,
        str_len_or_ind_ptr: ind_ptr,
    };
    let ipd = IpdRecord {
        sql_data_type: parameter_type,
        ..IpdRecord::default()
    };
    ParameterBinding::from_apd_ipd(&apd, &ipd)
}

fn convert(binding: &ParameterBinding) -> Result<(SnowflakeLogicalType, Value), JsonBindingError> {
    param_binding::convert_for_test(binding)
}

fn ds_struct(
    sign: sql::SmallInt,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    fraction: u32,
) -> sql::IntervalStruct {
    sql::IntervalStruct {
        interval_type: 0,
        interval_sign: sign,
        interval_value: sql::IntervalUnion {
            day_second: sql::DaySecond {
                day,
                hour,
                minute,
                second,
                fraction,
            },
        },
    }
}

fn ym_struct(sign: sql::SmallInt, year: u32, month: u32) -> sql::IntervalStruct {
    sql::IntervalStruct {
        interval_type: 0,
        interval_sign: sign,
        interval_value: sql::IntervalUnion {
            year_month: sql::YearMonth { year, month },
        },
    }
}

// =============================================================================
// Positive: exact-numeric C source → single-field SQL target.
// =============================================================================

#[test]
fn slong_to_year_emits_integer_literal_with_year_month_logical_type() -> TestResult {
    let value: i32 = 7;
    let binding = make_binding(
        CDataType::SLong,
        sql::SqlDataType(101), // SQL_INTERVAL_YEAR
        &value as *const _ as sql::Pointer,
        std::mem::size_of::<i32>() as sql::Len,
        std::ptr::null_mut(),
    );
    let (ty, v) = convert(&binding)?;
    assert_eq!(ty, SnowflakeLogicalType::IntervalYearMonth);
    assert_eq!(v, Value::String("7".to_string()));
    Ok(())
}

#[test]
fn negative_sbigint_to_month_preserves_sign() -> TestResult {
    let value: i64 = -42;
    let binding = make_binding(
        CDataType::SBigInt,
        sql::SqlDataType(102), // SQL_INTERVAL_MONTH
        &value as *const _ as sql::Pointer,
        std::mem::size_of::<i64>() as sql::Len,
        std::ptr::null_mut(),
    );
    let (_, v) = convert(&binding)?;
    assert_eq!(v, Value::String("-42".to_string()));
    Ok(())
}

#[test]
fn ulong_to_day_renders_unsigned_integer() -> TestResult {
    let value: u32 = 365;
    let binding = make_binding(
        CDataType::ULong,
        sql::SqlDataType(103), // SQL_INTERVAL_DAY
        &value as *const _ as sql::Pointer,
        std::mem::size_of::<u32>() as sql::Len,
        std::ptr::null_mut(),
    );
    let (ty, v) = convert(&binding)?;
    assert_eq!(ty, SnowflakeLogicalType::IntervalDayTime);
    assert_eq!(v, Value::String("365".to_string()));
    Ok(())
}

#[test]
fn bit_to_hour_renders_zero_or_one() -> TestResult {
    for &b in &[0u8, 1u8] {
        let binding = make_binding(
            CDataType::Bit,
            sql::SqlDataType(104), // SQL_INTERVAL_HOUR
            &b as *const _ as sql::Pointer,
            std::mem::size_of::<u8>() as sql::Len,
            std::ptr::null_mut(),
        );
        let (_, v) = convert(&binding)?;
        assert_eq!(v, Value::String(b.to_string()));
    }
    Ok(())
}

#[test]
fn slong_to_second_appends_default_fraction_width() -> TestResult {
    // Per ODBC "Interval Data Type Length", the seconds component
    // contributes "1 plus seconds precision" characters. With the spec
    // default of 6, a SECOND value bound from an exact-integer C type
    // must render with the canonical ".000000" suffix so consumers that
    // expect the full literal width keep round-tripping.
    let value: i32 = 30;
    let binding = make_binding(
        CDataType::SLong,
        sql::SqlDataType(106), // SQL_INTERVAL_SECOND
        &value as *const _ as sql::Pointer,
        std::mem::size_of::<i32>() as sql::Len,
        std::ptr::null_mut(),
    );
    let (_, v) = convert(&binding)?;
    assert_eq!(v, Value::String("30.000000".to_string()));
    Ok(())
}

// =============================================================================
// Positive: SQL_C_NUMERIC source (with and without fractional digits).
// =============================================================================

fn numeric_struct(precision: u8, scale: i8, sign: u8, magnitude: u128) -> sql::Numeric {
    sql::Numeric {
        precision,
        scale,
        sign,
        val: magnitude.to_le_bytes(),
    }
}

#[test]
fn numeric_integer_to_minute_emits_truncated_integer() -> TestResult {
    // NUMERIC(5,0) value = 90 → "90".
    let n = numeric_struct(5, 0, 1, 90);
    let binding = make_binding(
        CDataType::Numeric,
        sql::SqlDataType(105), // SQL_INTERVAL_MINUTE
        &n as *const _ as sql::Pointer,
        std::mem::size_of::<sql::Numeric>() as sql::Len,
        std::ptr::null_mut(),
    );
    let (_, v) = convert(&binding)?;
    assert_eq!(v, Value::String("90".to_string()));
    Ok(())
}

#[test]
fn numeric_with_fraction_to_year_truncates_to_integer() -> TestResult {
    // NUMERIC(5,2) value 12345 / 10^2 = 123.45 → "123" for non-SECOND
    // single-field targets (server reports 22015 truncation warning).
    let n = numeric_struct(5, 2, 1, 12345);
    let binding = make_binding(
        CDataType::Numeric,
        sql::SqlDataType(101), // SQL_INTERVAL_YEAR
        &n as *const _ as sql::Pointer,
        std::mem::size_of::<sql::Numeric>() as sql::Len,
        std::ptr::null_mut(),
    );
    let (_, v) = convert(&binding)?;
    assert_eq!(v, Value::String("123".to_string()));
    Ok(())
}

#[test]
fn numeric_with_fraction_to_second_preserves_microseconds() -> TestResult {
    // NUMERIC(7,3) value 5500 / 10^3 = 5.500 → "5.500000".
    let n = numeric_struct(7, 3, 1, 5500);
    let binding = make_binding(
        CDataType::Numeric,
        sql::SqlDataType(106), // SQL_INTERVAL_SECOND
        &n as *const _ as sql::Pointer,
        std::mem::size_of::<sql::Numeric>() as sql::Len,
        std::ptr::null_mut(),
    );
    let (_, v) = convert(&binding)?;
    assert_eq!(v, Value::String("5.500000".to_string()));
    Ok(())
}

#[test]
fn numeric_with_excess_scale_to_second_truncates_to_microseconds() -> TestResult {
    // NUMERIC(12,9) value 5_123_456_789 / 10^9 = 5.123456789 →
    // "5.123456" (the trailing 789 ns is truncated; spec-default
    // seconds precision = 6, matching `compute_interval_fraction`).
    let n = numeric_struct(12, 9, 1, 5_123_456_789);
    let binding = make_binding(
        CDataType::Numeric,
        sql::SqlDataType(106), // SQL_INTERVAL_SECOND
        &n as *const _ as sql::Pointer,
        std::mem::size_of::<sql::Numeric>() as sql::Len,
        std::ptr::null_mut(),
    );
    let (_, v) = convert(&binding)?;
    assert_eq!(v, Value::String("5.123456".to_string()));
    Ok(())
}

#[test]
fn negative_numeric_to_second_keeps_sign_and_fraction() -> TestResult {
    let n = numeric_struct(7, 3, 0, 5500); // sign=0 ⇒ negative
    let binding = make_binding(
        CDataType::Numeric,
        sql::SqlDataType(106), // SQL_INTERVAL_SECOND
        &n as *const _ as sql::Pointer,
        std::mem::size_of::<sql::Numeric>() as sql::Len,
        std::ptr::null_mut(),
    );
    let (_, v) = convert(&binding)?;
    assert_eq!(v, Value::String("-5.500000".to_string()));
    Ok(())
}

// =============================================================================
// Positive: same-family SQL_C_INTERVAL_* sources.
// =============================================================================

#[test]
fn c_interval_year_to_sql_year_to_month_target_renders_compound_literal() -> TestResult {
    // Cross-subtype but same-family is permitted (year-month → year-month);
    // `format_interval` uses the C subtype to choose which fields to read,
    // so SQL_C_INTERVAL_YEAR source bound to SQL_INTERVAL_YEAR_TO_MONTH
    // yields just the year field, which is already a valid YEAR_TO_MONTH
    // literal at month=00.
    let iv = ym_struct(0, 5, 0);
    let binding = make_binding(
        CDataType::IntervalYear,
        sql::SqlDataType(107), // SQL_INTERVAL_YEAR_TO_MONTH
        &iv as *const _ as sql::Pointer,
        std::mem::size_of::<sql::IntervalStruct>() as sql::Len,
        std::ptr::null_mut(),
    );
    let (ty, v) = convert(&binding)?;
    assert_eq!(ty, SnowflakeLogicalType::IntervalYearMonth);
    assert_eq!(v, Value::String("5".to_string()));
    Ok(())
}

#[test]
fn c_interval_day_to_second_to_sql_minute_target_renders_seconds() -> TestResult {
    // Same-family day-time C interval source bound to a single-field
    // SQL_INTERVAL_MINUTE target: the literal text comes from the C
    // subtype, not the SQL target, which is the correct pre-#980 behaviour
    // we preserve.
    let iv = ds_struct(0, 1, 2, 3, 4, 0);
    let binding = make_binding(
        CDataType::IntervalDayToSecond,
        sql::SqlDataType(105), // SQL_INTERVAL_MINUTE
        &iv as *const _ as sql::Pointer,
        std::mem::size_of::<sql::IntervalStruct>() as sql::Len,
        std::ptr::null_mut(),
    );
    let (_, v) = convert(&binding)?;
    assert_eq!(v, Value::String("1 02:03:04.000000".to_string()));
    Ok(())
}

// =============================================================================
// Negative: cross-family interval source.
// =============================================================================

#[test]
fn c_interval_day_to_sql_year_target_rejected_07006() {
    let iv = ds_struct(0, 5, 0, 0, 0, 0);
    let binding = make_binding(
        CDataType::IntervalDay,
        sql::SqlDataType(101), // SQL_INTERVAL_YEAR
        &iv as *const _ as sql::Pointer,
        std::mem::size_of::<sql::IntervalStruct>() as sql::Len,
        std::ptr::null_mut(),
    );
    let err = convert(&binding).expect_err("cross-family must error");
    assert!(
        matches!(err, JsonBindingError::UnsupportedCDataType { .. }),
        "expected UnsupportedCDataType (07006), got {err:?}",
    );
}

#[test]
fn c_interval_year_to_sql_day_target_rejected_07006() {
    let iv = ym_struct(0, 5, 0);
    let binding = make_binding(
        CDataType::IntervalYear,
        sql::SqlDataType(103), // SQL_INTERVAL_DAY
        &iv as *const _ as sql::Pointer,
        std::mem::size_of::<sql::IntervalStruct>() as sql::Len,
        std::ptr::null_mut(),
    );
    let err = convert(&binding).expect_err("cross-family must error");
    assert!(
        matches!(err, JsonBindingError::UnsupportedCDataType { .. }),
        "expected UnsupportedCDataType (07006), got {err:?}",
    );
}

// =============================================================================
// Negative: numeric C source → compound SQL target.
// =============================================================================

#[test]
fn slong_to_year_to_month_rejected_07006() {
    // ODBC Appendix D explicitly disallows numeric C sources for any
    // multi-field interval target — a single integer can't carry both
    // years and months (even SQL_C_NUMERIC, despite carrying scale).
    let value: i32 = 5;
    let binding = make_binding(
        CDataType::SLong,
        sql::SqlDataType(107), // SQL_INTERVAL_YEAR_TO_MONTH
        &value as *const _ as sql::Pointer,
        std::mem::size_of::<i32>() as sql::Len,
        std::ptr::null_mut(),
    );
    let err = convert(&binding).expect_err("numeric → compound must error");
    assert!(
        matches!(err, JsonBindingError::UnsupportedCDataType { .. }),
        "expected UnsupportedCDataType (07006), got {err:?}",
    );
}

#[test]
fn numeric_to_day_to_second_rejected_07006() {
    let n = numeric_struct(7, 3, 1, 5500);
    let binding = make_binding(
        CDataType::Numeric,
        sql::SqlDataType(110), // SQL_INTERVAL_DAY_TO_SECOND
        &n as *const _ as sql::Pointer,
        std::mem::size_of::<sql::Numeric>() as sql::Len,
        std::ptr::null_mut(),
    );
    let err = convert(&binding).expect_err("numeric → compound must error");
    assert!(
        matches!(err, JsonBindingError::UnsupportedCDataType { .. }),
        "expected UnsupportedCDataType (07006), got {err:?}",
    );
}

#[test]
fn ubigint_to_hour_to_minute_rejected_07006() {
    let value: u64 = 14;
    let binding = make_binding(
        CDataType::UBigInt,
        sql::SqlDataType(111), // SQL_INTERVAL_HOUR_TO_MINUTE
        &value as *const _ as sql::Pointer,
        std::mem::size_of::<u64>() as sql::Len,
        std::ptr::null_mut(),
    );
    let err = convert(&binding).expect_err("integer → compound must error");
    assert!(
        matches!(err, JsonBindingError::UnsupportedCDataType { .. }),
        "expected UnsupportedCDataType (07006), got {err:?}",
    );
}

// =============================================================================
// Negative: never-legal C sources (FLOAT/DOUBLE/BINARY/GUID/DATE/TIME).
// =============================================================================

#[test]
fn float_to_year_target_rejected_07006() {
    let value: f32 = 5.5;
    let binding = make_binding(
        CDataType::Float,
        sql::SqlDataType(101), // SQL_INTERVAL_YEAR
        &value as *const _ as sql::Pointer,
        std::mem::size_of::<f32>() as sql::Len,
        std::ptr::null_mut(),
    );
    let err = convert(&binding).expect_err("FLOAT → INTERVAL must error");
    assert!(
        matches!(err, JsonBindingError::UnsupportedCDataType { .. }),
        "expected UnsupportedCDataType (07006), got {err:?}",
    );
}

#[test]
fn double_to_second_target_rejected_07006() {
    let value: f64 = 5.5;
    let binding = make_binding(
        CDataType::Double,
        sql::SqlDataType(106), // SQL_INTERVAL_SECOND
        &value as *const _ as sql::Pointer,
        std::mem::size_of::<f64>() as sql::Len,
        std::ptr::null_mut(),
    );
    let err = convert(&binding).expect_err("DOUBLE → INTERVAL must error");
    assert!(
        matches!(err, JsonBindingError::UnsupportedCDataType { .. }),
        "expected UnsupportedCDataType (07006), got {err:?}",
    );
}

#[test]
fn binary_to_year_target_rejected_07006() {
    let buf: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
    let mut len: sql::Len = 4;
    let binding = make_binding(
        CDataType::Binary,
        sql::SqlDataType(101), // SQL_INTERVAL_YEAR
        buf.as_ptr() as sql::Pointer,
        buf.len() as sql::Len,
        &mut len,
    );
    let err = convert(&binding).expect_err("BINARY → INTERVAL must error");
    assert!(
        matches!(err, JsonBindingError::UnsupportedCDataType { .. }),
        "expected UnsupportedCDataType (07006), got {err:?}",
    );
}

#[test]
fn guid_to_year_to_month_target_rejected_07006() {
    let g = sql::Guid {
        d1: 0x1234_5678,
        d2: 0,
        d3: 0,
        d4: [0; 8],
    };
    let binding = make_binding(
        CDataType::Guid,
        sql::SqlDataType(107), // SQL_INTERVAL_YEAR_TO_MONTH
        &g as *const _ as sql::Pointer,
        std::mem::size_of::<sql::Guid>() as sql::Len,
        std::ptr::null_mut(),
    );
    let err = convert(&binding).expect_err("GUID → INTERVAL must error");
    assert!(
        matches!(err, JsonBindingError::UnsupportedCDataType { .. }),
        "expected UnsupportedCDataType (07006), got {err:?}",
    );
}

#[test]
fn date_to_year_target_rejected_07006() {
    let d = sql::Date {
        year: 2026,
        month: 5,
        day: 6,
    };
    let binding = make_binding(
        CDataType::TypeDate,
        sql::SqlDataType(101), // SQL_INTERVAL_YEAR
        &d as *const _ as sql::Pointer,
        std::mem::size_of::<sql::Date>() as sql::Len,
        std::ptr::null_mut(),
    );
    let err = convert(&binding).expect_err("DATE → INTERVAL must error");
    assert!(
        matches!(err, JsonBindingError::UnsupportedCDataType { .. }),
        "expected UnsupportedCDataType (07006), got {err:?}",
    );
}

#[test]
fn timestamp_to_day_to_second_rejected_07006() {
    let t = sql::Timestamp {
        year: 2026,
        month: 5,
        day: 6,
        hour: 12,
        minute: 0,
        second: 0,
        fraction: 0,
    };
    let binding = make_binding(
        CDataType::TypeTimestamp,
        sql::SqlDataType(110), // SQL_INTERVAL_DAY_TO_SECOND
        &t as *const _ as sql::Pointer,
        std::mem::size_of::<sql::Timestamp>() as sql::Len,
        std::ptr::null_mut(),
    );
    let err = convert(&binding).expect_err("TIMESTAMP → INTERVAL must error");
    assert!(
        matches!(err, JsonBindingError::UnsupportedCDataType { .. }),
        "expected UnsupportedCDataType (07006), got {err:?}",
    );
}

// =============================================================================
// Logical-type propagation.
// =============================================================================

#[test]
fn char_source_to_year_to_month_target_advertises_year_month_logical_type() -> TestResult {
    let s = b"5-11\0";
    let mut len: sql::Len = 4;
    let binding = make_binding(
        CDataType::Char,
        sql::SqlDataType(107), // SQL_INTERVAL_YEAR_TO_MONTH
        s.as_ptr() as sql::Pointer,
        5,
        &mut len,
    );
    let (ty, v) = convert(&binding)?;
    assert_eq!(ty, SnowflakeLogicalType::IntervalYearMonth);
    assert_eq!(v, Value::String("5-11".to_string()));
    Ok(())
}

#[test]
fn char_source_to_minute_to_second_target_advertises_day_time_logical_type() -> TestResult {
    let s = b"30:45.500000\0";
    let mut len: sql::Len = 12;
    let binding = make_binding(
        CDataType::Char,
        sql::SqlDataType(113), // SQL_INTERVAL_MINUTE_TO_SECOND
        s.as_ptr() as sql::Pointer,
        13,
        &mut len,
    );
    let (ty, v) = convert(&binding)?;
    assert_eq!(ty, SnowflakeLogicalType::IntervalDayTime);
    assert_eq!(v, Value::String("30:45.500000".to_string()));
    Ok(())
}
