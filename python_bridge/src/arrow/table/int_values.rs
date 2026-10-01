use arrow::array::{Array, ArrayRef, ArrowPrimitiveType, PrimitiveArray, PrimitiveBuilder};
use arrow::datatypes::{DataType, Decimal128Type, Int8Type, Int16Type, Int32Type, Int64Type};
use arrow::error::ArrowError;
use snafu::ResultExt;

use crate::arrow::error::{ColumnConvertSnafu, PlanError};

pub(super) fn map_primitive<O: ArrowPrimitiveType>(
    column: &ArrayRef,
    logical: &str,
    mut f: impl FnMut(i64) -> Result<O::Native, PlanError>,
) -> Result<PrimitiveArray<O>, PlanError> {
    map_i128(column, logical, false, |n| {
        let n = i64::try_from(n)
            .map_err(|_| {
                ArrowError::CastError(format!(
                    "{logical} table conversion value {n} does not fit i64"
                ))
            })
            .context(ColumnConvertSnafu)?;
        f(n)
    })
}

pub(super) fn map_mantissa<O: ArrowPrimitiveType>(
    column: &ArrayRef,
    logical: &str,
    f: impl FnMut(i128) -> Result<O::Native, PlanError>,
) -> Result<PrimitiveArray<O>, PlanError> {
    map_i128(column, logical, true, f)
}

fn map_i128<O: ArrowPrimitiveType>(
    column: &ArrayRef,
    logical: &str,
    allow_decimal: bool,
    mut f: impl FnMut(i128) -> Result<O::Native, PlanError>,
) -> Result<PrimitiveArray<O>, PlanError> {
    match column.data_type() {
        DataType::Int8 => walk::<Int8Type, O>(column, logical, &mut f),
        DataType::Int16 => walk::<Int16Type, O>(column, logical, &mut f),
        DataType::Int32 => walk::<Int32Type, O>(column, logical, &mut f),
        DataType::Int64 => walk::<Int64Type, O>(column, logical, &mut f),
        DataType::Decimal128(_, _) if allow_decimal => {
            walk::<Decimal128Type, O>(column, logical, &mut f)
        }
        other => {
            let expected = if allow_decimal {
                "an integer or decimal128 array"
            } else {
                "an integer array"
            };
            Err(ArrowError::CastError(format!(
                "{logical} table conversion expected {expected}, got {other}"
            )))
            .context(ColumnConvertSnafu)
        }
    }
}

fn walk<P, O>(
    column: &ArrayRef,
    logical: &str,
    f: &mut impl FnMut(i128) -> Result<O::Native, PlanError>,
) -> Result<PrimitiveArray<O>, PlanError>
where
    P: ArrowPrimitiveType,
    P::Native: Into<i128>,
    O: ArrowPrimitiveType,
{
    let array = column
        .as_any()
        .downcast_ref::<PrimitiveArray<P>>()
        .ok_or_else(|| {
            ArrowError::CastError(format!(
                "{logical} table conversion expected {}, got {}",
                P::DATA_TYPE,
                column.data_type()
            ))
        })
        .context(ColumnConvertSnafu)?;

    let mut builder = PrimitiveBuilder::<O>::with_capacity(array.len());
    if array.null_count() == 0 {
        for &n in array.values().iter() {
            builder.append_value(f(n.into())?);
        }
    } else {
        let values = array.values();
        for i in 0..array.len() {
            if array.is_valid(i) {
                builder.append_value(f(values[i].into())?);
            } else {
                builder.append_null();
            }
        }
    }
    Ok(builder.finish())
}
