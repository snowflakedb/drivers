//! Bind-parameter converters for SQL_INTERVAL_* targets.
//!
//! Until #980 these were routed through `SnowflakeVarchar`, which silently
//! accepted ~every C type. ODBC Appendix D ("Converting Data from C to SQL
//! Data Types") only permits a narrow set of C sources for each interval
//! family:
//!
//!   * **Single-field** YEAR / MONTH / DAY / HOUR / MINUTE / SECOND accept
//!     character types, every exact-numeric C type (signed/unsigned int,
//!     SQL_C_NUMERIC, SQL_C_BIT) and any C interval type from the same
//!     family (year-month or day-time).
//!   * **Compound** YEAR_TO_MONTH / DAY_TO_HOUR / DAY_TO_MINUTE /
//!     DAY_TO_SECOND / HOUR_TO_MINUTE / HOUR_TO_SECOND / MINUTE_TO_SECOND
//!     accept only character types and same-family C interval types — no
//!     numeric C source can produce more than one field.
//!   * SQL_C_FLOAT / SQL_C_DOUBLE / SQL_C_BINARY / SQL_C_GUID and any
//!     date/time C type are never legal for any interval target, and
//!     cross-family conversions (e.g. SQL_C_INTERVAL_DAY → SQL_INTERVAL_YEAR)
//!     are also disallowed.
//!
//! These converters reject every other source with SQLSTATE 07006
//! ("restricted data type attribute violation"), aligning the bind side
//! with the result side that already uses `INTERVAL_YEAR_MONTH` /
//! `INTERVAL_DAY_TIME` logical types.
//!
//! The JSON wire format is `{"type": "INTERVAL_YEAR_MONTH" | "INTERVAL_DAY_TIME",
//! "value": "<ANSI literal>"}`. The ANSI literal is the same one emitted
//! by `format_interval` for SQL_C_INTERVAL_* sources, so applications can
//! still round-trip intervals as text by binding SQL_C_CHAR / SQL_C_WCHAR.

use std::borrow::Cow;

use odbc_sys as sql;
use serde_json::Value;

use crate::api::{CDataType, ParameterBinding};
use crate::conversion::error::{JsonBindingError, UnsupportedCDataTypeSnafu};
use crate::conversion::param_binding::{
    read_char_str, read_numeric_struct, read_unaligned, read_wchar_str,
};
use crate::conversion::traits::{ReadODBC, SnowflakeLogicalType, SnowflakeType, WriteJson};

// =============================================================================
// Subtype enums — stable, family-typed view over SQL_INTERVAL_* concise codes.
// =============================================================================

/// Year-month interval subtype (SQL_INTERVAL_YEAR/MONTH/YEAR_TO_MONTH).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum YearMonthSubtype {
    Year,        // SQL_INTERVAL_YEAR (101)
    Month,       // SQL_INTERVAL_MONTH (102)
    YearToMonth, // SQL_INTERVAL_YEAR_TO_MONTH (107)
}

impl YearMonthSubtype {
    fn is_compound(self) -> bool {
        matches!(self, Self::YearToMonth)
    }
}

/// Day-time interval subtype, covering both single-field and compound
/// variants (SQL_INTERVAL_DAY..SECOND and SQL_INTERVAL_DAY_TO_HOUR..
/// MINUTE_TO_SECOND).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DayTimeSubtype {
    Day,            // SQL_INTERVAL_DAY (103)
    Hour,           // SQL_INTERVAL_HOUR (104)
    Minute,         // SQL_INTERVAL_MINUTE (105)
    Second,         // SQL_INTERVAL_SECOND (106)
    DayToHour,      // SQL_INTERVAL_DAY_TO_HOUR (108)
    DayToMinute,    // SQL_INTERVAL_DAY_TO_MINUTE (109)
    DayToSecond,    // SQL_INTERVAL_DAY_TO_SECOND (110)
    HourToMinute,   // SQL_INTERVAL_HOUR_TO_MINUTE (111)
    HourToSecond,   // SQL_INTERVAL_HOUR_TO_SECOND (112)
    MinuteToSecond, // SQL_INTERVAL_MINUTE_TO_SECOND (113)
}

