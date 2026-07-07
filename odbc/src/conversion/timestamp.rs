use crate::conversion::int_fmt;
use arrow::array::{Array, PrimitiveArray, StructArray};
use arrow::datatypes::{Int32Type, Int64Type};
use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use odbc_sys as sql;

use crate::api::CDataType;
use crate::api::ParameterBinding;
use crate::conversion::error::{
    BindingError, BindingNumericOutOfRangeSnafu, DatetimeFieldOverflowSnafu,
    InvalidCharacterValueForCastSnafu, InvalidDatetimeValueSnafu, NumericValueOutOfRangeSnafu,
    UnsupportedCDataTypeSnafu,
};
use crate::conversion::error::{
    ConversionError, DatetimeOutOfSqlRangeSnafu, InvalidArrowValueSnafu, ReadArrowError,
    SQL_DATETIME_YEAR_RANGE, UnsupportedOdbcTypeSnafu, WriteOdbcError,
};
use crate::conversion::param_binding::{
    TEMPORAL_CHAR_DIAG_MAX_CHARS, parse_temporal_char_input, read_binary_struct, read_char_str,
    read_unaligned, read_wchar_str,
};
use crate::conversion::traits::Binding;
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, WriteWire};
use crate::conversion::warning::{Warning, Warnings};
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};

/// Wire-protocol bias for TIMESTAMP_TZ offset minutes.
///
/// The Snowflake server expects the JSON `value` for a TIMESTAMP_TZ binding
/// in the form `"<epoch_nanoseconds> <offset_minutes_plus_1440>"`. The 1440
/// (= 24 * 60) bias keeps the second token non-negative for any legal
/// timezone offset and matches the legacy 3.16.0 ODBC and Python connectors.
const TZ_OFFSET_BIAS_MINUTES: i32 = 1440;

/// Style in which the `+/-HH:MM` offset suffix is appended to a
/// TIMESTAMP_TZ -> SQL_C_CHAR / SQL_C_WCHAR fetch result.
///
/// The variant is selected by inspecting `TIMESTAMP_TZ_OUTPUT_FORMAT` for
/// the longest matching offset token. Snowflake's date-time format grammar
/// (see <https://docs.snowflake.com/en/sql-reference/date-time-input-output>)
/// recognises three offset tokens; we mirror the same set so customers who
/// migrated from the 3.16.0 driver get the wire format they configured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TzOffsetFormat {
    /// `TZH:TZM` — colon-separated, e.g. `+05:30`.
    Colon,
    /// `TZHTZM`  — no separator, e.g. `+0530`.
    NoColon,
    /// `TZH`     — hour only, e.g. `+05`. Sub-hour offsets always emit
    /// `+HH:MM` instead so customers don't silently lose minutes; this is
    /// what the Snowflake server does for the same token.
    HourOnly,
}

/// Parse a `TIMESTAMP_TZ_OUTPUT_FORMAT` value to decide whether — and how —
/// the TZ -> CHAR/WCHAR fetch path should append the offset suffix.
///
/// Token detection is **opt-in**: the new behaviour is gated on the
/// customer explicitly setting a format string that contains an offset
/// token. An empty / missing format falls through to the legacy UTC-only
/// fetch behaviour, so existing applications see no change.
///
/// Tokenisation rules (matching Snowflake's format grammar):
///
/// 1. Snowflake double-quoted literal runs (`"..."`) are stripped first,
///    so a literal `'"server-side TZH note: " YYYY-MM-DD HH24:MI:SS'`
///    does **not** activate `HourOnly`. Toggling wire-format bytes on a
///    literal substring is a correctness bug — see PR #1068 review on
///    `timestamp.rs:70`.
/// 2. The remaining text is split on non-alphanumeric boundaries and
///    matched whole-token, so `TZHACK` / `TZHELP` / `literal_TZH_marker`
///    no longer false-fire as `HourOnly`.
/// 3. Longest match wins on a per-token basis: `TZH:TZM` (split into two
///    `TZH` and `TZM` tokens by step 2) is detected via the colon
///    sequence test below before we ever reach the bare-`TZH` arm.
/// 4. Match is case-insensitive to mirror Snowflake's format grammar
///    (the server treats `tzh:tzm` and `TZH:TZM` identically).
///
/// Snowflake also accepts `TZHM` (4-char compact, no colon, no `TZ`
/// prefix on the minutes) and bare `TZM`. The current driver only
/// renders the three documented variants; an unrecognised but
/// offset-shaped token (anything starting with `TZ`) emits a
/// `tracing::warn!` so a customer who configured `TZHM` and gets bare
/// UTC has at least *some* signal in the logs.
pub(crate) fn parse_tz_offset_format(format: &str) -> Option<TzOffsetFormat> {
    let stripped = strip_snowflake_quoted_literals(format);
    let upper = stripped.to_ascii_uppercase();

    // `TZH:TZM` is the only token that crosses a non-alphanumeric
    // boundary (the colon), so we detect it first before the
    // alphanumeric tokenizer. The colon-separated check is itself
    // boundary-anchored so `XTZH:TZMX` doesn't false-fire.
    if contains_token_pair(&upper, "TZH", "TZM", ':') {
        return Some(TzOffsetFormat::Colon);
    }

    // Walk the remaining alphanumeric-token sequence and look for an
    // exact whole-token match. We do a single pass and remember whether
    // any TZ-shaped token was seen so we can emit a diagnostic warning
    // for unrecognised variants like `TZHM` / `TZM`.
    let mut saw_unknown_tz_token: Option<String> = None;
    // Treat `_` as part of the surrounding token, not a separator. A
    // user-supplied identifier-shaped literal like `literal_TZH_marker`
    // is a single opaque token to us, not three (and `TZH` inside it
    // must not activate the offset suffix).
    for token in upper.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
        if token.is_empty() {
            continue;
        }
        match token {
            "TZHTZM" => return Some(TzOffsetFormat::NoColon),
            "TZH" => return Some(TzOffsetFormat::HourOnly),
            // Track but don't return — let a later, recognised token
            // win if the format string mixes them.
            t if t.starts_with("TZ") && saw_unknown_tz_token.is_none() => {
                saw_unknown_tz_token = Some(t.to_string());
            }
            _ => {}
        }
    }

    if let Some(unknown) = saw_unknown_tz_token {
        tracing::warn!(
            unknown_token = %unknown,
            "TIMESTAMP_TZ_OUTPUT_FORMAT contains a TZ-shaped token the driver does not render \
             (only TZH:TZM, TZHTZM, and TZH are supported); fetch will fall back to bare UTC"
        );
    }
    None
}

/// Strip Snowflake double-quoted literal runs so the tokenizer can't
/// false-fire on user-supplied literal text. Mirrors Snowflake's format
/// grammar where `"..."` is a literal that must be emitted verbatim.
///
/// We do not implement the full grammar (e.g. escaped `""` inside a
/// literal); the worst case of mishandling escapes is that a *real* TZ
/// token after the unmatched run wins, which is the same outcome as
/// before this fix and so a strict superset of the previous behaviour.
fn strip_snowflake_quoted_literals(format: &str) -> String {
    let mut out = String::with_capacity(format.len());
    let mut in_quote = false;
    for ch in format.chars() {
        if ch == '"' {
            in_quote = !in_quote;
            continue;
        }
        if !in_quote {
            out.push(ch);
        }
    }
    out
}

