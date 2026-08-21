use arrow::array::{
    Array, BinaryArray, BooleanArray, Int16Array, PrimitiveArray, StringArray, StringBuilder,
    StructArray,
};
use arrow::compute::cast;
use arrow::datatypes::{DataType, Date32Type, Field};

use super::decfloat::{format_decfloat, i128_from_big_endian_signed};
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
    Fixed(StringArray),
    Date(PrimitiveArray<Date32Type>),
    Variant(StringArray),
    Text(StringArray),
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
                // TODO: temporary string casting. We need to figure out how to handle precision loss.
                let utf8 = cast(column, &DataType::Utf8)
                    .map_err(|e| format!("could not cast FIXED column to string: {e}"))?;
                let array = utf8
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .cloned()
                    .ok_or_else(|| {
                        "cast of FIXED column did not yield a StringArray".to_string()
                    })?;
                Ok(Self::Fixed(array))
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
            Self::Fixed(array) => read_cell(array, row_index, || {
                SqlValue::String(array.value(row_index).to_string())
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

/// Decode a cell, mapping null to [`SqlValue::Null`] so each reader arm only
/// needs to describe the non-null case.
fn read_cell<A: Array>(array: &A, row_index: usize, decode: impl FnOnce() -> SqlValue) -> SqlValue {
    if array.is_null(row_index) {
        SqlValue::Null
    } else {
        decode()
    }
}

fn decfloat_field<T: Array + Clone + 'static>(
    array: &StructArray,
    name: &str,
) -> Result<T, String> {
    let child = array
        .column_by_name(name)
        .ok_or_else(|| format!("DECFLOAT struct is missing the {name:?} field"))?;
    child.as_any().downcast_ref::<T>().cloned().ok_or_else(|| {
        format!(
            "DECFLOAT {name:?} field could not be downcast; it is {}",
            child.data_type()
        )
    })
}

fn usize_from_metadata(field: &Field, key: &str) -> Result<usize, String> {
    let raw = field
        .metadata()
        .get(key)
        .ok_or_else(|| format!("column {:?} is missing {key} metadata", field.name()))?;
    raw.parse().map_err(|_| {
        format!(
            "column {:?} has non-numeric {key} metadata {raw:?}",
            field.name()
        )
    })
}