impl DayTimeSubtype {
    fn is_compound(self) -> bool {
        !matches!(self, Self::Day | Self::Hour | Self::Minute | Self::Second)
    }

    fn is_second(self) -> bool {
        matches!(self, Self::Second)
    }
}

/// Map a SQL data type code (101..=113) to its corresponding subtype, or
/// `None` if the code does not name a SQL_INTERVAL_* type. Used by the
/// `make_converter` factory in `param_binding.rs` to decide which of the
/// two interval converters to instantiate.
pub(crate) fn year_month_subtype_from_sql(code: i16) -> Option<YearMonthSubtype> {
    match code {
        101 => Some(YearMonthSubtype::Year),
        102 => Some(YearMonthSubtype::Month),
        107 => Some(YearMonthSubtype::YearToMonth),
        _ => None,
    }
}

pub(crate) fn day_time_subtype_from_sql(code: i16) -> Option<DayTimeSubtype> {
    match code {
        103 => Some(DayTimeSubtype::Day),
        104 => Some(DayTimeSubtype::Hour),
        105 => Some(DayTimeSubtype::Minute),
        106 => Some(DayTimeSubtype::Second),
        108 => Some(DayTimeSubtype::DayToHour),
        109 => Some(DayTimeSubtype::DayToMinute),
        110 => Some(DayTimeSubtype::DayToSecond),
        111 => Some(DayTimeSubtype::HourToMinute),
        112 => Some(DayTimeSubtype::HourToSecond),
        113 => Some(DayTimeSubtype::MinuteToSecond),
        _ => None,
    }
}

// =============================================================================
// Converter structs
// =============================================================================

pub(crate) struct SnowflakeIntervalYearMonth {
    pub subtype: YearMonthSubtype,
}

pub(crate) struct SnowflakeIntervalDayTime {
    pub subtype: DayTimeSubtype,
}

impl SnowflakeType for SnowflakeIntervalYearMonth {
    type Representation<'a> = Cow<'a, str>;
}

impl SnowflakeType for SnowflakeIntervalDayTime {
    type Representation<'a> = Cow<'a, str>;
}

// =============================================================================
// ReadODBC — strict source-type validation per ODBC Appendix D.
// =============================================================================

impl ReadODBC for SnowflakeIntervalYearMonth {
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, JsonBindingError> {
        let s = match binding.value_type {
            // Character sources are always legal — server-side parses the literal.
            CDataType::Default | CDataType::Char => read_char_str(binding)?,
            CDataType::WChar => read_wchar_str(binding)?,

            // Same-family C interval sources are always legal (single OR
            // compound target). Cross-family interval sources fall through
            // to the unsupported arm below.
            CDataType::IntervalYear | CDataType::IntervalMonth | CDataType::IntervalYearToMonth => {
                format_interval(binding)
            }

            // Exact-numeric sources only legal for single-field targets.
            CDataType::TinyInt | CDataType::STinyInt => {
                self.render_signed(read_unaligned::<i8>(binding) as i128, binding)?
            }
            CDataType::UTinyInt => {
                self.render_signed(read_unaligned::<u8>(binding) as i128, binding)?
            }
            CDataType::Short | CDataType::SShort => {
                self.render_signed(read_unaligned::<i16>(binding) as i128, binding)?
            }
            CDataType::UShort => {
                self.render_signed(read_unaligned::<u16>(binding) as i128, binding)?
            }
            CDataType::Long | CDataType::SLong => {
                self.render_signed(read_unaligned::<i32>(binding) as i128, binding)?
            }
            CDataType::ULong => {
                self.render_signed(read_unaligned::<u32>(binding) as i128, binding)?
            }
            CDataType::SBigInt => {
                self.render_signed(read_unaligned::<i64>(binding) as i128, binding)?
            }
            CDataType::UBigInt => {
                self.render_signed(read_unaligned::<u64>(binding) as i128, binding)?
            }
            CDataType::Bit => self.render_signed(read_unaligned::<u8>(binding) as i128, binding)?,
            CDataType::Numeric => {
                let (mantissa, scale) = read_numeric_struct(binding)?;
                self.render_numeric(mantissa, scale, binding)?
            }

            _ => return unsupported(binding.value_type),
        };
        Ok(Cow::Owned(s))
    }
}