/// Return `true` if `haystack` contains `left<sep>right` where both
/// `left` and `right` are bordered by non-alphanumeric chars (or the
/// string boundaries). Lets us spot `TZH:TZM` without false-firing on
/// `XTZH:TZMX`. All inputs are expected to be ASCII-uppercased.
fn contains_token_pair(haystack: &str, left: &str, right: &str, sep: char) -> bool {
    let needle = format!("{left}{sep}{right}");
    let bytes = haystack.as_bytes();
    let mut start = 0;
    while let Some(rel) = haystack[start..].find(&needle) {
        let begin = start + rel;
        let end = begin + needle.len();
        let prev_ok = begin == 0 || !is_ascii_alnum(bytes[begin - 1]);
        let next_ok = end == bytes.len() || !is_ascii_alnum(bytes[end]);
        if prev_ok && next_ok {
            return true;
        }
        start = begin + 1;
    }
    false
}

/// In-token predicate. Mirrors the splitter in `parse_tz_offset_format`
/// so `_TZH:TZM_` is treated as one opaque token, not as a colon-separated
/// pair surrounded by underscores. Any change here must keep the two in
/// lockstep or `contains_token_pair` will diverge from the splitter.
#[inline]
fn is_ascii_alnum(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// A TIMESTAMP_TZ value carrying both the UTC instant and its original
/// timezone offset in minutes.
///
/// `utc` is the wall-clock representation **at UTC** (`offset_minutes == 0`
/// means "this naive datetime is already UTC"). `offset_minutes` records the
/// offset that produced the *local* wall-clock the user originally saw, so
/// CHAR/WCHAR formatting and JSON binding can reconstruct it without losing
/// information. `SQL_C_TYPE_TIMESTAMP` reads ignore `offset_minutes` because
/// the ODBC struct has no field to store it (matches the spec rule that
/// "datetime with timezone -> datetime without timezone" converts to UTC).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TzInstant {
    pub utc: NaiveDateTime,
    pub offset_minutes: i32,
}

// =============================================================================
// Arrow reading helpers
// =============================================================================

/// Reject datetimes whose year falls outside [`SQL_DATETIME_YEAR_RANGE`].
/// Called from the `validate_value` impls on the timestamp types — the
/// SQL-range check is a policy concern (not a decode concern), so it
/// runs after `read_arrow_type` succeeds and surfaces as
/// `ConversionError::DatetimeOutOfSqlRange` (SQLSTATE 22008).
fn check_sql_year(dt: &NaiveDateTime) -> Result<(), ConversionError> {
    if !SQL_DATETIME_YEAR_RANGE.contains(&dt.year()) {
        return DatetimeOutOfSqlRangeSnafu {
            reason: format!(
                "TIMESTAMP year {} is outside SQL range 0001..9999",
                dt.year()
            ),
        }
        .fail();
    }
    Ok(())
}

/// Split a raw scaled epoch value into (epoch_seconds, nanoseconds).
///
/// Snowflake sends timestamp epoch values at varying scales:
///   scale 0 → seconds, scale 3 → milliseconds, scale 6 → microseconds, etc.
fn split_scaled_epoch(raw: i64, scale: u32) -> Result<(i64, u32), ReadArrowError> {
    if scale > 9 {
        return InvalidArrowValueSnafu {
            reason: format!("timestamp scale {scale} exceeds maximum of 9"),
        }
        .fail();
    }
    Ok(match scale {
        0 => (raw, 0u32),
        3 => {
            let secs = raw.div_euclid(1_000);
            let millis = raw.rem_euclid(1_000) as u32;
            (secs, millis * 1_000_000)
        }
        6 => {
            let secs = raw.div_euclid(1_000_000);
            let micros = raw.rem_euclid(1_000_000) as u32;
            (secs, micros * 1_000)
        }
        _ => {
            // Handles scales 1,2,4,5,7,8,9. The division 10^9 / 10^scale is
            // exact for all integer scales 0–9 because 10^scale always divides
            // evenly into 10^9. The guard `scale > 9` above ensures this.
            let divisor = 10i64.pow(scale);
            let secs = raw.div_euclid(divisor);
            let frac = raw.rem_euclid(divisor) as u32;
            let nanos = frac * (1_000_000_000u32 / divisor as u32);
            (secs, nanos)
        }
    })
}

fn read_struct_timestamp(
    array: &StructArray,
    row_idx: usize,
) -> Result<NaiveDateTime, ReadArrowError> {
    if array.is_null(row_idx) {
        return Err(ReadArrowError::NullValue {
            location: snafu::location!(),
        });
    }

    if array.num_columns() < 2 {
        return InvalidArrowValueSnafu {
            reason: format!(
                "timestamp struct has {} column(s), expected at least 2",
                array.num_columns()
            ),
        }
        .fail();
    }

    let epoch_array = array
        .column(0)
        .as_any()
        .downcast_ref::<PrimitiveArray<Int64Type>>()
        .ok_or_else(|| {
            InvalidArrowValueSnafu {
                reason: "timestamp struct column 0 is not Int64".to_string(),
            }
            .build()
        })?;
    let fraction_array = array
        .column(1)
        .as_any()
        .downcast_ref::<PrimitiveArray<Int32Type>>()
        .ok_or_else(|| {
            InvalidArrowValueSnafu {
                reason: "timestamp struct column 1 is not Int32".to_string(),
            }
            .build()
        })?;

    let epoch_seconds = epoch_array.value(row_idx);
    let fraction_nanos = fraction_array.value(row_idx);

    if !(0..1_000_000_000).contains(&fraction_nanos) {
        return InvalidArrowValueSnafu {
            reason: format!(
                "fraction_nanos={fraction_nanos} is out of valid range [0, 1_000_000_000)"
            ),
        }
        .fail();
    }

    DateTime::from_timestamp(epoch_seconds, fraction_nanos as u32)
        .map(|dt| dt.naive_utc())
        .ok_or_else(|| {
            InvalidArrowValueSnafu {
                reason: format!(
                    "epoch_seconds={epoch_seconds}, fraction_nanos={fraction_nanos} is out of range"
                ),
            }
            .build()
        })
}

/// Read a timestamp from a flat Int64 array where the value is an epoch in
/// units determined by `scale` (0 = seconds, 3 = millis, 6 = micros, etc.).
fn read_scaled_timestamp(
    array: &PrimitiveArray<Int64Type>,
    row_idx: usize,
    scale: u32,
) -> Result<NaiveDateTime, ReadArrowError> {
    if array.is_null(row_idx) {
        return Err(ReadArrowError::NullValue {
            location: snafu::location!(),
        });
    }

    let raw = array.value(row_idx);
    let (epoch_seconds, nanos) = split_scaled_epoch(raw, scale)?;

    DateTime::from_timestamp(epoch_seconds, nanos)
        .map(|dt| dt.naive_utc())
        .ok_or_else(|| {
            InvalidArrowValueSnafu {
                reason: format!(
                    "scaled epoch raw={raw}, scale={scale} produced out-of-range timestamp"
                ),
            }
            .build()
        })
}

