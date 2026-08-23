use arrow::array::{Array, BinaryArray, BooleanArray, PrimitiveArray, StringArray};
use arrow::compute::cast;
use arrow::datatypes::{DataType, Date32Type, Field};

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
