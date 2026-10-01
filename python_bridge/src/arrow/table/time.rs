use std::sync::Arc;

use arrow::array::ArrayRef;
use arrow::datatypes::{
    DataType, Field, Time32MillisecondType, Time32SecondType, Time64MicrosecondType, TimeUnit,
};

use super::int_values;
use super::utils;
use crate::arrow::error::{InvalidScaleSnafu, PlanError, TimeDoesNotFitSnafu};

pub(super) fn convert(field: &Field, column: ArrayRef) -> Result<ArrayRef, PlanError> {
    let scale = utils::scale(field)?;
    if !(0..=9).contains(&scale) {
        return InvalidScaleSnafu {
            scale,
            logical_type: "TIME",
        }
        .fail();
    }
    if scale == 0 {
        Ok(Arc::new(int_values::map_primitive::<Time32SecondType>(
            &column,
            "TIME",
            |raw| time32(utils::scale_time(raw, scale)),
        )?))
    } else if scale <= 3 {
        Ok(Arc::new(
            int_values::map_primitive::<Time32MillisecondType>(&column, "TIME", |raw| {
                time32(utils::scale_time(raw, scale))
            })?,
        ))
    } else {
        Ok(Arc::new(
            int_values::map_primitive::<Time64MicrosecondType>(&column, "TIME", |raw| {
                Ok(utils::scale_time(raw, scale))
            })?,
        ))
    }
}

pub(super) fn data_type(scale: i32) -> DataType {
    if scale == 0 {
        DataType::Time32(TimeUnit::Second)
    } else if scale <= 3 {
        DataType::Time32(TimeUnit::Millisecond)
    } else {
        DataType::Time64(TimeUnit::Microsecond)
    }
}

fn time32(value: i64) -> Result<i32, PlanError> {
    i32::try_from(value).map_err(|_| TimeDoesNotFitSnafu { value }.build())
}