/// Read a TIMESTAMP_TZ value (UTC instant + original offset) from a StructArray.
///
/// Snowflake uses different StructArray layouts depending on the declared scale:
///   - Scale 6-9: 3 columns `{epoch_sec: Int64, fraction_nanos: Int32, tz_offset_min: Int32}`
///   - Scale 0-5: 2 columns `{epoch_scaled: Int64, tz_offset_min: Int32}`
///
/// In both cases the epoch value already represents the UTC instant; the
/// `tz_offset_min` column carries the original timezone offset (Snowflake
/// pre-biases by +1440 on the wire, so we subtract here to recover the signed
/// offset in minutes). The offset is preserved alongside the UTC datetime so
/// downstream conversions can reconstruct the local wall-clock for
/// `SQL_C_CHAR`/`SQL_C_WCHAR` (with `+/-HH:MM` suffix) without losing
/// information. `SQL_C_TYPE_TIMESTAMP` ignores the offset because the ODBC
/// struct has no field to store it.
fn read_struct_timestamp_tz(
    array: &StructArray,
    row_idx: usize,
    scale: u32,
) -> Result<TzInstant, ReadArrowError> {
    if array.is_null(row_idx) {
        return Err(ReadArrowError::NullValue {
            location: snafu::location!(),
        });
    }

    let num_columns = array.num_columns();
    let utc = if num_columns == 3 {
        read_struct_timestamp(array, row_idx)?
    } else if num_columns == 2 {
        let epoch_array = array
            .column(0)
            .as_any()
            .downcast_ref::<PrimitiveArray<Int64Type>>()
            .ok_or_else(|| {
                InvalidArrowValueSnafu {
                    reason: "TIMESTAMP_TZ struct column 0 is not Int64".to_string(),
                }
                .build()
            })?;

        let raw = epoch_array.value(row_idx);
        let (epoch_seconds, nanos) = split_scaled_epoch(raw, scale)?;

        DateTime::from_timestamp(epoch_seconds, nanos)
            .map(|dt| dt.naive_utc())
            .ok_or_else(|| {
                InvalidArrowValueSnafu {
                    reason: format!(
                        "TZ scaled epoch raw={raw}, scale={scale} produced out-of-range timestamp"
                    ),
                }
                .build()
            })?
    } else {
        return InvalidArrowValueSnafu {
            reason: format!("TIMESTAMP_TZ struct has {num_columns} columns, expected 2 or 3"),
        }
        .fail();
    };

    // Offset always lives in the last column. The wire value is the
    // signed offset in minutes plus 1440 (Snowflake's positive-only
    // encoding); subtract the bias here so callers see the true offset.
    let offset_col_idx = num_columns - 1;
    let offset_array = array
        .column(offset_col_idx)
        .as_any()
        .downcast_ref::<PrimitiveArray<Int32Type>>()
        .ok_or_else(|| {
            InvalidArrowValueSnafu {
                reason: format!(
                    "TIMESTAMP_TZ struct column {offset_col_idx} is not Int32 (expected tz_offset_min)"
                ),
            }
            .build()
        })?;
    let offset_minutes = offset_array.value(row_idx) - TZ_OFFSET_BIAS_MINUTES;

    Ok(TzInstant {
        utc,
        offset_minutes,
    })
}

// =============================================================================
// ODBC write/read helpers (shared by all three timestamp types)
// =============================================================================

/// Format a `NaiveDateTime` as `YYYY-MM-DD HH:MM:SS[.fffffffff]` into a stack
/// buffer without any heap allocation, returning the filled slice as `&str`.
///
/// 48 bytes is sufficient: `YYYY-MM-DD HH:MM:SS.` = 20 bytes + up to 9 fractional
/// digits + signed/4-digit year headroom. If a future chrono release ever
/// widens this beyond the buffer, the caller receives a typed
/// `NumericValueOutOfRange` error instead of a silent truncation through
/// the unsafe `from_utf8_unchecked` below.
fn format_timestamp_string_into<'a>(
    dt: &NaiveDateTime,
    buf: &'a mut [u8; 48],
) -> Result<&'a str, WriteOdbcError> {
    let nanos = dt.nanosecond();
    // Hand-rolled digit writes instead of `write!`/`core::fmt`, which was the
    // dominant per-cell cost of TIMESTAMP→CHAR conversion. `put_year` is
    // byte-identical to the old `{:04}`; the bounded calendar fields use
    // exactly 2 digits.
    //
    // Max length is `year_width` (≤7 for chrono's representable range) + 15 for
    // `-MM-DD HH:MM:SS` + 10 for `.` and 9 fractional digits = ≤32 ≤ 48. Guard
    // anyway so a hypothetically wider year returns a typed error rather than
    // panicking on an out-of-bounds index across the FFI boundary.
    let needed = int_fmt::year_width(dt.year()) + 15 + if nanos != 0 { 10 } else { 0 };
    if needed > buf.len() {
        return NumericValueOutOfRangeSnafu {
            reason: format!(
                "timestamp value does not fit in the {}-byte format buffer",
                buf.len()
            ),
        }
        .fail();
    }

    let mut p = int_fmt::put_year(buf, 0, dt.year());
    buf[p] = b'-';
    p = int_fmt::put_padded(buf, p + 1, dt.month(), 2);
    buf[p] = b'-';
    p = int_fmt::put_padded(buf, p + 1, dt.day(), 2);
    buf[p] = b' ';
    p = int_fmt::put_padded(buf, p + 1, dt.hour(), 2);
    buf[p] = b':';
    p = int_fmt::put_padded(buf, p + 1, dt.minute(), 2);
    buf[p] = b':';
    p = int_fmt::put_padded(buf, p + 1, dt.second(), 2);
    if nanos != 0 {
        buf[p] = b'.';
        p = int_fmt::put_padded(buf, p + 1, nanos, 9);
        // Trim trailing zeros from the fractional part (matching the old
        // `.trim_end_matches('0')` behavior).
        while buf[p - 1] == b'0' {
            p -= 1;
        }
    }
    // SAFETY: only ASCII digits, '-', ':', ' ', and '.' were written above.
    Ok(unsafe { std::str::from_utf8_unchecked(&buf[..p]) })
}

/// Format a `TzInstant` as a wall-clock literal followed by the requested
/// `+/-HH[:]MM` (or `+/-HH`) offset suffix, into a stack buffer. The
/// wall-clock half mirrors `format_timestamp_string_into`; the offset half
/// is always preceded by a single ASCII space to match the Snowflake
/// server's default `YYYY-MM-DD HH24:MI:SS.FF TZHTZM` output and the
/// legacy 3.16.0 driver.
///
/// `TzOffsetFormat::HourOnly` falls back to `+HH:MM` when the offset has a
/// non-zero minute component; this matches Snowflake's behaviour for the
/// `TZH` token (it does not silently truncate sub-hour offsets like
/// `+05:30`).
fn format_timestamp_tz_string_into<'a>(
    value: &TzInstant,
    fmt: TzOffsetFormat,
    buf: &'a mut [u8; 64],
) -> Result<&'a str, WriteOdbcError> {
    let mut wall_buf = [0u8; 48];
    let wall = format_timestamp_string_into(&value.utc_at_offset(), &mut wall_buf)?;

    let abs_minutes = value.offset_minutes.unsigned_abs();
    let hours = abs_minutes / 60;
    let minutes = abs_minutes % 60;
    let sign = if value.offset_minutes < 0 { b'-' } else { b'+' };

    // Hand-rolled digit writes (see `format_timestamp_string_into`). Length is
    // wall (≤32) + ` ±HH[:MM]` (≤7) = ≤39 ≤ 64, so writes stay in-bounds; the
    // `?` above already surfaced any wall-clock overflow as a typed error.
    let wall_len = wall.len();
    buf[..wall_len].copy_from_slice(wall.as_bytes());
    let mut p = wall_len;
    buf[p] = b' ';
    buf[p + 1] = sign;
    p = int_fmt::put_padded(buf, p + 2, hours, 2);
    match fmt {
        TzOffsetFormat::Colon => {
            buf[p] = b':';
            p = int_fmt::put_padded(buf, p + 1, minutes, 2);
        }
        TzOffsetFormat::NoColon => {
            p = int_fmt::put_padded(buf, p, minutes, 2);
        }
        // `TZH` token: hour-only unless there is a sub-hour component, in
        // which case fall back to `±HH:MM` rather than dropping the minutes.
        TzOffsetFormat::HourOnly if minutes == 0 => {}
        TzOffsetFormat::HourOnly => {
            buf[p] = b':';
            p = int_fmt::put_padded(buf, p + 1, minutes, 2);
        }
    }
    // SAFETY: only ASCII digits, '-', '+', ':', ' ', and '.' were written.
    Ok(unsafe { std::str::from_utf8_unchecked(&buf[..p]) })
}