impl ReadODBC for SnowflakeIntervalDayTime {
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, JsonBindingError> {
        let s = match binding.value_type {
            CDataType::Default | CDataType::Char => read_char_str(binding)?,
            CDataType::WChar => read_wchar_str(binding)?,

            // All ten day-time C interval sources are legal for any
            // day-time SQL target (single or compound).
            CDataType::IntervalDay
            | CDataType::IntervalHour
            | CDataType::IntervalMinute
            | CDataType::IntervalSecond
            | CDataType::IntervalDayToHour
            | CDataType::IntervalDayToMinute
            | CDataType::IntervalDayToSecond
            | CDataType::IntervalHourToMinute
            | CDataType::IntervalHourToSecond
            | CDataType::IntervalMinuteToSecond => format_interval(binding),

            CDataType::TinyInt | CDataType::STinyInt => {
                self.render_signed(read_unaligned::<i8>(binding) as i128, binding)?
            }
            CDataType::UTinyInt => {
                self.render_signed(read_unaligned::<u8>(binding) as i128, binding)?
            }
            CDataType::Short | CDataType::SShort => {
                self.render_signed(read_unaligned::<i16>(binding) as i128, binding)?
            }
            CDataType::UShort => {
                self.render_signed(read_unaligned::<u16>(binding) as i128, binding)?
            }
            CDataType::Long | CDataType::SLong => {
                self.render_signed(read_unaligned::<i32>(binding) as i128, binding)?
            }
            CDataType::ULong => {
                self.render_signed(read_unaligned::<u32>(binding) as i128, binding)?
            }
            CDataType::SBigInt => {
                self.render_signed(read_unaligned::<i64>(binding) as i128, binding)?
            }
            CDataType::UBigInt => {
                self.render_signed(read_unaligned::<u64>(binding) as i128, binding)?
            }
            CDataType::Bit => self.render_signed(read_unaligned::<u8>(binding) as i128, binding)?,
            CDataType::Numeric => {
                let (mantissa, scale) = read_numeric_struct(binding)?;
                self.render_numeric(mantissa, scale, binding)?
            }

            _ => return unsupported(binding.value_type),
        };
        Ok(Cow::Owned(s))
    }
}

impl SnowflakeIntervalYearMonth {
    fn render_signed(
        &self,
        value: i128,
        binding: &ParameterBinding,
    ) -> Result<String, JsonBindingError> {
        if self.subtype.is_compound() {
            return unsupported::<String>(binding.value_type);
        }
        Ok(value.to_string())
    }

    fn render_numeric(
        &self,
        mantissa: i128,
        scale: i8,
        binding: &ParameterBinding,
    ) -> Result<String, JsonBindingError> {
        if self.subtype.is_compound() {
            return unsupported::<String>(binding.value_type);
        }
        // YEAR / MONTH are integer-valued; truncate any fractional digits
        // toward zero. The ODBC spec records this as truncation warning
        // 22015 ("interval field overflow") on the server side; we just
        // emit the integer literal here.
        Ok(format_integer_part(mantissa, scale))
    }
}

