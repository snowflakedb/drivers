use arrow::array::{
    Array, ArrowNumericType, BinaryArray, BooleanArray, Decimal128Array, Float64Array, Int16Array,
    Int64Array, PrimitiveArray, StringArray, StringBuilder, StructArray,
};
use arrow::datatypes::{DataType, Date32Type, Field, Int32Type, Int64Type};
use chrono::NaiveTime;

use super::column_reader_util::{
    decimal_string, read_cell, scale_from_metadata, usize_from_metadata, widen,
};
use super::decfloat::{decfloat_field, format_decfloat, i128_from_big_endian_signed};
use super::js_cell::JsCell;
use super::time_format;
use crate::session_params::SessionParams;
use sf_types::ReadArrowType;
use std::borrow::Cow;
use std::sync::Arc;

/// Snowflake's maximum TIME fractional-second precision (`TIME(0)` ..
/// `TIME(9)`). A local naming choice, not an established convention —
/// `odbc`'s and `sf_core`'s equivalent arithmetic leave this bare.
const MAX_TIME_SCALE: u32 = 9;
/// Seconds in a day — the exclusive upper bound for a valid
/// `secs_since_midnight` component of the `secs * 10^scale + frac` TIME
/// encoding (see [`validate_time_range`]).
const SECONDS_PER_DAY: i64 = 86_400;

/// Per-column state a `TIME` decoder needs beyond the raw Arrow array:
/// the column's declared scale (0-9 fractional-second digits) and the
/// session's `TIME_OUTPUT_FORMAT` at the time the stream was built. A
/// named struct, not a positional tuple, so the two `Time*` variants below
/// can't have `scale`/`format` swapped at a construction or match site.
///
/// `format` is an `Arc<str>` clone of `SessionParams::time_format`, not an
/// owned `String` copy — cloning an `Arc` is a refcount bump, not a heap
/// allocation, and every TIME column in a batch shares the same format.
pub(crate) struct TimeMeta {
    scale: u32,
    format: Arc<str>,
}

/// Decodes one Arrow column into [`JsCell`]s, one cell at a time.
///
/// Use it in two steps:
/// - [`for_field`](Self::for_field) inspects the column's `logicalType`,
///   picks the matching decoder, and holds onto the array. Runs on a worker
///   thread, so anything proportional to the batch size belongs here.
/// - [`read`](Self::read) returns the [`JsCell`] for a given row (or
///   [`JsCell::Null`]). Runs on the Node.js main thread; the caller converts
///   the cell through [`napi::bindgen_prelude::ToNapiValue`].
pub(crate) enum ColumnReader {
    Boolean(BooleanArray),
    Binary(BinaryArray),
    Date(PrimitiveArray<Date32Type>),
    TimeI32(PrimitiveArray<Int32Type>, TimeMeta),
    TimeI64(PrimitiveArray<Int64Type>, TimeMeta),
    Variant(StringArray),
    Text(StringArray),
    FixedInt { array: Int64Array, scale: u32 },
    FixedDecimal { array: Decimal128Array, scale: u32 },
    Real(Float64Array),
    Decfloat(StringArray),
}