impl TzInstant {
    /// Reconstruct the **local** wall-clock the user originally observed
    /// (`utc + offset_minutes`). Used when emitting a CHAR/WCHAR with the
    /// offset suffix preserved — printing the UTC wall-clock together with
    /// a non-UTC offset would describe a different instant.
    fn utc_at_offset(&self) -> NaiveDateTime {
        if self.offset_minutes == 0 {
            return self.utc;
        }
        // chrono::NaiveDateTime + chrono::Duration is checked; for any
        // offset within +/-1439 minutes (well past +/-14:00 the spec
        // requires) the addition cannot overflow a representable
        // NaiveDateTime, so unwrap_or returns the UTC value as a safe
        // fallback the formatter then renders without surprise.
        self.utc
            .checked_add_signed(chrono::Duration::minutes(self.offset_minutes as i64))
            .unwrap_or(self.utc)
    }
}

fn to_sql_timestamp(dt: &NaiveDateTime) -> sql::Timestamp {
    sql::Timestamp {
        year: dt.year() as i16,
        month: dt.month() as u16,
        day: dt.day() as u16,
        hour: dt.hour() as u16,
        minute: dt.minute() as u16,
        second: dt.second() as u16,
        fraction: dt.nanosecond(),
    }
}

fn write_timestamp_to_odbc(
    dt: &NaiveDateTime,
    binding: &Binding,
    get_data_offset: &mut Option<usize>,
) -> Result<Warnings, WriteOdbcError> {
    match binding.target_type {
        CDataType::Default | CDataType::TimeStamp | CDataType::TypeTimestamp => {
            let ts = to_sql_timestamp(dt);
            binding.write_fixed(ts);
            Ok(vec![])
        }
        CDataType::Char => {
            if binding.buffer_length > 0 && binding.buffer_length < 20 {
                return NumericValueOutOfRangeSnafu {
                    reason: "Buffer too small for SQL_C_CHAR timestamp (minimum 20 bytes)"
                        .to_string(),
                }
                .fail();
            }
            let mut buf = [0u8; 48];
            let s = format_timestamp_string_into(dt, &mut buf)?;
            Ok(binding.write_char_string(s, get_data_offset))
        }
        CDataType::WChar => {
            if binding.buffer_length > 0 && binding.buffer_length < 40 {
                return NumericValueOutOfRangeSnafu {
                    reason: "Buffer too small for SQL_C_WCHAR timestamp (minimum 40 bytes)"
                        .to_string(),
                }
                .fail();
            }
            let mut buf = [0u8; 48];
            let s = format_timestamp_string_into(dt, &mut buf)?;
            Ok(binding.write_wchar_string(s, get_data_offset))
        }
        CDataType::Date | CDataType::TypeDate => {
            let date = sql::Date {
                year: dt.year() as i16,
                month: dt.month() as u16,
                day: dt.day() as u16,
            };
            binding.write_fixed(date);
            let has_time =
                dt.hour() != 0 || dt.minute() != 0 || dt.second() != 0 || dt.nanosecond() != 0;
            if has_time {
                Ok(vec![Warning::NumericValueTruncated])
            } else {
                Ok(vec![])
            }
        }
        CDataType::Time | CDataType::TypeTime => {
            let time = sql::Time {
                hour: dt.hour() as u16,
                minute: dt.minute() as u16,
                second: dt.second() as u16,
            };
            binding.write_fixed(time);
            if dt.nanosecond() != 0 {
                Ok(vec![Warning::NumericValueTruncated])
            } else {
                Ok(vec![])
            }
        }
        CDataType::Binary => {
            let mut bytes = [0u8; std::mem::size_of::<sql::Timestamp>()];
            let ts = to_sql_timestamp(dt);
            // SAFETY: sql::Timestamp is a repr(C) POD struct. Writing into a
            // pre-zeroed buffer ensures any padding bytes are deterministic.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    &ts as *const sql::Timestamp as *const u8,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                );
            }
            let ts_bytes: &[u8] = &bytes;
            if binding.buffer_length > 0
                && (binding.buffer_length as usize) < std::mem::size_of::<sql::Timestamp>()
            {
                return NumericValueOutOfRangeSnafu {
                    reason: "Buffer too small for SQL_C_BINARY timestamp".to_string(),
                }
                .fail();
            }
            Ok(binding.write_binary(ts_bytes, get_data_offset))
        }
        _ => UnsupportedOdbcTypeSnafu {
            target_type: binding.target_type,
        }
        .fail(),
    }
}

fn read_timestamp_odbc(binding: &ParameterBinding) -> Result<NaiveDateTime, BindingError> {
    match binding.value_type {
        CDataType::TimeStamp | CDataType::TypeTimestamp => {
            let ts = read_unaligned::<sql::Timestamp>(binding);
            let date = NaiveDate::from_ymd_opt(ts.year as i32, ts.month as u32, ts.day as u32)
                .ok_or_else(|| {
                    InvalidDatetimeValueSnafu {
                        reason: format!(
                            "invalid date in SQL_C_TYPE_TIMESTAMP for TIMESTAMP target: \
                             year={}, month={}, day={}",
                            ts.year, ts.month, ts.day
                        ),
                    }
                    .build()
                })?;
            let time = NaiveTime::from_hms_nano_opt(
                ts.hour as u32,
                ts.minute as u32,
                ts.second as u32,
                ts.fraction,
            )
            .ok_or_else(|| {
                InvalidDatetimeValueSnafu {
                    reason: format!(
                        "invalid time in SQL_C_TYPE_TIMESTAMP for TIMESTAMP target: \
                         hour={}, minute={}, second={}, fraction={}",
                        ts.hour, ts.minute, ts.second, ts.fraction
                    ),
                }
                .build()
            })?;
            Ok(NaiveDateTime::new(date, time))
        }
        CDataType::Char | CDataType::WChar => {
            parse_temporal_char_input(binding, TS_CHAR_EXPECTED_FORMAT, |s| {
                NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                    .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f"))
                    .map_err(|_| ())
            })
        }
        // Bind SQL_C_TYPE_DATE into a TIMESTAMP column by combining the date
        // with midnight (matches the legacy 3.16.0 driver, which auto-promotes
        // a DATE source to a TIMESTAMP at 00:00:00.000000000).
        CDataType::Date | CDataType::TypeDate => {
            let d = read_unaligned::<sql::Date>(binding);
            let date = NaiveDate::from_ymd_opt(d.year as i32, d.month as u32, d.day as u32)
                .ok_or_else(|| {
                    InvalidDatetimeValueSnafu {
                        reason: format!(
                            "invalid date in SQL_C_TYPE_DATE for TIMESTAMP target: \
                             year={}, month={}, day={}",
                            d.year, d.month, d.day
                        ),
                    }
                    .build()
                })?;
            Ok(NaiveDateTime::new(date, NaiveTime::MIN))
        }
        // Bind SQL_C_TYPE_TIME into a TIMESTAMP column by pairing the time
        // with the current local date and a zero fractional-seconds field.
        // Per ODBC C-to-SQL spec (Appendix D, "C to SQL: Time"): "the date
        // fields of the timestamp structure are set to the current date and
        // the fractional seconds field is set to zero." This mirrors the
        // SnowflakeTime → SQL_C_TYPE_TIMESTAMP path in `time.rs`.
        CDataType::Time | CDataType::TypeTime => {
            let t = read_unaligned::<sql::Time>(binding);
            let time = NaiveTime::from_hms_opt(t.hour as u32, t.minute as u32, t.second as u32)
                .ok_or_else(|| {
                    InvalidDatetimeValueSnafu {
                        reason: format!(
                            "invalid time in SQL_C_TYPE_TIME for TIMESTAMP target: \
                             hour={}, minute={}, second={}",
                            t.hour, t.minute, t.second
                        ),
                    }
                    .build()
                })?;
            Ok(NaiveDateTime::new(chrono::Local::now().date_naive(), time))
        }
        CDataType::Binary => {
            let ts = read_binary_struct::<sql::Timestamp>(binding, "SQL_TIMESTAMP_STRUCT")?;
            let date = NaiveDate::from_ymd_opt(ts.year as i32, ts.month as u32, ts.day as u32)
                .ok_or_else(|| {
                    BindingNumericOutOfRangeSnafu {
                        reason: format!(
                            "invalid date from SQL_C_BINARY: year={}, month={}, day={}",
                            ts.year, ts.month, ts.day
                        ),
                    }
                    .build()
                })?;
            let time = NaiveTime::from_hms_nano_opt(
                ts.hour as u32,
                ts.minute as u32,
                ts.second as u32,
                ts.fraction,
            )
            .ok_or_else(|| {
                BindingNumericOutOfRangeSnafu {
                    reason: format!(
                        "invalid time from SQL_C_BINARY: hour={}, minute={}, second={}, fraction={}",
                        ts.hour, ts.minute, ts.second, ts.fraction
                    ),
                }
                .build()
            })?;
            Ok(NaiveDateTime::new(date, time))
        }
        _ => UnsupportedCDataTypeSnafu {
            c_type: binding.value_type,
        }
        .fail(),
    }
}

