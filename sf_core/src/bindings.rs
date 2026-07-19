//! Snowflake parameter-binding JSON construction from typed Rust values.
//!
//! This module builds the `bindings` JSON object Snowflake's query API expects
//! for server-side parameter binding, directly from a typed [`ParamValue`]
//! enum. The wire-text encodings are byte-for-byte identical to the ODBC
//! converters in `odbc/src/conversion/*` — the two paths must agree so a query
//! bound through either driver produces the same server-side value.
//!
//! ## Why every value is a JSON string
//!
//! Every Snowflake bind value travels on the wire as text: the JSON envelope
//! is `{"type": "<logical type>", "value": "<wire text>"}` and the server
//! parses `value` according to `type`. Emitting a bare JSON number or boolean
//! would change the parse path and is rejected by the server, so non-NULL
//! values are always [`serde_json::Value::String`]; NULL cells are JSON
//! `null`.
//!
//! ## Output shape
//!
//! Keys are 1-indexed strings (`"1"` is the first bind variable):
//!
//! - Single parameter set: `{"1": {"type": "FIXED", "value": "123"}, ...}`
//! - Array binding:        `{"1": {"type": "FIXED", "value": ["1", null, "3"]}, ...}`
//!
//! For an array binding the per-parameter `type` is derived from the first
//! non-NULL value in the column; a column that is entirely NULL is tagged
//! `ANY`.
//!
//! ## Interval values
//!
//! `INTERVAL_YEAR_MONTH` and `INTERVAL_DAY_TIME` are supplied as pre-formatted
//! literal strings and passed to the server verbatim — the builder does not
//! assemble them from components. See [`ParamValue::IntervalYearMonth`] and
//! [`ParamValue::IntervalDayTime`] for the accepted grammar and examples.

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, Snafu};

/// The Unix epoch (1970-01-01), used as the origin for `DATE` encoding.
///
/// Built with a `const` match rather than `.unwrap()` so the compiler proves
/// the date is valid; `from_ymd_opt(1970, 1, 1)` can never be `None`, so the
/// `unreachable!` arm is dead. Mirrors the `UNIX_EPOCH` constant in the ODBC
/// `date` converter.
const UNIX_EPOCH: NaiveDate = match NaiveDate::from_ymd_opt(1970, 1, 1) {
    Some(date) => date,
    None => unreachable!(),
};

/// Bias added to a `TIMESTAMP_TZ` offset before it is written to the wire.
///
/// The server expects the second token of a `TIMESTAMP_TZ` value as
/// `offset_minutes + 1440` (= 24 * 60). The bias keeps the token non-negative
/// for every legal timezone offset and matches the legacy ODBC / Python
/// connectors.
const TZ_OFFSET_BIAS_MINUTES: i32 = 1440;

/// Maximum magnitude, in minutes, of a legal `TIMESTAMP_TZ` offset.
///
/// The driver accepts offsets within +/-1439 minutes (past the +/-14:00 the
/// SQL spec requires), matching the ODBC converter. Validating against this
/// bound also guarantees `offset_minutes + TZ_OFFSET_BIAS_MINUTES` cannot
/// overflow `i32` for arbitrary caller input to `ParamValue::TimestampTz`.
const MAX_TZ_OFFSET_MINUTES: i32 = 1439;