impl ColumnReader {
    // TODO: figure better error handling
    pub(crate) fn for_field(
        field: &Field,
        column: &dyn Array,
        session_params: &SessionParams,
    ) -> Result<Self, String> {
        match field.metadata().get("logicalType").map(String::as_str) {
            Some("TEXT") => {
                let array = column
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .cloned()
                    .ok_or_else(|| {
                        "Arrow column could not be downcast to StringArray".to_string()
                    })?;
                Ok(Self::Text(array))
            }
            Some("BOOLEAN") => {
                let array = column
                    .as_any()
                    .downcast_ref::<BooleanArray>()
                    .cloned()
                    .ok_or_else(|| {
                        "Arrow column could not be downcast to BooleanArray".to_string()
                    })?;
                Ok(Self::Boolean(array))
            }
            Some("BINARY") => {
                let array = column
                    .as_any()
                    .downcast_ref::<BinaryArray>()
                    .cloned()
                    .ok_or_else(|| {
                        "Arrow column could not be downcast to BinaryArray".to_string()
                    })?;
                Ok(Self::Binary(array))
            }
            Some("FIXED") => {
                let scale = scale_from_metadata(field)?;
                match column.data_type() {
                    DataType::Decimal128(_, _) => {
                        let array = column
                            .as_any()
                            .downcast_ref::<Decimal128Array>()
                            .cloned()
                            .ok_or_else(|| {
                                "Arrow column could not be downcast to Decimal128Array".to_string()
                            })?;
                        Ok(Self::FixedDecimal { array, scale })
                    }
                    DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64 => {
                        let array = widen(column, &DataType::Int64, "Int64Array")?;
                        Ok(Self::FixedInt { array, scale })
                    }
                    other => Err(format!(
                        "FIXED column {:?} has unsupported Arrow type {other}",
                        field.name()
                    )),
                }
            }
            Some("DATE") => {
                let array = column
                    .as_any()
                    .downcast_ref::<PrimitiveArray<Date32Type>>()
                    .cloned()
                    .ok_or_else(|| {
                        "Arrow column could not be downcast to Date32 array".to_string()
                    })?;
                Ok(Self::Date(array))
            }
            Some("TIME") => {
                let scale: u32 = field
                    .metadata()
                    .get("scale")
                    .ok_or_else(|| format!("column {:?} is missing scale metadata", field.name()))?
                    .parse()
                    .map_err(|e| {
                        format!(
                            "column {:?} has non-numeric scale metadata: {e}",
                            field.name()
                        )
                    })?;
                if scale > MAX_TIME_SCALE {
                    return Err(format!(
                        "column {:?} has TIME scale {scale} exceeding maximum of {MAX_TIME_SCALE}",
                        field.name()
                    ));
                }
                let meta = TimeMeta {
                    scale,
                    format: session_params.time_format.clone(),
                };
                match column.data_type() {
                    DataType::Int32 => {
                        let array = column
                            .as_any()
                            .downcast_ref::<PrimitiveArray<Int32Type>>()
                            .cloned()
                            .ok_or_else(|| {
                                "Arrow column could not be downcast to Int32 array".to_string()
                            })?;
                        validate_time_range(&array, scale, field.name())?;
                        Ok(Self::TimeI32(array, meta))
                    }
                    DataType::Int64 => {
                        let array = column
                            .as_any()
                            .downcast_ref::<PrimitiveArray<Int64Type>>()
                            .cloned()
                            .ok_or_else(|| {
                                "Arrow column could not be downcast to Int64 array".to_string()
                            })?;
                        validate_time_range(&array, scale, field.name())?;
                        Ok(Self::TimeI64(array, meta))
                    }
                    other => Err(format!(
                        "column {:?} has unsupported TIME physical type {other:?}",
                        field.name()
                    )),
                }
            }
            Some("VARIANT" | "OBJECT" | "ARRAY") => {
                let array = column
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .cloned()
                    .ok_or_else(|| {
                        "Arrow column could not be downcast to StringArray for semi-structured column"
                            .to_string()
                    })?;
                Ok(Self::Variant(array))
            }
            Some("REAL") => Ok(Self::Real(widen(
                column,
                &DataType::Float64,
                "Float64Array",
            )?)),
            // TODO: DECFLOAT iterates over rows in `for_field`, making its `read` arm a
            // plain lookup like every other variant. Worth revisiting whether the
            // for_field/read split should just collapse into one eager step for all.
            Some("DECFLOAT") => {
                let struct_array = column
                    .as_any()
                    .downcast_ref::<StructArray>()
                    .cloned()
                    .ok_or_else(|| {
                        "Arrow column could not be downcast to StructArray".to_string()
                    })?;
                let exponent: Int16Array = decfloat_field(&struct_array, "exponent")?;
                let significand: BinaryArray = decfloat_field(&struct_array, "significand")?;
                let precision = usize_from_metadata(field, "precision")?;

                let mut builder = StringBuilder::new();
                for row in 0..struct_array.len() {
                    if struct_array.is_null(row) {
                        builder.append_null();
                    } else {
                        let sig = i128_from_big_endian_signed(significand.value(row))
                            .map_err(|e| format!("DECFLOAT significand at row {row}: {e}"))?;
                        builder.append_value(format_decfloat(sig, exponent.value(row), precision));
                    }
                }
                Ok(Self::Decfloat(builder.finish()))
            }
            Some(logical_type) => Err(format!(
                "no decoder registered for logicalType {logical_type:?}"
            )),
            None => Err(format!(
                "column {:?} is missing logicalType metadata",
                field.name()
            )),
        }
    }