/// Encode a `NaiveDateTime` as the epoch-nanoseconds string the Snowflake
/// server expects for TIMESTAMP_NTZ / TIMESTAMP_TZ binds.
///
/// Treating the value as already-UTC is correct for TIMESTAMP_NTZ (where the
/// server stores the bytes verbatim with no TZ interpretation) and for
/// TIMESTAMP_TZ (which carries its own offset alongside this instant). It is
/// **wrong** for TIMESTAMP_LTZ — see `write_timestamp_wire_wallclock` below
/// for the LTZ path.
fn write_timestamp_wire_epoch_nanos(value: NaiveDateTime) -> Result<String, BindingError> {
    let epoch_nanos = value.and_utc().timestamp_nanos_opt().ok_or_else(|| {
        UnsupportedCDataTypeSnafu {
            c_type: CDataType::TypeTimestamp,
        }
        .build()
    })?;
    Ok(epoch_nanos.to_string())
}

/// Encode a `NaiveDateTime` as a bare wall-clock literal string, with no
/// timezone offset suffix, for TIMESTAMP_LTZ JSON binds.
///
/// Mirrors the legacy 3.16.0 driver's JSON-bind path
/// (`Snowflake-odbc/Source/DataEngine/SFQueryExecutor.cpp`), which tags every
/// `SQL_SF_TIMESTAMP_{NTZ,LTZ,TZ}` parameter as `"type": "TEXT"` and emits a
/// bare `"YYYY-MM-DD HH:MM:SS.FFFFFFFFF"` string. The Snowflake server then
/// coerces the text into the destination column's logical type using the
/// **session timezone** to interpret the wall-clock for TIMESTAMP_LTZ:
///
///   `stored_utc = bound_wall_clock - session_tz_offset`
///
/// (BindUploader.cpp's process-local-offset format is a *separate* CSV-staging
/// path that this driver doesn't use — JSON binds always go through the
/// SFQueryExecutor TEXT path.)
fn write_timestamp_wire_wallclock(value: NaiveDateTime) -> Result<String, BindingError> {
    let mut buf = [0u8; 48];
    let wall_clock = format_timestamp_string_into(&value, &mut buf).map_err(|_| {
        UnsupportedCDataTypeSnafu {
            c_type: CDataType::TypeTimestamp,
        }
        .build()
    })?;
    Ok(wall_clock.to_string())
}

// =============================================================================
// TIMESTAMP_TZ-specific helpers
//
// TZ differs from NTZ/LTZ on two axes:
//   1. CHAR/WCHAR formatting must include a `+/-HH:MM` offset suffix so the
//      value is round-trippable as text (NTZ/LTZ have no offset to emit).
//   2. JSON binding must emit `<epoch_nanos> <offset_minutes + 1440>` so the
//      server stores the original instant *and* its offset (NTZ/LTZ only need
//      the epoch).
// `SQL_C_TYPE_TIMESTAMP` reads/writes intentionally drop the offset because
// the ODBC struct can't carry it -- the spec rule is "datetime with timezone
// -> datetime without timezone converts to UTC".
// =============================================================================

/// Static template used in both diagnostics and unit tests, so a future
/// change to the accepted grammar updates the user-facing message and the
/// pinning test in lockstep.
const TZ_CHAR_EXPECTED_FORMAT: &str = "YYYY-MM-DD HH:MM:SS[.fff] +/-HH:MM";

/// Expected literal shape for a `SQL_C_CHAR` / `SQL_C_WCHAR` source bound to an
/// offset-less TIMESTAMP (NTZ / LTZ) target, surfaced in the 22018 diagnostic
/// when parsing fails. The input length cap is shared with the other temporal
/// binds via [`TEMPORAL_CHAR_DIAG_MAX_CHARS`].
const TS_CHAR_EXPECTED_FORMAT: &str = "YYYY-MM-DD HH:MM:SS[.fffffffff]";

/// Read a TIMESTAMP_TZ value from a parameter binding. Captures both the UTC
/// instant and the offset so `write_timestamp_tz_wire` can emit the legacy
/// two-token wire format.
///
/// Bind paths:
/// - `SQL_C_TYPE_TIMESTAMP` / `SQL_C_BINARY`: the struct has no offset field,
///   so we treat the wall-clock as UTC (offset = 0). Matches the legacy
///   Python connector's treatment of a naive `datetime` bound to TIMESTAMP_TZ.
/// - `SQL_C_CHAR` / `SQL_C_WCHAR`: parse `YYYY-MM-DD HH:MM:SS[.fff] +/-HH:MM`;
///   if no offset suffix is present, fall back to the offset-less parser and
///   treat as UTC (offset = 0). A genuinely unparseable string surfaces as
///   `InvalidCharacterValueForCast` (mapped to SQLSTATE 22018), carrying a
///   truncated copy of the input plus the expected format so the caller
///   learns *what* was rejected and *why*. This is distinct from the 07006
///   ("Restricted data type attribute violation") that signals an
///   unsupported binding *shape*, and from 22008 ("Datetime field overflow")
///   that the JSON writer emits when the parsed instant exceeds the
///   nanosecond epoch range.
fn read_timestamp_tz_odbc(binding: &ParameterBinding) -> Result<TzInstant, BindingError> {
    match binding.value_type {
        CDataType::Char => {
            let s = read_char_str(binding)?;
            parse_tz_string_with_fallback(s.trim(), binding.value_type)
        }
        CDataType::WChar => {
            let s = read_wchar_str(binding)?;
            parse_tz_string_with_fallback(s.trim(), binding.value_type)
        }
        _ => {
            // Reuse the existing offset-less reader (handles
            // SQL_C_TYPE_TIMESTAMP, SQL_C_BINARY, etc.) and treat the
            // result as UTC + offset 0.
            let utc = read_timestamp_odbc(binding)?;
            Ok(TzInstant {
                utc,
                offset_minutes: 0,
            })
        }
    }
}

