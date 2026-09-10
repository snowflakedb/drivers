//! Generic batched `SQL_C_CHAR` fetch path, parametrized on a per-row kernel.
//!
//! The hot `SQLFetch` conversion of an Arrow value to bound `SQL_C_CHAR`
//! output is dominated not by value formatting but by the per-cell fixed
//! overhead of the generic [`Converter`](super::Converter) pipeline: the
//! `Result`/[`Warnings`] plumbing, the `validate_value` call, the
//! `write_odbc_type` target-type dispatch, and the stride bookkeeping — paid
//! once per (row × column).
//!
//! [`convert_char_range`] hoists all of that out of the row loop: it owns the
//! null-bitmap handling, the incremental striding, the error-slot skip, and
//! the warning collection exactly once, and delegates only the per-row
//! "read + format" step to a [`CharKernel`]. Each Snowflake type supplies a
//! small kernel that reuses its existing `format_*_into` helper, so a new type
//! is one kernel impl rather than another hand-copied loop.
//!
//! The loop is byte-for-byte equivalent to the generic per-cell path for the
//! `SQL_C_CHAR` target (value buffer, indicators, warnings, and any
//! type-specific range error), verified per kernel by the equivalence oracle
//! in `mod.rs` (`*_char_batch_tests`).

use crate::api::CDataType;
use crate::conversion::ColumnConverter;
use crate::conversion::error::{ConversionError, WriteOdbcError, WriteOdbcValueSnafu};
use crate::conversion::traits::{Binding, BindingStrides, LengthOrNull};
use crate::conversion::warning::Warnings;
use arrow::array::Array;
use snafu::ResultExt;

/// Stack scratch shared by every char kernel's `format_into`. Sized for the
/// widest formatter (REAL's `f64` `Display`, 384 bytes); every other kernel
/// (TIMESTAMP_TZ 64, NUMBER/NTZ/LTZ 48, DATE/TIME 32) fits inside it. A single
/// fixed size lets the loop own one reused buffer without const generics
/// (which cannot depend on an associated const), and lets each kernel slice
/// out the exact fixed-size array its existing helper expects — so no helper
/// signature (nor its `buf.len()`-based overflow message) has to change.
pub(crate) const CHAR_SCRATCH_LEN: usize = 384;

/// Per-row kernel for the batched `SQL_C_CHAR` fetch path: the `CDataType::Char`
/// arm of a Snowflake type's `write_odbc_type`, factored so that
/// [`convert_char_range`] can own the null handling, striding, error placement,
/// and warning collection. Kept byte-identical to the per-cell path by
/// reusing the same read/validate/format helpers and the same error wrapping.
pub(crate) trait CharKernel {
    /// Concrete Arrow array; downcast once per segment by the wrapper.
    type Array: Array + 'static;
    /// Materialised intermediate value (mirrors the type's
    /// `SnowflakeType::Representation`, e.g. `i128`, `NaiveDate`, `f64`).
    type Value;

    /// Read one **non-null** cell and apply the type's `validate_value`. The
    /// loop never calls this on a null cell — it writes the NULL indicator
    /// itself, matching [`nullable::Nullable`](super::nullable). Error typing
    /// mirrors the per-cell path exactly: a read failure is
    /// `.context(ReadArrowValueSnafu)`, a validation failure is the raw
    /// `ConversionError` (e.g. `DatetimeOutOfSqlRange`), so the returned error
    /// is placed into `outputs[i]` verbatim.
    fn read_validate(
        &self,
        array: &Self::Array,
        idx: usize,
    ) -> Result<Self::Value, ConversionError>;

    /// Format the validated value as ASCII into `scratch`, returning the slice
    /// to hand to `write_char_string`. Owns any type-specific pre-write guard
    /// (e.g. the DATE/TIME/TIMESTAMP minimum-buffer check) so undersized
    /// buffers fail before the value buffer is touched, exactly as the
    /// per-cell arm does.
    fn format_into<'s>(
        &self,
        value: &Self::Value,
        binding: &Binding,
        scratch: &'s mut [u8; CHAR_SCRATCH_LEN],
    ) -> Result<&'s str, WriteOdbcError>;

    /// Post-write range check. NUMBER overrides this for the whole-digits
    /// `SQLSTATE 22003` overflow; every other kernel uses the default no-op.
    fn post_write_check(&self, _s: &str, _binding: &Binding) -> Result<(), WriteOdbcError> {
        Ok(())
    }
}