/// A typed parameter value to bind. Each variant maps to a Snowflake logical
/// type and a canonical wire-text encoding.
#[derive(Debug, Clone, PartialEq)]
pub enum ParamValue {
    /// SQL `NULL`: `{"type": "ANY", "value": null}` (or a `null` array element).
    Null,
    /// Fixed-point / integer (`FIXED`); encoded as decimal digits.
    Fixed(i128),
    /// Floating point (`REAL`); non-finite values encode as `NaN`, `Infinity`,
    /// or `-Infinity`.
    Real(f64),
    /// Boolean (`BOOLEAN`); encoded as `"true"` / `"false"`.
    Boolean(bool),
    /// UTF-8 text (`TEXT`), passed through verbatim.
    Text(String),
    /// Binary (`BINARY`); encoded as lowercase hex, two chars per byte.
    Binary(Vec<u8>),
    /// Calendar date (`DATE`); encoded as milliseconds since 1970-01-01.
    Date(NaiveDate),
    /// Wall-clock time (`TIME`); encoded as nanoseconds since midnight.
    Time(NaiveTime),
    /// Timestamp taken as UTC (`TIMESTAMP_NTZ`); encoded as nanoseconds since
    /// the Unix epoch.
    TimestampNtz(NaiveDateTime),
    /// Local-timezone timestamp. Tagged `TEXT` (not `TIMESTAMP_LTZ`) and
    /// encoded as a bare `YYYY-MM-DD HH:MM:SS[.fffffffff]` literal that the
    /// server localizes in the session timezone.
    TimestampLtz(NaiveDateTime),
    /// Timestamp with timezone (`TIMESTAMP_TZ`); encoded as two space-separated
    /// tokens `"<epoch_nanos> <offset_minutes + 1440>"`.
    TimestampTz {
        /// The instant, in UTC.
        utc: NaiveDateTime,
        /// Original UTC offset in minutes (e.g. `330` for `+05:30`).
        offset_minutes: i32,
    },
    /// Year-month interval (`INTERVAL_YEAR_MONTH`), supplied as a
    /// **pre-formatted literal string** and sent to the server verbatim — the
    /// builder does not assemble it from components.
    ///
    /// Accepted forms (mirroring the ODBC converter output; `<sign>` is empty
    /// or `-`, `(2)` marks a field zero-padded to two digits):
    ///
    /// ```text
    /// <sign><years>                // YEAR          e.g. "5",   "-3"
    /// <sign><months>               // MONTH         e.g. "11",  "-6"
    /// <sign><years>-<months(2)>    // YEAR_TO_MONTH  e.g. "5-06", "-3-11"
    /// ```
    IntervalYearMonth(String),
    /// Day-time interval (`INTERVAL_DAY_TIME`), supplied as a **pre-formatted
    /// literal string** and sent to the server verbatim — the builder does not
    /// assemble it from components.
    ///
    /// Accepted forms (mirroring the ODBC converter output; `<sign>` is empty
    /// or `-`, `(2)` marks a field zero-padded to two digits, and the
    /// sub-second fraction is always six digits). The leading field is not
    /// zero-padded; interior `HH`/`MM`/`SS` fields are:
    ///
    /// ```text
    /// <sign><days>                                        // DAY              "5"
    /// <sign><hours>                                       // HOUR             "12"
    /// <sign><minutes>                                     // MINUTE           "30"
    /// <sign><seconds>.<micros(6)>                         // SECOND           "45.123456"
    /// <sign><days> <hours(2)>                             // DAY_TO_HOUR      "5 12"
    /// <sign><days> <hours(2)>:<minutes(2)>                // DAY_TO_MINUTE    "5 12:30"
    /// <sign><days> <hours(2)>:<minutes(2)>:<secs(2)>.<micros(6)> // DAY_TO_SECOND "5 12:30:45.123456"
    /// <sign><hours>:<minutes(2)>                          // HOUR_TO_MINUTE   "12:30"
    /// <sign><hours>:<minutes(2)>:<secs(2)>.<micros(6)>    // HOUR_TO_SECOND   "12:30:45.123456"
    /// <sign><minutes>:<secs(2)>.<micros(6)>               // MINUTE_TO_SECOND "30:45.123456"
    /// ```
    IntervalDayTime(String),
}

/// Errors produced while building the bindings JSON object.
#[derive(Debug, Snafu)]
pub enum BindingError {
    #[snafu(display("timestamp {value} is outside the representable nanosecond epoch range"))]
    TimestampOutOfRange { value: NaiveDateTime },
    #[snafu(display("binding column {index} mixes multiple logical types ({first} and {second})"))]
    MixedColumnTypes {
        index: usize,
        first: &'static str,
        second: &'static str,
    },
    #[snafu(display("binding column {index} has length {actual}, expected {expected}"))]
    MismatchedColumnLengths {
        index: usize,
        expected: usize,
        actual: usize,
    },
    #[snafu(display(
        "timezone offset {offset_minutes} minutes is outside the legal range +/-1439 minutes"
    ))]
    TimestampTzOffsetOutOfRange { offset_minutes: i32 },
    #[snafu(display("failed to serialize bindings to JSON"))]
    Serialization { source: serde_json::Error },
}