/// Try `YYYY-MM-DD HH:MM:SS[.fff] +/-HH:MM` first, then fall back to the
/// offset-less formats (treated as UTC). Returns
/// `InvalidCharacterValueForCast` (SQLSTATE 22018) if neither shape parses,
/// carrying a truncated copy of the input and the expected format template.
fn parse_tz_string_with_fallback(s: &str, c_type: CDataType) -> Result<TzInstant, BindingError> {
    for fmt in &["%Y-%m-%d %H:%M:%S%.f %:z", "%Y-%m-%d %H:%M:%S%.f%:z"] {
        if let Ok(dt) = DateTime::<FixedOffset>::parse_from_str(s, fmt) {
            return Ok(TzInstant {
                utc: dt.naive_utc(),
                offset_minutes: dt.offset().local_minus_utc() / 60,
            });
        }
    }
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f"))
        .map(|utc| TzInstant {
            utc,
            offset_minutes: 0,
        })
        .map_err(|_| {
            InvalidCharacterValueForCastSnafu {
                c_type,
                value: s
                    .chars()
                    .take(TEMPORAL_CHAR_DIAG_MAX_CHARS)
                    .collect::<String>(),
                expected_format: TZ_CHAR_EXPECTED_FORMAT,
            }
            .build()
        })
}

fn write_timestamp_tz_wire(value: TzInstant) -> Result<String, BindingError> {
    // `timestamp_nanos_opt` returns `None` only when the UTC instant would
    // overflow `i64` nanoseconds (~year 1677 to year 2262 outside this).
    // That's exactly what 22008 ("Datetime field overflow") describes per
    // ODBC Appendix D, so reusing the existing variant is more spec-correct
    // than the previous `UnsupportedCDataType` catch-all (which would have
    // surfaced as 07006 "Restricted data type attribute violation").
    let epoch_nanos = value.utc.and_utc().timestamp_nanos_opt().ok_or_else(|| {
        DatetimeFieldOverflowSnafu {
            reason: format!(
                "TIMESTAMP_TZ UTC instant {} exceeds the i64 nanosecond epoch range supported by the wire format",
                value.utc
            ),
        }
        .build()
    })?;
    let biased_offset = value.offset_minutes + TZ_OFFSET_BIAS_MINUTES;
    Ok(format!("{epoch_nanos} {biased_offset}"))
}

// =============================================================================
// Macro to generate the trait impls shared by NTZ, LTZ, and (potentially) TZ.
//
// The variation across variants is:
//   - The struct reader for StructArray (NTZ/LTZ use `read_struct_timestamp`;
//     the `tz` arm exists for completeness but the live `SnowflakeTimestampTz`
//     hand-writes its own impls because `Representation = TzInstant`, which
//     the macro's `@common` arm can't express).
//   - The `SnowflakeLogicalType` returned by `sf_type()`.
//   - The wire-format encoding in `write_wire` (NTZ/TZ use epoch nanoseconds;
//     LTZ uses a bare wall-clock literal string and tags `type=TEXT` so the
//     server interprets it in the session timezone -- see
//     `write_timestamp_wire_wallclock` for why). The `ltz_wallclock_string`
//     arm therefore intentionally skips the macro's `WriteWire` generation
//     and the LTZ `WriteWire` impl is written by hand below.
// =============================================================================

macro_rules! impl_snowflake_timestamp {
    // NTZ path: StructArray reader ignores scale, wire encodes as epoch_ns.
    ($name:ident, standard, $logical_type:expr) => {
        impl_snowflake_timestamp!(@struct_array_standard $name);
        impl_snowflake_timestamp!(@common $name);
        impl_snowflake_timestamp!(@write_wire_epoch_nanos $name, $logical_type);
    };

    // LTZ path: same readers as NTZ but wall-clock-string wire payload. The
    // `WriteWire` impl is intentionally NOT generated here — see the
    // hand-written `impl WriteWire for SnowflakeTimestampLtz` below.
    ($name:ident, ltz_wallclock_string) => {
        impl_snowflake_timestamp!(@struct_array_standard $name);
        impl_snowflake_timestamp!(@common $name);
    };

    // TZ path: StructArray reader uses scale to handle 2- vs 3-column layouts.
    ($name:ident, tz, $logical_type:expr) => {
        impl_snowflake_timestamp!(@struct_array_tz $name);
        impl_snowflake_timestamp!(@common $name);
        impl_snowflake_timestamp!(@write_wire_epoch_nanos $name, $logical_type);
    };

    (@struct_array_standard $name:ident) => {
        impl ReadArrowType<StructArray> for $name {
            fn read_arrow_type<'a>(
                &self,
                array: &'a StructArray,
                row_idx: usize,
            ) -> Result<Self::Representation<'a>, ReadArrowError> {
                read_struct_timestamp(array, row_idx)
            }
        }
    };

    (@struct_array_tz $name:ident) => {
        impl ReadArrowType<StructArray> for $name {
            fn read_arrow_type<'a>(
                &self,
                array: &'a StructArray,
                row_idx: usize,
            ) -> Result<Self::Representation<'a>, ReadArrowError> {
                read_struct_timestamp_tz(array, row_idx, self.scale)
            }
        }
    };

    (@common $name:ident) => {
        impl SnowflakeType for $name {
            type Representation<'a> = NaiveDateTime;

            fn validate_value(&self, value: &NaiveDateTime) -> Result<(), ConversionError> {
                check_sql_year(value)
            }
        }

        impl ReadArrowType<PrimitiveArray<Int64Type>> for $name {
            fn read_arrow_type<'a>(
                &self,
                array: &'a PrimitiveArray<Int64Type>,
                row_idx: usize,
            ) -> Result<Self::Representation<'a>, ReadArrowError> {
                read_scaled_timestamp(array, row_idx, self.scale)
            }
        }

        impl WriteODBCType for $name {
            fn sql_type(&self) -> sql::SqlDataType {
                sql::SqlDataType::TIMESTAMP
            }

            fn column_size(&self) -> sql::ULen {
                if self.scale == 0 {
                    19
                } else {
                    20 + self.scale as sql::ULen
                }
            }

            fn decimal_digits(&self) -> sql::SmallInt {
                self.scale as sql::SmallInt
            }

            fn write_odbc_type(
                &self,
                snowflake_value: Self::Representation<'_>,
                binding: &Binding,
                get_data_offset: &mut Option<usize>,
            ) -> Result<Warnings, WriteOdbcError> {
                write_timestamp_to_odbc(&snowflake_value, binding, get_data_offset)
            }
        }

        impl ReadODBC for $name {
            fn read_odbc<'a>(
                &self,
                binding: &'a ParameterBinding,
            ) -> Result<Self::Representation<'a>, BindingError> {
                read_timestamp_odbc(binding)
            }
        }
    };

    (@write_wire_epoch_nanos $name:ident, $logical_type:expr) => {
        impl WriteWire for $name {
            fn write_wire(
                &self,
                value: Self::Representation<'_>,
            ) -> Result<String, BindingError> {
                write_timestamp_wire_epoch_nanos(value)
            }

            fn sf_type(&self) -> SnowflakeLogicalType {
                $logical_type
            }
        }
    };
}

// =============================================================================
// TIMESTAMP_NTZ / TIMESTAMP_LTZ / TIMESTAMP_TZ
// =============================================================================

pub(crate) struct SnowflakeTimestampNtz {
    pub(crate) scale: u32,
}

