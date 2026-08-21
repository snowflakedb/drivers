use arrow::array::{
    Array, BinaryArray, BooleanArray, Decimal128Array, Int16Array, Int64Array, PrimitiveArray,
    StringArray, StringBuilder, StructArray,
};
use arrow::datatypes::{DataType, Date32Type, Field};

use super::column_reader_util::{
    decimal_string, read_cell, scale_from_metadata, usize_from_metadata, widen,
};
use super::decfloat::{decfloat_field, format_decfloat, i128_from_big_endian_signed};
use crate::session_params::SessionParams;
use crate::sql_value::SqlValue;

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
    Variant(StringArray),
    Text(StringArray),
    FixedInt { array: Int64Array, scale: u32 },
    FixedDecimal { array: Decimal128Array, scale: u32 },
    Decfloat(StringArray),
}

impl ColumnReader {
    // TODO: figure better error handling
    pub(crate) fn for_field(
        field: &Field,
        column: &dyn Array,
        // Not yet consumed by any arm — wired through so a future TIME
        // decoder can read it without threading a new parameter all the way
        // back down from `Connection::execute`.
        _session_params: &SessionParams,
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
            Self::Variant(array) => read_cell(array, row_index, || {
                SqlValue::String(array.value(row_index).to_string())
            }),
            Self::Text(array) => read_cell(array, row_index, || {
                SqlValue::String(array.value(row_index).to_string())
            }),
            Self::Decfloat(array) => read_cell(array, row_index, || {
                SqlValue::String(array.value(row_index).to_string())
            }),
        }
    }
}