/// Build the bindings JSON for a single parameter set (one row).
///
/// `params[i]` is bind variable `i + 1`. Each parameter emits
/// `{"type": <logical type>, "value": <string-or-null>}`; a [`ParamValue::Null`]
/// emits `{"type": "ANY", "value": null}`.
pub fn to_json_single(params: &[ParamValue]) -> Result<String, BindingError> {
    let mut bindings = Map::new();

    for (index, param) in params.iter().enumerate() {
        let (logical_type, wire) = encode(param)?;
        let value = wire.map_or(Value::Null, Value::String);

        let mut binding = Map::new();
        binding.insert("type".to_string(), Value::String(logical_type.to_string()));
        binding.insert("value".to_string(), value);

        bindings.insert((index + 1).to_string(), Value::Object(binding));
    }

    serde_json::to_string(&Value::Object(bindings)).context(SerializationSnafu)
}

/// Build the bindings JSON for an array binding (multiple rows).
///
/// `columns[i]` is the list of values for bind variable `i + 1` across all
/// rows, so every column must have the same length. Each parameter emits
/// `{"type": <logical type>, "value": [<string-or-null>, ...]}`.
///
/// The per-parameter `type` is taken from the first non-NULL value in the
/// column; a column that is entirely NULL is tagged `ANY`. NULL cells never
/// participate in type derivation, so an `ANY` (NULL) value does not count as a
/// distinct logical type when detecting mixed columns.
pub fn to_json_arrays(columns: &[Vec<ParamValue>]) -> Result<String, BindingError> {
    let mut bindings = Map::new();

    // The first column establishes the row count; every other column must
    // match it so the emitted value arrays stay aligned per bind variable.
    // An empty `columns` slice produces an empty object; a length-0 column is
    // only valid when every column is length 0.
    let expected = columns.first().map_or(0, |column| column.len());

    for (index, column) in columns.iter().enumerate() {
        if column.len() != expected {
            return MismatchedColumnLengthsSnafu {
                index,
                expected,
                actual: column.len(),
            }
            .fail();
        }

        // The wire format carries one `type` per bind variable, not per cell.
        // "ANY" doubles as the "not yet determined" sentinel: no non-NULL
        // value ever encodes to "ANY", so the first non-NULL value always
        // replaces it with a real logical type.
        let mut logical_type = "ANY";
        let mut values: Vec<Value> = Vec::with_capacity(column.len());

        for param in column {
            let (cell_type, wire) = encode(param)?;
            match wire {
                Some(text) => {
                    if logical_type == "ANY" {
                        logical_type = cell_type;
                    } else if logical_type != cell_type {
                        return MixedColumnTypesSnafu {
                            index,
                            first: logical_type,
                            second: cell_type,
                        }
                        .fail();
                    }
                    values.push(Value::String(text));
                }
                None => values.push(Value::Null),
            }
        }

        let mut binding = Map::new();
        binding.insert("type".to_string(), Value::String(logical_type.to_string()));
        binding.insert("value".to_string(), Value::Array(values));

        bindings.insert((index + 1).to_string(), Value::Object(binding));
    }

    serde_json::to_string(&Value::Object(bindings)).context(SerializationSnafu)
}