impl_snowflake_timestamp!(
    SnowflakeTimestampNtz,
    standard,
    SnowflakeLogicalType::TimestampNtz
);

pub(crate) struct SnowflakeTimestampLtz {
    pub(crate) scale: u32,
}

impl_snowflake_timestamp!(SnowflakeTimestampLtz, ltz_wallclock_string);

// LTZ-specific wire encoder. Unlike NTZ/TZ (which encode as epoch_ns and
// let the server take it verbatim), LTZ emits a bare wall-clock literal
// string and relies on the server to interpret it in the **session
// timezone**.
//
// The wire `type` is `TEXT`, NOT `TIMESTAMP_LTZ`. This mirrors the legacy
// 3.16.0 driver's
// `Snowflake-odbc/Source/DataEngine/SFQueryExecutor.cpp:613-618` which
// tags every `SQL_SF_TIMESTAMP_{NTZ,LTZ,TZ}` bind as `TEXT` (with the
// comment: "Maintain the existing behavior to treat the timestamp as wall
// clock time and attach the session offset in the server when binding").
// The Snowflake server then coerces the text into the destination
// column's logical type. Sending `type=TIMESTAMP_LTZ` with a string value
// is rejected by the server with SQLSTATE 22000 "Invalid bind value (...)
// for type (TIMESTAMP_LTZ)", which is what made every LTZ e2e test fail
// across all platforms before this fix.
impl WriteWire for SnowflakeTimestampLtz {
    fn write_wire(
        &self,
        value: <Self as SnowflakeType>::Representation<'_>,
    ) -> Result<String, BindingError> {
        write_timestamp_wire_wallclock(value)
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::Text
    }
}

pub(crate) struct SnowflakeTimestampTz {
    pub(crate) scale: u32,
    /// Set from the session's `TIMESTAMP_TZ_OUTPUT_FORMAT` at converter
    /// construction time. `None` means "keep the legacy UTC-only fetch
    /// behaviour" (the driver emits the bare wall-clock and drops the
    /// offset, matching ODBC's "drop offset" rule). `Some(_)` means the
    /// customer's format string contains a `TZH/TZM/TZHTZM` token, so
    /// CHAR/WCHAR fetches emit `<utc_wall_clock> <offset_suffix>` to
    /// preserve the original observer's offset.
    pub(crate) tz_offset_format: Option<TzOffsetFormat>,
}

impl SnowflakeType for SnowflakeTimestampTz {
    type Representation<'a> = TzInstant;

    fn validate_value(&self, value: &TzInstant) -> Result<(), ConversionError> {
        check_sql_year(&value.utc)
    }
}

impl ReadArrowType<StructArray> for SnowflakeTimestampTz {
    fn read_arrow_type<'a>(
        &self,
        array: &'a StructArray,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        read_struct_timestamp_tz(array, row_idx, self.scale)
    }
}

// TZ never actually arrives as a flat Int64 array in practice -- Snowflake
// always sends it as a Struct with the offset column. The trait impl exists
// only to satisfy `make_timestamp_converter!`, which compiles a flat-Int64
// branch for every timestamp type. If TZ ever did show up flat, treating the
// epoch as UTC + offset 0 is the safe fallback (matches what NTZ does).
impl ReadArrowType<PrimitiveArray<Int64Type>> for SnowflakeTimestampTz {
    fn read_arrow_type<'a>(
        &self,
        array: &'a PrimitiveArray<Int64Type>,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        let utc = read_scaled_timestamp(array, row_idx, self.scale)?;
        Ok(TzInstant {
            utc,
            offset_minutes: 0,
        })
    }
}

impl WriteODBCType for SnowflakeTimestampTz {
    fn sql_type(&self) -> sql::SqlDataType {
        sql::SqlDataType::TIMESTAMP
    }

    fn column_size(&self) -> sql::ULen {
        let base: sql::ULen = if self.scale == 0 {
            19
        } else {
            20 + self.scale as sql::ULen
        };
        // When the session asked for a format with an offset token, the
        // CHAR/WCHAR fetch path appends ` +HH:MM` (7 chars worst-case;
        // `+HHMM` = 6, `+HH` = 4 for the trimmed variants). Advertising
        // the worst case keeps applications that size buffers from the
        // column descriptor safe regardless of which token they chose.
        if self.tz_offset_format.is_some() {
            base + 7
        } else {
            base
        }
    }

    fn decimal_digits(&self) -> sql::SmallInt {
        self.scale as sql::SmallInt
    }

    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
        get_data_offset: &mut Option<usize>,
    ) -> Result<Warnings, WriteOdbcError> {
        // For SQL_C_CHAR / SQL_C_WCHAR specifically: if the session set
        // `TIMESTAMP_TZ_OUTPUT_FORMAT` to a value containing TZH/TZM/TZHTZM
        // tokens, render `<wall_clock> +/-HH:MM` (matching the legacy
        // 3.16.0 driver and the format the server itself produces for the
        // same token). For every other C target — and when no offset
        // format is configured — keep the legacy UTC-only behaviour: the
        // ODBC spec rule "datetime with timezone -> datetime without
        // timezone drops the offset" applies to the typed targets, and
        // omitting it from CHAR/WCHAR by default avoids surprising apps
        // that already parse the bare `YYYY-MM-DD HH:MM:SS` shape. The
        // original offset is still preserved on the *bind* side via
        // `write_timestamp_tz_wire`, so values round-trip correctly when
        // written back to the server regardless of fetch rendering.
        if let Some(fmt) = self.tz_offset_format
            && matches!(binding.target_type, CDataType::Char | CDataType::WChar)
        {
            return write_timestamp_tz_to_char(&snowflake_value, fmt, binding, get_data_offset);
        }
        write_timestamp_to_odbc(&snowflake_value.utc, binding, get_data_offset)
    }
}

/// CHAR/WCHAR writer that appends `+/-HH[:]MM` to the wall-clock string.
///
/// Buffer-size handling follows the standard ODBC fetch contract: render
/// the full string into a 64-byte stack buffer, then let
/// `write_char_string` / `write_wchar_string` apply the spec-mandated
/// 01004 ("String data, right truncation") + `SQL_SUCCESS_WITH_INFO` +
/// indicator-set-to-untruncated-length contract when the application
/// buffer is too small. Pre-emptively rejecting with 22003 ("Numeric
/// value out of range") would be the wrong category and would also
/// reject buffers that *would* have fit a shorter rendering (e.g. a
/// 25-byte buffer for a `HourOnly` whole-hour value at ~23 chars). See
/// PR #1068 review on `timestamp.rs:993`.
fn write_timestamp_tz_to_char(
    value: &TzInstant,
    fmt: TzOffsetFormat,
    binding: &Binding,
    get_data_offset: &mut Option<usize>,
) -> Result<Warnings, WriteOdbcError> {
    // 20 (`YYYY-MM-DD HH:MM:SS.`) + up to 9 fractional digits + 1 space
    // + 6 (`+HH:MM`) = 36 characters worst case, well under the 64-byte
    // stack buffer.
    let mut buf = [0u8; 64];
    let s = format_timestamp_tz_string_into(value, fmt, &mut buf)?;
    let warnings = if matches!(binding.target_type, CDataType::WChar) {
        binding.write_wchar_string(s, get_data_offset)
    } else {
        binding.write_char_string(s, get_data_offset)
    };
    Ok(warnings)
}

impl ReadODBC for SnowflakeTimestampTz {
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, BindingError> {
        read_timestamp_tz_odbc(binding)
    }
}

impl WriteWire for SnowflakeTimestampTz {
    fn write_wire(&self, value: Self::Representation<'_>) -> Result<String, BindingError> {
        write_timestamp_tz_wire(value)
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::TimestampTz
    }
}

