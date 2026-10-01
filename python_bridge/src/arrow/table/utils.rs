use arrow::datatypes::Field;

use crate::arrow::error::{InvalidMetadataSnafu, MissingMetadataSnafu, PlanError};

pub(super) fn scale(field: &Field) -> Result<i32, PlanError> {
    match parsed_scale(field)? {
        Some(scale) => Ok(scale),
        None => MissingMetadataSnafu {
            key: "scale".to_string(),
            column: field.name().to_string(),
        }
        .fail(),
    }
}

pub(super) fn scale_or(field: &Field, default: i32) -> Result<i32, PlanError> {
    Ok(parsed_scale(field)?.unwrap_or(default))
}

pub(super) fn scale_time(raw: i64, scale: i32) -> i64 {
    if scale == 0 {
        raw
    } else if scale <= 3 {
        raw.wrapping_mul(10i64.pow((3 - scale) as u32))
    } else if scale <= 6 {
        raw.wrapping_mul(10i64.pow((6 - scale) as u32))
    } else {
        raw.wrapping_div(10i64.pow((scale - 6) as u32))
    }
}

fn parsed_scale(field: &Field) -> Result<Option<i32>, PlanError> {
    let Some(scale) = field.metadata().get("scale") else {
        return Ok(None);
    };
    scale.parse().map(Some).map_err(|_| {
        InvalidMetadataSnafu {
            key: "scale".to_string(),
            value: scale.clone(),
        }
        .build()
    })
}
