use crate::common::arrow_extract_value::ArrowExtractError;
use arrow::record_batch::RecordBatch;

/// Converts a single row of an Arrow `RecordBatch` into a strongly typed value.
///
/// Implementors of this trait define how to construct `Self` from the values at a given
/// `row_idx` in the provided `RecordBatch`, typically by delegating to field-level
/// deserialization helpers.
///
/// Unlike `ArrowDeserialize`, which focuses on converting Arrow arrays or individual
/// Arrow values into Rust types, `ArrowConvertRow` provides a row-oriented convenience
/// API that assembles an entire struct from a record batch row in one step.
pub trait ArrowConvertRow: Sized {
    fn from_arrow_row(batch: &RecordBatch, row_idx: usize) -> Result<Self, ArrowExtractError>;
}
