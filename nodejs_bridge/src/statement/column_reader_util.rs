//! Arrow-array decoding helpers shared across [`super::column_reader::ColumnReader`] arms.

use super::js_cell::JsCell;
use arrow::array::{Array, Decimal128Array, Int8Array, Int16Array, Int32Array, Int64Array};
use arrow::datatypes::{DataType, Field};
use sf_types::{ReadArrowError, ReadArrowType, SnowflakeFixed};

/// Returns [`JsCell::Null`] when the Arrow cell is null so each reader arm
/// only needs to describe the non-null case.
pub(super) fn read_cell<'a, A: Array>(
    array: &'a A,
    row_index: usize,
    value: impl FnOnce() -> JsCell<'a>,
) -> JsCell<'a> {
    if array.is_null(row_index) {
        JsCell::Null
    } else {
        value()
    }
}

pub(super) fn usize_from_metadata(field: &Field, key: &str) -> Result<usize, String> {
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

pub(super) fn downcast_array<T: Array + Clone + 'static>(
    column: &dyn Array,
    target: &str,
) -> Result<T, String> {
    column
        .as_any()
        .downcast_ref::<T>()
        .cloned()
        .ok_or_else(|| format!("Arrow column could not be downcast to {target}"))
}

/// A Snowflake integer column kept on the physical Arrow width the server sent,
/// widened to `i128` only when a cell is read. The FIXED mantissa,
/// `INTERVAL_YEAR_MONTH` month count, and `INTERVAL_DAY_TIME` nanosecond count
/// share this representation: all three arrive as a single signed integer whose
/// width varies with magnitude, and differ only in how the widened value is
/// rendered downstream.
pub(super) enum IntColumn {
    I8(Int8Array),
    I16(Int16Array),
    I32(Int32Array),
    I64(Int64Array),
    Decimal(Decimal128Array),
}

impl IntColumn {
    pub(super) fn from_column(
        column: &dyn Array,
        logical_type: &str,
        column_name: &str,
    ) -> Result<Self, String> {
        Ok(match column.data_type() {
            DataType::Int8 => Self::I8(downcast_array(column, "Int8Array")?),
            DataType::Int16 => Self::I16(downcast_array(column, "Int16Array")?),
            DataType::Int32 => Self::I32(downcast_array(column, "Int32Array")?),
            DataType::Int64 => Self::I64(downcast_array(column, "Int64Array")?),
            DataType::Decimal128(_, _) => Self::Decimal(downcast_array(column, "Decimal128Array")?),
            other => {
                return Err(format!(
                    "{logical_type} column {column_name:?} has unsupported Arrow type {other}"
                ));
            }
        })
    }

    pub(super) fn get(&self, row_index: usize) -> Result<i128, ReadArrowError> {
        match self {
            Self::I8(array) => SnowflakeFixed.read_arrow_type(array, row_index),
            Self::I16(array) => SnowflakeFixed.read_arrow_type(array, row_index),
            Self::I32(array) => SnowflakeFixed.read_arrow_type(array, row_index),
            Self::I64(array) => SnowflakeFixed.read_arrow_type(array, row_index),
            Self::Decimal(array) => SnowflakeFixed.read_arrow_type(array, row_index),
        }
    }
}

pub(super) fn scale_from_metadata(field: &Field) -> Result<u32, String> {
    let raw = field
        .metadata()
        .get("scale")
        .ok_or_else(|| format!("FIXED column {:?} is missing scale metadata", field.name()))?;
    raw.parse().map_err(|_| {
        format!(
            "FIXED column {:?} has non-numeric scale metadata {raw:?}",
            field.name()
        )
    })
}

/// Renders `unscaled x 10^-scale` as an exact decimal string — every digit the
/// server sent, no rounding.
///
/// FIXED type is handed to JS in this form rather than as an `f64` so the numeric
/// policy lives on the JS side: `Number()` there reproduces the old driver
/// exactly (it applied `Number()` to the server's decimal text)
///
/// Same three-case shape as ODBC's `format_decimal_into`
/// (`odbc/src/conversion/number.rs`), which writes into a fixed-size buffer for
/// zero-allocation formatting; this version allocates a `String` instead since
/// napi calls aren't as allocation-sensitive as ODBC's `SQLGetData` hot path.
pub(super) fn decimal_string(unscaled: i128, scale: u32) -> String {
    let sign = if unscaled.is_negative() { "-" } else { "" };
    let digits = unscaled.unsigned_abs().to_string();
    let scale = scale as usize;
    if scale == 0 {
        return format!("{sign}{digits}");
    }
    if let Some(split) = digits.len().checked_sub(scale).filter(|split| *split > 0) {
        let (int_part, frac_part) = digits.split_at(split);
        format!("{sign}{int_part}.{frac_part}")
    } else {
        format!("{sign}0.{digits:0>scale$}")
    }
}