impl SnowflakeIntervalDayTime {
    fn render_signed(
        &self,
        value: i128,
        binding: &ParameterBinding,
    ) -> Result<String, JsonBindingError> {
        if self.subtype.is_compound() {
            return unsupported::<String>(binding.value_type);
        }
        if self.subtype.is_second() {
            // Integer C source bound to SECOND has no fractional part; emit
            // the canonical "<int>.000000" so the literal width matches the
            // spec-default seconds precision.
            Ok(format!("{value}.000000"))
        } else {
            Ok(value.to_string())
        }
    }

    fn render_numeric(
        &self,
        mantissa: i128,
        scale: i8,
        binding: &ParameterBinding,
    ) -> Result<String, JsonBindingError> {
        if self.subtype.is_compound() {
            return unsupported::<String>(binding.value_type);
        }
        if self.subtype.is_second() {
            // Preserve up to 6 fractional digits; anything beyond that is
            // truncated (the server reports 22015 for lost precision).
            Ok(format_seconds_value(mantissa, scale))
        } else {
            // Non-SECOND single-field targets are integer-valued; truncate
            // any fractional component.
            Ok(format_integer_part(mantissa, scale))
        }
    }
}

fn unsupported<T>(c_type: CDataType) -> Result<T, JsonBindingError> {
    UnsupportedCDataTypeSnafu { c_type }.fail()
}

// =============================================================================
// Numeric → text formatting helpers (used for integer-valued targets and
// SECOND fractional rendering).
// =============================================================================

/// Render the integer part of a scaled decimal value, truncating any
/// fractional digits toward zero. Used for non-SECOND single-field
/// interval targets, which are integer-valued per the ODBC spec.
fn format_integer_part(value: i128, scale: i8) -> String {
    if scale == 0 {
        return value.to_string();
    }
    if scale < 0 {
        // Negative scale means trailing zeros — the value is already
        // integral; just append the zeros.
        let trailing_zeros = if scale == i8::MIN {
            (i8::MAX as usize) + 1
        } else {
            (-scale) as usize
        };
        let mut s = value.to_string();
        s.extend(std::iter::repeat_n('0', trailing_zeros));
        return s;
    }
    let scale = scale as u32;
    let divisor = match 10i128.checked_pow(scale) {
        Some(d) => d,
        // scale exceeds i128 magnitude: any non-zero value is < divisor,
        // so truncated integer is 0.
        None => return "0".to_string(),
    };
    (value / divisor).to_string()
}

/// Render a scaled decimal value as an ANSI INTERVAL_SECOND literal:
/// `<sign><int>.<6-digit-fraction>`. Excess scale beyond microseconds is
/// truncated toward zero (matches the spec-default seconds precision of 6
/// and how the result-side `compute_interval_fraction` handles `scale > 6`).
fn format_seconds_value(value: i128, scale: i8) -> String {
    let is_neg = value < 0;
    let abs = value.unsigned_abs();
    let (int_part, frac_us) = split_scaled_to_seconds(abs, scale);
    // Sign is suppressed when the resulting magnitude is exactly zero, so
    // we don't emit "-0.000000" for a value that rounded down to nothing.
    let sign = if is_neg && (int_part > 0 || frac_us > 0) {
        "-"
    } else {
        ""
    };
    format!("{sign}{int_part}.{frac_us:06}")
}

/// Split a `(magnitude, scale)` pair into `(integer_seconds, fraction_us)`,
/// truncating any precision beyond 6 fractional digits.
fn split_scaled_to_seconds(abs: u128, scale: i8) -> (u128, u32) {
    if scale <= 0 {
        let trailing = if scale == 0 {
            0u32
        } else if scale == i8::MIN {
            // Avoid overflow when negating MIN; values with this scale are
            // astronomically large and exceed any sane interval, but we
            // still produce a deterministic output rather than panicking.
            return (u128::MAX, 0);
        } else {
            (-scale) as u32
        };
        let multiplier = 10u128.checked_pow(trailing).unwrap_or(u128::MAX);
        let int_part = abs.saturating_mul(multiplier);
        return (int_part, 0);
    }
    let scale = scale as u32;
    let divisor = match 10u128.checked_pow(scale) {
        Some(d) => d,
        None => return (0, 0),
    };
    let int_part = abs / divisor;
    let frac_part = abs % divisor;
    let frac_us = if scale >= 6 {
        // Truncate excess scale beyond microseconds.
        let extra = 10u128.pow(scale - 6);
        (frac_part / extra) as u32
    } else {
        let pad = 10u128.pow(6 - scale);
        (frac_part * pad) as u32
    };
    (int_part, frac_us)
}