    pub(crate) fn read(&self, row_index: usize) -> JsCell<'_> {
        match self {
            Self::Boolean(array) => read_cell(array, row_index, || {
                let value = sf_types::SnowflakeBoolean
                    .read_arrow_type(array, row_index)
                    .unwrap_or_else(|_| {
                        unreachable!("non-null BooleanArray cell always decodes to a bool")
                    });
                JsCell::Bool(value)
            }),
            Self::Binary(array) => read_cell(array, row_index, || {
                let value = sf_types::SnowflakeBinary
                    .read_arrow_type(array, row_index)
                    .unwrap_or_else(|_| {
                        unreachable!("non-null BINARY cell always decodes to bytes")
                    });
                JsCell::Buffer(value)
            }),
            Self::FixedInt { array, scale } => read_cell(array, row_index, || {
                let mantissa = sf_types::SnowflakeFixed
                    .read_arrow_type(array, row_index)
                    .unwrap_or_else(|_| {
                        unreachable!(
                            "non-null integer FIXED cell always decodes to an i128 mantissa"
                        )
                    });
                JsCell::Str(Cow::Owned(decimal_string(mantissa, *scale)))
            }),
            Self::FixedDecimal { array, scale } => read_cell(array, row_index, || {
                let mantissa = sf_types::SnowflakeFixed
                    .read_arrow_type(array, row_index)
                    .unwrap_or_else(|_| {
                        unreachable!(
                            "non-null Decimal128 FIXED cell always decodes to an i128 mantissa"
                        )
                    });
                JsCell::Str(Cow::Owned(decimal_string(mantissa, *scale)))
            }),
            Self::Date(array) => read_cell(array, row_index, || {
                // The Arrow `Date32` → `NaiveDate` decode is shared with the
                // ODBC and Python front ends via `sf_types`; only the
                // JS-specific mapping to a midnight `NaiveDateTime` (what napi
                // renders as a JavaScript `Date`) stays here. `read_cell`
                // already excluded NULL, and a non-null `Date32` cell always
                // decodes, so the reader cannot error on this path.
                let date = sf_types::SnowflakeDate
                    .read_arrow_type(array, row_index)
                    .unwrap_or_else(|_| {
                        unreachable!("non-null Date32 cell always decodes to a NaiveDate")
                    });
                JsCell::Date(date.and_time(NaiveTime::MIN))
            }),
            Self::TimeI32(array, meta) => read_cell(array, row_index, || {
                JsCell::Str(Cow::Owned(render_time(array, row_index, meta)))
            }),
            Self::TimeI64(array, meta) => read_cell(array, row_index, || {
                JsCell::Str(Cow::Owned(render_time(array, row_index, meta)))
            }),
            Self::Variant(array) => read_cell(array, row_index, || {
                JsCell::Str(Cow::Borrowed(array.value(row_index)))
            }),
            Self::Text(array) => read_cell(array, row_index, || {
                // The Arrow `Utf8` → `&str` decode is shared with the ODBC and
                // Python front ends via `sf_types`; only the borrow into a
                // `JsCell` stays here. `read_cell` already excluded NULL, and a
                // non-null `Utf8` cell always decodes, so the reader cannot
                // error on this path.
                let value = sf_types::SnowflakeText
                    .read_arrow_type(array, row_index)
                    .unwrap_or_else(|_| {
                        unreachable!("non-null Utf8 cell always decodes to a &str")
                    });
                JsCell::Str(Cow::Borrowed(value))
            }),
            Self::Real(array) => read_cell(array, row_index, || {
                let value = sf_types::SnowflakeReal
                    .read_arrow_type(array, row_index)
                    .unwrap_or_else(|_| {
                        unreachable!("non-null Float64 cell always decodes to an f64")
                    });
                JsCell::Number(value)
            }),
            Self::Decfloat(array) => read_cell(array, row_index, || {
                JsCell::Str(Cow::Borrowed(array.value(row_index)))
            }),
        }
    }
}

