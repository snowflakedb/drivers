mod scale;
mod tz;

use std::sync::Arc;

use arrow::array::ArrayRef;
use arrow::datatypes::{DataType, Field};
use arrow::error::ArrowError;
use snafu::ResultExt;

use self::scale::TimestampScale;
use crate::arrow::error::{ColumnConvertSnafu, PlanError};

pub(super) use tz::convert_tz;

pub(super) fn schema_type(
    field: &Field,
    logical: &str,
    timezone: Option<&str>,
    force_microsecond: bool,
) -> Result<DataType, PlanError> {
    let scale = TimestampScale::parse(field, logical, force_microsecond)?;
    Ok(DataType::Timestamp(scale.unit(), timezone.map(Arc::from)))
}

pub(super) fn convert_ntz(
    field: &Field,
    column: ArrayRef,
    force_microsecond: bool,
) -> Result<ArrayRef, PlanError> {
    convert(field, column, force_microsecond, None, "TIMESTAMP_NTZ")
}

pub(super) fn convert_ltz(
    field: &Field,
    column: ArrayRef,
    force_microsecond: bool,
    timezone: Option<&str>,
) -> Result<ArrayRef, PlanError> {
    convert(field, column, force_microsecond, timezone, "TIMESTAMP_LTZ")
}

fn convert(
    field: &Field,
    column: ArrayRef,
    force_microsecond: bool,
    timezone: Option<&str>,
    logical: &'static str,
) -> Result<ArrayRef, PlanError> {
    let scale = TimestampScale::parse(field, logical, force_microsecond)?;
    match column.data_type() {
        DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64 => {
            scale::emit_int(&scale, &column, logical, timezone)
        }
        DataType::Struct(_) => convert_struct(scale, &column, timezone, logical),
        other => Err(ArrowError::CastError(format!(
            "{logical} table conversion expected an integer or struct array, got {other}"
        )))
        .context(ColumnConvertSnafu),
    }
}

fn convert_struct(
    scale: TimestampScale,
    column: &ArrayRef,
    timezone: Option<&str>,
    logical: &str,
) -> Result<ArrayRef, PlanError> {
    let (array, epoch, frac) = scale::struct_columns(column, logical, true)?;
    let scale = scale.with_struct_micros(array, epoch, frac)?;
    scale::emit_struct(&scale, array, epoch, frac, timezone)
}
