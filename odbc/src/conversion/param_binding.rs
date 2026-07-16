use std::{
    ffi::{CStr, c_char},
    slice, str,
};

#[cfg(test)]
use std::mem;

use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt};

use crate::api::CDataType;
use crate::api::TimestampSubtype;
use crate::api::encoding::{OdbcEncoding, Wide, WideChar, wchar_byte_size, wide_strlen_bounded};
use crate::api::{ApdDescriptor, IpdDescriptor, ParameterBinding, SQL_PARAM_IGNORE};
use odbc_sys as sql;

use super::binary::SnowflakeBinary;
use super::boolean::SnowflakeBoolean;
use super::date::SnowflakeDate;
#[cfg(not(windows))]
use super::error::InvalidUtf8Snafu;
use super::error::{
    BindingError, BindingNumericOutOfRangeSnafu, InvalidCharacterValueForCastSnafu,
    InvalidParameterIndicesSnafu, NullPointerSnafu, NumericMagnitudeOverflowSnafu,
    SerializationSnafu, UnsupportedCDataTypeSnafu, UnsupportedParameterTypeSnafu,
    WCharConversionSnafu,
};
use super::interval::{
    SnowflakeIntervalDayTime, SnowflakeIntervalYearMonth, day_time_subtype_from_sql,
    read_single_field_interval_i128, year_month_subtype_from_sql,
};
use super::number::{NumericSqlType, SnowflakeNumber};
use super::real::SnowflakeReal;
use super::time::SnowflakeTime;
use super::timestamp::{SnowflakeTimestampLtz, SnowflakeTimestampNtz, SnowflakeTimestampTz};
use super::traits::{ReadODBC, SnowflakeLogicalType, SnowflakeType, WriteWire};
use super::varchar::SnowflakeVarchar;

// =============================================================================
// ParamConverter trait (public interface)
// =============================================================================

/// Trait for converting an ODBC parameter binding into the canonical wire-text
/// payload Snowflake's parameter-binding protocol expects.
///
/// Returns `(sf_type, text)`; the format-specific encoder
/// (`odbc_bindings_to_json` for inline JSON, `odbc_bindings_to_csv` for the
/// stage-binding CSV file) supplies the envelope around the text.  Keeping
/// converters format-agnostic means every type has a single source of truth
/// for its wire representation.
pub(crate) trait ParamConverter {
    fn convert(
        &self,
        binding: &ParameterBinding,
    ) -> Result<(SnowflakeLogicalType, String), BindingError>;
}

/// Generic adapter: any type implementing `ReadODBC + WriteWire` automatically
/// gets a `ParamConverter` implementation via this wrapper.
struct WireParamConverter<T: ReadODBC + WriteWire> {
    snowflake_type: T,
}

impl<T: ReadODBC + WriteWire> ParamConverter for WireParamConverter<T> {
    fn convert(
        &self,
        binding: &ParameterBinding,
    ) -> Result<(SnowflakeLogicalType, String), BindingError> {
        let value = self.snowflake_type.read_odbc(binding)?;
        let text = self.snowflake_type.write_wire(value)?;
        Ok((self.snowflake_type.sf_type(), text))
    }
}

/// Parameter-only Snowflake type for SQL_DECIMAL / SQL_NUMERIC: reads the
/// value as canonical decimal text (covering every C source ODBC Appendix D
/// permits) and tags the wire payload as `FIXED` so the server applies
/// numeric semantics.
///
/// `Representation = String` so `WriteWire::write_wire` is the identity and
/// the generic `WireParamConverter` adapter handles the dispatch — no
/// special-case `ParamConverter` impl needed.
pub(crate) struct SnowflakeDecimal;

impl SnowflakeType for SnowflakeDecimal {
    type Representation<'a> = String;
}

impl ReadODBC for SnowflakeDecimal {
    fn read_odbc<'a>(
        &self,
        binding: &'a ParameterBinding,
    ) -> Result<Self::Representation<'a>, BindingError> {
        let s = match binding.value_type {
            CDataType::Char => read_char_str(binding)?,
            CDataType::WChar => read_wchar_str(binding)?,
            CDataType::Long | CDataType::SLong => read_unaligned::<i32>(binding).to_string(),
            CDataType::Short | CDataType::SShort => read_unaligned::<i16>(binding).to_string(),
            CDataType::SBigInt => read_unaligned::<i64>(binding).to_string(),
            CDataType::ULong => read_unaligned::<u32>(binding).to_string(),
            CDataType::UShort => read_unaligned::<u16>(binding).to_string(),
            CDataType::UBigInt => read_unaligned::<u64>(binding).to_string(),
            CDataType::TinyInt | CDataType::STinyInt => read_unaligned::<i8>(binding).to_string(),
            CDataType::UTinyInt => read_unaligned::<u8>(binding).to_string(),
            CDataType::Double => read_unaligned::<f64>(binding).to_string(),
            CDataType::Float => read_unaligned::<f32>(binding).to_string(),
            CDataType::Bit => read_unaligned::<u8>(binding).to_string(),
            CDataType::Numeric => {
                let (value, scale) = read_numeric_struct(binding)?;
                format_numeric_value(value, scale)
            }
            CDataType::Binary => {
                let len = buffer_data_len(binding);
                if len == std::mem::size_of::<sql::Numeric>() {
                    let (value, scale) = read_numeric_struct(binding)?;
                    format_numeric_value(value, scale)
                } else {
                    return BindingNumericOutOfRangeSnafu {
                        reason: format!(
                            "SQL_C_BINARY buffer length {len} does not match SQL_NUMERIC_STRUCT size ({})",
                            std::mem::size_of::<sql::Numeric>()
                        ),
                    }.fail();
                }
            }
            // Single-field SQL_C_INTERVAL_* sources resolve to the integer
            // count of the leading interval field per ODBC Appendix D
            // ("C to SQL Data Types: Interval"). For SQL_C_INTERVAL_SECOND
            // any sub-second `fraction` is truncated toward zero —
            // SQL_DECIMAL / SQL_NUMERIC targets carry their own scale on
            // the server, but the wire representation here is the integer
            // leading-field text, matching how `SnowflakeNumber::read_odbc`
            // handles the same source for the integer SQL targets. Compound
            // interval C types (YEAR_TO_MONTH, DAY_TO_*, HOUR_TO_*,
            // MINUTE_TO_SECOND) carry more than one field and have no
            // single-integer mapping; they fall through to the unsupported
            // arm below.
            CDataType::IntervalYear
            | CDataType::IntervalMonth
            | CDataType::IntervalDay
            | CDataType::IntervalHour
            | CDataType::IntervalMinute
            | CDataType::IntervalSecond => read_single_field_interval_i128(binding).to_string(),
            _ => {
                return UnsupportedParameterTypeSnafu {
                    sql_type: sql::SqlDataType::DECIMAL,
                }
                .fail();
            }
        };
        Ok(s)
    }
}

impl WriteWire for SnowflakeDecimal {
    fn write_wire(&self, value: Self::Representation<'_>) -> Result<String, BindingError> {
        Ok(value)
    }

    fn sf_type(&self) -> SnowflakeLogicalType {
        SnowflakeLogicalType::Fixed
    }
}

// =============================================================================
// Factory
// =============================================================================

// Concise SQL_INTERVAL_* type codes (101..=113) live in
// `odbc/src/conversion/interval.rs` as the `YearMonthSubtype` /
// `DayTimeSubtype` enums; the `*_subtype_from_sql` helpers map the FFI
// integer back to the typed family used by the dedicated converters.
// `odbc_sys::SqlDataType` does not expose named constants for these
// codes, so the `make_converter` arms below match on the helpers
// (returning `Option<…Subtype>`) rather than on raw `sql::SqlDataType`
// values.

/// Select the appropriate `ParamConverter` for the given parameter binding.
/// The SQL type determines the Snowflake logical type, which in turn knows
/// how to read various C data types from the ODBC buffer.
///
/// For `SQL_TYPE_TIMESTAMP` the dispatch additionally consults
/// `binding.sf_subtype`: `None` (and `Some(Ntz)`) routes to TIMESTAMP_NTZ
/// for backward compatibility with Tableau/Excel/Power BI; `Some(Ltz)` and
/// `Some(Tz)` route to the matching Snowflake logical type. Vendor-code
/// normalisation happens at `bind_parameter` time, so by the time we get
/// here `sql_data_type` is always a standard ODBC code.
fn make_converter(binding: &ParameterBinding) -> Result<Box<dyn ParamConverter>, BindingError> {
    let sql_type = &binding.sql_data_type;
    match *sql_type {
        sql::SqlDataType::INTEGER
        | sql::SqlDataType::SMALLINT
        | sql::SqlDataType::EXT_BIG_INT
        | sql::SqlDataType::EXT_TINY_INT => Ok(Box::new(WireParamConverter {
            snowflake_type: SnowflakeNumber {
                scale: 0,
                precision: 19,
                sql_type: NumericSqlType::BigInt,
            },
        })),

        sql::SqlDataType::REAL | sql::SqlDataType::FLOAT | sql::SqlDataType::DOUBLE => {
            Ok(Box::new(WireParamConverter {
                snowflake_type: SnowflakeReal,
            }))
        }

        sql::SqlDataType::VARCHAR
        | sql::SqlDataType::CHAR
        | sql::SqlDataType::EXT_LONG_VARCHAR
        | sql::SqlDataType::EXT_W_CHAR
        | sql::SqlDataType::EXT_W_VARCHAR
        | sql::SqlDataType::EXT_W_LONG_VARCHAR => Ok(Box::new(WireParamConverter {
            snowflake_type: SnowflakeVarchar {
                len: 0,
                is_semi_structured: false,
            },
        })),

        sql::SqlDataType::DECIMAL | sql::SqlDataType::NUMERIC => Ok(Box::new(WireParamConverter {
            snowflake_type: SnowflakeDecimal,
        })),

        sql::SqlDataType::EXT_BIT => Ok(Box::new(WireParamConverter {
            snowflake_type: SnowflakeBoolean,
        })),

        sql::SqlDataType::EXT_BINARY
        | sql::SqlDataType::EXT_VAR_BINARY
        | sql::SqlDataType::EXT_LONG_VAR_BINARY => Ok(Box::new(WireParamConverter {
            snowflake_type: SnowflakeBinary { len: 0 },
        })),

        // ODBC 3.x SQL_TYPE_DATE (=91) and its ODBC 2.x predecessor SQL_DATE
        // (=9, exposed in `odbc-sys` as `SqlDataType::DATETIME` because the
        // header value is shared with the datetime-header code) route to the
        // same converter. Per ODBC Appendix G, ODBC 3.x drivers must accept
        // either spelling and treat them as identical at the API boundary.
        sql::SqlDataType::DATE | sql::SqlDataType::DATETIME => Ok(Box::new(WireParamConverter {
            snowflake_type: SnowflakeDate,
        })),

        // ODBC 3.x SQL_TYPE_TIME (=92) and its ODBC 2.x predecessor SQL_TIME
        // (=10, exposed in `odbc-sys` as `EXT_TIME_OR_INTERVAL` because the
        // header value is shared with the interval-header code) route to the
        // same converter. Bare value 10 is unambiguously SQL_TIME at the
        // SQLBindParameter boundary: the interval subtypes use codes
        // 101-113 and are matched by their own guarded arms below.
        sql::SqlDataType::TIME | sql::SqlDataType::EXT_TIME_OR_INTERVAL => {
            Ok(Box::new(WireParamConverter {
                snowflake_type: SnowflakeTime { scale: 9 },
            }))
        }

        // ODBC 3.x SQL_TYPE_TIMESTAMP (=93) and its ODBC 2.x predecessor
        // SQL_TIMESTAMP (=11, exposed as `EXT_TIMESTAMP`) route to the same
        // converter (already covered before this PR; documented here for
        // symmetry with the new DATE / TIME alias arms above).
        //
        // SQL_TYPE_TIMESTAMP (93) routing depends on the optional Snowflake
        // vendor opt-in. Default (no opt-in) maps to TIMESTAMP_NTZ for
        // backward compatibility with Tableau/Excel/Power BI; explicit LTZ
        // and TZ opt-ins map to the corresponding Snowflake logical types.
        sql::SqlDataType::TIMESTAMP | sql::SqlDataType::EXT_TIMESTAMP => match binding.sf_subtype {
            None | Some(TimestampSubtype::Ntz) => Ok(Box::new(WireParamConverter {
                snowflake_type: SnowflakeTimestampNtz { scale: 9 },
            })),
            Some(TimestampSubtype::Ltz) => Ok(Box::new(WireParamConverter {
                snowflake_type: SnowflakeTimestampLtz { scale: 9 },
            })),
            // TZ binding emits the legacy two-token wire format
            // `"<epoch_nanoseconds> <offset_minutes_plus_1440>"` so the
            // server stores the original instant *and* its timezone
            // offset. SQL_C_TYPE_TIMESTAMP / SQL_C_BINARY binds (no
            // offset field) are serialised as UTC + offset 0, matching
            // the legacy Python connector's handling of naive
            // `datetime` values bound to TIMESTAMP_TZ. SQL_C_CHAR /
            // SQL_C_WCHAR binds parse a `+/-HH:MM` suffix from the
            // string and preserve that offset on the wire.
            //
            // `tz_offset_format` is a fetch-side concern only -- the
            // bind path's `WriteWire` always emits the offset
            // unconditionally -- so `None` is correct here.
            Some(TimestampSubtype::Tz) => Ok(Box::new(WireParamConverter {
                snowflake_type: SnowflakeTimestampTz {
                    scale: 9,
                    tz_offset_format: None,
                },
            })),
        },

        // SQL_INTERVAL_* parameter types route through dedicated
        // year-month / day-time converters that enforce the C-source
        // restrictions in ODBC Appendix D ("C to SQL" for INTERVAL targets):
        // single-field targets accept character + every exact-numeric C
        // type + same-family C interval types; compound targets accept
        // only character types and same-family C interval types. The
        // emitted wire `type` is `INTERVAL_YEAR_MONTH` / `INTERVAL_DAY_TIME`
        // to mirror the result-side logical type GS already uses for
        // native INTERVAL columns (see `sf_core::SnowflakeLogicalType`).
        sql_type if year_month_subtype_from_sql(sql_type.0).is_some() => {
            let subtype = year_month_subtype_from_sql(sql_type.0).expect("guarded by match arm");
            Ok(Box::new(WireParamConverter {
                snowflake_type: SnowflakeIntervalYearMonth { subtype },
            }))
        }
        sql_type if day_time_subtype_from_sql(sql_type.0).is_some() => {
            let subtype = day_time_subtype_from_sql(sql_type.0).expect("guarded by match arm");
            Ok(Box::new(WireParamConverter {
                snowflake_type: SnowflakeIntervalDayTime { subtype },
            }))
        }

        _ => {
            tracing::error!(
                "Unsupported SQL data type for parameter binding: {:?}",
                sql_type
            );
            UnsupportedParameterTypeSnafu {
                sql_type: *sql_type,
            }
            .fail()
        }
    }
}

// =============================================================================
// Pipeline
// =============================================================================

/// Convert ODBC parameter bindings (from APD + IPD descriptors) to JSON
/// string format for server-side binding.
///
/// # Safety contract
/// The APD records' `data_ptr` pointers must remain valid for the duration
/// of this call. If `str_len_or_ind_ptr` is non-null, it must also point to
/// valid memory for reads.
///
/// # Wire format
/// Single-row (`apd.array_size == 1`, scalar `value`):
/// ```json
/// {
///   "1": {"type": "FIXED", "value": "123"},
///   "2": {"type": "TEXT",  "value": "hello"}
/// }
/// ```
/// Multi-row (`apd.array_size > 1`, array `value`; NULL rows are JSON `null`):
/// ```json
/// {
///   "1": {"type": "FIXED", "value": ["1", "2", null]},
///   "2": {"type": "TEXT",  "value": ["a", null, "c"]}
/// }
/// ```
/// The per-parameter `type` is the Snowflake logical type derived from the
/// first non-NULL row (it does not vary across rows of an array binding);
/// if every row is NULL the type defaults to `ANY`.
pub fn odbc_bindings_to_json(
    apd: &ApdDescriptor,
    ipd: &IpdDescriptor,
    max_params: u16,
) -> Result<String, BindingError> {
    let array_size = apd.array_size.max(1);
    let bind_type = apd.bind_type;
    let bind_offset = if apd.bind_offset_ptr.is_null() {
        0
    } else {
        unsafe { *apd.bind_offset_ptr }
    };
    let mut json_bindings = Map::new();

    for param_num in 1..=max_params {
        let apd_rec = apd.records.get(&param_num).with_context(|| {
            tracing::error!(
                "odbc_bindings_to_json: APD record #{param_num} not found. \
                 Parameter bindings must be contiguous and start at 1.",
            );
            InvalidParameterIndicesSnafu
        })?;
        let ipd_rec = ipd.records.get(&param_num).with_context(|| {
            tracing::error!(
                "odbc_bindings_to_json: IPD record #{param_num} not found. \
                 Parameter bindings must be contiguous and start at 1.",
            );
            InvalidParameterIndicesSnafu
        })?;

        let mut snowflake_type = SnowflakeLogicalType::Any;
        let mut values: Vec<Value> = Vec::with_capacity(array_size);

        for row_idx in 0..array_size {
            // SQL_ATTR_PARAM_OPERATION_PTR: skip sets marked SQL_PARAM_IGNORE so
            // every parameter omits the same rows and the value arrays stay aligned.
            if param_set_ignored(apd, row_idx) {
                continue;
            }
            let binding = binding_for_row(apd_rec, ipd_rec, row_idx, bind_type, bind_offset);

            // NULL rows are emitted as JSON `null`; the per-parameter
            // `type` is taken from the first non-NULL row (Snowflake's
            // wire format expects one type per parameter, not per cell).
            // Routing every non-NULL value through `Value::String` lets
            // serde apply JSON escape rules (control characters, embedded
            // `"`, etc.) without us hand-rolling them in each converter.
            if is_null_indicator(&binding) {
                values.push(Value::Null);
                continue;
            }
            if binding.parameter_value_ptr.is_null() {
                return NullPointerSnafu.fail();
            }
            let converter = make_converter(&binding)?;
            let (sf_type, text) = converter.convert(&binding)?;
            snowflake_type = sf_type;
            values.push(Value::String(text));
        }

        let value = if array_size == 1 {
            values.into_iter().next().unwrap_or(Value::Null)
        } else {
            Value::Array(values)
        };

        let mut binding_obj = Map::new();
        binding_obj.insert(
            "type".to_string(),
            Value::String(snowflake_type.as_str().to_string()),
        );
        binding_obj.insert("value".to_string(), value);

        json_bindings.insert(param_num.to_string(), Value::Object(binding_obj));
    }

    serde_json::to_string(&Value::Object(json_bindings)).context(SerializationSnafu)
}

