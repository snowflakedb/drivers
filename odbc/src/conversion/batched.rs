//! Per-segment batched Arrow → ODBC conversion.
//!
//! `Converter<A, T>::convert_arrow_range` (added in PR #927) downcasts the
//! Arrow array once per segment and then iterates rows with statically-
//! dispatched calls to `read_arrow_type` / `write_odbc_type`. This module
//! adds a thin trait — [`BatchedWrite`] — that gives the concrete
//! `(ArrowArrayType, SnowflakeType)` pair an opportunity to override that
//! per-row loop with a tight, hoisted version (resolve `target_type` once,
//! cache scale lookups, skip `Result`/`Vec<Warning>` building when the
//! output is known to fit, etc.).
//!
//! Pairs without a hot path use the [`batched_write_default_impl`] macro,
//! which delegates straight back to [`write_odbc_segment_per_row`] — the
//! same per-cell loop that lived inside `Converter::convert_arrow_range`
//! before this module existed.

use arrow::array::{Array, BooleanArray, GenericByteArray, PrimitiveArray, StructArray};
use arrow::datatypes::{Float64Type, GenericBinaryType, Int64Type, Utf8Type};
use snafu::ResultExt;

use crate::conversion::error::{ReadArrowValueSnafu, WriteOdbcValueSnafu};
use crate::conversion::nullable::Nullable;
use crate::conversion::warning::Warnings;
use crate::conversion::{Binding, BindingStrides, ConversionError, ReadArrowType, WriteODBCType};

/// Batched Arrow → ODBC conversion called once per `(column, segment)`.
///
/// Implementors **may** ignore `outputs[i]` when it already holds `Err` —
/// the row-major "first error aborts the row" semantics from the per-cell
/// path are preserved by the helper below; specialised impls just need to
/// honour the same invariant.
pub trait BatchedWrite<ArrowArrayType: Array>:
    WriteODBCType + ReadArrowType<ArrowArrayType>
{
    fn write_odbc_segment(
        &self,
        array: &ArrowArrayType,
        arrow_row_range: std::ops::Range<usize>,
        base_binding: &Binding,
        out_row_start: usize,
        strides: BindingStrides,
        outputs: &mut [Result<Warnings, ConversionError>],
    );
}

/// Per-row fallback used by both default impls and as a backstop inside
/// specialised impls when a particular `target_type` isn't on the fast
/// path.
pub fn write_odbc_segment_per_row<A, T>(
    converter: &T,
    array: &A,
    arrow_row_range: std::ops::Range<usize>,
    base_binding: &Binding,
    out_row_start: usize,
    strides: BindingStrides,
    outputs: &mut [Result<Warnings, ConversionError>],
) where
    A: Array,
    T: WriteODBCType + ReadArrowType<A>,
{
    for (i, batch_idx) in arrow_row_range.enumerate() {
        if outputs[i].is_err() {
            continue;
        }
        let binding = match strides.for_row(base_binding, out_row_start + i) {
            Ok(b) => b,
            Err(e) => {
                outputs[i] = Err(e);
                continue;
            }
        };
        let result = converter
            .read_arrow_type(array, batch_idx)
            .context(ReadArrowValueSnafu)
            .and_then(|value| {
                converter
                    .write_odbc_type(value, &binding, &mut None)
                    .context(WriteOdbcValueSnafu)
            });
        match result {
            Ok(w) => {
                if let Ok(existing) = &mut outputs[i] {
                    existing.extend(w);
                }
            }
            Err(e) => outputs[i] = Err(e),
        }
    }
}

/// Generate a default [`BatchedWrite`] impl for an `(A, T)` pair that has
/// no hot-path specialisation — the body just delegates to the per-row
/// helper.
macro_rules! batched_write_default_impl {
    ($snowflake_type:ty, $arrow_type:ty) => {
        impl $crate::conversion::batched::BatchedWrite<$arrow_type> for $snowflake_type {
            fn write_odbc_segment(
                &self,
                array: &$arrow_type,
                arrow_row_range: std::ops::Range<usize>,
                base_binding: &$crate::conversion::Binding,
                out_row_start: usize,
                strides: $crate::conversion::BindingStrides,
                outputs: &mut [Result<
                    $crate::conversion::warning::Warnings,
                    $crate::conversion::ConversionError,
                >],
            ) {
                $crate::conversion::batched::write_odbc_segment_per_row(
                    self,
                    array,
                    arrow_row_range,
                    base_binding,
                    out_row_start,
                    strides,
                    outputs,
                );
            }
        }
    };
}
pub(crate) use batched_write_default_impl;

// ---------------------------------------------------------------------------
// Default per-row impls for (A, T) pairs without a hot-path override.
// Specialised overrides for SnowflakeNumber, SnowflakeDate, SnowflakeReal
// live in their respective modules.
// ---------------------------------------------------------------------------

batched_write_default_impl!(
    crate::conversion::varchar::SnowflakeVarchar,
    GenericByteArray<Utf8Type>
);
batched_write_default_impl!(
    crate::conversion::time::SnowflakeTime,
    PrimitiveArray<Int64Type>
);
batched_write_default_impl!(
    crate::conversion::timestamp::SnowflakeTimestampNtz,
    PrimitiveArray<Int64Type>
);
batched_write_default_impl!(
    crate::conversion::timestamp::SnowflakeTimestampNtz,
    StructArray
);
batched_write_default_impl!(
    crate::conversion::timestamp::SnowflakeTimestampLtz,
    PrimitiveArray<Int64Type>
);
batched_write_default_impl!(
    crate::conversion::timestamp::SnowflakeTimestampLtz,
    StructArray
);
batched_write_default_impl!(
    crate::conversion::timestamp::SnowflakeTimestampTz,
    PrimitiveArray<Int64Type>
);
batched_write_default_impl!(
    crate::conversion::timestamp::SnowflakeTimestampTz,
    StructArray
);
batched_write_default_impl!(crate::conversion::boolean::SnowflakeBoolean, BooleanArray);
batched_write_default_impl!(
    crate::conversion::binary::SnowflakeBinary,
    GenericByteArray<GenericBinaryType<i32>>
);
batched_write_default_impl!(crate::conversion::decfloat::SnowflakeDecfloat, StructArray);
batched_write_default_impl!(
    crate::conversion::real::SnowflakeReal,
    PrimitiveArray<Float64Type>
);

/// `Nullable<T>` delegates to `T`'s impl when the segment has no nulls
/// (Arrow's `null_count()` is O(1) cached). With nulls present we fall back
/// to the per-cell path so `Nullable::write_odbc_type` can write
/// `SQL_NULL_DATA` for null rows.
impl<A: Array, T: BatchedWrite<A>> BatchedWrite<A> for Nullable<T> {
    fn write_odbc_segment(
        &self,
        array: &A,
        arrow_row_range: std::ops::Range<usize>,
        base_binding: &Binding,
        out_row_start: usize,
        strides: BindingStrides,
        outputs: &mut [Result<Warnings, ConversionError>],
    ) {
        if array.null_count() == 0 {
            self.value.write_odbc_segment(
                array,
                arrow_row_range,
                base_binding,
                out_row_start,
                strides,
                outputs,
            );
        } else {
            write_odbc_segment_per_row(
                self,
                array,
                arrow_row_range,
                base_binding,
                out_row_start,
                strides,
                outputs,
            );
        }
    }
}
