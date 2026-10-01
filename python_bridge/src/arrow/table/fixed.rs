use std::sync::Arc;

use arrow::array::ArrayRef;
use arrow::datatypes::{DataType, Decimal128Type, Field, Float64Type};
use snafu::ResultExt;

use super::int_values;
use super::utils;
use crate::arrow::error::{ColumnConvertSnafu, InvalidMetadataSnafu, PlanError};
use crate::arrow::scaled_f64::scaled_f64;

pub(super) fn convert(
    field: &Field,
    column: ArrayRef,
    number_to_decimal: bool,
) -> Result<ArrayRef, PlanError> {
    let scale = utils::scale_or(field, 0)?;
    if scale <= 0 || matches!(column.data_type(), DataType::Decimal128(_, _)) {
        return Ok(column);
    }
    if number_to_decimal {
        to_decimal128(&column, scale)
    } else {
        to_float64(&column, scale)
    }
}

fn to_decimal128(column: &ArrayRef, scale: i32) -> Result<ArrayRef, PlanError> {
    let scale = i8::try_from(scale).map_err(|_| {
        InvalidMetadataSnafu {
            key: "scale".to_string(),
            value: scale.to_string(),
        }
        .build()
    })?;
    let array =
        int_values::map_primitive::<Decimal128Type>(column, "FIXED", |n| Ok(i128::from(n)))?
            .with_precision_and_scale(38, scale)
            .context(ColumnConvertSnafu)?;
    Ok(Arc::new(array))
}

fn to_float64(column: &ArrayRef, scale: i32) -> Result<ArrayRef, PlanError> {
    Ok(Arc::new(int_values::map_primitive::<Float64Type>(
        column,
        "FIXED",
        |n| Ok(scaled_f64(i128::from(n), -scale)),
    )?))
}