/// Encode a single value into its `(logical type, wire text)` pair.
///
/// Returns `None` for the wire text when the value is [`ParamValue::Null`]
/// (which the callers render as JSON `null`); every other variant returns
/// `Some(text)` where `text` is placed verbatim into a JSON string (serde
/// applies the JSON escaping).
fn encode(value: &ParamValue) -> Result<(&'static str, Option<String>), BindingError> {
    Ok(match value {
        ParamValue::Null => ("ANY", None),
        ParamValue::Fixed(v) => ("FIXED", Some(v.to_string())),
        ParamValue::Real(v) => ("REAL", Some(encode_real(*v))),
        ParamValue::Boolean(v) => ("BOOLEAN", Some(v.to_string())),
        ParamValue::Text(v) => ("TEXT", Some(v.clone())),
        ParamValue::Binary(v) => ("BINARY", Some(encode_binary(v))),
        ParamValue::Date(v) => ("DATE", Some(encode_date(*v))),
        ParamValue::Time(v) => ("TIME", Some(encode_time(*v))),
        ParamValue::TimestampNtz(v) => ("TIMESTAMP_NTZ", Some(encode_epoch_nanos(*v)?)),
        // LTZ is intentionally tagged "TEXT", not "TIMESTAMP_LTZ": the server
        // rejects a string value tagged `TIMESTAMP_LTZ`. It receives a bare
        // wall-clock literal and re-interprets it in the session timezone.
        ParamValue::TimestampLtz(v) => ("TEXT", Some(encode_wallclock(*v))),
        ParamValue::TimestampTz {
            utc,
            offset_minutes,
        } => ("TIMESTAMP_TZ", Some(encode_tz(*utc, *offset_minutes)?)),
        ParamValue::IntervalYearMonth(v) => ("INTERVAL_YEAR_MONTH", Some(v.clone())),
        ParamValue::IntervalDayTime(v) => ("INTERVAL_DAY_TIME", Some(v.clone())),
    })
}

