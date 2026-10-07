use std::sync::Arc;

use arrow::array::ArrayRef;
use arrow::datatypes::{DurationNanosecondType, Field};

use super::int_values;
use super::utils;
use crate::arrow::error::PlanError;

pub(super) fn convert(field: &Field, column: ArrayRef) -> Result<ArrayRef, PlanError> {
    utils::scale(field)?;
    Ok(Arc::new(
        int_values::map_mantissa::<DurationNanosecondType>(
            &column,
            "INTERVAL_DAY_TIME",
            |nanos| Ok(nanos as i64),
        )?,
    ))
}