pub fn odbc_bindings_to_csv(
    apd: &ApdDescriptor,
    ipd: &IpdDescriptor,
    max_params: u16,
) -> Result<String, BindingError> {
    let array_size = apd.array_size;
    let bind_type = apd.bind_type;
    let bind_offset = if apd.bind_offset_ptr.is_null() {
        0
    } else {
        unsafe { *apd.bind_offset_ptr }
    };
    let mut output = String::new();

    for row_idx in 0..array_size {
        // SQL_ATTR_PARAM_OPERATION_PTR: skip parameter sets marked SQL_PARAM_IGNORE.
        if param_set_ignored(apd, row_idx) {
            continue;
        }
        for param_num in 1..=max_params {
            let apd_rec = apd.records.get(&param_num).with_context(|| {
                tracing::error!(
                    "odbc_bindings_to_csv: APD record #{param_num} not found. \
                     Parameter bindings must be contiguous and start at 1.",
                );
                InvalidParameterIndicesSnafu
            })?;
            let ipd_rec = ipd.records.get(&param_num).with_context(|| {
                tracing::error!(
                    "odbc_bindings_to_csv: IPD record #{param_num} not found. \
                     Parameter bindings must be contiguous and start at 1.",
                );
                InvalidParameterIndicesSnafu
            })?;

            if param_num > 1 {
                output.push(',');
            }

            let binding = binding_for_row(apd_rec, ipd_rec, row_idx, bind_type, bind_offset);

            if is_null_indicator(&binding) {
                continue;
            }
            if binding.parameter_value_ptr.is_null() {
                return NullPointerSnafu.fail();
            }

            let converter = make_converter(&binding)?;
            let (_, text) = converter.convert(&binding)?;
            append_escaped_csv_cell(&mut output, &text);
        }
        output.push('\n');
    }
    Ok(output)
}

/// Whether parameter set `row_idx` is marked `SQL_PARAM_IGNORE` via the APD's
/// `SQL_ATTR_PARAM_OPERATION_PTR` array. A null pointer (the common case) means
/// every set is processed, preserving behavior when the attribute is unset.
fn param_set_ignored(apd: &ApdDescriptor, row_idx: usize) -> bool {
    if apd.array_status_ptr.is_null() {
        return false;
    }
    // Safety: the application owns an array of at least `apd.array_size`
    // `SQLUSMALLINT`s when it sets SQL_ATTR_PARAM_OPERATION_PTR; `row_idx` is
    // always < array_size at every call site.
    unsafe { *apd.array_status_ptr.add(row_idx) == SQL_PARAM_IGNORE }
}

///
/// **Column-wise binding** (`bind_type == 0`, i.e. `SQL_PARAM_BIND_BY_COLUMN`):
/// * For each column the application provided a contiguous array of values.
///   Row `i`'s data is `data_ptr + bind_offset + i * buffer_length` bytes.
/// * Row `i`'s indicator is at `str_len_or_ind_ptr + bind_offset + i`.
///
/// **Row-wise binding** (`bind_type == row_size`):
/// * The application provided a single buffer where each row occupies
///   `bind_type` bytes.  The base pointers are first shifted by `bind_offset`
///   bytes (honouring `SQL_ATTR_PARAM_BIND_OFFSET_PTR`); then row strides are
///   applied on top.
///
/// `bind_offset` is the dereferenced value of `APD.SQL_DESC_BIND_OFFSET_PTR`
/// (i.e. `*SQL_ATTR_PARAM_BIND_OFFSET_PTR`), or 0 when that pointer is null.
fn binding_for_row(
    apd_rec: &crate::api::ApdRecord,
    ipd_rec: &crate::api::IpdRecord,
    row_idx: usize,
    bind_type: sql::ULen,
    bind_offset: sql::Len,
) -> ParameterBinding {
    use std::mem::size_of;

    let (data_ptr, str_len_or_ind_ptr) = if bind_type == 0 {
        // Fixed-size types are bound with BufferLength=0; stride by the C type's octet length.
        let stride = apd_rec
            .value_type
            .fixed_size()
            .unwrap_or(apd_rec.buffer_length as usize);
        let data_ptr = if apd_rec.data_ptr.is_null() {
            apd_rec.data_ptr
        } else {
            unsafe {
                (apd_rec.data_ptr as *mut u8)
                    .offset(bind_offset)
                    .add(row_idx * stride) as sql::Pointer
            }
        };
        let ind_ptr = if apd_rec.str_len_or_ind_ptr.is_null() {
            apd_rec.str_len_or_ind_ptr
        } else {
            unsafe {
                (apd_rec.str_len_or_ind_ptr as *mut u8)
                    .offset(bind_offset)
                    .add(row_idx * size_of::<sql::Len>()) as *mut sql::Len
            }
        };
        (data_ptr, ind_ptr)
    } else {
        // Row-wise: the entire row occupies `bind_type` bytes; stride == bind_type.
        let row_stride = bind_type;
        let data_ptr = if apd_rec.data_ptr.is_null() {
            apd_rec.data_ptr
        } else {
            unsafe {
                (apd_rec.data_ptr as *mut u8)
                    .offset(bind_offset)
                    .add(row_idx * row_stride) as sql::Pointer
            }
        };
        let ind_ptr = if apd_rec.str_len_or_ind_ptr.is_null() {
            apd_rec.str_len_or_ind_ptr
        } else {
            // Indicator lives inside the row struct; its byte offset from the
            // row base is fixed (same as for row 0).  We advance by full
            // `row_stride` bytes per row.
            let base_offset = unsafe {
                (apd_rec.str_len_or_ind_ptr as *mut u8).offset_from(apd_rec.data_ptr as *mut u8)
            };
            unsafe {
                (apd_rec.data_ptr as *mut u8)
                    .offset(bind_offset)
                    .add(row_idx * row_stride)
                    .offset(base_offset) as *mut sql::Len
            }
        };
        (data_ptr, ind_ptr)
    };

    let _ = size_of::<sql::Len>(); // keep import live
    ParameterBinding {
        sql_data_type: ipd_rec.sql_data_type,
        value_type: apd_rec.value_type,
        parameter_value_ptr: data_ptr,
        buffer_length: apd_rec.buffer_length,
        str_len_or_ind_ptr,
        sf_subtype: ipd_rec.sf_subtype,
    }
}

/// Append `s` to `out` using always-quoted RFC-4180 rules:
/// * if `s` is empty, write `""` (reserved for empty string; NULL is written as an absent cell).
/// * otherwise, wrap in `"..."` and double every embedded `"`.
///
/// All non-null string cells are always quoted — never written bare — because
/// the SYSTEM$BIND stage sets `escape_unenclosed_field=NONE`.  Even with that
/// set, quoting every string is the safer canonical form: `ESCAPE = NONE` is
/// already the default for enclosed fields, and enclosed fields are immune to
/// any future change of the unenclosed-field escape setting.
fn append_escaped_csv_cell(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        if ch == '"' {
            out.push_str("\"\"");
        } else {
            out.push(ch);
        }
    }
    out.push('"');
}

// =============================================================================
// Helpers — raw pointer reads
// =============================================================================

fn is_null_indicator(binding: &ParameterBinding) -> bool {
    !binding.str_len_or_ind_ptr.is_null()
        && unsafe { *binding.str_len_or_ind_ptr == sql::NULL_DATA }
}

/// Read a fixed-size value using `read_unaligned` for ODBC pointer safety.
pub(crate) fn read_unaligned<T: Copy>(binding: &ParameterBinding) -> T {
    unsafe { std::ptr::read_unaligned(binding.parameter_value_ptr as *const T) }
}

/// Read and decode an `SQL_NUMERIC_STRUCT` from the parameter buffer.
///
/// Returns `(signed_value, scale)` where `signed_value` is the integer
/// mantissa with sign applied, and `scale` is the number of decimal digits
/// after the point. The caller divides by `10^scale` to recover the true
/// numeric value.
///
/// Returns an error if the magnitude exceeds the representable `i128` range.
pub(crate) fn read_numeric_struct(binding: &ParameterBinding) -> Result<(i128, i8), BindingError> {
    let ns = read_unaligned::<sql::Numeric>(binding);
    let magnitude = u128::from_le_bytes(ns.val);
    let negative_min_magnitude = (i128::MAX as u128) + 1;
    let signed = if ns.sign == 0 {
        if magnitude == negative_min_magnitude {
            i128::MIN
        } else if magnitude <= i128::MAX as u128 {
            -(magnitude as i128)
        } else {
            return NumericMagnitudeOverflowSnafu {
                reason: format!(
                    "SQL_NUMERIC_STRUCT magnitude {magnitude} exceeds i128 negative range"
                ),
            }
            .fail();
        }
    } else if ns.sign == 1 {
        if magnitude <= i128::MAX as u128 {
            magnitude as i128
        } else {
            return NumericMagnitudeOverflowSnafu {
                reason: format!(
                    "SQL_NUMERIC_STRUCT magnitude {magnitude} exceeds i128 positive range"
                ),
            }
            .fail();
        }
    } else {
        return NumericMagnitudeOverflowSnafu {
            reason: format!(
                "SQL_NUMERIC_STRUCT sign {} is invalid; expected 0 or 1",
                ns.sign
            ),
        }
        .fail();
    };
    Ok((signed, ns.scale))
}

/// Format a scaled integer value into its decimal string representation.
/// For example, `(12345, 2)` becomes `"123.45"`.
///
/// Uses string manipulation rather than arithmetic scaling to avoid
/// overflow when `value` is large or `scale` is very negative.
pub(crate) fn format_numeric_value(value: i128, scale: i8) -> String {
    if scale == 0 {
        return value.to_string();
    }

    let is_negative = value < 0;
    let abs = value.unsigned_abs();
    let mut s = abs.to_string();

    if scale < 0 {
        let trailing_zeros = if scale == i8::MIN {
            (i8::MAX as usize) + 1
        } else {
            (-scale) as usize
        };
        s.extend(std::iter::repeat_n('0', trailing_zeros));
        if is_negative {
            s.insert(0, '-');
        }
        return s;
    }

    let scale = scale as usize;
    while s.len() <= scale {
        s.insert(0, '0');
    }
    let decimal_pos = s.len() - scale;
    s.insert(decimal_pos, '.');
    if is_negative {
        s.insert(0, '-');
    }
    s
}

/// Determine the actual byte length of buffer data, using the length/indicator
/// pointer if available, falling back to `buffer_length`.
///
/// Negative `buffer_length` values (e.g. `SQL_NTS`) are treated as zero.
/// Indicated length is clamped to `buffer_length` to prevent over-reads.
pub(crate) fn buffer_data_len(binding: &ParameterBinding) -> usize {
    let max_len = if binding.buffer_length < 0 {
        0
    } else {
        binding.buffer_length as usize
    };

    if !binding.str_len_or_ind_ptr.is_null() {
        let indicated_len = unsafe { *binding.str_len_or_ind_ptr };
        if indicated_len >= 0 {
            let indicated = indicated_len as usize;
            return if max_len > 0 {
                indicated.min(max_len)
            } else {
                indicated
            };
        }
    }

    max_len
}

/// Read a fixed-size POD struct `T` from an `SQL_C_BINARY` parameter buffer,
/// rejecting buffers whose length does not exactly match `size_of::<T>()`.
///
/// `struct_name` is used only to produce a descriptive error message
/// (e.g. `"SQL_DATE_STRUCT"`) when the length check fails.
pub(crate) fn read_binary_struct<T: Copy>(
    binding: &ParameterBinding,
    struct_name: &str,
) -> Result<T, BindingError> {
    let len = buffer_data_len(binding);
    let expected = std::mem::size_of::<T>();
    if len != expected {
        return BindingNumericOutOfRangeSnafu {
            reason: format!(
                "SQL_C_BINARY buffer length {len} does not match {struct_name} size ({expected})"
            ),
        }
        .fail();
    }
    Ok(read_unaligned::<T>(binding))
}

/// Convert bytes from the system's ANSI code page to a Rust UTF-8 `String`.
///
/// On Windows, SQL_C_CHAR data uses the active ANSI code page (ACP), which may
/// not be UTF-8. We call `MultiByteToWideChar(CP_ACP, …)` to widen to UTF-16,
/// then convert the UTF-16 to a Rust `String`.
#[cfg(windows)]
fn acp_bytes_to_string(bytes: &[u8]) -> Result<String, BindingError> {
    if bytes.is_empty() {
        return Ok(String::new());
    }

    use std::ptr;

    unsafe extern "system" {
        fn MultiByteToWideChar(
            code_page: u32,
            dw_flags: u32,
            lp_multi_byte_str: *const u8,
            cb_multi_byte: i32,
            lp_wide_char_str: *mut u16,
            cch_wide_char: i32,
        ) -> i32;
    }

    const CP_ACP: u32 = 0;

    let result = unsafe {
        let wide_len = MultiByteToWideChar(
            CP_ACP,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            ptr::null_mut(),
            0,
        );
        if wide_len <= 0 {
            return AcpConversionSnafu.fail();
        }

        let mut wide_buf = vec![0u16; wide_len as usize];
        let written = MultiByteToWideChar(
            CP_ACP,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            wide_buf.as_mut_ptr(),
            wide_len,
        );
        if written <= 0 {
            return AcpConversionSnafu.fail();
        }

        String::from_utf16(&wide_buf[..written as usize]).map_err(|_| AcpConversionSnafu.build())
    };
    result
}

#[cfg(not(windows))]
fn acp_bytes_to_string(bytes: &[u8]) -> Result<String, BindingError> {
    str::from_utf8(bytes)
        .context(InvalidUtf8Snafu)
        .map(|s| s.to_string())
}

#[cfg(windows)]
use super::error::AcpConversionSnafu;

/// Read a SQL_C_CHAR value, converting from the system ANSI code page to UTF-8.
///
/// Per ODBC spec: when the indicator is SQL_NTS or the indicator pointer is
/// NULL, character data is null-terminated. Otherwise we use the indicated
/// length (clamped to buffer_length).
pub(crate) fn read_char_str(binding: &ParameterBinding) -> Result<String, BindingError> {
    let null_terminated =
        binding.str_len_or_ind_ptr.is_null() || unsafe { *binding.str_len_or_ind_ptr } == sql::NTS;

    let bytes = if null_terminated {
        unsafe { CStr::from_ptr(binding.parameter_value_ptr as *const c_char).to_bytes() }
    } else {
        let len = buffer_data_len(binding);
        unsafe { slice::from_raw_parts(binding.parameter_value_ptr as *const u8, len) }
    };

    acp_bytes_to_string(bytes)
}

/// Read a `SQL_C_WCHAR` value and convert to a UTF-8 string. The DM-side
/// wide-character encoding (UTF-16 or UTF-32) is the one negotiated at
/// driver startup; see [`WideChar`] and the `encoding` module for details.
///
/// When `StrLen_or_IndPtr` is NULL or points to `SQL_NTS`, the buffer is
/// treated as null-terminated (scans for the first null DM-side unit,
/// bounded by `buffer_length`). Otherwise the indicated byte length is used.
pub(crate) fn read_wchar_str(binding: &ParameterBinding) -> Result<String, BindingError> {
    let null_terminated =
        binding.str_len_or_ind_ptr.is_null() || unsafe { *binding.str_len_or_ind_ptr } == sql::NTS;
    let unit_size = wchar_byte_size();
    let ptr = binding.parameter_value_ptr as *const WideChar;

    let unit_len = if null_terminated {
        let max_units = if binding.buffer_length > 0 {
            binding.buffer_length as usize / unit_size
        } else {
            // ODBC spec: when the indicator is SQL_NTS, the application
            // is required to NUL-terminate the buffer. A buggy or
            // malicious binding with no terminator turns the scan into
            // an unbounded read; the warn below makes that misuse
            // visible without changing behaviour for spec-conformant
            // callers (who may legitimately bind huge SQL_NTS strings
            // and would be hurt by a hard cap).
            tracing::warn!(
                buffer_length = binding.buffer_length,
                "SQL_NTS wide parameter bound with non-positive buffer_length; \
                 falling back to an unbounded scan that relies on the \
                 application-supplied NUL terminator"
            );
            usize::MAX
        };
        // Safety: ODBC requires the application to NUL-terminate
        // `SQL_C_WCHAR` values bound with `SQL_NTS`; `wide_strlen_bounded`
        // stops at the first zero DM-side unit. The `tracing::warn!`
        // above surfaces the misuse case where that contract is broken.
        unsafe { wide_strlen_bounded(ptr, max_units) }
    } else {
        buffer_data_len(binding) / unit_size
    };
    Wide::read_string(ptr, unit_len as i32).map_err(|_| WCharConversionSnafu.build())
}

/// Upper bound on the number of input characters copied into a 22018
/// (`InvalidCharacterValueForCast`) diagnostic record for a CHAR/WCHAR temporal
/// bind. ODBC diagnostic-record buffers are bounded, so an adversarial caller
/// must not be able to blow them up by binding a megabyte literal.
pub(crate) const TEMPORAL_CHAR_DIAG_MAX_CHARS: usize = 64;

/// Shared CHAR/WCHAR dispatch for temporal parameter binds (DATE / TIME /
/// TIMESTAMP). Reads the bound character payload (ANSI or wide), then parses
/// the trimmed text with `try_parse`. On parse failure it surfaces SQLSTATE
/// 22018 (`InvalidCharacterValueForCast`) carrying a length-capped copy of the
/// input plus the `expected_format` template, so the conversions in `date.rs`,
/// `time.rs` and `timestamp.rs` don't each repeat the read/trim/diagnostic
/// boilerplate. Callers must only dispatch `CDataType::Char` / `CDataType::WChar`
/// here; any other value type yields the 07006 `UnsupportedCDataType` error.
pub(crate) fn parse_temporal_char_input<T>(
    binding: &ParameterBinding,
    expected_format: &'static str,
    try_parse: impl Fn(&str) -> Result<T, ()>,
) -> Result<T, BindingError> {
    let s = match binding.value_type {
        CDataType::Char => read_char_str(binding)?,
        CDataType::WChar => read_wchar_str(binding)?,
        other => return UnsupportedCDataTypeSnafu { c_type: other }.fail(),
    };
    try_parse(s.trim()).map_err(|()| {
        InvalidCharacterValueForCastSnafu {
            c_type: binding.value_type,
            value: s
                .chars()
                .take(TEMPORAL_CHAR_DIAG_MAX_CHARS)
                .collect::<String>(),
            expected_format,
        }
        .build()
    })
}