/// Encode an `f64` for the `REAL` wire type.
///
/// The JSON bind parser requires the full words `Infinity` / `-Infinity`
/// (Rust's `Display` emits `inf` / `-inf`, which the server rejects). `NaN`
/// already matches Rust's output and finite values pass through unchanged.
fn encode_real(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value == f64::INFINITY {
        "Infinity".to_string()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else {
        value.to_string()
    }
}

/// Encode bytes as lowercase hex, two chars per byte (matching the ODBC
/// `BINARY` converter's `hex_encode_lowercase`).
fn encode_binary(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Encode a `NaiveDate` as milliseconds since the Unix epoch. Whole days are
/// widened to milliseconds, matching the ODBC `DATE` converter.
fn encode_date(date: NaiveDate) -> String {
    let millis = (date - UNIX_EPOCH).num_days() * 86_400_000;
    millis.to_string()
}

/// Encode a `NaiveTime` as nanoseconds since midnight (matching the ODBC
/// `TIME` converter).
fn encode_time(time: NaiveTime) -> String {
    let secs = time.num_seconds_from_midnight() as i64;
    let nanos = time.nanosecond() as i64;
    let total_nanos = secs * 1_000_000_000 + nanos;
    total_nanos.to_string()
}

/// Encode a `NaiveDateTime` as epoch nanoseconds for `TIMESTAMP_NTZ`.
///
/// The value is treated as already-UTC (correct for NTZ, where the server
/// stores the bytes verbatim). Returns [`BindingError::TimestampOutOfRange`]
/// when the instant falls outside the i64 nanosecond epoch range
/// (~1677..2262).
fn encode_epoch_nanos(dt: NaiveDateTime) -> Result<String, BindingError> {
    let epoch_nanos = dt
        .and_utc()
        .timestamp_nanos_opt()
        .context(TimestampOutOfRangeSnafu { value: dt })?;
    Ok(epoch_nanos.to_string())
}

/// Encode a `TIMESTAMP_TZ` as `"<epoch_nanos> <offset_minutes + 1440>"`.
///
/// The bias ([`TZ_OFFSET_BIAS_MINUTES`]) keeps the offset token non-negative
/// for any legal offset. `offset_minutes` is caller-supplied on the public API,
/// so it is validated against [`MAX_TZ_OFFSET_MINUTES`] first: this rejects
/// nonsensical offsets and guarantees the biased sum cannot overflow `i32`.
/// Returns [`BindingError::TimestampTzOffsetOutOfRange`] for an out-of-range
/// offset, or [`BindingError::TimestampOutOfRange`] when the UTC instant falls
/// outside the i64 nanosecond epoch range.
fn encode_tz(utc: NaiveDateTime, offset_minutes: i32) -> Result<String, BindingError> {
    if !(-MAX_TZ_OFFSET_MINUTES..=MAX_TZ_OFFSET_MINUTES).contains(&offset_minutes) {
        return TimestampTzOffsetOutOfRangeSnafu { offset_minutes }.fail();
    }
    let epoch_nanos = utc
        .and_utc()
        .timestamp_nanos_opt()
        .context(TimestampOutOfRangeSnafu { value: utc })?;
    Ok(format!(
        "{epoch_nanos} {}",
        offset_minutes + TZ_OFFSET_BIAS_MINUTES
    ))
}

/// Encode a `NaiveDateTime` as a bare `YYYY-MM-DD HH:MM:SS[.fraction]`
/// wall-clock literal for `TIMESTAMP_LTZ` binds (which are tagged `TEXT`).
///
/// The year is the absolute value zero-padded to a minimum of four digits,
/// with a leading `-` for negative (proleptic) years. The fractional part is
/// emitted only when the nanosecond component is non-zero, as nine digits with
/// trailing zeros trimmed.
fn encode_wallclock(dt: NaiveDateTime) -> String {
    let year = dt.year();
    let mut out = if year < 0 {
        format!("-{:04}", year.unsigned_abs())
    } else {
        format!("{:04}", year.unsigned_abs())
    };
    out.push_str(&format!(
        "-{:02}-{:02} {:02}:{:02}:{:02}",
        dt.month(),
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second()
    ));

    let nanos = dt.nanosecond();
    if nanos != 0 {
        out.push('.');
        // `nanos != 0` guarantees at least one non-zero digit remains, so
        // trimming trailing zeros never strips the whole fractional part.
        out.push_str(format!("{:09}", nanos).trim_end_matches('0'));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Value {
        serde_json::from_str(json).expect("produced JSON must parse")
    }

    fn single(param: ParamValue) -> Value {
        parse(&to_json_single(&[param]).expect("encode single"))
    }

    #[test]
    fn fixed_encodes_as_string() {
        let v = single(ParamValue::Fixed(-42));
        assert_eq!(v["1"]["type"], "FIXED");
        assert!(v["1"]["value"].is_string(), "value must be a JSON string");
        assert_eq!(v["1"]["value"], "-42");
    }

    #[test]
    fn boolean_encodes_as_string() {
        let v = single(ParamValue::Boolean(true));
        assert_eq!(v["1"]["type"], "BOOLEAN");
        assert!(v["1"]["value"].is_string());
        assert_eq!(v["1"]["value"], "true");
    }

    #[test]
    fn binary_encodes_as_lowercase_hex() {
        let v = single(ParamValue::Binary(vec![0xDE, 0xAD, 0xBE, 0xEF]));
        assert_eq!(v["1"]["type"], "BINARY");
        assert_eq!(v["1"]["value"], "deadbeef");
    }

    #[test]
    fn date_epoch_is_zero() {
        let v = single(ParamValue::Date(
            NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
        ));
        assert_eq!(v["1"]["type"], "DATE");
        assert_eq!(v["1"]["value"], "0");
    }

    #[test]
    fn date_one_day_after_epoch() {
        let v = single(ParamValue::Date(
            NaiveDate::from_ymd_opt(1970, 1, 2).unwrap(),
        ));
        assert_eq!(v["1"]["type"], "DATE");
        assert_eq!(v["1"]["value"], "86400000");
    }

    #[test]
    fn time_encodes_nanos_from_midnight() {
        let t = NaiveTime::from_hms_nano_opt(0, 0, 1, 123).unwrap();
        let v = single(ParamValue::Time(t));
        assert_eq!(v["1"]["type"], "TIME");
        assert_eq!(v["1"]["value"], "1000000123");
    }

    #[test]
    fn timestamp_ntz_epoch_is_zero() {
        let dt = NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let v = single(ParamValue::TimestampNtz(dt));
        assert_eq!(v["1"]["type"], "TIMESTAMP_NTZ");
        assert_eq!(v["1"]["value"], "0");
    }

    #[test]
    fn timestamp_tz_encodes_epoch_and_biased_offset() {
        let utc = NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let v = single(ParamValue::TimestampTz {
            utc,
            offset_minutes: 330,
        });
        assert_eq!(v["1"]["type"], "TIMESTAMP_TZ");
        assert_eq!(v["1"]["value"], "0 1770");
    }

    #[test]
    fn timestamp_tz_offset_out_of_range_is_rejected() {
        let utc = NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        // i32::MAX would overflow `offset_minutes + 1440`; the builder must
        // return an error rather than panic (debug) or wrap (release).
        let err = to_json_single(&[ParamValue::TimestampTz {
            utc,
            offset_minutes: i32::MAX,
        }])
        .expect_err("out-of-range offset must be rejected");
        assert!(matches!(
            err,
            BindingError::TimestampTzOffsetOutOfRange { offset_minutes } if offset_minutes == i32::MAX
        ));
    }

    #[test]
    fn timestamp_tz_offset_boundaries_are_accepted() {
        let utc = NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        for (offset_minutes, biased) in [(MAX_TZ_OFFSET_MINUTES, 2879), (-MAX_TZ_OFFSET_MINUTES, 1)]
        {
            let v = single(ParamValue::TimestampTz {
                utc,
                offset_minutes,
            });
            assert_eq!(v["1"]["type"], "TIMESTAMP_TZ");
            assert_eq!(v["1"]["value"], Value::String(format!("0 {biased}")));
        }
    }

    #[test]
    fn interval_year_month_is_verbatim() {
        let v = single(ParamValue::IntervalYearMonth("5-06".to_string()));
        assert_eq!(v["1"]["type"], "INTERVAL_YEAR_MONTH");
        assert_eq!(v["1"]["value"], "5-06");
    }

    #[test]
    fn interval_day_time_is_verbatim() {
        let v = single(ParamValue::IntervalDayTime("1 02:03:04.5".to_string()));
        assert_eq!(v["1"]["type"], "INTERVAL_DAY_TIME");
        assert_eq!(v["1"]["value"], "1 02:03:04.5");
    }

    #[test]
    fn real_special_and_finite_values() {
        assert_eq!(single(ParamValue::Real(f64::NAN))["1"]["value"], "NaN");
        assert_eq!(
            single(ParamValue::Real(f64::INFINITY))["1"]["value"],
            "Infinity"
        );
        assert_eq!(
            single(ParamValue::Real(f64::NEG_INFINITY))["1"]["value"],
            "-Infinity"
        );

        let finite = single(ParamValue::Real(1.5));
        assert_eq!(finite["1"]["type"], "REAL");
        assert!(finite["1"]["value"].is_string());
        assert_eq!(finite["1"]["value"], "1.5");
    }

    #[test]
    fn null_single_is_any_null() {
        let v = single(ParamValue::Null);
        assert_eq!(v["1"]["type"], "ANY");
        assert!(v["1"]["value"].is_null());
    }

    #[test]
    fn text_requiring_escaping_round_trips() {
        let raw = "a\"b\nc\t\u{1F600} café";
        let v = single(ParamValue::Text(raw.to_string()));
        assert_eq!(v["1"]["type"], "TEXT");
        assert_eq!(v["1"]["value"].as_str().unwrap(), raw);
    }

    #[test]
    fn ltz_with_nanos_trims_trailing_zeros() {
        let dt = NaiveDate::from_ymd_opt(2024, 1, 2)
            .unwrap()
            .and_hms_nano_opt(3, 4, 5, 123_456_000)
            .unwrap();
        let v = single(ParamValue::TimestampLtz(dt));
        assert_eq!(v["1"]["type"], "TEXT");
        assert_eq!(v["1"]["value"], "2024-01-02 03:04:05.123456");
    }

    #[test]
    fn ltz_with_zero_nanos_has_no_fraction() {
        let dt = NaiveDate::from_ymd_opt(2024, 1, 2)
            .unwrap()
            .and_hms_opt(3, 4, 5)
            .unwrap();
        let v = single(ParamValue::TimestampLtz(dt));
        assert_eq!(v["1"]["type"], "TEXT");
        assert_eq!(v["1"]["value"], "2024-01-02 03:04:05");
    }

    #[test]
    fn timestamp_ntz_out_of_range_errors() {
        let dt = NaiveDate::from_ymd_opt(3000, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let err = to_json_single(&[ParamValue::TimestampNtz(dt)]).unwrap_err();
        assert!(
            matches!(err, BindingError::TimestampOutOfRange { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn arrays_two_columns_three_rows_with_null() {
        let columns = vec![
            vec![ParamValue::Fixed(1), ParamValue::Null, ParamValue::Fixed(3)],
            vec![
                ParamValue::Text("a".to_string()),
                ParamValue::Text("b".to_string()),
                ParamValue::Null,
            ],
        ];
        let v = parse(&to_json_arrays(&columns).expect("encode arrays"));

        assert_eq!(v["1"]["type"], "FIXED");
        assert_eq!(v["1"]["value"], serde_json::json!(["1", null, "3"]));

        assert_eq!(v["2"]["type"], "TEXT");
        assert_eq!(v["2"]["value"], serde_json::json!(["a", "b", null]));
    }

    #[test]
    fn arrays_all_null_column_is_any() {
        let columns = vec![vec![ParamValue::Null, ParamValue::Null]];
        let v = parse(&to_json_arrays(&columns).expect("encode arrays"));
        assert_eq!(v["1"]["type"], "ANY");
        assert_eq!(v["1"]["value"], serde_json::json!([null, null]));
    }

    #[test]
    fn arrays_mixed_types_error() {
        let columns = vec![vec![
            ParamValue::Fixed(1),
            ParamValue::Text("x".to_string()),
        ]];
        let err = to_json_arrays(&columns).unwrap_err();
        assert!(
            matches!(
                err,
                BindingError::MixedColumnTypes {
                    index: 0,
                    first: "FIXED",
                    second: "TEXT"
                }
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn arrays_mismatched_lengths_error() {
        let columns = vec![
            vec![ParamValue::Fixed(1), ParamValue::Fixed(2)],
            vec![ParamValue::Fixed(3)],
        ];
        let err = to_json_arrays(&columns).unwrap_err();
        assert!(
            matches!(
                err,
                BindingError::MismatchedColumnLengths {
                    index: 1,
                    expected: 2,
                    actual: 1
                }
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn arrays_leading_null_does_not_fix_type_to_any() {
        let columns = vec![vec![
            ParamValue::Null,
            ParamValue::Fixed(7),
            ParamValue::Null,
        ]];
        let v = parse(&to_json_arrays(&columns).expect("encode arrays"));
        assert_eq!(v["1"]["type"], "FIXED");
        assert_eq!(v["1"]["value"], serde_json::json!([null, "7", null]));
    }

    #[test]
    fn empty_params_and_columns_emit_empty_object() {
        assert_eq!(to_json_single(&[]).unwrap(), "{}");
        assert_eq!(to_json_arrays(&[]).unwrap(), "{}");
    }

    #[test]
    fn arrays_all_zero_length_columns_are_allowed() {
        let columns = vec![Vec::new(), Vec::new()];
        let v = parse(&to_json_arrays(&columns).expect("encode arrays"));
        assert_eq!(v["1"]["type"], "ANY");
        assert_eq!(v["1"]["value"], serde_json::json!([]));
        assert_eq!(v["2"]["type"], "ANY");
        assert_eq!(v["2"]["value"], serde_json::json!([]));
    }

    #[test]
    fn keys_are_one_indexed() {
        let json =
            to_json_single(&[ParamValue::Fixed(10), ParamValue::Text("x".to_string())]).unwrap();
        let v = parse(&json);
        assert_eq!(v["1"]["value"], "10");
        assert_eq!(v["2"]["value"], "x");
        assert!(v.get("0").is_none(), "keys must start at 1, not 0");
    }
}
