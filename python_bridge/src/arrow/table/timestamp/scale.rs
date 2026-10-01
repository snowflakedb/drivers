use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, AsArray, Int32Array, Int64Array, PrimitiveArray, PrimitiveBuilder,
    StructArray, TimestampMicrosecondArray, TimestampMillisecondArray, TimestampNanosecondArray,
    TimestampSecondArray,
};
use arrow::datatypes::{ArrowPrimitiveType, Field, Int32Type, Int64Type, TimeUnit};
use arrow::error::ArrowError;
use sf_types::scaled_epoch_to_nanos;
use snafu::ResultExt;

use super::super::int_values;
use super::super::utils;
use crate::arrow::error::{
    ColumnConvertSnafu, InvalidScaleSnafu, PlanError, TimestampOverflowSnafu,
};

const NANOS_PER_SECOND: i128 = 1_000_000_000;

pub(super) struct TimestampScale {
    scale: i32,
    force_microsecond: bool,
    micros: bool,
}

impl TimestampScale {
    pub(super) fn parse(
        field: &Field,
        logical: &str,
        force_microsecond: bool,
    ) -> Result<Self, PlanError> {
        let scale = utils::scale(field)?;
        if !(0..=9).contains(&scale) {
            return InvalidScaleSnafu {
                scale,
                logical_type: logical,
            }
            .fail();
        }
        Ok(Self {
            scale,
            force_microsecond,
            micros: force_microsecond && scale > 6,
        })
    }

    pub(super) fn with_struct_micros(
        self,
        parent: &StructArray,
        epoch: &Int64Array,
        frac: Option<&Int32Array>,
    ) -> Result<Self, PlanError> {
        let micros = if self.scale <= 6 {
            false
        } else if self.force_microsecond {
            true
        } else if let Some(frac) = frac {
            overflow_requires_micros(parent, epoch, frac)?
        } else {
            false
        };
        Ok(Self { micros, ..self })
    }

    pub(super) fn unit(&self) -> TimeUnit {
        if self.scale == 0 {
            TimeUnit::Second
        } else if self.scale <= 3 {
            TimeUnit::Millisecond
        } else if self.scale <= 6 || self.micros {
            TimeUnit::Microsecond
        } else {
            TimeUnit::Nanosecond
        }
    }

    fn scale_int(&self, raw: i64) -> Result<i64, PlanError> {
        if self.scale <= 6 {
            return Ok(if self.scale == 0 {
                raw
            } else if self.scale <= 3 {
                raw.wrapping_mul(10i64.pow((3 - self.scale) as u32))
            } else {
                raw.wrapping_mul(10i64.pow((6 - self.scale) as u32))
            });
        }
        if self.micros {
            return Ok(raw.wrapping_div(10i64.pow((self.scale - 6) as u32)));
        }
        scaled_epoch_to_nanos(raw, self.scale as u32)
            .map_err(|err| ArrowError::ComputeError(err.to_string()))
            .context(ColumnConvertSnafu)
    }

    fn struct_value(&self, epoch: i64, frac: i32) -> Result<i64, PlanError> {
        if self.scale == 0 {
            return Ok(epoch);
        }
        if self.scale <= 3 {
            return Ok(epoch.wrapping_mul(10i64.pow((3 - self.scale) as u32))
                + i64::from(frac) / 10i64.pow(6));
        }
        if self.scale <= 6 {
            return Ok(epoch.wrapping_mul(10i64.pow(6)) + i64::from(frac) / 10i64.pow(3));
        }
        if self.micros {
            return Ok(epoch.wrapping_mul(1_000_000) + i64::from(frac / 1000));
        }
        i64::try_from(i128::from(epoch) * NANOS_PER_SECOND + i128::from(frac)).map_err(|_| {
            TimestampOverflowSnafu {
                epoch,
                frac: i64::from(frac),
            }
            .build()
        })
    }
}