/// Test-only entry point that mirrors what `odbc_bindings_to_json` does
/// for a single `ParameterBinding`: pick the right converter via
/// `make_converter` and run `.convert(...)`. Exposed to sibling unit
/// test modules (e.g. `interval_tests`) so they don't have to reach into
/// the private factory.
#[cfg(test)]
pub(crate) fn convert_for_test(
    binding: &ParameterBinding,
) -> Result<(SnowflakeLogicalType, String), BindingError> {
    let converter = make_converter(binding)?;
    converter.convert(binding)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::CDataType;
    use crate::api::types::{SQL_SF_TIMESTAMP_LTZ, SQL_SF_TIMESTAMP_NTZ, SQL_SF_TIMESTAMP_TZ};
    use crate::api::{ApdRecord, IpdRecord};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn make_binding(
        value_type: CDataType,
        parameter_type: sql::SqlDataType,
        ptr: sql::Pointer,
        buffer_length: sql::Len,
        ind_ptr: *mut sql::Len,
    ) -> ParameterBinding {
        // Mirror what `bind_parameter` does: normalise vendor codes to the
        // standard SQL_TYPE_TIMESTAMP and stash the subtype on `sf_subtype`
        // so converter dispatch sees the same shape it would in production.
        let sf_subtype = TimestampSubtype::from_parameter_type(parameter_type);
        let sql_data_type = if sf_subtype.is_some() {
            sql::SqlDataType::TIMESTAMP
        } else {
            parameter_type
        };
        ParameterBinding {
            sql_data_type,
            value_type,
            parameter_value_ptr: ptr,
            buffer_length,
            str_len_or_ind_ptr: ind_ptr,
            sf_subtype,
        }
    }

    fn make_descriptors(
        params: Vec<(
            u16,
            CDataType,
            sql::SqlDataType,
            sql::Pointer,
            sql::Len,
            *mut sql::Len,
        )>,
    ) -> (ApdDescriptor, IpdDescriptor) {
        let mut apd = ApdDescriptor::new();
        let mut ipd = IpdDescriptor::new();
        for (num, value_type, parameter_type, ptr, buf_len, ind_ptr) in params {
            apd.records.insert(
                num,
                ApdRecord {
                    value_type,
                    data_ptr: ptr,
                    buffer_length: buf_len,
                    str_len_or_ind_ptr: ind_ptr,
                },
            );
            ipd.records.insert(
                num,
                IpdRecord {
                    sql_data_type: parameter_type,
                    ..IpdRecord::default()
                },
            );
        }
        (apd, ipd)
    }

    fn convert_binding(
        binding: &ParameterBinding,
    ) -> Result<(SnowflakeLogicalType, String), BindingError> {
        let converter = make_converter(binding)?;
        converter.convert(binding)
    }

    // -- read_wchar_str tests -------------------------------------------------

    #[test]
    fn read_wchar_str_with_explicit_length() -> TestResult {
        let data: [u16; 4] = ['h' as u16, 'i' as u16, '!' as u16, 0];
        let mut ind: sql::Len = 3 * mem::size_of::<u16>() as sql::Len;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::VARCHAR,
            data.as_ptr() as sql::Pointer,
            (4 * mem::size_of::<u16>()) as sql::Len,
            &mut ind,
        );
        assert_eq!(read_wchar_str(&binding)?, "hi!");
        Ok(())
    }

    #[test]
    fn read_wchar_str_with_sql_nts() -> TestResult {
        let data: [u16; 4] = ['h' as u16, 'i' as u16, '!' as u16, 0];
        let mut ind: sql::Len = sql::NTS;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::VARCHAR,
            data.as_ptr() as sql::Pointer,
            (4 * mem::size_of::<u16>()) as sql::Len,
            &mut ind,
        );
        assert_eq!(read_wchar_str(&binding)?, "hi!");
        Ok(())
    }

    #[test]
    fn read_wchar_str_with_null_indicator() -> TestResult {
        let data: [u16; 4] = ['h' as u16, 'i' as u16, '!' as u16, 0];
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::VARCHAR,
            data.as_ptr() as sql::Pointer,
            (4 * mem::size_of::<u16>()) as sql::Len,
            std::ptr::null_mut(),
        );
        assert_eq!(read_wchar_str(&binding)?, "hi!");
        Ok(())
    }

    #[test]
    fn read_wchar_str_sql_nts_zero_buffer_length() -> TestResult {
        let data: [u16; 4] = ['h' as u16, 'i' as u16, '!' as u16, 0];
        let mut ind: sql::Len = sql::NTS;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::VARCHAR,
            data.as_ptr() as sql::Pointer,
            0,
            &mut ind,
        );
        assert_eq!(read_wchar_str(&binding)?, "hi!");
        Ok(())
    }

    // -- ParamConverter tests per type ----------------------------------------

    #[test]
    fn convert_integer_i32() -> TestResult {
        let val: i32 = 42;
        let binding = make_binding(
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "42".to_string());
        Ok(())
    }

    #[test]
    fn convert_integer_i16() -> TestResult {
        let val: i16 = -7;
        let binding = make_binding(
            CDataType::Short,
            sql::SqlDataType::SMALLINT,
            &val as *const i16 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "-7".to_string());
        Ok(())
    }

    #[test]
    fn convert_integer_i64() -> TestResult {
        let val: i64 = 9_999_999_999;
        let binding = make_binding(
            CDataType::SBigInt,
            sql::SqlDataType::EXT_BIG_INT,
            &val as *const i64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "9999999999".to_string());
        Ok(())
    }

    #[test]
    fn convert_unsigned_u32() -> TestResult {
        let val: u32 = 4_000_000_000;
        let binding = make_binding(
            CDataType::ULong,
            sql::SqlDataType::INTEGER,
            &val as *const u32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "4000000000".to_string());
        Ok(())
    }

    #[test]
    fn convert_unsigned_u16() -> TestResult {
        let val: u16 = 65535;
        let binding = make_binding(
            CDataType::UShort,
            sql::SqlDataType::SMALLINT,
            &val as *const u16 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "65535".to_string());
        Ok(())
    }

    #[test]
    fn convert_unsigned_u64() -> TestResult {
        let val: u64 = 1_000_000_000_000;
        let binding = make_binding(
            CDataType::UBigInt,
            sql::SqlDataType::EXT_BIG_INT,
            &val as *const u64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "1000000000000".to_string());
        Ok(())
    }

    #[test]
    fn convert_unsigned_u8() -> TestResult {
        let val: u8 = 255;
        let binding = make_binding(
            CDataType::UTinyInt,
            sql::SqlDataType::EXT_TINY_INT,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "255".to_string());
        Ok(())
    }

    #[test]
    fn convert_signed_i8() -> TestResult {
        let val: i8 = -128;
        let binding = make_binding(
            CDataType::STinyInt,
            sql::SqlDataType::EXT_TINY_INT,
            &val as *const i8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "-128".to_string());
        Ok(())
    }

    #[test]
    fn convert_float_f64() -> TestResult {
        let val: f64 = 1.234;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::DOUBLE,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, "1.234".to_string());
        Ok(())
    }

    #[test]
    fn convert_float_f32() -> TestResult {
        let val: f32 = 1.5;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::REAL,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert!(v.starts_with("1.5"));
        Ok(())
    }

    #[test]
    fn convert_char_nts() -> TestResult {
        let val = b"hello\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::VARCHAR,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, "hello".to_string());
        Ok(())
    }

    #[test]
    fn convert_char_with_length() -> TestResult {
        let val = b"hello world";
        let mut ind: sql::Len = 5;
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::VARCHAR,
            val.as_ptr() as sql::Pointer,
            11,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, "hello".to_string());
        Ok(())
    }

    #[test]
    fn convert_bit_true() -> TestResult {
        let val: u8 = 1;
        let binding = make_binding(
            CDataType::Bit,
            sql::SqlDataType::EXT_BIT,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_bit_false() -> TestResult {
        let val: u8 = 0;
        let binding = make_binding(
            CDataType::Bit,
            sql::SqlDataType::EXT_BIT,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    // -- C types → BOOLEAN (SQL_BIT) ------------------------------------------

    #[test]
    fn convert_char_to_boolean_true() -> TestResult {
        let val = b"1\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_char_to_boolean_false() -> TestResult {
        let val = b"0\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_char_true_string_to_boolean() -> TestResult {
        let val = b"true\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_char_false_string_to_boolean() -> TestResult {
        let val = b"false\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_wchar_to_boolean_true() -> TestResult {
        let val: [u16; 1] = [b'1' as u16];
        let mut ind: sql::Len = 2;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            2,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_wchar_to_boolean_false() -> TestResult {
        let val: [u16; 1] = [b'0' as u16];
        let mut ind: sql::Len = 2;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            2,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_slong_to_boolean_true() -> TestResult {
        let val: i32 = 42;
        let binding = make_binding(
            CDataType::SLong,
            sql::SqlDataType::EXT_BIT,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_slong_to_boolean_false() -> TestResult {
        let val: i32 = 0;
        let binding = make_binding(
            CDataType::SLong,
            sql::SqlDataType::EXT_BIT,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_sbigint_to_boolean_true() -> TestResult {
        let val: i64 = -1;
        let binding = make_binding(
            CDataType::SBigInt,
            sql::SqlDataType::EXT_BIT,
            &val as *const i64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_double_to_boolean_true() -> TestResult {
        let val: f64 = 1.5;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::EXT_BIT,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_double_to_boolean_false() -> TestResult {
        let val: f64 = 0.0;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::EXT_BIT,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_float_to_boolean_true() -> TestResult {
        let val: f32 = 0.5;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::EXT_BIT,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_numeric_to_boolean_true() -> TestResult {
        let n = sql::Numeric {
            precision: 10,
            scale: 0,
            sign: 1,
            val: 1u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::EXT_BIT,
            &n as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_numeric_to_boolean_false() -> TestResult {
        let n = sql::Numeric {
            precision: 10,
            scale: 0,
            sign: 1,
            val: 0u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::EXT_BIT,
            &n as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_binary_to_boolean_true() -> TestResult {
        let val: [u8; 1] = [0x01];
        let mut ind: sql::Len = 1;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            1,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_binary_to_boolean_false() -> TestResult {
        let val: [u8; 1] = [0x00];
        let mut ind: sql::Len = 1;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            1,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_stinyint_to_boolean_true() -> TestResult {
        let val: i8 = -1;
        let binding = make_binding(
            CDataType::STinyInt,
            sql::SqlDataType::EXT_BIT,
            &val as *const i8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_stinyint_to_boolean_false() -> TestResult {
        let val: i8 = 0;
        let binding = make_binding(
            CDataType::STinyInt,
            sql::SqlDataType::EXT_BIT,
            &val as *const i8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_utinyint_to_boolean_true() -> TestResult {
        let val: u8 = 255;
        let binding = make_binding(
            CDataType::UTinyInt,
            sql::SqlDataType::EXT_BIT,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_ulong_to_boolean_true() -> TestResult {
        let val: u32 = 1;
        let binding = make_binding(
            CDataType::ULong,
            sql::SqlDataType::EXT_BIT,
            &val as *const u32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_ulong_to_boolean_false() -> TestResult {
        let val: u32 = 0;
        let binding = make_binding(
            CDataType::ULong,
            sql::SqlDataType::EXT_BIT,
            &val as *const u32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_ushort_to_boolean_true() -> TestResult {
        let val: u16 = 1;
        let binding = make_binding(
            CDataType::UShort,
            sql::SqlDataType::EXT_BIT,
            &val as *const u16 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_ushort_to_boolean_false() -> TestResult {
        let val: u16 = 0;
        let binding = make_binding(
            CDataType::UShort,
            sql::SqlDataType::EXT_BIT,
            &val as *const u16 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_ubigint_to_boolean_true() -> TestResult {
        let val: u64 = 1;
        let binding = make_binding(
            CDataType::UBigInt,
            sql::SqlDataType::EXT_BIT,
            &val as *const u64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_ubigint_to_boolean_false() -> TestResult {
        let val: u64 = 0;
        let binding = make_binding(
            CDataType::UBigInt,
            sql::SqlDataType::EXT_BIT,
            &val as *const u64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_float_to_boolean_false() -> TestResult {
        let val: f32 = 0.0;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::EXT_BIT,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_float_nan_to_boolean_fails() {
        let val: f32 = f32::NAN;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::EXT_BIT,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Float NaN should not convert to boolean"
        );
    }

    #[test]
    fn convert_float_inf_to_boolean_fails() {
        let val: f32 = f32::INFINITY;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::EXT_BIT,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Float infinity should not convert to boolean"
        );
    }

    #[test]
    fn convert_double_nan_to_boolean_fails() {
        let val: f64 = f64::NAN;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::EXT_BIT,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Double NaN should not convert to boolean"
        );
    }

    #[test]
    fn convert_double_inf_to_boolean_fails() {
        let val: f64 = f64::INFINITY;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::EXT_BIT,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Double infinity should not convert to boolean"
        );
    }

    #[test]
    fn convert_double_neg_inf_to_boolean_fails() {
        let val: f64 = f64::NEG_INFINITY;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::EXT_BIT,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Double -infinity should not convert to boolean"
        );
    }

    #[test]
    fn convert_slong_negative_to_boolean_true() -> TestResult {
        let val: i32 = -99;
        let binding = make_binding(
            CDataType::SLong,
            sql::SqlDataType::EXT_BIT,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_char_nan_to_boolean_fails() {
        let val = b"NaN\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "NaN should not be accepted as a boolean value"
        );
    }

    #[test]
    fn convert_char_inf_to_boolean_fails() {
        let val = b"inf\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "inf should not be accepted as a boolean value"
        );
    }

    #[test]
    fn convert_char_garbage_to_boolean_fails() {
        let val = b"hello\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Non-numeric non-boolean string should fail"
        );
    }

    // ---------------------------------------------------------------------
    // Invalid CHAR / WCHAR -> DATE / TIME / TIMESTAMP must surface as
    // SQLSTATE 22018 (`InvalidCharacterValueForCast`), NOT 07006
    // (`UnsupportedCDataType`). Per ODBC Appendix D ("Converting Data from
    // C to SQL Data Types") a SQL_C_CHAR / SQL_C_WCHAR source IS a supported
    // binding for temporal targets; a value that doesn't match the accepted
    // grammar is a *data* error (22018), not an *unsupported-conversion*
    // error (07006). Returning 07006 wrongly tells the app the conversion
    // itself is unavailable.

    fn assert_invalid_char_value_for_cast(
        err: BindingError,
        want_c_type: CDataType,
        want_value: &str,
        want_format: &str,
    ) {
        match err {
            BindingError::InvalidCharacterValueForCast {
                c_type,
                value,
                expected_format,
                ..
            } => {
                assert_eq!(c_type, want_c_type, "c_type mismatch");
                assert_eq!(value, want_value, "rejected value mismatch");
                assert_eq!(expected_format, want_format, "expected_format mismatch");
            }
            other => panic!("expected InvalidCharacterValueForCast (22018), got {other:?}"),
        }
    }

    #[test]
    fn convert_char_garbage_to_date_returns_22018() {
        let val = b"not-a-date\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DATE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let err = convert_binding(&binding).expect_err("garbage must not parse as DATE");
        assert_invalid_char_value_for_cast(err, CDataType::Char, "not-a-date", "YYYY-MM-DD");
    }

    #[test]
    fn convert_wchar_garbage_to_date_returns_22018() {
        let val: [u16; 11] = [
            b'n' as u16,
            b'o' as u16,
            b't' as u16,
            b'-' as u16,
            b'a' as u16,
            b'-' as u16,
            b'd' as u16,
            b'a' as u16,
            b't' as u16,
            b'e' as u16,
            0,
        ];
        let mut ind: sql::Len = sql::NTS;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::DATE,
            val.as_ptr() as sql::Pointer,
            (val.len() * mem::size_of::<u16>()) as sql::Len,
            &mut ind,
        );
        let err = convert_binding(&binding).expect_err("garbage must not parse as DATE");
        assert_invalid_char_value_for_cast(err, CDataType::WChar, "not-a-date", "YYYY-MM-DD");
    }

    #[test]
    fn convert_char_garbage_to_time_returns_22018() {
        let val = b"not-a-time\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::TIME,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let err = convert_binding(&binding).expect_err("garbage must not parse as TIME");
        assert_invalid_char_value_for_cast(
            err,
            CDataType::Char,
            "not-a-time",
            "HH:MM:SS[.fffffffff]",
        );
    }

    #[test]
    fn convert_char_garbage_to_timestamp_returns_22018() {
        let val = b"not-a-timestamp\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::TIMESTAMP,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let err = convert_binding(&binding).expect_err("garbage must not parse as TIMESTAMP");
        assert_invalid_char_value_for_cast(
            err,
            CDataType::Char,
            "not-a-timestamp",
            "YYYY-MM-DD HH:MM:SS[.fffffffff]",
        );
    }

    #[test]
    fn convert_char_valid_date_still_succeeds() {
        // Guard the happy path: the new error mapping must not regress
        // acceptance of a well-formed literal.
        let val = b"2024-01-15\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DATE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, _v) = convert_binding(&binding).expect("valid date literal must convert");
        assert_eq!(ty, SnowflakeLogicalType::Date);
    }

    #[test]
    fn convert_sshort_to_boolean_false() -> TestResult {
        let val: i16 = 0;
        let binding = make_binding(
            CDataType::SShort,
            sql::SqlDataType::EXT_BIT,
            &val as *const i16 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_sbigint_to_boolean_false() -> TestResult {
        let val: i64 = 0;
        let binding = make_binding(
            CDataType::SBigInt,
            sql::SqlDataType::EXT_BIT,
            &val as *const i64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_char_numeric_nonzero_to_boolean() -> TestResult {
        let val = b"42\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_char_negative_to_boolean() -> TestResult {
        let val = b"-1\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_char_float_string_to_boolean() -> TestResult {
        let val = b"0.5\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_char_float_zero_string_to_boolean() -> TestResult {
        let val = b"0.0\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_char_neg_zero_string_to_boolean() -> TestResult {
        let val = b"-0.0\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_float_neg_zero_to_boolean_false() -> TestResult {
        let val: f32 = -0.0;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::EXT_BIT,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_double_neg_zero_to_boolean_false() -> TestResult {
        let val: f64 = -0.0;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::EXT_BIT,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_float_neg_inf_to_boolean_fails() {
        let val: f32 = f32::NEG_INFINITY;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::EXT_BIT,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Float -infinity should not convert to boolean"
        );
    }

    #[test]
    fn convert_binary_multibyte_to_boolean_fails() {
        let val: [u8; 3] = [0x00, 0x01, 0x00];
        let mut ind: sql::Len = 3;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            3,
            &mut ind,
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Multi-byte binary should be rejected for SQL_BIT (ODBC spec: len must equal 1)"
        );
    }

    #[test]
    fn convert_binary_empty_to_boolean_fails() {
        let val: [u8; 0] = [];
        let mut ind: sql::Len = 0;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            0,
            &mut ind,
        );
        assert!(
            convert_binding(&binding).is_err(),
            "Empty binary should be rejected for SQL_BIT (ODBC spec: len must equal 1)"
        );
    }

    #[test]
    fn convert_binary() -> TestResult {
        let val: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
        let mut ind: sql::Len = 4;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::EXT_BINARY,
            val.as_ptr() as sql::Pointer,
            4,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Binary);
        assert_eq!(v, "deadbeef".to_string());
        Ok(())
    }

    #[test]
    fn convert_null_data() -> TestResult {
        let mut ind: sql::Len = sql::NULL_DATA;
        let (apd, ipd) = make_descriptors(vec![(
            1,
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            std::ptr::null_mut(),
            0,
            &mut ind,
        )]);
        let json = odbc_bindings_to_json(&apd, &ipd, apd.desc_count().max(ipd.desc_count()))?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed["1"]["type"], "ANY");
        assert!(parsed["1"]["value"].is_null());
        Ok(())
    }

    #[test]
    fn convert_null_pointer_without_indicator_fails() {
        let (apd, ipd) = make_descriptors(vec![(
            1,
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
        )]);
        assert!(odbc_bindings_to_json(&apd, &ipd, apd.desc_count().max(ipd.desc_count())).is_err());
    }

    #[test]
    fn convert_unsupported_sql_type() {
        let val: i32 = 1;
        let binding = make_binding(
            CDataType::Long,
            sql::SqlDataType(9999),
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(make_converter(&binding).is_err());
    }

    // -- vendor TIMESTAMP code routing ---------------------------------------
    //
    // The Snowflake-specific vendor codes `SQL_SF_TIMESTAMP_LTZ` (2000),
    // `SQL_SF_TIMESTAMP_TZ` (2001), and `SQL_SF_TIMESTAMP_NTZ` (2002) -- mirror
    // the legacy 3.16.0 driver -- are routed through `make_converter` so
    // applications can opt into the matching wire `SnowflakeLogicalType`
    // rather than always landing on NTZ via the standard `SQL_TYPE_TIMESTAMP`.

    #[test]
    fn ntz_vendor_code_routes_to_timestamp_ntz_logical_type() -> TestResult {
        let ts = sql::Timestamp {
            year: 2024,
            month: 6,
            day: 1,
            hour: 12,
            minute: 0,
            second: 0,
            fraction: 0,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            SQL_SF_TIMESTAMP_NTZ,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, _) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::TimestampNtz);
        Ok(())
    }

    #[test]
    fn ltz_vendor_code_routes_to_text_logical_type() -> TestResult {
        // The legacy 3.16.0 driver
        // (`Snowflake-odbc/Source/DataEngine/SFQueryExecutor.cpp:613-618`) tags
        // every `SQL_SF_TIMESTAMP_{NTZ,LTZ,TZ}` bind as `TEXT` and lets the
        // server's column-type coercion parse the wall-clock string into the
        // destination logical type. Sending `type=TIMESTAMP_LTZ` with a string
        // value is rejected by the server with SQLSTATE 22000 ("Invalid bind
        // value (...) for type (TIMESTAMP_LTZ)"). This test pins the wire
        // contract.
        let ts = sql::Timestamp {
            year: 2024,
            month: 6,
            day: 1,
            hour: 12,
            minute: 0,
            second: 0,
            fraction: 0,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            SQL_SF_TIMESTAMP_LTZ,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, _) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        Ok(())
    }

    #[test]
    fn standard_sql_timestamp_still_routes_to_ntz_for_backward_compat() -> TestResult {
        // Tableau / Excel / Power BI bind via the standard `SQL_TYPE_TIMESTAMP`
        // (93) today and expect an NTZ logical type. Adding the vendor codes
        // must not change that legacy route.
        let ts = sql::Timestamp {
            year: 2024,
            month: 6,
            day: 1,
            hour: 12,
            minute: 0,
            second: 0,
            fraction: 0,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            sql::SqlDataType::TIMESTAMP,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, _) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::TimestampNtz);
        Ok(())
    }

    #[test]
    fn tz_vendor_code_routes_to_timestamp_tz_logical_type() -> TestResult {
        // SQL_C_TYPE_TIMESTAMP has no offset field; the converter treats the
        // wall-clock as UTC (offset = 0) and emits the legacy two-token wire
        // format. The logical type must be TimestampTz so the server stores
        // the value with the offset side-channel rather than as plain NTZ.
        let ts = sql::Timestamp {
            year: 2024,
            month: 6,
            day: 1,
            hour: 12,
            minute: 0,
            second: 0,
            fraction: 0,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            SQL_SF_TIMESTAMP_TZ,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        // Restack note: HEAD originally asserted that TZ binding errored
        // out (because pre-#1005 the driver explicitly rejected the
        // subtype). Now that #1005 wires `Some(TimestampSubtype::Tz)`
        // to a real `SnowflakeTimestampTz` converter, the test verifies
        // the wire-format the converter actually emits.
        let (ty, value) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::TimestampTz);
        // Wire token shape: "<epoch_nanos> <offset_minutes_plus_1440>".
        let parts: Vec<&str> = value.split(' ').collect();
        assert_eq!(
            parts.len(),
            2,
            "expected `<epoch_ns> <offset>`, got {value}"
        );
        // Naive struct -> offset 0 -> wire token = bias = 1440.
        assert_eq!(parts[1], "1440");
        Ok(())
    }

    #[test]
    fn tz_vendor_code_with_char_input_parses_offset_suffix() -> TestResult {
        // SQL_C_CHAR with a `+/-HH:MM` suffix: the offset must round-trip into
        // the second wire token (signed offset + 1440 bias).
        let s = b"2024-01-15 14:30:45 +05:30";
        let mut ind: sql::Len = s.len() as sql::Len;
        let binding = make_binding(
            CDataType::Char,
            SQL_SF_TIMESTAMP_TZ,
            s.as_ptr() as sql::Pointer,
            s.len() as sql::Len,
            &mut ind as *mut sql::Len,
        );
        let (ty, value) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::TimestampTz);
        let parts: Vec<&str> = value.split(' ').collect();
        // 2024-01-15 14:30:45 +05:30 -> 09:00:45 UTC -> 1705309245000000000 ns
        // offset 330 + 1440 = 1770
        assert_eq!(parts[0], "1705309245000000000");
        assert_eq!(parts[1], "1770");
        Ok(())
    }

    // -- end-to-end pipeline tests -------------------------------------------

    #[test]
    fn pipeline_integer_binding() -> TestResult {
        let val: i32 = 99;
        let binding = make_binding(
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, json_val) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(json_val, "99".to_string());
        Ok(())
    }

    #[test]
    fn pipeline_full_json_output() -> TestResult {
        let val: i32 = 7;
        let (apd, ipd) = make_descriptors(vec![(
            1,
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        )]);
        let json = odbc_bindings_to_json(&apd, &ipd, apd.desc_count().max(ipd.desc_count()))?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed["1"]["type"], "FIXED");
        assert_eq!(parsed["1"]["value"], "7");
        Ok(())
    }

    #[test]
    fn pipeline_null_json_output() -> TestResult {
        let mut ind: sql::Len = sql::NULL_DATA;
        let (apd, ipd) = make_descriptors(vec![(
            1,
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            std::ptr::null_mut(),
            0,
            &mut ind,
        )]);
        let json = odbc_bindings_to_json(&apd, &ipd, apd.desc_count().max(ipd.desc_count()))?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed["1"]["type"], "ANY");
        assert!(parsed["1"]["value"].is_null());
        Ok(())
    }

    #[test]
    fn pipeline_non_contiguous_params_error() {
        let val: i32 = 1;
        let (mut apd, mut ipd) = make_descriptors(vec![(
            1,
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        )]);
        apd.records.insert(
            3,
            ApdRecord {
                value_type: CDataType::Long,
                data_ptr: &val as *const i32 as sql::Pointer,
                buffer_length: 0,
                str_len_or_ind_ptr: std::ptr::null_mut(),
            },
        );
        ipd.records.insert(
            3,
            IpdRecord {
                sql_data_type: sql::SqlDataType::INTEGER,
                ..IpdRecord::default()
            },
        );
        assert!(odbc_bindings_to_json(&apd, &ipd, apd.desc_count().max(ipd.desc_count())).is_err());
    }

    #[test]
    fn max_params_zero_skips_phantom_dae_binding() -> TestResult {
        // Simulates: SQLPrepare("SELECT 1") → 0 markers, then
        // SQLBindParameter(1, ..., (SQLPOINTER)1, ..., SQL_DATA_AT_EXEC).
        // The DM may or may not strip phantom bindings, so we test the
        // serializer directly. With max_params=0 the dummy pointer at
        // address 0x1 must never be dereferenced.
        let mut dae_ind: sql::Len = sql::DATA_AT_EXEC;
        let (apd, ipd) = make_descriptors(vec![(
            1,
            CDataType::Char,
            sql::SqlDataType::VARCHAR,
            1usize as sql::Pointer, // dummy DAE token, not a real address
            0,
            &mut dae_ind,
        )]);
        let json = odbc_bindings_to_json(&apd, &ipd, 0)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed, serde_json::json!({}));
        Ok(())
    }

    #[test]
    fn max_params_caps_serialization_to_valid_range() -> TestResult {
        // Simulates: SQLPrepare("SELECT ?") → 1 marker, then two bindings:
        //   param 1 = valid integer
        //   param 2 = phantom DAE bind with dummy pointer
        // With max_params=1, only param 1 is serialized; the dummy pointer
        // for param 2 is never touched.
        let val: i32 = 42;
        let mut dae_ind: sql::Len = sql::DATA_AT_EXEC;
        let (apd, ipd) = make_descriptors(vec![
            (
                1,
                CDataType::Long,
                sql::SqlDataType::INTEGER,
                &val as *const i32 as sql::Pointer,
                0,
                std::ptr::null_mut(),
            ),
            (
                2,
                CDataType::Char,
                sql::SqlDataType::VARCHAR,
                1usize as sql::Pointer, // dummy DAE token
                0,
                &mut dae_ind,
            ),
        ]);
        let json = odbc_bindings_to_json(&apd, &ipd, 1)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed["1"]["type"], "FIXED");
        assert_eq!(parsed["1"]["value"], "42");
        assert!(parsed.get("2").is_none());
        Ok(())
    }

    #[test]
    fn json_multi_row_column_wise_emits_value_arrays() -> TestResult {
        // 3-row column-wise array binding for one INTEGER and one VARCHAR
        // parameter. The JSON wire form must wrap each parameter's row
        // values in an array — the scalar form is only valid when
        // `apd.array_size == 1`.
        let ids: [i32; 3] = [10, 20, 30];
        const NAME_BUF: usize = 8;
        let mut names = [0u8; NAME_BUF * 3];
        for (i, s) in ["a", "bb", "ccc"].iter().enumerate() {
            names[i * NAME_BUF..i * NAME_BUF + s.len()].copy_from_slice(s.as_bytes());
        }
        let mut name_inds: [sql::Len; 3] = [1, 2, 3];

        let (mut apd, ipd) = make_descriptors(vec![
            (
                1,
                CDataType::Long,
                sql::SqlDataType::INTEGER,
                ids.as_ptr() as sql::Pointer,
                mem::size_of::<i32>() as sql::Len,
                std::ptr::null_mut(),
            ),
            (
                2,
                CDataType::Char,
                sql::SqlDataType::VARCHAR,
                names.as_ptr() as sql::Pointer,
                NAME_BUF as sql::Len,
                name_inds.as_mut_ptr(),
            ),
        ]);
        apd.array_size = 3;

        let json = odbc_bindings_to_json(&apd, &ipd, 2)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed["1"]["type"], "FIXED");
        assert_eq!(parsed["1"]["value"], serde_json::json!(["10", "20", "30"]));
        assert_eq!(parsed["2"]["type"], "TEXT");
        assert_eq!(parsed["2"]["value"], serde_json::json!(["a", "bb", "ccc"]));
        Ok(())
    }

    #[test]
    fn json_multi_row_skips_param_ignore_sets() -> TestResult {
        // SNOW-3235553: SQL_ATTR_PARAM_OPERATION_PTR marks the middle set
        // SQL_PARAM_IGNORE, so only rows 0 and 2 are serialized for every param.
        use crate::api::SQL_PARAM_PROCEED;
        let ids: [i32; 3] = [10, 20, 30];
        const NAME_BUF: usize = 8;
        let mut names = [0u8; NAME_BUF * 3];
        for (i, s) in ["a", "bb", "ccc"].iter().enumerate() {
            names[i * NAME_BUF..i * NAME_BUF + s.len()].copy_from_slice(s.as_bytes());
        }
        let mut name_inds: [sql::Len; 3] = [1, 2, 3];

        let (mut apd, ipd) = make_descriptors(vec![
            (
                1,
                CDataType::Long,
                sql::SqlDataType::INTEGER,
                ids.as_ptr() as sql::Pointer,
                mem::size_of::<i32>() as sql::Len,
                std::ptr::null_mut(),
            ),
            (
                2,
                CDataType::Char,
                sql::SqlDataType::VARCHAR,
                names.as_ptr() as sql::Pointer,
                NAME_BUF as sql::Len,
                name_inds.as_mut_ptr(),
            ),
        ]);
        apd.array_size = 3;
        let ops: [u16; 3] = [SQL_PARAM_PROCEED, SQL_PARAM_IGNORE, SQL_PARAM_PROCEED];
        apd.array_status_ptr = ops.as_ptr() as *mut u16;

        let json = odbc_bindings_to_json(&apd, &ipd, 2)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed["1"]["value"], serde_json::json!(["10", "30"]));
        assert_eq!(parsed["2"]["value"], serde_json::json!(["a", "ccc"]));
        Ok(())
    }

    #[test]
    fn json_all_param_ignore_yields_empty_value_arrays() -> TestResult {
        // SNOW-3235553: when every set is marked SQL_PARAM_IGNORE, all rows are
        // skipped and each parameter's value array is empty. Verifies the
        // driver-side serialization stays well-formed (no panic, no stray or
        // misaligned values) when the operation array skips everything. The
        // server's response to the resulting empty INSERT is an e2e deferral
        // tracked in large_bindings.feature.
        let ids: [i32; 2] = [10, 20];
        let (mut apd, ipd) = make_descriptors(vec![(
            1,
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            ids.as_ptr() as sql::Pointer,
            mem::size_of::<i32>() as sql::Len,
            std::ptr::null_mut(),
        )]);
        apd.array_size = 2;
        let ops: [u16; 2] = [SQL_PARAM_IGNORE, SQL_PARAM_IGNORE];
        apd.array_status_ptr = ops.as_ptr() as *mut u16;

        let json = odbc_bindings_to_json(&apd, &ipd, 1)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed["1"]["value"], serde_json::json!([]));
        Ok(())
    }

    #[test]
    fn json_multi_row_null_indicator_yields_json_null_cell() -> TestResult {
        // Mixed-NULL multi-row binding: row 1 of the INTEGER column is NULL.
        // The per-parameter `type` is still FIXED (taken from non-NULL rows);
        // the NULL row becomes JSON `null` inside the value array.
        let ids: [i32; 3] = [1, 0, 3];
        let mut id_inds: [sql::Len; 3] = [0, sql::NULL_DATA, 0];

        let (mut apd, ipd) = make_descriptors(vec![(
            1,
            CDataType::Long,
            sql::SqlDataType::INTEGER,
            ids.as_ptr() as sql::Pointer,
            mem::size_of::<i32>() as sql::Len,
            id_inds.as_mut_ptr(),
        )]);
        apd.array_size = 3;

        let json = odbc_bindings_to_json(&apd, &ipd, 1)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed["1"]["type"], "FIXED");
        assert_eq!(
            parsed["1"]["value"],
            serde_json::json!(["1", serde_json::Value::Null, "3"])
        );
        Ok(())
    }

    #[test]
    fn json_column_wise_fixed_types_with_zero_buffer_length_stride_by_type_size() -> TestResult {
        // SNOW-3720841: fixed-size types bound with BufferLength=0 must stride by the C type size.
        let ids: [i32; 3] = [10, 20, 30];
        let bigs: [i64; 3] = [100, 200, 300];

        let (mut apd, ipd) = make_descriptors(vec![
            (
                1,
                CDataType::SLong,
                sql::SqlDataType::INTEGER,
                ids.as_ptr() as sql::Pointer,
                0,
                std::ptr::null_mut(),
            ),
            (
                2,
                CDataType::SBigInt,
                sql::SqlDataType::EXT_BIG_INT,
                bigs.as_ptr() as sql::Pointer,
                0,
                std::ptr::null_mut(),
            ),
        ]);
        apd.array_size = 3;

        let json = odbc_bindings_to_json(&apd, &ipd, 2)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed["1"]["type"], "FIXED");
        assert_eq!(parsed["1"]["value"], serde_json::json!(["10", "20", "30"]));
        assert_eq!(parsed["2"]["type"], "FIXED");
        assert_eq!(
            parsed["2"]["value"],
            serde_json::json!(["100", "200", "300"])
        );
        Ok(())
    }

    #[test]
    fn csv_column_wise_fixed_type_with_zero_buffer_length_strides_by_type_size() -> TestResult {
        // SNOW-3720841: same BufferLength=0 stride fix on the CSV (stage) bind path.
        let ids: [i32; 3] = [10, 20, 30];

        let (mut apd, ipd) = make_descriptors(vec![(
            1,
            CDataType::SLong,
            sql::SqlDataType::INTEGER,
            ids.as_ptr() as sql::Pointer,
            0,
            std::ptr::null_mut(),
        )]);
        apd.array_size = 3;

        let csv = odbc_bindings_to_csv(&apd, &ipd, 1)?;
        // One row per line; before the fix every line was "10".
        assert_eq!(csv, "\"10\"\n\"20\"\n\"30\"\n");
        Ok(())
    }

    #[test]
    fn convert_char_as_integer() -> TestResult {
        let val = b"12345\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::INTEGER,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "12345".to_string());
        Ok(())
    }

    #[test]
    fn convert_char_as_real() -> TestResult {
        let val = b"3.14\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, "3.14".to_string());
        Ok(())
    }

    // Non-finite numeric-literal rejection (SQLSTATE 22018)
    //
    // Rust's f64::from_str accepts "Infinity", "-Infinity" and "NaN", but the
    // ODBC "numeric-literal" grammar (MS ODBC spec, Appendix C) does not
    // permit these tokens. The driver rejects them client-side so the caller
    // sees InvalidCharacterValueForCast instead of a value that only works
    // for SQL_REAL/SQL_DOUBLE targets.

    #[test]
    fn convert_char_infinity_as_real_rejected() {
        let val = b"Infinity\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidNumericLiteral { .. })
        ));
    }

    #[test]
    fn convert_char_neg_infinity_as_real_rejected() {
        let val = b"-Infinity\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidNumericLiteral { .. })
        ));
    }

    #[test]
    fn convert_char_nan_as_real_rejected() {
        let val = b"NaN\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidNumericLiteral { .. })
        ));
    }

    #[test]
    fn convert_wchar_infinity_as_real_rejected() {
        let val: [u16; 9] = [
            b'I' as u16,
            b'n' as u16,
            b'f' as u16,
            b'i' as u16,
            b'n' as u16,
            b'i' as u16,
            b't' as u16,
            b'y' as u16,
            0,
        ];
        let mut ind: sql::Len = sql::NTS;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            (val.len() * mem::size_of::<u16>()) as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidNumericLiteral { .. })
        ));
    }

    #[test]
    fn convert_wchar_neg_infinity_as_real_rejected() {
        let val: [u16; 10] = [
            b'-' as u16,
            b'I' as u16,
            b'n' as u16,
            b'f' as u16,
            b'i' as u16,
            b'n' as u16,
            b'i' as u16,
            b't' as u16,
            b'y' as u16,
            0,
        ];
        let mut ind: sql::Len = sql::NTS;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            (val.len() * mem::size_of::<u16>()) as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidNumericLiteral { .. })
        ));
    }

    #[test]
    fn convert_wchar_nan_as_real_rejected() {
        let val: [u16; 4] = [b'N' as u16, b'a' as u16, b'N' as u16, 0];
        let mut ind: sql::Len = sql::NTS;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            (val.len() * mem::size_of::<u16>()) as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidNumericLiteral { .. })
        ));
    }

    // Overflow vs. explicit-non-finite-token discrimination
    //
    // Rust's `f64::from_str` overflows well-formed numeric literals whose
    // magnitude exceeds `f64::MAX` (e.g. "1e309") silently to +/-inf. Those
    // literals are valid ODBC numeric-literals; only the magnitude is out of
    // range, so the spec-aligned SQLSTATE is 22003 (NumericMagnitudeOverflow),
    // not 22018 (InvalidNumericLiteral, reserved for tokens that aren't in
    // the ODBC numeric-literal grammar at all). The next four tests pin both
    // halves of that contract.

    #[test]
    fn convert_char_overflow_as_real_overflows() {
        let val = b"1e309\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::NumericMagnitudeOverflow { .. })
        ));
    }

    #[test]
    fn convert_char_neg_overflow_as_real_overflows() {
        let val = b"-1e309\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::NumericMagnitudeOverflow { .. })
        ));
    }

    #[test]
    fn convert_wchar_overflow_as_real_overflows() {
        // UTF-16 of "1e309"
        let val: [u16; 6] = [
            b'1' as u16,
            b'e' as u16,
            b'3' as u16,
            b'0' as u16,
            b'9' as u16,
            0,
        ];
        let mut ind: sql::Len = sql::NTS;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            (val.len() * mem::size_of::<u16>()) as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::NumericMagnitudeOverflow { .. })
        ));
    }

    #[test]
    fn convert_char_lowercase_inf_as_real_rejected() {
        // Lock in case-insensitive token detection: a future "let's only
        // match the canonical \"Infinity\" spelling" regression must fail
        // this test, since "inf" is also accepted by Rust's parser.
        let val = b"inf\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidNumericLiteral { .. })
        ));
    }

    // Malformed numeric literal: SQL_C_CHAR is a supported source type for
    // SQL_REAL/DOUBLE, so a parse failure must surface as InvalidNumericLiteral
    // (SQLSTATE 22018), not UnsupportedCDataType (07006). Same contract for
    // SQL_C_WCHAR below.
    #[test]
    fn convert_char_garbage_as_real_rejected() {
        let val = b"hello\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidNumericLiteral { .. })
        ));
    }

    #[test]
    fn convert_wchar_garbage_as_real_rejected() {
        // UTF-16 of "hello"
        let val: [u16; 6] = [
            b'h' as u16,
            b'e' as u16,
            b'l' as u16,
            b'l' as u16,
            b'o' as u16,
            0,
        ];
        let mut ind: sql::Len = sql::NTS;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            (val.len() * mem::size_of::<u16>()) as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidNumericLiteral { .. })
        ));
    }

    // SQL_C_WCHAR mirror of `convert_char_neg_overflow_as_real_overflows`:
    // a well-formed but range-overflowing literal must surface as
    // NumericMagnitudeOverflow (22003) on the WChar path too.
    #[test]
    fn convert_wchar_neg_overflow_as_real_overflows() {
        // UTF-16 of "-1e309"
        let val: [u16; 7] = [
            b'-' as u16,
            b'1' as u16,
            b'e' as u16,
            b'3' as u16,
            b'0' as u16,
            b'9' as u16,
            0,
        ];
        let mut ind: sql::Len = sql::NTS;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            (val.len() * mem::size_of::<u16>()) as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::NumericMagnitudeOverflow { .. })
        ));
    }

    // Case-insensitive token detection: Rust's parser accepts `inf`,
    // `infinity`, `nan` in arbitrary case, and signed prefixes. The
    // explicit-non-finite-token detector must cover that whole surface so
    // mixed-case spellings cannot escape the 22018 path and end up on the
    // server (or get reclassified as 22003 overflow).
    #[test]
    fn convert_char_mixed_case_infinity_as_real_rejected() {
        let val = b"InFiNiTy\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidNumericLiteral { .. })
        ));
    }

    #[test]
    fn convert_char_uppercase_inf_as_real_rejected() {
        let val = b"INF\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidNumericLiteral { .. })
        ));
    }

    #[test]
    fn convert_char_lowercase_nan_as_real_rejected() {
        let val = b"nan\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidNumericLiteral { .. })
        ));
    }

    #[test]
    fn convert_char_mixed_case_nan_as_real_rejected() {
        let val = b"NaN\0";
        // (NaN is the canonical spelling, but we also accept arbitrary case.)
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidNumericLiteral { .. })
        ));
        // Also exercise a non-canonical case to catch any narrowing of the
        // detector.
        let val2 = b"nAn\0";
        let binding2 = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val2.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding2),
            Err(BindingError::InvalidNumericLiteral { .. })
        ));
    }

    #[test]
    fn convert_char_signed_lowercase_infinity_as_real_rejected() {
        let val = b"+infinity\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidNumericLiteral { .. })
        ));
    }

    #[test]
    fn convert_wchar_mixed_case_infinity_as_real_rejected() {
        // UTF-16 of "iNFINITY"
        let val: [u16; 9] = [
            b'i' as u16,
            b'N' as u16,
            b'F' as u16,
            b'I' as u16,
            b'N' as u16,
            b'I' as u16,
            b'T' as u16,
            b'Y' as u16,
            0,
        ];
        let mut ind: sql::Len = sql::NTS;
        let binding = make_binding(
            CDataType::WChar,
            sql::SqlDataType::DOUBLE,
            val.as_ptr() as sql::Pointer,
            (val.len() * mem::size_of::<u16>()) as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidNumericLiteral { .. })
        ));
    }

    // -- Structured C types → VARCHAR -----------------------------------------
    //
    // These tests live here (not in varchar.rs) because they validate the full
    // C-to-SQL pipeline: make_converter → ParamConverter::convert → ReadODBC +
    // WriteJson. This mirrors all other conversion tests above (integer, float,
    // char, bit, binary) which also exercise the end-to-end binding path.

    #[test]
    fn convert_timestamp_as_varchar() -> TestResult {
        let ts = sql::Timestamp {
            year: 2024,
            month: 1,
            day: 15,
            hour: 10,
            minute: 30,
            second: 45,
            fraction: 0,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            sql::SqlDataType::VARCHAR,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, "2024-01-15 10:30:45".to_string());
        Ok(())
    }

    #[test]
    fn convert_timestamp_with_fraction_as_varchar() -> TestResult {
        let ts = sql::Timestamp {
            year: 1,
            month: 1,
            day: 1,
            hour: 1,
            minute: 1,
            second: 1,
            fraction: 1,
        };
        let binding = make_binding(
            CDataType::TimeStamp,
            sql::SqlDataType::VARCHAR,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(
            v,
            Value::String("0001-01-01 01:01:01.000000001".to_string())
        );
        Ok(())
    }

    #[test]
    fn convert_date_as_varchar() -> TestResult {
        let d = sql::Date {
            year: 2024,
            month: 12,
            day: 25,
        };
        let binding = make_binding(
            CDataType::TypeDate,
            sql::SqlDataType::VARCHAR,
            &d as *const sql::Date as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, "2024-12-25".to_string());
        Ok(())
    }

    #[test]
    fn convert_time_as_varchar() -> TestResult {
        let t = sql::Time {
            hour: 14,
            minute: 30,
            second: 59,
        };
        let binding = make_binding(
            CDataType::TypeTime,
            sql::SqlDataType::VARCHAR,
            &t as *const sql::Time as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, "14:30:59".to_string());
        Ok(())
    }

    // -- Cross-temporal binds (DATE↔TIMESTAMP, TIME↔TIMESTAMP) ----------------
    //
    // These mirror the legacy 3.16.0 behavior, which itself implements the
    // ODBC spec (Appendix D, "Converting Data from C to SQL Data Types"):
    //   - SQL_C_TYPE_TIMESTAMP → SQL_DATE: extract the date portion. SQLSTATE
    //     22008 ("Datetime field overflow") if the time portion is nonzero.
    //   - SQL_C_TYPE_TIMESTAMP → SQL_TIME: extract the whole-second time
    //     portion. SQLSTATE 22008 if the fractional-seconds portion is
    //     nonzero. The date portion is silently discarded.
    //   - SQL_C_TYPE_DATE → SQL_TIMESTAMP*: combine the date with 00:00:00.
    //   - SQL_C_TYPE_TIME → SQL_TIMESTAMP*: pair the time with the current
    //     local date and zero fractional seconds.
    //
    // Two distinct error classes apply to the structs themselves:
    //   - 22007 ("Invalid datetime format") — struct field outside its legal
    //     range (e.g. month=13, hour=25), via BindingError::InvalidDatetimeValue.
    //   - 22008 ("Datetime field overflow") — discarded portion is non-zero
    //     when narrowing TIMESTAMP → DATE/TIME, via
    //     BindingError::DatetimeFieldOverflow.
    // Both are distinct from 07006 ("Restricted data type attribute
    // violation"), which would incorrectly signal that the conversion itself
    // is unsupported.

    #[test]
    fn convert_timestamp_as_date_extracts_date_part() -> TestResult {
        // ODBC Appendix D: TIMESTAMP → DATE only succeeds when the discarded
        // time portion is exactly zero. Use midnight so we exercise the
        // happy-path date extraction; the 22008-on-nonzero-time behavior is
        // covered by `convert_timestamp_as_date_rejects_nonzero_hour` and
        // `convert_timestamp_as_date_rejects_nonzero_fraction`.
        let ts = sql::Timestamp {
            year: 2024,
            month: 12,
            day: 25,
            hour: 0,
            minute: 0,
            second: 0,
            fraction: 0,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            sql::SqlDataType::DATE,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Date);
        let expected_millis = (chrono::NaiveDate::from_ymd_opt(2024, 12, 25).unwrap()
            - chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
        .num_days()
            * 86_400_000;
        assert_eq!(v, expected_millis.to_string());
        Ok(())
    }

    #[test]
    fn convert_timestamp_as_time_extracts_time_part() -> TestResult {
        // ODBC Appendix D: TIMESTAMP → TIME only succeeds when the discarded
        // fractional-seconds portion is exactly zero. The whole-second h/m/s
        // are preserved and the date portion is silently dropped.
        let ts = sql::Timestamp {
            year: 2024,
            month: 1,
            day: 15,
            hour: 12,
            minute: 30,
            second: 45,
            fraction: 0,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            sql::SqlDataType::TIME,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Time);
        // 12:30:45 -> 45045s -> 45_045_000_000_000ns since midnight.
        assert_eq!(v, "45045000000000".to_string());
        Ok(())
    }

    #[test]
    fn convert_date_as_timestamp_combines_with_midnight() -> TestResult {
        let d = sql::Date {
            year: 2024,
            month: 6,
            day: 1,
        };
        let binding = make_binding(
            CDataType::TypeDate,
            sql::SqlDataType::TIMESTAMP,
            &d as *const sql::Date as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::TimestampNtz);
        let expected_nanos = chrono::NaiveDate::from_ymd_opt(2024, 6, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_nanos_opt()
            .unwrap();
        assert_eq!(v, expected_nanos.to_string());
        Ok(())
    }

    #[test]
    fn convert_date_as_timestamp_rejects_invalid_date() {
        let d = sql::Date {
            year: 2024,
            month: 13,
            day: 1,
        };
        let binding = make_binding(
            CDataType::TypeDate,
            sql::SqlDataType::TIMESTAMP,
            &d as *const sql::Date as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        // Per ODBC Appendix D ("C to SQL: Date"), an invalid date in a
        // SQL_C_TYPE_DATE bound to a SQL_TYPE_TIMESTAMP target must surface
        // as SQLSTATE 22007 (Invalid datetime format), not 07006 (restricted
        // data type attribute violation).
        let err = convert_binding(&binding).expect_err("invalid date must error");
        assert!(
            matches!(err, BindingError::InvalidDatetimeValue { .. }),
            "expected InvalidDatetimeValue, got {err:?}"
        );
    }

    // -- SQL_INTERVAL_* parameter targets -------------------------------------
    //
    // SQL_INTERVAL_* parameter types are routed through dedicated
    // `SnowflakeIntervalYearMonth` / `SnowflakeIntervalDayTime` converters
    // (see `conversion/interval.rs`). These tests verify both ends of the
    // pipeline:
    //   - the factory (make_converter) accepts every SQL_INTERVAL_* concise
    //     code (101..=113) and dispatches to the correct family converter
    //     with the right SnowflakeLogicalType (INTERVAL_YEAR_MONTH or
    //     INTERVAL_DAY_TIME),
    //   - `format_interval` renders SQL_C_INTERVAL_* structs into the ANSI
    //     literal text the spec defines for each subtype, including sign and
    //     fractional seconds.
    //
    // C-source restrictions enforced by the new converters (rejection of
    // FLOAT/DOUBLE/BINARY/GUID/numeric-into-compound, etc.) are exercised
    // separately by the unit tests in `conversion/interval_tests.rs`.

    fn ym_interval(sign: sql::SmallInt, year: u32, month: u32) -> sql::IntervalStruct {
        sql::IntervalStruct {
            interval_type: 0, // ignored: we trust the C value type, not the struct field
            interval_sign: sign,
            interval_value: sql::IntervalUnion {
                year_month: sql::YearMonth { year, month },
            },
        }
    }

    fn ds_interval(
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

    /// Bind an interval struct to a SQL_INTERVAL_* parameter (raw concise
    /// type code 101..=113, since odbc-sys lacks named constants for these).
    fn convert_interval(
        c_type: CDataType,
        sql_code: i16,
        iv: &sql::IntervalStruct,
    ) -> Result<(SnowflakeLogicalType, String), BindingError> {
        let binding = make_binding(
            c_type,
            sql::SqlDataType(sql_code),
            iv as *const sql::IntervalStruct as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        convert_binding(&binding)
    }

    #[test]
    fn convert_interval_year_basic() -> TestResult {
        let iv = ym_interval(0, 5, 0);
        let (ty, v) = convert_interval(CDataType::IntervalYear, 101, &iv)?;
        assert_eq!(ty, SnowflakeLogicalType::IntervalYearMonth);
        assert_eq!(v, "5".to_string());
        Ok(())
    }

    #[test]
    fn convert_interval_month_negative() -> TestResult {
        let iv = ym_interval(1, 0, 11);
        let (ty, v) = convert_interval(CDataType::IntervalMonth, 102, &iv)?;
        assert_eq!(ty, SnowflakeLogicalType::IntervalYearMonth);
        assert_eq!(v, "-11".to_string());
        Ok(())
    }

    #[test]
    fn convert_interval_day_basic() -> TestResult {
        // Single-field SQL_C_INTERVAL_DAY: only `day_second.day` is
        // populated. The formatter must not look at `hour`/`minute`/etc.
        let iv = ds_interval(0, 42, 0, 0, 0, 0);
        let (ty, v) = convert_interval(CDataType::IntervalDay, 103, &iv)?;
        assert_eq!(ty, SnowflakeLogicalType::IntervalDayTime);
        assert_eq!(v, "42".to_string());
        Ok(())
    }

    #[test]
    fn convert_interval_hour_basic() -> TestResult {
        // Single-field SQL_C_INTERVAL_HOUR: only `day_second.hour` is read.
        let iv = ds_interval(0, 0, 23, 0, 0, 0);
        let (_, v) = convert_interval(CDataType::IntervalHour, 104, &iv)?;
        assert_eq!(v, "23".to_string());
        Ok(())
    }

    #[test]
    fn convert_interval_minute_negative() -> TestResult {
        // Single-field SQL_C_INTERVAL_MINUTE with sign — confirms only
        // `day_second.minute` is consulted alongside `interval_sign`.
        let iv = ds_interval(1, 0, 0, 90, 0, 0);
        let (_, v) = convert_interval(CDataType::IntervalMinute, 105, &iv)?;
        assert_eq!(v, "-90".to_string());
        Ok(())
    }

    #[test]
    fn convert_interval_year_to_month() -> TestResult {
        let iv = ym_interval(0, 5, 11);
        let (_, v) = convert_interval(CDataType::IntervalYearToMonth, 107, &iv)?;
        assert_eq!(v, "5-11".to_string());
        Ok(())
    }

    #[test]
    fn convert_interval_day_to_second_with_fraction() -> TestResult {
        // 10 days, 12 hours, 30 minutes, 59.500000 seconds. `fraction` is
        // in microseconds (matches the unit produced by
        // `numeric_helpers::compute_interval_fraction`) and is rendered at
        // the full 6-digit width per the ODBC "Interval Data Type Length"
        // spec (default seconds precision = 6).
        let iv = ds_interval(0, 10, 12, 30, 59, 500_000);
        let (_, v) = convert_interval(CDataType::IntervalDayToSecond, 110, &iv)?;
        assert_eq!(v, "10 12:30:59.500000".to_string());
        Ok(())
    }

    #[test]
    fn convert_interval_second_full_precision() -> TestResult {
        // 1.000001s — verify microsecond precision is preserved (the unit
        // chosen by the rest of the conversion path; see
        // `numeric_helpers::compute_interval_fraction`).
        let iv = ds_interval(0, 0, 0, 0, 1, 1);
        let (_, v) = convert_interval(CDataType::IntervalSecond, 106, &iv)?;
        assert_eq!(v, "1.000001".to_string());
        Ok(())
    }

    #[test]
    fn convert_interval_negative_day_to_second() -> TestResult {
        // Hour is a non-leading field (after a space), so it's zero-padded
        // to 2 chars per the ODBC "Interval Data Type Length" spec; the
        // seconds component is always rendered with a 6-digit fraction.
        let iv = ds_interval(1, 1, 2, 3, 4, 0);
        let (_, v) = convert_interval(CDataType::IntervalDayToSecond, 110, &iv)?;
        assert_eq!(v, "-1 02:03:04.000000".to_string());
        Ok(())
    }

    #[test]
    fn convert_interval_hour_to_minute() -> TestResult {
        let iv = ds_interval(0, 0, 14, 7, 0, 0);
        let (_, v) = convert_interval(CDataType::IntervalHourToMinute, 111, &iv)?;
        assert_eq!(v, "14:07".to_string());
        Ok(())
    }

    #[test]
    fn convert_interval_day_to_hour() -> TestResult {
        // Hour is non-leading (after the space separator) and zero-padded
        // to 2 chars per ODBC "Interval Data Type Length".
        let iv = ds_interval(0, 3, 7, 0, 0, 0);
        let (_, v) = convert_interval(CDataType::IntervalDayToHour, 108, &iv)?;
        assert_eq!(v, "3 07".to_string());
        Ok(())
    }

    #[test]
    fn convert_interval_day_to_minute() -> TestResult {
        // Both hour and minute sub-fields are zero-padded; the leading
        // day field is rendered as-is.
        let iv = ds_interval(0, 3, 7, 5, 0, 0);
        let (_, v) = convert_interval(CDataType::IntervalDayToMinute, 109, &iv)?;
        assert_eq!(v, "3 07:05".to_string());
        Ok(())
    }

    #[test]
    fn convert_interval_hour_to_second_with_fraction() -> TestResult {
        // Minute and second are zero-padded; the seconds fraction is
        // emitted at the canonical 6-digit microsecond width per the
        // ODBC spec (no trimming).
        let iv = ds_interval(0, 0, 12, 30, 59, 250_000);
        let (_, v) = convert_interval(CDataType::IntervalHourToSecond, 112, &iv)?;
        assert_eq!(v, "12:30:59.250000".to_string());
        Ok(())
    }

    #[test]
    fn convert_interval_minute_to_second_no_fraction() -> TestResult {
        // Sub-field seconds are zero-padded to 2 digits per ODBC spec
        // (matches the formatting of HH:MM in HOUR_TO_MINUTE) and the
        // seconds fraction is always emitted at the canonical 6-digit
        // width — even when zero — so applications round-trip the literal
        // through legacy ODBC and other spec-conforming consumers.
        let iv = ds_interval(0, 0, 0, 30, 7, 0);
        let (_, v) = convert_interval(CDataType::IntervalMinuteToSecond, 113, &iv)?;
        assert_eq!(v, "30:07.000000".to_string());
        Ok(())
    }

    #[test]
    fn convert_text_value_to_interval_target_passes_through() -> TestResult {
        // Applications routinely send the interval as a text literal even
        // when the SQL parameter type is SQL_INTERVAL_* — verify SQL_C_CHAR
        // is accepted and the JSON `type` is the spec-aligned
        // INTERVAL_YEAR_MONTH (not the legacy TEXT).
        let s = b"5-11\0";
        let mut len: sql::Len = 4;
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType(107), // SQL_INTERVAL_YEAR_TO_MONTH
            s.as_ptr() as sql::Pointer,
            5,
            &mut len,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::IntervalYearMonth);
        assert_eq!(v, "5-11".to_string());
        Ok(())
    }

    #[test]
    fn convert_guid_to_text() -> TestResult {
        // SQLGUID is d1:u32, d2:u16, d3:u16, d4:[u8;8]; canonical text form
        // is `XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX` with uppercase hex,
        // matching the Windows COM/ODBC convention.
        let g = sql::Guid {
            d1: 0x1234_5678,
            d2: 0x1234,
            d3: 0x1234,
            d4: [0xAB, 0xCD, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
        };
        let binding = make_binding(
            CDataType::Guid,
            sql::SqlDataType::VARCHAR,
            &g as *const sql::Guid as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(
            v,
            Value::String("12345678-1234-1234-ABCD-010203040506".to_string())
        );
        Ok(())
    }

    #[test]
    fn convert_guid_zero_pads_components() -> TestResult {
        // Each component must be zero-padded to its full hex width even when
        // the numeric value is small (e.g. d1=1 must render as "00000001").
        let g = sql::Guid {
            d1: 1,
            d2: 2,
            d3: 3,
            d4: [0, 0, 0, 0, 0, 0, 0, 0],
        };
        let binding = make_binding(
            CDataType::Guid,
            sql::SqlDataType::VARCHAR,
            &g as *const sql::Guid as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (_, v) = convert_binding(&binding)?;
        assert_eq!(
            v,
            Value::String("00000001-0002-0003-0000-000000000000".to_string())
        );
        Ok(())
    }

    #[test]
    fn convert_interval_factory_rejects_unknown_codes() {
        // Code 100 sits just below SQL_INTERVAL_YEAR (101); make sure we
        // didn't accidentally widen the range.
        let iv = ym_interval(0, 1, 0);
        let err = convert_interval(CDataType::IntervalYear, 100, &iv);
        assert!(
            err.is_err(),
            "code 100 should not be a valid INTERVAL target"
        );
    }

    #[test]
    fn convert_time_as_timestamp_uses_current_date_and_zero_fraction() -> TestResult {
        let t = sql::Time {
            hour: 14,
            minute: 30,
            second: 45,
        };
        let binding = make_binding(
            CDataType::TypeTime,
            sql::SqlDataType::TIMESTAMP,
            &t as *const sql::Time as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );

        let today_before = chrono::Local::now().date_naive();
        let (ty, v) = convert_binding(&binding)?;
        let today_after = chrono::Local::now().date_naive();

        assert_eq!(ty, SnowflakeLogicalType::TimestampNtz);

        let nanos: i64 = v.parse().expect("nanos must parse as i64");
        let dt = chrono::DateTime::from_timestamp_nanos(nanos).naive_utc();

        // The time component is preserved exactly with a zero fractional part.
        assert_eq!(
            dt.time(),
            chrono::NaiveTime::from_hms_opt(14, 30, 45).unwrap()
        );
        // The date component is "current date" — anywhere in the window the
        // call took to execute (handles midnight rollover gracefully).
        assert!(
            dt.date() >= today_before && dt.date() <= today_after,
            "date {} not within [{}, {}]",
            dt.date(),
            today_before,
            today_after
        );
        Ok(())
    }

    #[test]
    fn convert_time_as_timestamp_rejects_invalid_time() {
        let t = sql::Time {
            hour: 25,
            minute: 0,
            second: 0,
        };
        let binding = make_binding(
            CDataType::TypeTime,
            sql::SqlDataType::TIMESTAMP,
            &t as *const sql::Time as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        // Per ODBC Appendix D ("C to SQL: Time"), an invalid time in a
        // SQL_C_TYPE_TIME bound to a SQL_TYPE_TIMESTAMP target must surface
        // as SQLSTATE 22007.
        let err = convert_binding(&binding).expect_err("invalid time must error");
        assert!(
            matches!(err, BindingError::InvalidDatetimeValue { .. }),
            "expected InvalidDatetimeValue, got {err:?}"
        );
    }

    #[test]
    fn convert_timestamp_as_date_rejects_invalid_date() {
        let ts = sql::Timestamp {
            year: 2024,
            month: 13,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            fraction: 0,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            sql::SqlDataType::DATE,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        // Per ODBC Appendix D ("C to SQL: Timestamp"), an invalid date in a
        // SQL_C_TYPE_TIMESTAMP bound to a SQL_TYPE_DATE target must surface
        // as SQLSTATE 22007.
        let err = convert_binding(&binding).expect_err("invalid date must error");
        assert!(
            matches!(err, BindingError::InvalidDatetimeValue { .. }),
            "expected InvalidDatetimeValue, got {err:?}"
        );
    }

    #[test]
    fn convert_timestamp_as_time_rejects_invalid_time() {
        let ts = sql::Timestamp {
            year: 2024,
            month: 1,
            day: 1,
            hour: 25,
            minute: 0,
            second: 0,
            fraction: 0,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            sql::SqlDataType::TIME,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        // Per ODBC Appendix D ("C to SQL: Timestamp"), an invalid time in a
        // SQL_C_TYPE_TIMESTAMP bound to a SQL_TYPE_TIME target must surface
        // as SQLSTATE 22007.
        let err = convert_binding(&binding).expect_err("invalid time must error");
        assert!(
            matches!(err, BindingError::InvalidDatetimeValue { .. }),
            "expected InvalidDatetimeValue, got {err:?}"
        );
    }

    // -- TIMESTAMP → DATE / TIME truncation overflow (SQLSTATE 22008) --------
    //
    // Per ODBC Appendix D ("Converting Data from C to SQL Data Types"):
    //   - TIMESTAMP → DATE: 22008 if the time portion of the timestamp is
    //     nonzero (any of hour / minute / second / fraction).
    //   - TIMESTAMP → TIME: 22008 if the fractional seconds portion is
    //     nonzero.
    // This matches the legacy 3.16.0 driver, which surfaces SQL_ERROR with
    // SQLSTATE=22008 and NativeError=40520 in these cases.
    //
    // Error precedence: an out-of-range struct field always wins over the
    // narrowing rule. If a SQL_TIMESTAMP_STRUCT has both an invalid field
    // (e.g. hour=25, fraction>999_999_999) AND a non-zero discarded portion,
    // the result must be 22007 (InvalidDatetimeValue), not 22008. The
    // *_22007_takes_precedence_over_22008 tests below pin this down.

    #[test]
    fn convert_timestamp_as_date_rejects_nonzero_hour() {
        let ts = sql::Timestamp {
            year: 2026,
            month: 4,
            day: 13,
            hour: 14,
            minute: 30,
            second: 45,
            fraction: 0,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            sql::SqlDataType::DATE,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let err = convert_binding(&binding).expect_err("nonzero time must overflow");
        assert!(
            matches!(err, BindingError::DatetimeFieldOverflow { .. }),
            "expected DatetimeFieldOverflow, got {err:?}"
        );
    }

    #[test]
    fn convert_timestamp_as_date_rejects_nonzero_fraction() {
        let ts = sql::Timestamp {
            year: 2026,
            month: 4,
            day: 13,
            hour: 0,
            minute: 0,
            second: 0,
            fraction: 1,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            sql::SqlDataType::DATE,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let err = convert_binding(&binding).expect_err("nonzero fraction must overflow");
        assert!(
            matches!(err, BindingError::DatetimeFieldOverflow { .. }),
            "expected DatetimeFieldOverflow, got {err:?}"
        );
    }

    #[test]
    fn convert_timestamp_as_time_rejects_nonzero_fraction() {
        let ts = sql::Timestamp {
            year: 2026,
            month: 4,
            day: 13,
            hour: 14,
            minute: 30,
            second: 45,
            fraction: 500_000_000,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            sql::SqlDataType::TIME,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let err = convert_binding(&binding).expect_err("nonzero fraction must overflow");
        assert!(
            matches!(err, BindingError::DatetimeFieldOverflow { .. }),
            "expected DatetimeFieldOverflow, got {err:?}"
        );
    }

    // -- 22007 takes precedence over 22008 (regression for SQLSTATE mapping) -

    #[test]
    fn convert_timestamp_as_date_invalid_hour_takes_precedence_over_22008() {
        // hour=25 makes the struct itself malformed (22007), AND the time
        // portion is non-zero which would also trigger the narrowing rule
        // (22008). The struct-validity error must win.
        let ts = sql::Timestamp {
            year: 2026,
            month: 4,
            day: 13,
            hour: 25,
            minute: 0,
            second: 0,
            fraction: 0,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            sql::SqlDataType::DATE,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let err = convert_binding(&binding).expect_err("invalid struct must error");
        assert!(
            matches!(err, BindingError::InvalidDatetimeValue { .. }),
            "expected InvalidDatetimeValue (22007), got {err:?}"
        );
    }

    #[test]
    fn convert_timestamp_as_date_invalid_fraction_takes_precedence_over_22008() {
        // fraction = 3_000_000_000 ns is out of the legal [0, 999_999_999]
        // range and is also non-zero (would trigger the 22008 narrowing
        // rule). The struct-validity error must win.
        let ts = sql::Timestamp {
            year: 2026,
            month: 4,
            day: 13,
            hour: 0,
            minute: 0,
            second: 0,
            fraction: 3_000_000_000,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            sql::SqlDataType::DATE,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let err = convert_binding(&binding).expect_err("invalid fraction must error");
        assert!(
            matches!(err, BindingError::InvalidDatetimeValue { .. }),
            "expected InvalidDatetimeValue (22007), got {err:?}"
        );
    }

    #[test]
    fn convert_timestamp_as_time_invalid_hour_takes_precedence_over_22008() {
        // hour=25 + non-zero fraction: 22007 must win over 22008.
        let ts = sql::Timestamp {
            year: 2026,
            month: 4,
            day: 13,
            hour: 25,
            minute: 0,
            second: 0,
            fraction: 500_000_000,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            sql::SqlDataType::TIME,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let err = convert_binding(&binding).expect_err("invalid struct must error");
        assert!(
            matches!(err, BindingError::InvalidDatetimeValue { .. }),
            "expected InvalidDatetimeValue (22007), got {err:?}"
        );
    }

    #[test]
    fn convert_timestamp_as_time_invalid_fraction_takes_precedence_over_22008() {
        // fraction out of [0, 999_999_999] AND non-zero: 22007 wins.
        let ts = sql::Timestamp {
            year: 2026,
            month: 4,
            day: 13,
            hour: 14,
            minute: 30,
            second: 45,
            fraction: 3_000_000_000,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            sql::SqlDataType::TIME,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let err = convert_binding(&binding).expect_err("invalid fraction must error");
        assert!(
            matches!(err, BindingError::InvalidDatetimeValue { .. }),
            "expected InvalidDatetimeValue (22007), got {err:?}"
        );
    }

    #[test]
    fn convert_timestamp_as_time_invalid_date_returns_22007() {
        // The date portion is going to be silently discarded, but it must
        // still be a syntactically valid Y/M/D — otherwise the *struct*
        // itself is malformed and we must surface 22007. month=13 with an
        // otherwise valid time would have silently succeeded before the
        // date-validation step was added to this arm.
        let ts = sql::Timestamp {
            year: 2024,
            month: 13,
            day: 1,
            hour: 14,
            minute: 30,
            second: 45,
            fraction: 0,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            sql::SqlDataType::TIME,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let err = convert_binding(&binding).expect_err("invalid date in TS → TIME must error");
        assert!(
            matches!(err, BindingError::InvalidDatetimeValue { .. }),
            "expected InvalidDatetimeValue (22007), got {err:?}"
        );
    }

    // The same-type pass-through arms (DATE→DATE, TIME→TIME, TIMESTAMP→
    // TIMESTAMP) must also report invalid struct fields as 22007 — not as
    // 07006 — so the SQLSTATE is consistent with the cross-temporal arms.

    #[test]
    fn convert_date_as_date_rejects_invalid_date() {
        let d = sql::Date {
            year: 2024,
            month: 13,
            day: 1,
        };
        let binding = make_binding(
            CDataType::TypeDate,
            sql::SqlDataType::DATE,
            &d as *const sql::Date as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let err = convert_binding(&binding).expect_err("invalid date must error");
        assert!(
            matches!(err, BindingError::InvalidDatetimeValue { .. }),
            "expected InvalidDatetimeValue, got {err:?}"
        );
    }

    #[test]
    fn convert_time_as_time_rejects_invalid_time() {
        let t = sql::Time {
            hour: 25,
            minute: 0,
            second: 0,
        };
        let binding = make_binding(
            CDataType::TypeTime,
            sql::SqlDataType::TIME,
            &t as *const sql::Time as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let err = convert_binding(&binding).expect_err("invalid time must error");
        assert!(
            matches!(err, BindingError::InvalidDatetimeValue { .. }),
            "expected InvalidDatetimeValue, got {err:?}"
        );
    }

    #[test]
    fn convert_timestamp_as_timestamp_rejects_invalid_date() {
        let ts = sql::Timestamp {
            year: 2024,
            month: 13,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            fraction: 0,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            sql::SqlDataType::TIMESTAMP,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let err = convert_binding(&binding).expect_err("invalid date must error");
        assert!(
            matches!(err, BindingError::InvalidDatetimeValue { .. }),
            "expected InvalidDatetimeValue, got {err:?}"
        );
    }

    #[test]
    fn convert_timestamp_as_timestamp_rejects_invalid_time() {
        let ts = sql::Timestamp {
            year: 2024,
            month: 1,
            day: 1,
            hour: 25,
            minute: 0,
            second: 0,
            fraction: 0,
        };
        let binding = make_binding(
            CDataType::TypeTimestamp,
            sql::SqlDataType::TIMESTAMP,
            &ts as *const sql::Timestamp as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let err = convert_binding(&binding).expect_err("invalid time must error");
        assert!(
            matches!(err, BindingError::InvalidDatetimeValue { .. }),
            "expected InvalidDatetimeValue, got {err:?}"
        );
    }

    #[test]
    fn convert_numeric_as_varchar() -> TestResult {
        let n = sql::Numeric {
            precision: 10,
            scale: 0,
            sign: 1,
            val: 42u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::VARCHAR,
            &n as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, "42".to_string());
        Ok(())
    }

    #[test]
    fn convert_negative_numeric_as_varchar() -> TestResult {
        let n = sql::Numeric {
            precision: 10,
            scale: 2,
            sign: 0,
            val: 12345u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::VARCHAR,
            &n as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, "-123.45".to_string());
        Ok(())
    }

    #[test]
    fn convert_numeric_small_scale_as_varchar() -> TestResult {
        let n = sql::Numeric {
            precision: 5,
            scale: 3,
            sign: 1,
            val: 5u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::VARCHAR,
            &n as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, "0.005".to_string());
        Ok(())
    }

    #[test]
    fn convert_numeric_negative_scale_as_varchar() -> TestResult {
        let n = sql::Numeric {
            precision: 10,
            scale: -2,
            sign: 1,
            val: 123u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::VARCHAR,
            &n as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, "12300".to_string());
        Ok(())
    }

    #[test]
    fn convert_numeric_negative_scale_negative_value_as_varchar() -> TestResult {
        let n = sql::Numeric {
            precision: 10,
            scale: -3,
            sign: 0,
            val: 5u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::VARCHAR,
            &n as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, "-5000".to_string());
        Ok(())
    }

    #[test]
    fn convert_binary_as_varchar() -> TestResult {
        let val: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
        let mut ind: sql::Len = 4;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::VARCHAR,
            val.as_ptr() as sql::Pointer,
            4,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, "deadbeef".to_string());
        Ok(())
    }

    // -- format_numeric_value tests -------------------------------------------

    #[test]
    fn format_numeric_value_no_scale() {
        assert_eq!(format_numeric_value(42, 0), "42");
        assert_eq!(format_numeric_value(-42, 0), "-42");
        assert_eq!(format_numeric_value(0, 0), "0");
    }

    #[test]
    fn format_numeric_value_positive_scale() {
        assert_eq!(format_numeric_value(12345, 2), "123.45");
        assert_eq!(format_numeric_value(-12345, 2), "-123.45");
        assert_eq!(format_numeric_value(5, 3), "0.005");
    }

    #[test]
    fn format_numeric_value_negative_scale() {
        assert_eq!(format_numeric_value(42, -2), "4200");
        assert_eq!(format_numeric_value(-5, -3), "-5000");
    }

    #[test]
    fn format_numeric_value_negative_scale_large_value() {
        let large = i128::MAX / 2;
        let result = format_numeric_value(large, -1);
        assert_eq!(result, format!("{}0", large));
    }

    #[test]
    fn format_numeric_value_negative_scale_i8_min() {
        let result = format_numeric_value(1, i8::MIN);
        assert!(result.starts_with('1'));
        assert_eq!(result.len(), 129); // "1" + 128 zeros
    }

    // -- read_numeric_struct tests --------------------------------------------

    #[test]
    fn read_numeric_struct_positive() {
        let ns = sql::Numeric {
            precision: 10,
            scale: 0,
            sign: 1,
            val: 42u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::INTEGER,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (value, scale) = read_numeric_struct(&binding).unwrap();
        assert_eq!(value, 42);
        assert_eq!(scale, 0);
    }

    #[test]
    fn read_numeric_struct_negative() {
        let ns = sql::Numeric {
            precision: 10,
            scale: 0,
            sign: 0,
            val: 99u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::INTEGER,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (value, scale) = read_numeric_struct(&binding).unwrap();
        assert_eq!(value, -99);
        assert_eq!(scale, 0);
    }

    #[test]
    fn read_numeric_struct_with_scale() {
        let ns = sql::Numeric {
            precision: 10,
            scale: 3,
            val: 12345u128.to_le_bytes(),
            sign: 1,
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::DECIMAL,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (value, scale) = read_numeric_struct(&binding).unwrap();
        assert_eq!(value, 12345);
        assert_eq!(scale, 3);
    }

    #[test]
    fn read_numeric_struct_overflow_positive() {
        let ns = sql::Numeric {
            precision: 38,
            scale: 0,
            sign: 1,
            val: u128::MAX.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::INTEGER,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(read_numeric_struct(&binding).is_err());
    }

    #[test]
    fn read_numeric_struct_negative_min() {
        let magnitude = (i128::MAX as u128) + 1;
        let ns = sql::Numeric {
            precision: 38,
            scale: 0,
            sign: 0,
            val: magnitude.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::INTEGER,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (value, _) = read_numeric_struct(&binding).unwrap();
        assert_eq!(value, i128::MIN);
    }

    // -- cross-type numeric conversion tests ----------------------------------

    #[test]
    fn convert_float_as_integer() -> TestResult {
        let val: f32 = 42.0;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::INTEGER,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "42".to_string());
        Ok(())
    }

    #[test]
    fn convert_float_nan_as_integer_fails() {
        let val: f32 = f32::NAN;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::INTEGER,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::NumericMagnitudeOverflow { .. })
        ));
    }

    #[test]
    fn convert_double_infinity_as_integer_fails() {
        let val: f64 = f64::INFINITY;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::INTEGER,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::NumericMagnitudeOverflow { .. })
        ));
    }

    #[test]
    fn convert_double_neg_infinity_as_integer_fails() {
        let val: f64 = f64::NEG_INFINITY;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::INTEGER,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::NumericMagnitudeOverflow { .. })
        ));
    }

    #[test]
    fn convert_double_as_integer() -> TestResult {
        let val: f64 = -123.0;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::INTEGER,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "-123".to_string());
        Ok(())
    }

    #[test]
    fn convert_double_truncates_fraction_for_integer() -> TestResult {
        let val: f64 = 42.99;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::INTEGER,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "42".to_string());
        Ok(())
    }

    #[test]
    fn convert_bit_as_integer() -> TestResult {
        let val: u8 = 1;
        let binding = make_binding(
            CDataType::Bit,
            sql::SqlDataType::INTEGER,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "1".to_string());
        Ok(())
    }

    #[test]
    fn convert_numeric_as_integer() -> TestResult {
        let ns = sql::Numeric {
            precision: 10,
            scale: 0,
            sign: 1,
            val: 999u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::INTEGER,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "999".to_string());
        Ok(())
    }

    #[test]
    fn convert_numeric_with_scale_as_integer() -> TestResult {
        let ns = sql::Numeric {
            precision: 10,
            scale: 2,
            sign: 1,
            val: 4299u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::INTEGER,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "42".to_string());
        Ok(())
    }

    #[test]
    fn convert_numeric_extreme_negative_scale_as_integer_fails() {
        let ns = sql::Numeric {
            precision: 38,
            scale: i8::MIN,
            sign: 1,
            val: 1u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::INTEGER,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(convert_binding(&binding).is_err());
    }

    #[test]
    fn convert_numeric_large_positive_scale_as_integer_fails() {
        let ns = sql::Numeric {
            precision: 38,
            scale: 100,
            sign: 1,
            val: 42u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::INTEGER,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(convert_binding(&binding).is_err());
    }

    #[test]
    fn convert_default_as_real() -> TestResult {
        let val: f64 = 4.25;
        let binding = make_binding(
            CDataType::Default,
            sql::SqlDataType::DOUBLE,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, "4.25".to_string());
        Ok(())
    }

    #[test]
    fn convert_slong_as_real() -> TestResult {
        let val: i32 = 42;
        let binding = make_binding(
            CDataType::SLong,
            sql::SqlDataType::DOUBLE,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, "42".to_string());
        Ok(())
    }

    #[test]
    fn convert_sbigint_as_real() -> TestResult {
        let val: i64 = 1_000_000;
        let binding = make_binding(
            CDataType::SBigInt,
            sql::SqlDataType::DOUBLE,
            &val as *const i64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, "1000000".to_string());
        Ok(())
    }

    #[test]
    fn convert_bit_as_real() -> TestResult {
        let val: u8 = 1;
        let binding = make_binding(
            CDataType::Bit,
            sql::SqlDataType::DOUBLE,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, "1".to_string());
        Ok(())
    }

    #[test]
    fn convert_numeric_as_real() -> TestResult {
        let ns = sql::Numeric {
            precision: 10,
            scale: 2,
            sign: 1,
            val: 314u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::DOUBLE,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, "3.14".to_string());
        Ok(())
    }

    #[test]
    fn convert_default_as_boolean_true() -> TestResult {
        let val: u8 = 1;
        let binding = make_binding(
            CDataType::Default,
            sql::SqlDataType::EXT_BIT,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_default_as_boolean_false() -> TestResult {
        let val: u8 = 0;
        let binding = make_binding(
            CDataType::Default,
            sql::SqlDataType::EXT_BIT,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_slong_as_boolean() -> TestResult {
        let val: i32 = 1;
        let binding = make_binding(
            CDataType::SLong,
            sql::SqlDataType::EXT_BIT,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_slong_zero_as_boolean() -> TestResult {
        let val: i32 = 0;
        let binding = make_binding(
            CDataType::SLong,
            sql::SqlDataType::EXT_BIT,
            &val as *const i32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_float_as_boolean() -> TestResult {
        let val: f32 = 1.0;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::EXT_BIT,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_float_nan_as_boolean_fails() {
        let val: f32 = f32::NAN;
        let binding = make_binding(
            CDataType::Float,
            sql::SqlDataType::EXT_BIT,
            &val as *const f32 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidBooleanValue { .. })
        ));
    }

    #[test]
    fn convert_double_infinity_as_boolean_fails() {
        let val: f64 = f64::INFINITY;
        let binding = make_binding(
            CDataType::Double,
            sql::SqlDataType::EXT_BIT,
            &val as *const f64 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidBooleanValue { .. })
        ));
    }

    #[test]
    fn convert_numeric_as_boolean() -> TestResult {
        let ns = sql::Numeric {
            precision: 10,
            scale: 0,
            sign: 1,
            val: 1u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::EXT_BIT,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_bit_as_decimal() -> TestResult {
        let val: u8 = 1;
        let binding = make_binding(
            CDataType::Bit,
            sql::SqlDataType::DECIMAL,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "1".to_string());
        Ok(())
    }

    #[test]
    fn convert_numeric_as_decimal() -> TestResult {
        let ns = sql::Numeric {
            precision: 10,
            scale: 3,
            sign: 1,
            val: 12345678u128.to_le_bytes(),
        };
        let binding = make_binding(
            CDataType::Numeric,
            sql::SqlDataType::DECIMAL,
            &ns as *const sql::Numeric as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "12345.678".to_string());
        Ok(())
    }

    #[test]
    fn convert_bit_zero_as_varchar() -> TestResult {
        let val: u8 = 0;
        let binding = make_binding(
            CDataType::Bit,
            sql::SqlDataType::VARCHAR,
            &val as *const u8 as sql::Pointer,
            0,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Text);
        assert_eq!(v, "0".to_string());
        Ok(())
    }

    #[test]
    fn convert_char_as_boolean_true() -> TestResult {
        let val = b"1\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "true".to_string());
        Ok(())
    }

    #[test]
    fn convert_char_as_boolean_false() -> TestResult {
        let val = b"0\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Boolean);
        assert_eq!(v, "false".to_string());
        Ok(())
    }

    #[test]
    fn convert_char_nan_as_boolean_fails() {
        let val = b"NaN\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidBooleanValue { .. })
        ));
    }

    #[test]
    fn convert_char_infinity_as_boolean_fails() {
        let val = b"inf\0";
        let binding = make_binding(
            CDataType::Char,
            sql::SqlDataType::EXT_BIT,
            val.as_ptr() as sql::Pointer,
            sql::NTS,
            std::ptr::null_mut(),
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::InvalidBooleanValue { .. })
        ));
    }

    // =========================================================================
    // SQL_C_BINARY → SQL_INTEGER / SQL_BIGINT / SQL_SMALLINT / SQL_TINYINT
    // =========================================================================

    #[test]
    fn convert_binary_4bytes_to_integer() -> TestResult {
        let val: i32 = 42;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::INTEGER,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "42".to_string());
        Ok(())
    }

    #[test]
    fn convert_binary_8bytes_to_bigint() -> TestResult {
        let val: i64 = 9_999_999_999;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::EXT_BIG_INT,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "9999999999".to_string());
        Ok(())
    }

    #[test]
    fn convert_binary_2bytes_to_smallint() -> TestResult {
        let val: i16 = -7;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::SMALLINT,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "-7".to_string());
        Ok(())
    }

    #[test]
    fn convert_binary_1byte_to_tinyint() -> TestResult {
        let val: i8 = 127;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::EXT_TINY_INT,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "127".to_string());
        Ok(())
    }

    #[test]
    fn convert_binary_wrong_size_to_integer_fails() {
        let bytes: [u8; 3] = [1, 2, 3];
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::INTEGER,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::BindingNumericOutOfRange { .. })
        ));
    }

    #[test]
    fn convert_binary_4bytes_to_bigint_fails() {
        let val: i32 = 42;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::EXT_BIG_INT,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::BindingNumericOutOfRange { .. })
        ));
    }

    #[test]
    fn convert_binary_8bytes_to_real_fails() {
        let val: f64 = 3.125;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::REAL,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::BindingNumericOutOfRange { .. })
        ));
    }

    // =========================================================================
    // SQL_C_BINARY → SQL_FLOAT / SQL_DOUBLE / SQL_REAL
    // =========================================================================

    #[test]
    fn convert_binary_8bytes_to_double() -> TestResult {
        let val: f64 = 3.125;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::DOUBLE,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, "3.125".to_string());
        Ok(())
    }

    #[test]
    fn convert_binary_4bytes_to_real() -> TestResult {
        let val: f32 = 2.5;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::REAL,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        assert_eq!(v, "2.5".to_string());
        Ok(())
    }

    // Per MS ODBC "C to SQL: Binary" spec, SQL_C_BINARY -> SQL_REAL/DOUBLE is
    // specified to do a length-equals check only and then pass the bytes
    // through. NaN and +/-Infinity are valid IEEE-754 values and Snowflake
    // FLOAT columns accept them, so the driver forwards them to the server
    // rather than rejecting client-side. These tests pin that behavior.

    #[test]
    fn convert_binary_nan_to_double_forwards_to_server() -> TestResult {
        let val: f64 = f64::NAN;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::DOUBLE,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        // Rust's Display for f64::NAN is the literal "NaN" — the server-side
        // JSON binding parser accepts the same literal for FLOAT targets.
        assert_eq!(v, "NaN".to_string());
        Ok(())
    }

    #[test]
    fn convert_binary_infinity_to_real_forwards_to_server() -> TestResult {
        let val: f32 = f32::INFINITY;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::REAL,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        // The driver's `SnowflakeReal::write_json` maps non-finite floats to
        // the literals Snowflake's JSON bind parser accepts: "Infinity" /
        // "-Infinity" / "NaN". Rust's `Display` for f32::INFINITY is the
        // short form "inf", which the server rejects.
        assert_eq!(v, "Infinity".to_string());
        Ok(())
    }

    #[test]
    fn convert_binary_negative_infinity_to_real_forwards_to_server() -> TestResult {
        let val: f32 = f32::NEG_INFINITY;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::REAL,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Real);
        // Same rationale as the +Infinity case: `SnowflakeReal::write_json`
        // emits the full "-Infinity" literal Snowflake's JSON bind parser
        // accepts, not Rust's short "-inf" form.
        assert_eq!(v, "-Infinity".to_string());
        Ok(())
    }

    #[test]
    fn convert_binary_wrong_size_to_double_fails() {
        let bytes: [u8; 3] = [1, 2, 3];
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::DOUBLE,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::BindingNumericOutOfRange { .. })
        ));
    }

    // =========================================================================
    // SQL_C_BINARY → SQL_DECIMAL / SQL_NUMERIC (via SnowflakeDecimal)
    // =========================================================================

    #[test]
    fn convert_binary_numeric_struct_to_decimal() -> TestResult {
        let numeric = sql::Numeric {
            precision: 10,
            scale: 2,
            sign: 1,
            val: {
                let mut v = [0u8; 16];
                let bytes = 12345u128.to_le_bytes();
                v.copy_from_slice(&bytes);
                v
            },
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &numeric as *const _ as *const u8,
                mem::size_of::<sql::Numeric>(),
            )
        };
        let mut ind: sql::Len = mem::size_of::<sql::Numeric>() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::DECIMAL,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "123.45".to_string());
        Ok(())
    }

    #[test]
    fn convert_binary_wrong_size_to_decimal_fails() {
        let bytes: [u8; 10] = [0; 10];
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::DECIMAL,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::BindingNumericOutOfRange { .. })
        ));
    }

    // =========================================================================
    // SQL_C_BINARY → SQL_DATE
    // =========================================================================

    #[test]
    fn convert_binary_to_date() -> TestResult {
        let date = sql::Date {
            year: 2025,
            month: 3,
            day: 26,
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(&date as *const _ as *const u8, mem::size_of::<sql::Date>())
        };
        let mut ind: sql::Len = mem::size_of::<sql::Date>() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::DATE,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Date);
        let expected_millis = (chrono::NaiveDate::from_ymd_opt(2025, 3, 26).unwrap()
            - chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
        .num_days()
            * 86_400_000;
        assert_eq!(v, expected_millis.to_string());
        Ok(())
    }

    #[test]
    fn convert_binary_wrong_size_to_date_fails() {
        let bytes: [u8; 4] = [0; 4];
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::DATE,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::BindingNumericOutOfRange { .. })
        ));
    }

    // =========================================================================
    // SQL_C_BINARY → SQL_TIME
    // =========================================================================

    #[test]
    fn convert_binary_to_time() -> TestResult {
        let time = sql::Time {
            hour: 14,
            minute: 30,
            second: 45,
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(&time as *const _ as *const u8, mem::size_of::<sql::Time>())
        };
        let mut ind: sql::Len = mem::size_of::<sql::Time>() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::TIME,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Time);
        let nanos = 14 * 3600 * 1_000_000_000i64 + 30 * 60 * 1_000_000_000 + 45 * 1_000_000_000;
        assert_eq!(v, nanos.to_string());
        Ok(())
    }

    #[test]
    fn convert_binary_wrong_size_to_time_fails() {
        let bytes: [u8; 4] = [0; 4];
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::TIME,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::BindingNumericOutOfRange { .. })
        ));
    }

    // =========================================================================
    // SQL_C_BINARY → SQL_TIMESTAMP
    // =========================================================================

    #[test]
    fn convert_binary_to_timestamp() -> TestResult {
        let ts = sql::Timestamp {
            year: 2025,
            month: 3,
            day: 26,
            hour: 14,
            minute: 30,
            second: 45,
            fraction: 0,
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &ts as *const _ as *const u8,
                mem::size_of::<sql::Timestamp>(),
            )
        };
        let mut ind: sql::Len = mem::size_of::<sql::Timestamp>() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::TIMESTAMP,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::TimestampNtz);
        let expected_nanos = chrono::NaiveDate::from_ymd_opt(2025, 3, 26)
            .unwrap()
            .and_hms_opt(14, 30, 45)
            .unwrap()
            .and_utc()
            .timestamp_nanos_opt()
            .unwrap();
        assert_eq!(v, expected_nanos.to_string());
        Ok(())
    }

    #[test]
    fn convert_binary_wrong_size_to_timestamp_fails() {
        let bytes: [u8; 8] = [0; 8];
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::TIMESTAMP,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        assert!(matches!(
            convert_binding(&binding),
            Err(BindingError::BindingNumericOutOfRange { .. })
        ));
    }

    #[test]
    fn convert_binary_negative_i32_to_integer() -> TestResult {
        let val: i32 = -100;
        let bytes = val.to_ne_bytes();
        let mut ind: sql::Len = bytes.len() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::INTEGER,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::Fixed);
        assert_eq!(v, "-100".to_string());
        Ok(())
    }

    #[test]
    fn convert_binary_timestamp_with_fraction() -> TestResult {
        let ts = sql::Timestamp {
            year: 2025,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            fraction: 500_000_000,
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &ts as *const _ as *const u8,
                mem::size_of::<sql::Timestamp>(),
            )
        };
        let mut ind: sql::Len = mem::size_of::<sql::Timestamp>() as sql::Len;
        let binding = make_binding(
            CDataType::Binary,
            sql::SqlDataType::TIMESTAMP,
            bytes.as_ptr() as sql::Pointer,
            bytes.len() as sql::Len,
            &mut ind,
        );
        let (ty, v) = convert_binding(&binding)?;
        assert_eq!(ty, SnowflakeLogicalType::TimestampNtz);
        let expected_nanos = chrono::NaiveDate::from_ymd_opt(2025, 1, 1)
            .unwrap()
            .and_hms_nano_opt(0, 0, 0, 500_000_000)
            .unwrap()
            .and_utc()
            .timestamp_nanos_opt()
            .unwrap();
        assert_eq!(v, expected_nanos.to_string());
        Ok(())
    }

    fn csv_cell(s: &str) -> String {
        let mut buf = String::new();
        append_escaped_csv_cell(&mut buf, s);
        buf
    }

    #[test]
    fn csv_cell_always_quoted_even_without_specials() {
        // All non-empty strings are always wrapped in quotes so that
        // multi-byte UTF-8 characters are sent through Snowflake's
        // quoted-field path where ESCAPE is NONE by default.
        assert_eq!(csv_cell("hello"), "\"hello\"");
        assert_eq!(csv_cell("123"), "\"123\"");
        assert_eq!(csv_cell("2024-01-01"), "\"2024-01-01\"");
    }

    #[test]
    fn csv_cell_empty_string_is_quoted_pair_to_distinguish_from_null() {
        // An empty *unquoted* CSV field is consumed as NULL on the server side;
        // empty strings must round-trip as a literal `""` to preserve that
        // distinction. This matches JDBC's `BindUploader.bindValueToCSV` rule.
        assert_eq!(csv_cell(""), "\"\"");
    }

    #[test]
    fn csv_cell_quotes_when_contains_comma() {
        assert_eq!(csv_cell("a,b"), "\"a,b\"");
    }

    #[test]
    fn csv_cell_doubles_inner_quotes() {
        assert_eq!(csv_cell("she said \"hi\""), "\"she said \"\"hi\"\"\"");
    }

    #[test]
    fn csv_cell_quotes_when_contains_newline_or_cr() {
        assert_eq!(csv_cell("line1\nline2"), "\"line1\nline2\"");
        assert_eq!(csv_cell("line1\rline2"), "\"line1\rline2\"");
        assert_eq!(csv_cell("a\r\nb"), "\"a\r\nb\"");
    }

    #[test]
    fn csv_cell_quotes_when_contains_only_quote() {
        assert_eq!(csv_cell("\""), "\"\"\"\"");
    }

    #[test]
    fn csv_cell_quotes_when_contains_backslash() {
        assert_eq!(csv_cell("a\\b"), "\"a\\b\"");
        assert_eq!(csv_cell("\\"), "\"\\\"");
    }

    #[test]
    fn csv_cell_utf8_multibyte_is_quoted() {
        // Multi-byte UTF-8 (e.g. CJK) must be quoted so Snowflake's
        // byte-level escape scanner (ESCAPE_UNENCLOSED_FIELD='\\') cannot
        // misinterpret high bytes as escape sequences, which would cause NULL
        // to be stored instead of the intended string.
        let s = "\u{65e5}\u{672c}\u{8a9e}6"; // 日本語6
        assert_eq!(csv_cell(s), "\"日本語6\"");
    }

    #[allow(clippy::type_complexity)]
    fn build_type_matrix_row() -> (ApdDescriptor, IpdDescriptor, u16, Box<TypeMatrixStorage>) {
        let mut storage = Box::new(TypeMatrixStorage::default());
        storage.i32_v = 42;
        storage.i16_v = -7;
        storage.i64_v = 9_000_000_000;
        storage.i8_v = 5;
        storage.f32_v = 1.5;
        storage.f64_v = 3.25;
        storage.decimal = *b"123.456";
        storage.bit_v = 1;
        storage.binary = [0xDE, 0xAD];
        storage.ascii = *b"hello";
        // 日本語 — 3 chars × 3 bytes UTF-8 = 9 bytes
        storage.multibyte[..9].copy_from_slice("\u{65e5}\u{672c}\u{8a9e}".as_bytes());
        storage.multibyte_len = 9;
        storage.date = sql::Date {
            year: 2024,
            month: 1,
            day: 1,
        };
        storage.time = sql::Time {
            hour: 12,
            minute: 30,
            second: 45,
        };
        storage.timestamp = sql::Timestamp {
            year: 2024,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            fraction: 0,
        };
        storage.decimal_ind = storage.decimal.len() as sql::Len;
        storage.binary_ind = storage.binary.len() as sql::Len;
        storage.ascii_ind = storage.ascii.len() as sql::Len;
        storage.multibyte_ind = storage.multibyte_len as sql::Len;
        storage.null_ind = sql::NULL_DATA;

        let cols: Vec<(
            u16,
            CDataType,
            sql::SqlDataType,
            sql::Pointer,
            sql::Len,
            *mut sql::Len,
        )> = vec![
            (
                1,
                CDataType::Long,
                sql::SqlDataType::INTEGER,
                &raw const storage.i32_v as sql::Pointer,
                mem::size_of::<i32>() as sql::Len,
                std::ptr::null_mut(),
            ),
            (
                2,
                CDataType::Short,
                sql::SqlDataType::SMALLINT,
                &raw const storage.i16_v as sql::Pointer,
                mem::size_of::<i16>() as sql::Len,
                std::ptr::null_mut(),
            ),
            (
                3,
                CDataType::SBigInt,
                sql::SqlDataType::EXT_BIG_INT,
                &raw const storage.i64_v as sql::Pointer,
                mem::size_of::<i64>() as sql::Len,
                std::ptr::null_mut(),
            ),
            (
                4,
                CDataType::STinyInt,
                sql::SqlDataType::EXT_TINY_INT,
                &raw const storage.i8_v as sql::Pointer,
                mem::size_of::<i8>() as sql::Len,
                std::ptr::null_mut(),
            ),
            (
                5,
                CDataType::Float,
                sql::SqlDataType::REAL,
                &raw const storage.f32_v as sql::Pointer,
                mem::size_of::<f32>() as sql::Len,
                std::ptr::null_mut(),
            ),
            (
                6,
                CDataType::Double,
                sql::SqlDataType::DOUBLE,
                &raw const storage.f64_v as sql::Pointer,
                mem::size_of::<f64>() as sql::Len,
                std::ptr::null_mut(),
            ),
            (
                7,
                CDataType::Char,
                sql::SqlDataType::DECIMAL,
                storage.decimal.as_ptr() as sql::Pointer,
                storage.decimal.len() as sql::Len,
                &raw mut storage.decimal_ind,
            ),
            (
                8,
                CDataType::Bit,
                sql::SqlDataType::EXT_BIT,
                &raw const storage.bit_v as sql::Pointer,
                mem::size_of::<u8>() as sql::Len,
                std::ptr::null_mut(),
            ),
            (
                9,
                CDataType::Binary,
                sql::SqlDataType::EXT_BINARY,
                storage.binary.as_ptr() as sql::Pointer,
                storage.binary.len() as sql::Len,
                &raw mut storage.binary_ind,
            ),
            (
                10,
                CDataType::Char,
                sql::SqlDataType::VARCHAR,
                storage.ascii.as_ptr() as sql::Pointer,
                storage.ascii.len() as sql::Len,
                &raw mut storage.ascii_ind,
            ),
            (
                11,
                CDataType::Char,
                sql::SqlDataType::VARCHAR,
                storage.multibyte.as_ptr() as sql::Pointer,
                storage.multibyte.len() as sql::Len,
                &raw mut storage.multibyte_ind,
            ),
            (
                12,
                CDataType::TypeDate,
                sql::SqlDataType::DATE,
                &raw const storage.date as sql::Pointer,
                mem::size_of::<sql::Date>() as sql::Len,
                std::ptr::null_mut(),
            ),
            (
                13,
                CDataType::TypeTime,
                sql::SqlDataType::TIME,
                &raw const storage.time as sql::Pointer,
                mem::size_of::<sql::Time>() as sql::Len,
                std::ptr::null_mut(),
            ),
            (
                14,
                CDataType::TypeTimestamp,
                sql::SqlDataType::TIMESTAMP,
                &raw const storage.timestamp as sql::Pointer,
                mem::size_of::<sql::Timestamp>() as sql::Len,
                std::ptr::null_mut(),
            ),
            (
                15,
                CDataType::Long,
                sql::SqlDataType::INTEGER,
                std::ptr::null_mut(),
                0,
                &raw mut storage.null_ind,
            ),
        ];
        let (apd, ipd) = make_descriptors(cols);
        (apd, ipd, 15, storage)
    }

    #[derive(Default)]
    struct TypeMatrixStorage {
        i32_v: i32,
        i16_v: i16,
        i64_v: i64,
        i8_v: i8,
        f32_v: f32,
        f64_v: f64,
        decimal: [u8; 7],
        bit_v: u8,
        binary: [u8; 2],
        ascii: [u8; 5],
        multibyte: [u8; 16],
        multibyte_len: usize,
        date: sql::Date,
        time: sql::Time,
        timestamp: sql::Timestamp,
        decimal_ind: sql::Len,
        binary_ind: sql::Len,
        ascii_ind: sql::Len,
        multibyte_ind: sql::Len,
        null_ind: sql::Len,
    }

    struct VarcharHazardStorage {
        ids: [i32; 7],
        txt: [u8; 16 * 7],
        txt_inds: [sql::Len; 7],
    }

    fn build_varchar_hazard_rows() -> (ApdDescriptor, IpdDescriptor, u16, Box<VarcharHazardStorage>)
    {
        const SLOT: usize = 16;
        let mut storage = Box::new(VarcharHazardStorage {
            ids: [0, 1, 2, 3, 4, 5, 6],
            txt: [0; SLOT * 7],
            txt_inds: [0; 7],
        });

        let payloads: [Option<&[u8]>; 7] = [
            Some(b"val,0"),
            Some(b"say\"1\""),
            Some(b"a\nb"),
            Some(b"C:\\dir\\3"),
            Some(b""),
            None, // SQL NULL — indicator override below
            Some("\u{65e5}\u{672c}\u{8a9e}".as_bytes()),
        ];
        for (i, payload) in payloads.iter().enumerate() {
            match payload {
                Some(bytes) => {
                    storage.txt[i * SLOT..i * SLOT + bytes.len()].copy_from_slice(bytes);
                    storage.txt_inds[i] = bytes.len() as sql::Len;
                }
                None => {
                    storage.txt_inds[i] = sql::NULL_DATA;
                }
            }
        }

        let cols: Vec<(
            u16,
            CDataType,
            sql::SqlDataType,
            sql::Pointer,
            sql::Len,
            *mut sql::Len,
        )> = vec![
            (
                1,
                CDataType::Long,
                sql::SqlDataType::INTEGER,
                storage.ids.as_ptr() as sql::Pointer,
                mem::size_of::<i32>() as sql::Len,
                std::ptr::null_mut(),
            ),
            (
                2,
                CDataType::Char,
                sql::SqlDataType::VARCHAR,
                storage.txt.as_ptr() as sql::Pointer,
                SLOT as sql::Len,
                storage.txt_inds.as_mut_ptr(),
            ),
        ];
        let (mut apd, ipd) = make_descriptors(cols);
        apd.array_size = 7;
        (apd, ipd, 2, storage)
    }

    #[test]
    fn csv_bindings_match_reference_fixture() -> TestResult {
        let mut actual = String::new();
        {
            let (apd, ipd, n, _storage) = build_type_matrix_row();
            actual.push_str(&odbc_bindings_to_csv(&apd, &ipd, n)?);
        }
        {
            let (apd, ipd, n, _storage) = build_varchar_hazard_rows();
            actual.push_str(&odbc_bindings_to_csv(&apd, &ipd, n)?);
        }

        const REFERENCE: &[u8] = include_bytes!("testdata/large_bindings_csv_reference.csv");
        assert_eq!(
            actual.as_bytes(),
            REFERENCE,
            "CSV bindings must match the reference fixture byte-for-byte"
        );
        Ok(())
    }

    #[test]
    fn csv_two_int_columns_one_row() -> TestResult {
        let v1: i32 = 42;
        let v2: i32 = -7;
        let (apd, ipd) = make_descriptors(vec![
            (
                1,
                CDataType::Long,
                sql::SqlDataType::INTEGER,
                &v1 as *const i32 as sql::Pointer,
                mem::size_of::<i32>() as sql::Len,
                std::ptr::null_mut(),
            ),
            (
                2,
                CDataType::Long,
                sql::SqlDataType::INTEGER,
                &v2 as *const i32 as sql::Pointer,
                mem::size_of::<i32>() as sql::Len,
                std::ptr::null_mut(),
            ),
        ]);
        let csv = odbc_bindings_to_csv(&apd, &ipd, 2)?;
        assert_eq!(csv, "\"42\",\"-7\"\n");
        Ok(())
    }

    #[test]
    fn csv_varchar_with_specials_is_quoted() -> TestResult {
        let s = b"a, \"b\"\nc";
        let mut ind: sql::Len = s.len() as sql::Len;
        let (apd, ipd) = make_descriptors(vec![(
            1,
            CDataType::Char,
            sql::SqlDataType::VARCHAR,
            s.as_ptr() as sql::Pointer,
            s.len() as sql::Len,
            &mut ind,
        )]);
        let csv = odbc_bindings_to_csv(&apd, &ipd, 1)?;
        assert_eq!(csv, "\"a, \"\"b\"\"\nc\"\n");
        Ok(())
    }

    #[test]
    fn csv_null_indicator_yields_empty_field() -> TestResult {
        // Two columns: param 1 is NULL, param 2 is `7`. Expect the row to be
        // `,7\n` — leading empty field encodes SQL NULL.
        let v: i32 = 7;
        let mut null_ind: sql::Len = sql::NULL_DATA;
        let (apd, ipd) = make_descriptors(vec![
            (
                1,
                CDataType::Long,
                sql::SqlDataType::INTEGER,
                std::ptr::null_mut(),
                0,
                &mut null_ind,
            ),
            (
                2,
                CDataType::Long,
                sql::SqlDataType::INTEGER,
                &v as *const i32 as sql::Pointer,
                mem::size_of::<i32>() as sql::Len,
                std::ptr::null_mut(),
            ),
        ]);
        let csv = odbc_bindings_to_csv(&apd, &ipd, 2)?;
        assert_eq!(csv, ",\"7\"\n");
        Ok(())
    }

    #[test]
    fn csv_binary_is_lowercase_hex_bare() -> TestResult {
        let bytes: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
        let mut ind: sql::Len = 4;
        let (apd, ipd) = make_descriptors(vec![(
            1,
            CDataType::Binary,
            sql::SqlDataType::EXT_BINARY,
            bytes.as_ptr() as sql::Pointer,
            4,
            &mut ind,
        )]);
        let csv = odbc_bindings_to_csv(&apd, &ipd, 1)?;
        assert_eq!(csv, "\"deadbeef\"\n");
        Ok(())
    }

    #[test]
    fn csv_empty_string_field_is_quoted_pair_in_full_row() -> TestResult {
        // Empty string column followed by an integer column. We expect the
        // first cell to be `""` (so the server doesn't confuse it with NULL)
        // and the second to be the integer quoted.
        let empty: [u8; 0] = [];
        let mut empty_ind: sql::Len = 0;
        let v: i32 = 1;
        let (apd, ipd) = make_descriptors(vec![
            (
                1,
                CDataType::Char,
                sql::SqlDataType::VARCHAR,
                empty.as_ptr() as sql::Pointer,
                0,
                &mut empty_ind,
            ),
            (
                2,
                CDataType::Long,
                sql::SqlDataType::INTEGER,
                &v as *const i32 as sql::Pointer,
                mem::size_of::<i32>() as sql::Len,
                std::ptr::null_mut(),
            ),
        ]);
        let csv = odbc_bindings_to_csv(&apd, &ipd, 2)?;
        assert_eq!(csv, "\"\",\"1\"\n");
        Ok(())
    }

    #[test]
    fn csv_zero_params_returns_just_newline() -> TestResult {
        let (apd, ipd) = make_descriptors(vec![]);
        let csv = odbc_bindings_to_csv(&apd, &ipd, 0)?;
        // Zero columns produces just the row terminator. (Real code never
        // calls this path because `apply_parameter_bindings` short-circuits
        // when `effective_count == 0`, but the function itself stays total.)
        assert_eq!(csv, "\n");
        Ok(())
    }
}
