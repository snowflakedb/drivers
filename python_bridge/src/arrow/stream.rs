use arrow::array::RecordBatchReader;
use arrow::datatypes::SchemaRef;
use arrow::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use arrow::record_batch::RecordBatch;
use snafu::{ResultExt, ensure};

use crate::arrow::error::{
    BatchReadSnafu, NullStreamPointerSnafu, ReaderCreateSnafu, StreamError, StreamNotReleasedSnafu,
};

/// Consumes an Arrow C Data Interface stream and yields [`RecordBatch`]es.
///
/// The stream pointer is owned by this struct; dropping it releases the
/// underlying `ArrowArrayStream`
pub(crate) struct RowStream {
    reader: ArrowArrayStreamReader,
}

impl RowStream {
    pub(crate) fn from_stream_ptr(stream_ptr: i64) -> std::result::Result<Self, StreamError> {
        let raw = stream_ptr as *mut FFI_ArrowArrayStream;
        ensure_stream_releasable(raw)?;

        // SAFETY: `stream_ptr` is transferred from sf-core; ownership moves here.
        let owned_stream = unsafe { FFI_ArrowArrayStream::from_raw(raw) };
        let reader = ArrowArrayStreamReader::try_new(owned_stream).context(ReaderCreateSnafu)?;

        Ok(Self { reader })
    }

    pub(crate) fn schema(&self) -> SchemaRef {
        self.reader.schema()
    }

    pub(crate) fn load_next_batch(
        &mut self,
    ) -> std::result::Result<Option<RecordBatch>, StreamError> {
        loop {
            match self.reader.next() {
                Some(batch) => {
                    let batch = batch.context(BatchReadSnafu)?;
                    if batch.num_rows() == 0 {
                        continue;
                    }
                    return Ok(Some(batch));
                }
                None => return Ok(None),
            }
        }
    }
}

fn ensure_stream_releasable(
    raw: *mut FFI_ArrowArrayStream,
) -> std::result::Result<(), StreamError> {
    ensure!(!raw.is_null(), NullStreamPointerSnafu);
    // SAFETY: `raw` is non-null; `release` is a field of the C ABI struct.
    ensure!(unsafe { (*raw).release.is_some() }, StreamNotReleasedSnafu);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::Int32Array;
    use arrow::datatypes::{DataType, Field, Schema};

    use super::*;
    use crate::arrow::test_support::{stream_ptr_from_batches, unreleased_stream_ptr};

    fn int_batch(values: Vec<i32>) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![Field::new("c0", DataType::Int32, true)]));
        let array = Arc::new(Int32Array::from(values));
        RecordBatch::try_new(schema.clone(), vec![array]).unwrap()
    }

    // Mirrors `CArrowStreamIterator::from_stream` rejecting `stream == nullptr`.
    #[test]
    fn from_stream_rejects_null_pointer() {
        assert!(matches!(
            RowStream::from_stream_ptr(0),
            Err(StreamError::NullStreamPointer { .. })
        ));
    }

    // Mirrors `CArrowStreamIterator::from_stream` rejecting `stream->release == nullptr`.
    #[test]
    fn from_stream_rejects_stream_without_release_callback() {
        let stream_ptr = unreleased_stream_ptr();
        assert!(matches!(
            RowStream::from_stream_ptr(stream_ptr),
            Err(StreamError::StreamNotReleased { .. })
        ));
        // Test helper allocates on the heap; iterator construction failed, so free it.
        unsafe {
            drop(Box::from_raw(stream_ptr as *mut FFI_ArrowArrayStream));
        }
    }

    // Mirrors schema initialization in `CArrowStreamIterator::from_stream`
    // (`schema.n_children` / column converter setup).
    #[test]
    fn from_stream_reads_schema_with_column_count() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("a", DataType::Int32, true),
            Field::new("b", DataType::Int32, true),
        ]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int32Array::from(vec![1])),
                Arc::new(Int32Array::from(vec![2])),
            ],
        )
        .unwrap();
        let stream_ptr = stream_ptr_from_batches(vec![batch], schema.clone());

        let stream = RowStream::from_stream_ptr(stream_ptr).unwrap();
        assert_eq!(stream.schema().fields().len(), schema.fields().len());
        assert_eq!(stream.schema().as_ref(), schema.as_ref());
    }

    // Mirrors `CArrowStreamIterator::loadNextBatch` skipping zero-row batches.
    #[test]
    fn load_next_batch_skips_empty_batches() {
        let schema = Arc::new(Schema::new(vec![Field::new("c0", DataType::Int32, true)]));
        let batches = vec![int_batch(vec![]), int_batch(vec![7, 8])];
        let stream_ptr = stream_ptr_from_batches(batches, schema);

        let mut stream = RowStream::from_stream_ptr(stream_ptr).unwrap();
        let batch = stream.load_next_batch().unwrap().unwrap();
        assert_eq!(batch.num_rows(), 2);
        assert_eq!(
            batch
                .column(0)
                .as_any()
                .downcast_ref::<Int32Array>()
                .unwrap()
                .values(),
            &[7, 8]
        );
    }

    // Mirrors `loadNextBatch` returning false once the stream is exhausted
    // (`array.release == nullptr`).
    #[test]
    fn load_next_batch_returns_none_when_stream_exhausted() {
        let batch = int_batch(vec![1, 2, 3]);
        let schema = batch.schema();
        let stream_ptr = stream_ptr_from_batches(vec![batch], schema);

        let mut stream = RowStream::from_stream_ptr(stream_ptr).unwrap();

        assert_eq!(stream.load_next_batch().unwrap().unwrap().num_rows(), 3);
        assert!(stream.load_next_batch().unwrap().is_none());
    }

    // Mirrors repeated `loadNextBatch` calls after exhaustion staying at EOF.
    #[test]
    fn load_next_batch_returns_none_after_stream_exhausted() {
        let batch = int_batch(vec![1]);
        let schema = batch.schema();
        let stream_ptr = stream_ptr_from_batches(vec![batch], schema);

        let mut stream = RowStream::from_stream_ptr(stream_ptr).unwrap();
        assert!(stream.load_next_batch().unwrap().is_some());
        assert!(stream.load_next_batch().unwrap().is_none());
        assert!(stream.load_next_batch().unwrap().is_none());
    }

    // Mirrors row iteration crossing batch boundaries (`next` / `nextN` loop body).
    #[test]
    fn load_next_batch_reads_batches_sequentially() {
        let schema = Arc::new(Schema::new(vec![Field::new("c0", DataType::Int32, true)]));
        let batches = vec![int_batch(vec![1, 2, 3]), int_batch(vec![4, 5])];
        let stream_ptr = stream_ptr_from_batches(batches, schema);

        let mut stream = RowStream::from_stream_ptr(stream_ptr).unwrap();

        let batch1 = stream.load_next_batch().unwrap().unwrap();
        assert_eq!(batch1.num_rows(), 3);
        assert_eq!(
            batch1
                .column(0)
                .as_any()
                .downcast_ref::<Int32Array>()
                .unwrap()
                .values(),
            &[1, 2, 3]
        );

        let batch2 = stream.load_next_batch().unwrap().unwrap();
        assert_eq!(batch2.num_rows(), 2);
        assert!(stream.load_next_batch().unwrap().is_none());
    }
}