// =============================================================================
// format_interval — formats an SQL_INTERVAL_STRUCT into its ANSI literal
// using the C source's subtype to select the active fields.
// =============================================================================

/// Format a `SQL_INTERVAL_STRUCT` as the ANSI SQL interval literal text
/// the ODBC spec specifies for each `SQL_C_INTERVAL_*` subtype. The chosen
/// fields come from `binding.value_type`, not the struct's `interval_type`
/// field — drivers MUST trust the C type set on the binding (some
/// applications never bother filling in `interval_type`).
///
/// We deliberately do NOT copy the whole `SQL_INTERVAL_STRUCT`, nor even
/// the active `YearMonth` / `DaySecond` variant of its union, up front:
/// applications routinely populate only the specific field(s) their
/// `SQL_C_INTERVAL_*` subtype requires (e.g. just `intval.year_month.year`
/// for `SQL_C_INTERVAL_YEAR`). A wholesale `read_unaligned::<YearMonth>`
/// would still copy `month`, and a `read_unaligned::<DaySecond>` would
/// copy all five fields — including bytes the app never wrote. Reading
/// uninitialised memory through `read_unaligned` is undefined behaviour
/// in Rust even when the resulting value's type tolerates every bit
/// pattern, so each match arm reads only the specific u32 fields its
/// variant uses, at the offsets fixed by the `repr(C)` layout in
/// `odbc_sys::IntervalStruct`. See the per-arm comments below for the
/// exact offset table.
pub(crate) fn format_interval(binding: &ParameterBinding) -> String {
    // SAFETY: callers (`SnowflakeVarchar::read_odbc` and the interval
    // converters) only dispatch here for a SQL_C_INTERVAL_* C type; for
    // those types ODBC requires the application to pass a buffer of at
    // least `sizeof(SQL_INTERVAL_STRUCT)`, so every offset we read below
    // stays within the application-supplied buffer.
    //
    //   offset  0  interval_type   c_int    (4 bytes, ignored — see below)
    //   offset  4  interval_sign   i16      (2 bytes, read once)
    //   offset  6  padding                  (2 bytes, before the 4-aligned union)
    //   offset  8  year_month.year / day_second.day        (u32)
    //   offset 12  year_month.month / day_second.hour      (u32)
    //   offset 16  day_second.minute                       (u32)
    //   offset 20  day_second.second                       (u32)
    //   offset 24  day_second.fraction                     (u32, microseconds)
    const SIGN_OFFSET: usize = 4;
    const F0_OFFSET: usize = 8; // year / day
    const F1_OFFSET: usize = 12; // month / hour
    const F2_OFFSET: usize = 16; // minute
    const F3_OFFSET: usize = 20; // second
    const F4_OFFSET: usize = 24; // fraction (us)

    let base = binding.parameter_value_ptr as *const u8;
    let sign_raw: sql::SmallInt =
        unsafe { std::ptr::read_unaligned(base.add(SIGN_OFFSET) as *const sql::SmallInt) };
    let sign = if sign_raw != 0 { "-" } else { "" };
    let read_u32 = |off: usize| unsafe { std::ptr::read_unaligned(base.add(off) as *const u32) };

    /// Render `<seconds>.<fraction>` with the fraction zero-padded to 6
    /// digits and the decimal point always present. Per the ODBC spec
    /// (https://learn.microsoft.com/en-us/sql/odbc/reference/appendixes/interval-data-type-length)
    /// the seconds-precision component contributes "1 plus the express or
    /// implied seconds precision" characters, defaulting to 6 fractional
    /// digits. When `pad_int` is true the integer part is also zero-padded
    /// to 2 digits — used when seconds appears as a sub-field after a `:`
    /// (e.g. `12:30:05.000000`); when false (INTERVAL_SECOND leading
    /// field), the integer is rendered as-is since the leading-field
    /// precision can exceed two digits.
    fn fmt_seconds(second: u32, fraction: u32, pad_int: bool) -> String {
        let int_part = if pad_int {
            format!("{second:02}")
        } else {
            second.to_string()
        };
        format!("{int_part}.{fraction:06}")
    }

    match binding.value_type {
        CDataType::IntervalYear => format!("{sign}{}", read_u32(F0_OFFSET)),
        CDataType::IntervalMonth => format!("{sign}{}", read_u32(F1_OFFSET)),
        CDataType::IntervalDay => format!("{sign}{}", read_u32(F0_OFFSET)),
        CDataType::IntervalHour => format!("{sign}{}", read_u32(F1_OFFSET)),
        CDataType::IntervalMinute => format!("{sign}{}", read_u32(F2_OFFSET)),
        CDataType::IntervalSecond => format!(
            "{sign}{}",
            fmt_seconds(read_u32(F3_OFFSET), read_u32(F4_OFFSET), false),
        ),
        CDataType::IntervalYearToMonth => {
            format!("{sign}{}-{:02}", read_u32(F0_OFFSET), read_u32(F1_OFFSET))
        }
        CDataType::IntervalDayToHour => {
            format!("{sign}{} {:02}", read_u32(F0_OFFSET), read_u32(F1_OFFSET))
        }
        CDataType::IntervalDayToMinute => format!(
            "{sign}{} {:02}:{:02}",
            read_u32(F0_OFFSET),
            read_u32(F1_OFFSET),
            read_u32(F2_OFFSET),
        ),
        CDataType::IntervalDayToSecond => format!(
            "{sign}{} {:02}:{:02}:{}",
            read_u32(F0_OFFSET),
            read_u32(F1_OFFSET),
            read_u32(F2_OFFSET),
            fmt_seconds(read_u32(F3_OFFSET), read_u32(F4_OFFSET), true),
        ),
        CDataType::IntervalHourToMinute => {
            format!("{sign}{}:{:02}", read_u32(F1_OFFSET), read_u32(F2_OFFSET))
        }
        CDataType::IntervalHourToSecond => format!(
            "{sign}{}:{:02}:{}",
            read_u32(F1_OFFSET),
            read_u32(F2_OFFSET),
            fmt_seconds(read_u32(F3_OFFSET), read_u32(F4_OFFSET), true),
        ),
        CDataType::IntervalMinuteToSecond => format!(
            "{sign}{}:{}",
            read_u32(F2_OFFSET),
            fmt_seconds(read_u32(F3_OFFSET), read_u32(F4_OFFSET), true),
        ),
        // Callers gate on a C interval type; anything else here is a bug.
        other => unreachable!("format_interval called with non-interval C type {other:?}"),
    }
}

// =============================================================================
// WriteJson — emit `{"type": "INTERVAL_YEAR_MONTH" | "INTERVAL_DAY_TIME",
// "value": "<literal>"}`.
// =============================================================================

impl WriteJson for SnowflakeIntervalYearMonth {
    fn write_json(&self, value: Self::Representation<'_>) -> Result<Value, JsonBindingError> {
        Ok(Value::String(value.into_owned()))
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::IntervalYearMonth
    }
}

impl WriteJson for SnowflakeIntervalDayTime {
    fn write_json(&self, value: Self::Representation<'_>) -> Result<Value, JsonBindingError> {
        Ok(Value::String(value.into_owned()))
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::IntervalDayTime
    }
}