/// The single batched `SQL_C_CHAR` loop, shared by every [`CharKernel`]. It is
/// the specialised counterpart to `Converter::convert_arrow_range`: it
/// downcasts once (the caller passes the concrete `K::Array`), then reads,
/// formats, and writes in one tight loop with no per-cell `write_odbc_type`
/// match or trait indirection.
///
/// Returns `false` (having written nothing) to *decline* the batch — on
/// first-row stride overflow, or when a non-nullable column unexpectedly
/// carries nulls — so the caller can fall back to the generic per-cell path
/// and reproduce its exact behavior for those cold cases.
// 8 args mirror `Converter::convert_arrow_range`'s contract (binding + strides
// + outputs); grouping them into a struct would just move the noise.
#[allow(clippy::too_many_arguments)]
pub(crate) fn convert_char_range<K: CharKernel>(
    kernel: &K,
    nullable: bool,
    array: &K::Array,
    arrow_row_range: std::ops::Range<usize>,
    base_binding: &Binding,
    out_row_start: usize,
    strides: BindingStrides,
    outputs: &mut [Result<Warnings, ConversionError>],
) -> bool {
    // A non-nullable column should never carry nulls; if one somehow does,
    // decline so the generic path reproduces its read-error behavior exactly
    // rather than us inventing a null indicator.
    if !nullable && array.null_count() > 0 {
        return false;
    }

    // Materialize the first row's binding + the constant per-row stride, exactly
    // as the generic path does. On the pathological first-row overflow, decline.
    let Ok(mut binding) = strides.for_row(base_binding, out_row_start) else {
        return false;
    };
    let (value_stride, indicator_stride) =
        strides.row_step(base_binding.target_type, base_binding.buffer_length);

    // Reused across rows; each kernel fully writes the prefix it returns, so
    // there is no need to re-zero per row.
    let mut scratch = [0u8; CHAR_SCRATCH_LEN];
    // When the whole segment is null-free, skip the per-row `is_null` probe.
    let no_nulls = array.null_count() == 0;

    for (i, batch_idx) in arrow_row_range.enumerate() {
        if i > 0 {
            binding = binding.stepped(value_stride, indicator_stride);
        }
        if outputs[i].is_err() {
            continue;
        }

        // NULL cell (nullable columns only — see the null_count guard above):
        // mirrors `Nullable::write_odbc_type`'s `None` arm.
        if !no_nulls && array.is_null(batch_idx) {
            if let Err(e) = binding
                .write_length_or_null(LengthOrNull::Null)
                .context(WriteOdbcValueSnafu)
            {
                outputs[i] = Err(e);
            }
            continue;
        }

        let result = (|| {
            let value = kernel.read_validate(array, batch_idx)?;
            let s = kernel
                .format_into(&value, &binding, &mut scratch)
                .context(WriteOdbcValueSnafu)?;
            let warnings = binding.write_char_string(s, &mut None);
            kernel
                .post_write_check(s, &binding)
                .context(WriteOdbcValueSnafu)?;
            Ok::<Warnings, ConversionError>(warnings)
        })();

        match result {
            Ok(w) => {
                // Warnings are rare; gating the `outputs[i]` index + extend on the
                // empty-warning check is worth ~3% on NUMBER fetches. Kept as a
                // nested `if` (not a `&& let` chain) per review preference — the
                // clippy collapse suggestion would reintroduce the let-chain.
                #[allow(clippy::collapsible_if)]
                if !w.is_empty() {
                    if let Ok(existing) = &mut outputs[i] {
                        existing.extend(w);
                    }
                }
            }
            Err(e) => outputs[i] = Err(e),
        }
    }
    true
}

/// Wraps the generic converter for a type, intercepting the hot `SQL_C_CHAR`
/// range conversion with the batched [`convert_char_range`] and delegating
/// every other target — and the single-cell `SQLGetData` path — to the generic
/// per-cell converter unchanged. If the batched path declines (stride overflow,
/// or a non-nullable column carrying nulls) it also falls back to the generic
/// path, so behavior is identical in every case.
pub(crate) struct CharBatchConverter<K: CharKernel> {
    pub(super) inner: Box<dyn ColumnConverter>,
    pub(super) kernel: K,
    pub(super) nullable: bool,
}

impl<K: CharKernel + 'static> ColumnConverter for CharBatchConverter<K> {
    fn convert_arrow_value(
        &self,
        array: &dyn Array,
        row_idx: usize,
        binding: &Binding,
        get_data_offset: &mut Option<usize>,
    ) -> Result<Warnings, ConversionError> {
        self.inner
            .convert_arrow_value(array, row_idx, binding, get_data_offset)
    }

    fn convert_arrow_range(
        &self,
        array: &dyn Array,
        arrow_row_range: std::ops::Range<usize>,
        base_binding: &Binding,
        out_row_start: usize,
        strides: BindingStrides,
        outputs: &mut [Result<Warnings, ConversionError>],
    ) {
        if base_binding.target_type == CDataType::Char
            && let Some(arr) = array.as_any().downcast_ref::<K::Array>()
            && convert_char_range(
                &self.kernel,
                self.nullable,
                arr,
                arrow_row_range.clone(),
                base_binding,
                out_row_start,
                strides,
                outputs,
            )
        {
            return;
        }
        self.inner.convert_arrow_range(
            array,
            arrow_row_range,
            base_binding,
            out_row_start,
            strides,
            outputs,
        );
    }
}
