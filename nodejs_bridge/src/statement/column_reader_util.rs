//! Arrow-array decoding helpers shared across [`super::column_reader::ColumnReader`] arms.

use super::js_cell::JsCell;
use arrow::array::Array;
use arrow::compute::cast;
use arrow::datatypes::{DataType, Field};

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

pub(super) fn widen<T: Array + Clone + 'static>(
    column: &dyn Array,
    to: &DataType,
    target: &str,
) -> Result<T, String> {
    let widened =
        cast(column, to).map_err(|e| format!("could not cast column to {target}: {e}"))?;
    widened
        .as_any()
        .downcast_ref::<T>()
        .cloned()
        .ok_or_else(|| format!("cast of column did not yield a {target}"))
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
