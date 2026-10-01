use arrow::array::ArrayRef;
use arrow::datatypes::Field;

use super::scale::{self, TimestampScale};
use crate::arrow::error::{InvalidMetadataSnafu, PlanError, UnknownByteLengthSnafu};

const LOGICAL: &str = "TIMESTAMP_TZ";

pub fn convert_tz(
    field: &Field,
    column: ArrayRef,
    force_microsecond: bool,
    timezone: Option<&str>,
) -> Result<ArrayRef, PlanError> {
    let scale = TimestampScale::parse(field, LOGICAL, force_microsecond)?;
    let byte_length = byte_length(field)?;
    if byte_length != 8 && byte_length != 16 {
        return UnknownByteLengthSnafu { byte_length }.fail();
    }
    let (array, epoch, frac) = scale::struct_columns(&column, LOGICAL, byte_length == 16)?;
    let scale = scale.with_struct_micros(array, epoch, frac)?;
    scale::emit_struct(&scale, array, epoch, frac, timezone)
}

fn byte_length(field: &Field) -> Result<i32, PlanError> {
    let Some(value) = field.metadata().get("byteLength") else {
        return Ok(16);
    };
    value.parse().map_err(|_| {
        InvalidMetadataSnafu {
            key: "byteLength".to_string(),
            value: value.clone(),
        }
        .build()
    })
}
