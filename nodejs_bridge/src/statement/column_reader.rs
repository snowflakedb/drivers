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
use super::time_format;
use crate::session_params::SessionParams;
use crate::sql_value::SqlValue;
use std::sync::Arc;

/// Snowflake's maximum TIME fractional-second precision (`TIME(0)` ..
/// `TIME(9)`). A local naming choice, not an established convention —
/// `odbc`'s and `sf_core`'s equivalent arithmetic leave this bare.
const MAX_TIME_SCALE: u32 = 9;
/// Seconds in a day — the exclusive upper bound for a valid
/// `secs_since_midnight` component of the `secs * 10^scale + frac` TIME
/// encoding (see [`validate_time_range`]/[`decode_time`]).
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

/// Decodes one Arrow column into [`SqlValue`]s, one cell at a time.
///
/// Use it in two steps:
/// - [`for_field`](Self::for_field) inspects the column's `logicalType`,
///   picks the matching decoder, and holds onto the array.
/// - [`read`](Self::read) returns the [`SqlValue`] for a given row (or
///   [`SqlValue::Null`]).
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

    pub(crate) fn read(&self, row_index: usize) -> SqlValue {
        match self {
            Self::Boolean(array) => {
                read_cell(array, row_index, || SqlValue::Bool(array.value(row_index)))
            }
            Self::Binary(array) => read_cell(array, row_index, || {
                SqlValue::Binary(array.value(row_index).to_vec())
            }),
            Self::FixedInt { array, scale } => read_cell(array, row_index, || {
                SqlValue::String(decimal_string(array.value(row_index) as i128, *scale))
            }),
            Self::FixedDecimal { array, scale } => read_cell(array, row_index, || {
                SqlValue::String(decimal_string(array.value(row_index), *scale))
            }),
            Self::Date(array) => read_cell(array, row_index, || {
                SqlValue::Date(Date32Type::to_naive_date(array.value(row_index)))
            }),
            Self::TimeI32(array, meta) => read_cell(array, row_index, || {
                SqlValue::String(decode_time(array.value(row_index) as i64, meta))
            }),
            Self::TimeI64(array, meta) => read_cell(array, row_index, || {
                SqlValue::String(decode_time(array.value(row_index), meta))
            }),
            Self::Variant(array) => read_cell(array, row_index, || {
                SqlValue::String(array.value(row_index).to_string())
            }),
            Self::Text(array) => read_cell(array, row_index, || {
                SqlValue::String(array.value(row_index).to_string())
            }),
            Self::Real(array) => {
                read_cell(array, row_index, || SqlValue::Float(array.value(row_index)))
            }
            Self::Decfloat(array) => read_cell(array, row_index, || {
                SqlValue::String(array.value(row_index).to_string())
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

/// Decodes a raw `secs_since_midnight * 10^scale + frac` cell into the
/// column's `TIME_OUTPUT_FORMAT`-rendered string. Infallible: `for_field`
/// already validated every cell in this array via [`validate_time_range`],
/// so the arithmetic below provably cannot produce an out-of-range
/// `NaiveTime`.
fn decode_time(raw: i64, meta: &TimeMeta) -> String {
    let divisor = 10i64.pow(meta.scale);
    let secs = (raw / divisor) as u32;
    let frac = (raw % divisor) as u32;
    let nanos = frac * 10u32.pow(MAX_TIME_SCALE - meta.scale);
    debug_assert!(
        secs < SECONDS_PER_DAY as u32,
        "for_field's validation should already guarantee secs is in range, got {secs}"
    );
    let time = match NaiveTime::from_num_seconds_from_midnight_opt(secs, nanos) {
        Some(time) => time,
        // Unreachable in practice (see debug_assert above — for_field
        // already validated every raw cell in this column); fall back to
        // midnight rather than panicking on a decode path.
        None => NaiveTime::MIN,
    };
    time_format::render(time, meta.scale, &meta.format)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Int32Array, Int64Array};
    use std::collections::HashMap;

    fn time_field(scale: &str) -> Field {
        let mut metadata = HashMap::new();
        metadata.insert("logicalType".to_string(), "TIME".to_string());
        metadata.insert("scale".to_string(), scale.to_string());
        Field::new("T", DataType::Int32, true).with_metadata(metadata)
    }

    fn session_params(time_format: &str) -> SessionParams {
        SessionParams {
            time_format: Arc::from(time_format),
        }
    }

    fn as_string(value: SqlValue) -> String {
        match value {
            SqlValue::String(s) => s,
            _ => panic!("expected SqlValue::String, got a different SqlValue variant"),
        }
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

    #[test]
    fn time_i32_decodes_valid_values_and_nulls() {
        let field = time_field("3");
        // 10:30:00.123 at scale 3: secs=37_800, frac=123.
        let array = Int32Array::from(vec![Some(37_800_123), None]);
        let reader =
            ColumnReader::for_field(&field, &array, &session_params("HH24:MI:SS")).unwrap();
        assert_eq!(as_string(reader.read(0)), "10:30:00");
        assert!(matches!(reader.read(1), SqlValue::Null));
    }

    #[test]
    fn time_i64_decodes_valid_values_with_full_precision_format() {
        let field = time_field("9");
        // 10:30:00.123456789 at scale 9.
        let raw: i64 = 37_800 * 1_000_000_000 + 123_456_789;
        let array = Int64Array::from(vec![Some(raw)]);
        let reader =
            ColumnReader::for_field(&field, &array, &session_params("HH24:MI:SS.FF9")).unwrap();
        assert_eq!(as_string(reader.read(0)), "10:30:00.123456789");
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
        let reader =
            ColumnReader::for_field(&field, &array, &session_params("HH24:MI:SS")).unwrap();
        for row_index in 0..3 {
            assert!(
                matches!(reader.read(row_index), SqlValue::Null),
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
}