/// Validates every non-null cell in a TIME column's raw array is a legal
/// `secs_since_midnight * 10^scale + frac` encoding — negative, and
/// `secs >= SECONDS_PER_DAY`, are both wire-corruption signals worth
/// rejecting before any row is decoded, not silently clamped.
///
/// Checks the whole array in one vectorized pass (the batch is already
/// fully materialized in memory by this point, so this costs nothing extra
/// on the happy path) and only falls back to a per-cell scan — to name the
/// specific offending row — when the aggregate check fails.
fn validate_time_range<T>(
    array: &PrimitiveArray<T>,
    scale: u32,
    column_name: &str,
) -> Result<(), String>
where
    T: ArrowNumericType,
    T::Native: Into<i64>,
{
    let divisor = 10i64.pow(scale);
    let max_raw = divisor * SECONDS_PER_DAY;

    let min_raw: i64 = arrow::compute::min(array).map(Into::into).unwrap_or(0);
    let max_raw_seen: i64 = arrow::compute::max(array).map(Into::into).unwrap_or(0);
    if min_raw >= 0 && max_raw_seen < max_raw {
        return Ok(());
    }

    for (row_index, value) in array.iter().enumerate() {
        let Some(raw) = value else { continue };
        let raw: i64 = raw.into();
        if raw < 0 || raw >= max_raw {
            return Err(format!(
                "column {column_name:?} row {row_index} has out-of-range TIME value {raw} \
                 (scale {scale}, expected 0..{max_raw})"
            ));
        }
    }
    // Unreachable in practice — the aggregate check above failed, so a
    // per-cell scan must find the offending row. Kept as a defensive error
    // rather than a panic in case `min`/`max` and `iter()` ever disagree.
    Err(format!(
        "column {column_name:?} failed TIME range validation (scale {scale}) but no \
         offending row could be located"
    ))
}

