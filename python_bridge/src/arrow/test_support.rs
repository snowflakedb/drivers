use arrow::array::RecordBatchReader;
use arrow::datatypes::SchemaRef;
use arrow::ffi_stream::FFI_ArrowArrayStream;
use arrow::record_batch::{RecordBatch, RecordBatchIterator};

pub(crate) fn stream_ptr_from_batches(batches: Vec<RecordBatch>, schema: SchemaRef) -> i64 {
    let iter = RecordBatchIterator::new(batches.into_iter().map(Ok), schema);
    let ffi_stream = FFI_ArrowArrayStream::new(Box::new(iter) as Box<dyn RecordBatchReader + Send>);
    Box::into_raw(Box::new(ffi_stream)) as i64
}

#[cfg(test)]
pub(crate) fn unreleased_stream_ptr() -> i64 {
    Box::into_raw(Box::new(FFI_ArrowArrayStream::empty())) as i64
}