#[cfg(test)]
mod format_timestamp_string_into_tests {
    use super::format_timestamp_string_into;
    use chrono::{DateTime, NaiveDate};

    fn fmt(secs: i64, nanos: u32) -> String {
        let dt = DateTime::from_timestamp(secs, nanos)
            .expect("DateTime::from_timestamp with in-range inputs")
            .naive_utc();
        let mut buf = [0u8; 48];
        format_timestamp_string_into(&dt, &mut buf)
            .expect("format_timestamp_string_into")
            .to_string()
    }

    // 2023-11-14 22:13:20 UTC, an arbitrary mid-range instant used to exercise
    // the fractional-seconds trimming paths.
    const REF_EPOCH: i64 = 1_700_000_000;

    #[test]
    fn no_fractional_seconds() {
        assert_eq!(fmt(0, 0), "1970-01-01 00:00:00");
        assert_eq!(fmt(REF_EPOCH, 0), "2023-11-14 22:13:20");
    }

    #[test]
    fn with_fractional_seconds_various_trailing_zero_counts() {
        // Trailing-zero trimming is the interesting behavior to preserve.
        assert_eq!(fmt(REF_EPOCH, 1), "2023-11-14 22:13:20.000000001");
        assert_eq!(fmt(REF_EPOCH, 10), "2023-11-14 22:13:20.00000001");
        assert_eq!(fmt(REF_EPOCH, 123_000_000), "2023-11-14 22:13:20.123");
        assert_eq!(fmt(REF_EPOCH, 123_456_789), "2023-11-14 22:13:20.123456789");
        assert_eq!(fmt(REF_EPOCH, 999_999_999), "2023-11-14 22:13:20.999999999");
    }

    #[test]
    fn pre_epoch_timestamp() {
        assert_eq!(fmt(-1_000, 0), "1969-12-31 23:43:20");
        assert_eq!(fmt(-1_000, 500_000), "1969-12-31 23:43:20.0005");
    }

    #[test]
    fn year_padding() {
        let dt = NaiveDate::from_ymd_opt(1, 1, 1)
            .expect("NaiveDate::from_ymd_opt with in-range inputs")
            .and_hms_opt(0, 0, 0)
            .expect("NaiveDate::and_hms_opt with in-range inputs");
        let mut buf = [0u8; 48];
        assert_eq!(
            format_timestamp_string_into(&dt, &mut buf).expect("format_timestamp_string_into"),
            "0001-01-01 00:00:00"
        );
    }
}

#[cfg(test)]
mod parse_tz_string_with_fallback_tests {
    use super::*;
    use crate::conversion::error::BindingError;

    #[test]
    fn unparseable_string_returns_invalid_character_value_for_cast() {
        // The new error path: a string that matches none of the accepted
        // formats must surface as `InvalidCharacterValueForCast` so the
        // outer `to_sql_state` mapping returns 22018, not the previous
        // 07006 from the catch-all `UnsupportedCDataType`. See PR #1005
        // review on `timestamp.rs:643`.
        let err = parse_tz_string_with_fallback("not-a-timestamp", CDataType::Char)
            .expect_err("garbage input must not parse");
        match err {
            BindingError::InvalidCharacterValueForCast {
                c_type,
                value,
                expected_format,
                ..
            } => {
                assert!(matches!(c_type, CDataType::Char));
                assert_eq!(value, "not-a-timestamp");
                assert_eq!(expected_format, TZ_CHAR_EXPECTED_FORMAT);
            }
            other => panic!("expected InvalidCharacterValueForCast, got {other:?}"),
        }
    }

    #[test]
    fn long_input_is_truncated_in_diagnostic() {
        // Diagnostic-record buffers are bounded; pin the truncation
        // contract so an adversarial caller can't blow them up by binding
        // a megabyte literal. The expected format is static, so we only
        // need to assert on `value.len()` here.
        let huge = "x".repeat(1024);
        let err = parse_tz_string_with_fallback(&huge, CDataType::WChar)
            .expect_err("garbage input must not parse");
        match err {
            BindingError::InvalidCharacterValueForCast { value, .. } => {
                assert_eq!(
                    value.len(),
                    TEMPORAL_CHAR_DIAG_MAX_CHARS,
                    "diagnostic value must be truncated to TEMPORAL_CHAR_DIAG_MAX_CHARS"
                );
            }
            other => panic!("expected InvalidCharacterValueForCast, got {other:?}"),
        }
    }

    #[test]
    fn offset_suffix_parses_and_preserves_offset() {
        // Sanity check that the happy path still works alongside the new
        // error path -- a valid `+05:30` suffix yields the right
        // `offset_minutes` and the offset-applied UTC instant.
        let ti = parse_tz_string_with_fallback("2024-03-15 14:30:45 +05:30", CDataType::Char)
            .expect("valid TZ string parses");
        assert_eq!(ti.offset_minutes, 5 * 60 + 30);
        // 14:30 +05:30 -> 09:00 UTC
        assert_eq!(
            ti.utc.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2024-03-15 09:00:45"
        );
    }

    #[test]
    fn offsetless_input_falls_back_to_utc() {
        // Backward-compat path: a string with no offset suffix is treated
        // as UTC. A regression that flipped this to a parse failure would
        // break every legacy app that binds a naive timestamp string to a
        // TZ column.
        let ti = parse_tz_string_with_fallback("2024-03-15 14:30:45", CDataType::Char)
            .expect("offset-less input must parse as UTC");
        assert_eq!(ti.offset_minutes, 0);
        assert_eq!(
            ti.utc.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2024-03-15 14:30:45"
        );
    }
}

#[cfg(test)]
mod write_timestamp_tz_wire_tests {
    use super::*;

    #[test]
    fn out_of_range_instant_returns_datetime_field_overflow() {
        // `chrono::NaiveDateTime::and_utc().timestamp_nanos_opt()` returns
        // `None` outside roughly 1677-09-21..2262-04-11 because it can't
        // fit in `i64` nanoseconds. This must surface as 22008 (Datetime
        // field overflow), not the previous 07006 from the catch-all
        // `UnsupportedCDataType`. See PR #1005 review on `timestamp.rs:643`.
        let out_of_range = NaiveDate::from_ymd_opt(9999, 12, 31)
            .and_then(|d| d.and_hms_opt(23, 59, 59))
            .expect("constant inputs");
        let err = write_timestamp_tz_wire(TzInstant {
            utc: out_of_range,
            offset_minutes: 0,
        })
        .expect_err("year 9999 cannot fit in i64 nanoseconds");
        assert!(
            matches!(err, BindingError::DatetimeFieldOverflow { .. }),
            "expected DatetimeFieldOverflow, got {err:?}"
        );
    }

    #[test]
    fn in_range_instant_emits_two_token_wire_format() {
        // Sanity check that the happy path still works -- the format is
        // `<epoch_ns> <offset_minutes + 1440>`. A regression that reorders
        // tokens or drops the bias would be caught by this unit test
        // before it hit the wire.
        let dt = NaiveDate::from_ymd_opt(2024, 3, 15)
            .and_then(|d| d.and_hms_opt(9, 0, 45))
            .expect("constant inputs");
        let v = write_timestamp_tz_wire(TzInstant {
            utc: dt,
            offset_minutes: 5 * 60 + 30,
        })
        .expect("in-range UTC instant serialises");
        // 2024-03-15T09:00:45 UTC == 1710493245 epoch seconds == 1710493245000000000 ns.
        // 330 + 1440 = 1770.
        assert_eq!(v, "1710493245000000000 1770");
    }
}