fn render_time<T>(array: &PrimitiveArray<T>, row_index: usize, meta: &TimeMeta) -> String
where
    T: ArrowNumericType,
    T::Native: Into<i64>,
{
    let time = sf_types::SnowflakeTime { scale: meta.scale }
        .read_arrow_type(array, row_index)
        .unwrap_or_else(|_| {
            unreachable!("non-null, range-validated TIME cell always decodes to a NaiveTime")
        });
    time_format::render(time, meta.scale, &meta.format)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{
        BinaryArray, BooleanArray, Decimal128Array, Float32Array, Float64Array, Int8Array,
        Int16Array, Int32Array, Int64Array, StringArray, StructArray,
    };
    use arrow::buffer::NullBuffer;
    use arrow::datatypes::{DataType, Field};
    use chrono::{NaiveDate, NaiveTime};
    use std::borrow::Cow;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn field(logical: &str, data_type: DataType, extra: &[(&str, &str)]) -> Field {
        let mut metadata = HashMap::new();
        metadata.insert("logicalType".to_string(), logical.to_string());
        for (key, value) in extra {
            metadata.insert((*key).to_string(), (*value).to_string());
        }
        Field::new("C", data_type, true).with_metadata(metadata)
    }

    fn session_params(time_format: &str) -> SessionParams {
        SessionParams {
            time_format: Arc::from(time_format),
        }
    }

    fn reader_with_format(field: &Field, column: &dyn Array, time_format: &str) -> ColumnReader {
        ColumnReader::for_field(field, column, &session_params(time_format)).unwrap()
    }

    fn reader(field: &Field, column: &dyn Array) -> ColumnReader {
        reader_with_format(field, column, "HH24:MI:SS")
    }

    // `ColumnReader` has no `Debug` impl (deliberately — see
    // code-review-design-discipline.md principle 5), so `Result::unwrap_err`
    // can't be used directly; extract the error by hand instead.
    fn expect_err(result: Result<ColumnReader, String>) -> String {
        match result {
            Err(e) => e,
            Ok(_) => panic!("expected an Err, got Ok"),
        }
    }

    fn str_cell(s: &str) -> JsCell<'_> {
        JsCell::Str(Cow::Borrowed(s))
    }

    #[test]
    fn boolean_reads_true_false_and_null() {
        let field = field("BOOLEAN", DataType::Boolean, &[]);
        let array = BooleanArray::from(vec![Some(true), Some(false), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader, ColumnReader::Boolean(_)),
            "BOOLEAN should route to the Boolean arm"
        );
        assert_eq!(reader.read(0), JsCell::Bool(true));
        assert_eq!(reader.read(1), JsCell::Bool(false));
        assert_eq!(reader.read(2), JsCell::Null);
    }

    #[test]
    fn binary_reads_bytes_and_null() {
        let field = field("BINARY", DataType::Binary, &[]);
        let array = BinaryArray::from(vec![Some(b"\xab\xcd".as_slice()), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader, ColumnReader::Binary(_)),
            "BINARY should route to the Binary arm"
        );
        assert_eq!(reader.read(0), JsCell::Buffer(&[0xab, 0xcd]));
        assert_eq!(reader.read(1), JsCell::Null);
    }

    #[test]
    fn date_reads_utc_midnight_and_null() {
        let date = NaiveDate::from_ymd_opt(2016, 1, 21).unwrap();
        let field = field("DATE", DataType::Date32, &[]);
        let array =
            PrimitiveArray::<Date32Type>::from(vec![Some(Date32Type::from_naive_date(date)), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader, ColumnReader::Date(_)),
            "DATE should route to the Date arm"
        );
        assert_eq!(reader.read(0), JsCell::Date(date.and_time(NaiveTime::MIN)));
        assert_eq!(reader.read(1), JsCell::Null);
    }

    fn time_field(scale: &str) -> Field {
        field("TIME", DataType::Int32, &[("scale", scale)])
    }

    #[test]
    fn time_i32_reads_formatted_string_and_null() {
        // 10:30:00.123 at scale 3: secs=37_800, frac=123.
        let field = time_field("3");
        let array = Int32Array::from(vec![Some(37_800_123), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader, ColumnReader::TimeI32(_, _)),
            "Int32 TIME should route to the TimeI32 arm"
        );
        assert_eq!(reader.read(0), str_cell("10:30:00"));
        assert_eq!(reader.read(1), JsCell::Null);
    }

    #[test]
    fn time_i64_reads_full_precision_string() {
        let field = field("TIME", DataType::Int64, &[("scale", "9")]);
        let raw: i64 = 37_800 * 1_000_000_000 + 123_456_789;
        let array = Int64Array::from(vec![Some(raw)]);
        let reader = reader_with_format(&field, &array, "HH24:MI:SS.FF9");
        assert!(
            matches!(reader, ColumnReader::TimeI64(_, _)),
            "Int64 TIME should route to the TimeI64 arm"
        );
        assert_eq!(reader.read(0), str_cell("10:30:00.123456789"));
    }

    #[test]
    fn for_field_rejects_negative_time_value() {
        let field = time_field("0");
        let array = Int32Array::from(vec![Some(-1)]);
        let err = expect_err(ColumnReader::for_field(
            &field,
            &array,
            &session_params("HH24:MI:SS"),
        ));
        assert!(
            err.contains("row 0"),
            "error should name the offending row, got: {err}"
        );
        assert!(
            err.contains("-1"),
            "error should include the raw value, got: {err}"
        );
    }

    #[test]
    fn for_field_rejects_seconds_out_of_range() {
        let field = time_field("0");
        // One past the last legal second-of-day (SECONDS_PER_DAY - 1).
        let array = Int32Array::from(vec![Some(SECONDS_PER_DAY as i32)]);
        let err = expect_err(ColumnReader::for_field(
            &field,
            &array,
            &session_params("HH24:MI:SS"),
        ));
        assert!(
            err.contains("row 0"),
            "error should name the offending row, got: {err}"
        );
        assert!(
            err.contains("86400"),
            "error should include the raw value, got: {err}"
        );
    }

    /// Distinguishes a real per-cell scan from a naive implementation that
    /// always reports row 0 once the aggregate min/max check fails.
    #[test]
    fn for_field_names_the_specific_offending_row_not_just_the_first() {
        let field = time_field("0");
        let array = Int32Array::from(vec![Some(0), Some(43_200), Some(-5)]);
        let err = expect_err(ColumnReader::for_field(
            &field,
            &array,
            &session_params("HH24:MI:SS"),
        ));
        assert!(
            err.contains("row 2"),
            "error should name row 2 specifically, got: {err}"
        );
    }

    #[test]
    fn for_field_rejects_scale_above_nine() {
        let field = time_field("10");
        let array = Int32Array::from(vec![Some(0)]);
        let err = expect_err(ColumnReader::for_field(
            &field,
            &array,
            &session_params("HH24:MI:SS"),
        ));
        assert!(
            err.contains("scale 10"),
            "error should name the invalid scale, got: {err}"
        );
    }

    #[test]
    fn for_field_rejects_missing_scale_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("logicalType".to_string(), "TIME".to_string());
        let field = Field::new("T", DataType::Int32, true).with_metadata(metadata);
        let array = Int32Array::from(vec![Some(0)]);
        let err = expect_err(ColumnReader::for_field(
            &field,
            &array,
            &session_params("HH24:MI:SS"),
        ));
        assert!(
            err.contains("missing scale metadata"),
            "error should name the missing scale metadata, got: {err}"
        );
    }

    /// `validate_time_range`'s aggregate check leans on
    /// `arrow::compute::min`/`max`, which both return `None` when every
    /// value in the array is null (the `unwrap_or(0)` fallback). An
    /// all-null column should not be spuriously rejected — 0/0 is trivially
    /// within range — and every row should still read back as `Null`.
    #[test]
    fn for_field_accepts_all_null_time_column_and_reads_null() {
        let field = time_field("3");
        let array = Int32Array::from(vec![None, None, None]);
        let reader = reader(&field, &array);
        for row_index in 0..3 {
            assert_eq!(
                reader.read(row_index),
                JsCell::Null,
                "row {row_index} of an all-null TIME column should read as Null"
            );
        }
    }

    #[test]
    fn for_field_rejects_unsupported_physical_type_for_time() {
        let field = time_field("0");
        let array = BooleanArray::from(vec![true]);
        let err = expect_err(ColumnReader::for_field(
            &field,
            &array,
            &session_params("HH24:MI:SS"),
        ));
        assert!(
            err.contains("unsupported TIME physical type"),
            "error should name the unsupported type, got: {err}"
        );
    }

    #[test]
    fn text_reads_string_and_null() {
        let field = field("TEXT", DataType::Utf8, &[]);
        let array = StringArray::from(vec![Some("hello"), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader, ColumnReader::Text(_)),
            "TEXT should route to the Text arm"
        );
        assert_eq!(reader.read(0), str_cell("hello"));
        assert_eq!(reader.read(1), JsCell::Null);
    }

    #[test]
    fn variant_reads_json_text_and_null() {
        let field = field("VARIANT", DataType::Utf8, &[]);
        let array = StringArray::from(vec![Some("{\"a\":1}"), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader, ColumnReader::Variant(_)),
            "VARIANT should route to the Variant arm"
        );
        assert_eq!(reader.read(0), str_cell("{\"a\":1}"));
        assert_eq!(reader.read(1), JsCell::Null);
    }

    #[test]
    fn object_and_array_logical_types_route_to_variant() {
        let array = StringArray::from(vec![Some("{}")]);
        assert!(
            matches!(
                reader(&field("OBJECT", DataType::Utf8, &[]), &array),
                ColumnReader::Variant(_)
            ),
            "OBJECT should share the Variant arm"
        );
        assert!(
            matches!(
                reader(&field("ARRAY", DataType::Utf8, &[]), &array),
                ColumnReader::Variant(_)
            ),
            "ARRAY should share the Variant arm"
        );
    }

    #[test]
    fn fixed_int_reads_decimal_strings_and_null() {
        let field = field("FIXED", DataType::Int64, &[("scale", "0")]);
        let array = Int64Array::from(vec![Some(42), Some(-1), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader, ColumnReader::FixedInt { .. }),
            "Int64 FIXED should route to the FixedInt arm"
        );
        assert_eq!(reader.read(0), str_cell("42"));
        assert_eq!(reader.read(1), str_cell("-1"));
        assert_eq!(reader.read(2), JsCell::Null);
    }

    #[test]
    fn fixed_int_reads_scaled_and_leading_zero_fraction() {
        let scaled = field("FIXED", DataType::Int64, &[("scale", "2")]);
        let array = Int64Array::from(vec![Some(123)]);
        assert_eq!(reader(&scaled, &array).read(0), str_cell("1.23"));

        let leading_zeros = field("FIXED", DataType::Int64, &[("scale", "5")]);
        let array = Int64Array::from(vec![Some(12)]);
        assert_eq!(reader(&leading_zeros, &array).read(0), str_cell("0.00012"));
    }

    #[test]
    fn fixed_int8_widens_and_reads_string() {
        let field = field("FIXED", DataType::Int8, &[("scale", "0")]);
        let array = Int8Array::from(vec![Some(42i8)]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader, ColumnReader::FixedInt { .. }),
            "Int8 FIXED should widen to the FixedInt arm"
        );
        assert_eq!(reader.read(0), str_cell("42"));
    }

    #[test]
    fn fixed_decimal128_reads_scaled_string_and_null() {
        let field = field("FIXED", DataType::Decimal128(38, 2), &[("scale", "2")]);
        let array = Decimal128Array::from(vec![Some(12345i128), None])
            .with_precision_and_scale(38, 2)
            .unwrap();
        let reader = reader(&field, &array);
        assert!(
            matches!(reader, ColumnReader::FixedDecimal { .. }),
            "Decimal128 FIXED should route to the FixedDecimal arm"
        );
        assert_eq!(reader.read(0), str_cell("123.45"));
        assert_eq!(reader.read(1), JsCell::Null);
    }

    #[test]
    fn real_reads_float_and_null() {
        let field = field("REAL", DataType::Float64, &[]);
        let array = Float64Array::from(vec![Some(1.5), None]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader, ColumnReader::Real(_)),
            "Float64 REAL should route to the Real arm"
        );
        assert_eq!(reader.read(0), JsCell::Number(1.5));
        assert_eq!(reader.read(1), JsCell::Null);
    }

    #[test]
    fn real_float32_widens_and_reads_float() {
        let field = field("REAL", DataType::Float32, &[]);
        let array = Float32Array::from(vec![Some(1.5f32)]);
        let reader = reader(&field, &array);
        assert!(
            matches!(reader, ColumnReader::Real(_)),
            "Float32 REAL should widen to the Real arm"
        );
        assert_eq!(reader.read(0), JsCell::Number(1.5));
    }

    fn decfloat_struct(
        exponents: Vec<Option<i16>>,
        significands: Vec<Option<&[u8]>>,
    ) -> StructArray {
        let nulls: Vec<bool> = exponents.iter().map(Option::is_some).collect();
        let fields = vec![
            Field::new("exponent", DataType::Int16, true),
            Field::new("significand", DataType::Binary, true),
        ];
        StructArray::try_new(
            fields.into(),
            vec![
                Arc::new(Int16Array::from(exponents)),
                Arc::new(BinaryArray::from(significands)),
            ],
            Some(NullBuffer::from(nulls)),
        )
        .unwrap()
    }

    #[test]
    fn decfloat_reads_formatted_string_and_null() {
        // 123456 * 10^-3 → "123.456" at precision 38 (plain notation).
        let array = decfloat_struct(vec![Some(-3), None], vec![Some(&[0x01, 0xe2, 0x40]), None]);
        let field = field(
            "DECFLOAT",
            array.data_type().clone(),
            &[("precision", "38")],
        );
        let reader = reader(&field, &array);
        assert!(
            matches!(reader, ColumnReader::Decfloat(_)),
            "DECFLOAT should route to the Decfloat arm"
        );
        assert_eq!(reader.read(0), str_cell("123.456"));
        assert_eq!(reader.read(1), JsCell::Null);
    }
}
