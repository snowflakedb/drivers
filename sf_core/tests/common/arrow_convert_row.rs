use crate::common::arrow_extract_value::ArrowExtractError;
use arrow::record_batch::RecordBatch;

pub trait ArrowConvertRow: Sized {
    fn from_arrow_row(batch: &RecordBatch, row_idx: usize) -> Result<Self, ArrowExtractError>;
}