pub(super) fn emit_int(
    scale: &TimestampScale,
    column: &ArrayRef,
    logical: &str,
    timezone: Option<&str>,
) -> Result<ArrayRef, PlanError> {
    let values =
        int_values::map_primitive::<Int64Type>(column, logical, |raw| scale.scale_int(raw))?;
    Ok(stamp(values, scale.unit(), timezone))
}

pub(super) fn emit_struct(
    scale: &TimestampScale,
    parent: &StructArray,
    epoch: &Int64Array,
    frac: Option<&Int32Array>,
    timezone: Option<&str>,
) -> Result<ArrayRef, PlanError> {
    let mut builder = PrimitiveBuilder::<Int64Type>::with_capacity(parent.len());
    for i in 0..parent.len() {
        if !parent.is_valid(i) {
            builder.append_null();
            continue;
        }
        let value = match frac {
            Some(frac) => scale.struct_value(epoch.value(i), frac.value(i))?,
            None => scale.scale_int(epoch.value(i))?,
        };
        builder.append_value(value);
    }
    Ok(stamp(builder.finish(), scale.unit(), timezone))
}

pub(super) fn struct_columns<'a>(
    column: &'a ArrayRef,
    logical: &str,
    with_fraction: bool,
) -> Result<(&'a StructArray, &'a Int64Array, Option<&'a Int32Array>), PlanError> {
    let array = column
        .as_struct_opt()
        .ok_or_else(|| {
            ArrowError::CastError(format!(
                "{logical} table conversion expected a struct array, got {}",
                column.data_type()
            ))
        })
        .context(ColumnConvertSnafu)?;
    let epoch = primitive::<Int64Type>(array, "epoch", logical)?;
    let frac = if with_fraction {
        Some(primitive::<Int32Type>(array, "fraction", logical)?)
    } else {
        None
    };
    Ok((array, epoch, frac))
}

fn primitive<'a, P: ArrowPrimitiveType>(
    array: &'a StructArray,
    name: &str,
    logical: &str,
) -> Result<&'a PrimitiveArray<P>, PlanError> {
    let column = array
        .column_by_name(name)
        .ok_or_else(|| {
            ArrowError::CastError(format!(
                "{logical} table conversion missing struct field '{name}'"
            ))
        })
        .context(ColumnConvertSnafu)?;
    column
        .as_primitive_opt::<P>()
        .ok_or_else(|| {
            ArrowError::CastError(format!(
                "{logical} table conversion expected {} for '{name}', got {}",
                P::DATA_TYPE,
                column.data_type()
            ))
        })
        .context(ColumnConvertSnafu)
}

fn stamp(array: Int64Array, unit: TimeUnit, timezone: Option<&str>) -> ArrayRef {
    let (_, values, nulls) = array.into_parts();
    match unit {
        TimeUnit::Second => {
            Arc::new(TimestampSecondArray::new(values, nulls).with_timezone_opt(timezone))
        }
        TimeUnit::Millisecond => {
            Arc::new(TimestampMillisecondArray::new(values, nulls).with_timezone_opt(timezone))
        }
        TimeUnit::Microsecond => {
            Arc::new(TimestampMicrosecondArray::new(values, nulls).with_timezone_opt(timezone))
        }
        TimeUnit::Nanosecond => {
            Arc::new(TimestampNanosecondArray::new(values, nulls).with_timezone_opt(timezone))
        }
    }
}

fn overflow_requires_micros(
    parent: &StructArray,
    epoch: &Int64Array,
    frac: &Int32Array,
) -> Result<bool, PlanError> {
    let mut downscale = false;
    for i in 0..parent.len() {
        if !parent.is_valid(i) {
            continue;
        }
        let epoch = epoch.value(i);
        let frac = frac.value(i);
        if i64::try_from(i128::from(epoch) * NANOS_PER_SECOND + i128::from(frac)).is_ok() {
            continue;
        }
        if frac % 1000 != 0 {
            return TimestampOverflowSnafu {
                epoch,
                frac: i64::from(frac),
            }
            .fail();
        }
        downscale = true;
    }
    Ok(downscale)
}
