use std::pin::Pin;

use arrow::array::{RecordBatch, RecordBatchReader};
use arrow::datatypes::SchemaRef;
use arrow::error::ArrowError;
use futures::Stream;
use futures::StreamExt;
use snafu::ResultExt;
use tokio::task::AbortHandle;

use super::error::*;
use super::prefetch::PrefetchChunkReader;
use super::{ChunkError, drain_reader_to_batches, inject_nullable_schema};

type BatchStream = Pin<Box<dyn Stream<Item = Result<RecordBatch, ArrowError>> + Send>>;

/// Yields Arrow [`RecordBatch`]es from a prefetch pipeline or a materialized
/// batch stream.
///
/// Dropping a fetcher that holds a prefetch [`AbortHandle`] aborts the
/// background coordinator from [`PrefetchChunkReader::open`]. That coordinator
/// can be parked in a download or memory-budget wait, so closing the reader's
/// channel would not wake it. Fetchers built from materialized batches hold no
/// handle and abort nothing on drop.
pub(crate) struct AsyncArrowBatchFetcher {
    schema: SchemaRef,
    source: BatchStream,
    state: ConsumeState,
    abort_handle: Option<AbortHandle>,
}

#[derive(Clone, Copy, Debug)]
enum ConsumeState {
    Open,
    Eof,
    Failed,
}

impl AsyncArrowBatchFetcher {
    pub(crate) fn new(
        reader: PrefetchChunkReader,
        abort_handle: Option<AbortHandle>,
        nullable_flags: Option<&[bool]>,
    ) -> Self {
        let schema = reader.schema();
        Self::from_stream(Box::pin(reader), schema, abort_handle, nullable_flags)
    }

    pub(crate) fn from_batches(schema: SchemaRef, batches: Vec<RecordBatch>) -> Self {
        Self::from_stream(
            Box::pin(futures::stream::iter(batches.into_iter().map(Ok))),
            schema,
            None,
            None,
        )
    }

    pub(crate) async fn from_record_batch_reader(
        reader: Box<dyn RecordBatchReader + Send>,
    ) -> Result<Self, ChunkError> {
        let (schema, batches) =
            tokio::task::spawn_blocking(move || drain_reader_to_batches(reader))
                .await
                .context(SpawnBlockingSnafu)??;
        Ok(Self::from_batches(schema, batches))
    }

    fn from_stream(
        source: BatchStream,
        schema: SchemaRef,
        abort_handle: Option<AbortHandle>,
        nullable_flags: Option<&[bool]>,
    ) -> Self {
        Self {
            schema: inject_nullable_schema(schema, nullable_flags),
            source,
            state: ConsumeState::Open,
            abort_handle,
        }
    }

    pub(crate) fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    pub(crate) async fn next_batch(&mut self) -> Result<Option<RecordBatch>, ChunkError> {
        if let Some(done) = self.terminal_result() {
            return done;
        }
        match self.source.next().await {
            Some(Ok(batch)) => Ok(Some(batch)),
            Some(Err(e)) => {
                self.state = ConsumeState::Failed;
                Err(e).context(ChunkReadSnafu)
            }
            None => {
                self.state = ConsumeState::Eof;
                Ok(None)
            }
        }
    }

    fn terminal_result(&self) -> Option<Result<Option<RecordBatch>, ChunkError>> {
        match self.state {
            ConsumeState::Open => None,
            ConsumeState::Failed => Some(AbandonedPipelineSnafu.fail()),
            ConsumeState::Eof => Some(Ok(None)),
        }
    }
}

impl Drop for AsyncArrowBatchFetcher {
    /// Aborts the prefetch coordinator when this fetcher holds its handle.
    fn drop(&mut self) {
        if let Some(handle) = &self.abort_handle {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, VecDeque};
    use std::sync::Arc;
    use std::time::Duration;

    use arrow::array::{Array, ArrayRef, Int64Array, RecordBatch};
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::ipc::writer::StreamWriter;
    use tokio::sync::Semaphore;
    use tokio::task::AbortHandle;

    use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

    use super::*;
    use crate::chunks::mock::FileChunkDownloader;
    use crate::chunks::prefetch::{ArrowChunkParser, DownloadChunk, PrefetchChunkReader};
    use crate::chunks::{
        ChunkDownloadData, ChunkError, PrefetchConfig, spawn_initial_arrow_decode,
    };

    /// Serves Arrow IPC bytes held in memory, keyed by `ChunkDownloadData::url`.
    #[derive(Clone)]
    struct InMemoryChunkDownloader {
        payloads: Arc<HashMap<String, Vec<u8>>>,
    }

    impl DownloadChunk for InMemoryChunkDownloader {
        async fn download_chunk(&self, chunk: ChunkDownloadData) -> Result<Vec<u8>, ArrowError> {
            self.payloads
                .get(&chunk.url)
                .cloned()
                .ok_or_else(|| ArrowError::InvalidArgumentError(chunk.url))
        }
    }

    /// Never yields a chunk until the test adds a permit, so the prefetch
    /// coordinator stays parked awaiting an in-flight download.
    #[derive(Clone)]
    struct StalledChunkDownloader {
        gate: Arc<Semaphore>,
    }

    impl DownloadChunk for StalledChunkDownloader {
        async fn download_chunk(&self, _chunk: ChunkDownloadData) -> Result<Vec<u8>, ArrowError> {
            let _permit = self
                .gate
                .acquire()
                .await
                .map_err(|e| ArrowError::ExternalError(Box::new(e)))?;
            Ok(Vec::new())
        }
    }

    fn encode_arrow_ipc(schema: SchemaRef, batches: &[RecordBatch]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut writer = StreamWriter::try_new(&mut buf, schema.as_ref())
                .expect("StreamWriter should accept schema");
            for batch in batches {
                writer
                    .write(batch)
                    .expect("StreamWriter should accept batch");
            }
            writer.finish().expect("StreamWriter should finish");
        }
        buf
    }

    fn int64_batch(values: Vec<i64>) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![Field::new("n", DataType::Int64, false)]));
        RecordBatch::try_new(schema, vec![Arc::new(Int64Array::from(values)) as ArrayRef])
            .expect("RecordBatch should build")
    }

    fn int64_arrow_ipc(values: Vec<i64>) -> Vec<u8> {
        let batch = int64_batch(values);
        let schema = batch.schema();
        encode_arrow_ipc(schema, &[batch])
    }

    fn int64_batch_base64(values: Vec<i64>) -> String {
        BASE64.encode(int64_arrow_ipc(values))
    }

    fn chunk_at(url: &str) -> ChunkDownloadData {
        ChunkDownloadData {
            url: url.to_owned(),
            row_count: 1,
            uncompressed_size: 64,
            compressed_size: 64,
            headers: Default::default(),
        }
    }

    fn single_threaded_prefetch() -> PrefetchConfig {
        PrefetchConfig {
            prefetch_threads: 1,
            memory_limit_mb: 0,
        }
    }

    fn int64_values(batch: &RecordBatch) -> Vec<i64> {
        let column = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .expect("column 0 should be Int64");
        (0..column.len()).map(|i| column.value(i)).collect()
    }

    async fn collect_all(fetcher: &mut AsyncArrowBatchFetcher) -> Vec<i64> {
        let mut values = Vec::new();
        while let Some(batch) = fetcher
            .next_batch()
            .await
            .expect("every batch should decode")
        {
            values.extend(int64_values(&batch));
        }
        values
    }

    async fn await_task_finished(handle: &AbortHandle, context: &str) {
        let finished = tokio::time::timeout(Duration::from_secs(5), async {
            while !handle.is_finished() {
                tokio::task::yield_now().await;
            }
        })
        .await;
        assert!(finished.is_ok(), "{context}");
    }

    #[tokio::test]
    async fn chunk_download_failure_reports_chunk_read_then_abandoned_pipeline() {
        let (reader, abort_handle) = PrefetchChunkReader::open(
            Some(spawn_initial_arrow_decode(int64_batch_base64(vec![1]))),
            VecDeque::from([chunk_at("/nonexistent/async-arrow-chunk.arrow")]),
            FileChunkDownloader,
            ArrowChunkParser,
            &PrefetchConfig::default(),
        )
        .await
        .expect("fetcher should open on inline initial");
        let mut fetcher = AsyncArrowBatchFetcher::new(reader, Some(abort_handle), None);

        fetcher
            .next_batch()
            .await
            .expect("initial batch should succeed")
            .expect("initial batch should exist");

        let result = fetcher.next_batch().await;
        assert!(
            matches!(result, Err(ChunkError::ChunkRead { .. })),
            "got {result:?}"
        );

        let second = fetcher.next_batch().await;
        assert!(
            matches!(second, Err(ChunkError::AbandonedPipeline { .. })),
            "a later poll after a chunk error must keep failing, not Ok(None); got {second:?}"
        );
    }

    #[tokio::test]
    async fn eof_is_sticky_across_repeated_next_batch() {
        let (reader, abort_handle) = PrefetchChunkReader::open(
            Some(spawn_initial_arrow_decode(int64_batch_base64(vec![1, 2]))),
            VecDeque::new(),
            FileChunkDownloader,
            ArrowChunkParser,
            &PrefetchConfig::default(),
        )
        .await
        .expect("fetcher should open on inline initial");
        let mut fetcher = AsyncArrowBatchFetcher::new(reader, Some(abort_handle), None);

        let batch = fetcher
            .next_batch()
            .await
            .expect("initial batch should succeed")
            .expect("initial batch should exist");
        assert_eq!(int64_values(&batch), vec![1, 2]);

        for poll in 0..3 {
            let past_eof = fetcher.next_batch().await;
            assert!(
                matches!(past_eof, Ok(None)),
                "poll {poll} past the end must stay at end-of-stream; got {past_eof:?}"
            );
        }
    }

    #[tokio::test]
    async fn remote_chunks_arrive_in_request_order_behind_a_single_channel_slot() {
        let payloads = HashMap::from([
            ("chunk-1".to_owned(), int64_arrow_ipc(vec![2, 3])),
            ("chunk-2".to_owned(), int64_arrow_ipc(vec![4])),
            ("chunk-3".to_owned(), int64_arrow_ipc(vec![5, 6])),
        ]);
        let (reader, abort_handle) = PrefetchChunkReader::open(
            Some(spawn_initial_arrow_decode(int64_batch_base64(vec![1]))),
            VecDeque::from([
                chunk_at("chunk-1"),
                chunk_at("chunk-2"),
                chunk_at("chunk-3"),
            ]),
            InMemoryChunkDownloader {
                payloads: Arc::new(payloads),
            },
            ArrowChunkParser,
            &single_threaded_prefetch(),
        )
        .await
        .expect("fetcher should open on inline initial");
        let mut fetcher = AsyncArrowBatchFetcher::new(reader, Some(abort_handle), None);

        assert_eq!(collect_all(&mut fetcher).await, vec![1, 2, 3, 4, 5, 6]);
    }

    #[tokio::test]
    async fn dropping_the_fetcher_abandons_a_prefetch_still_awaiting_a_download() {
        let gate = Arc::new(Semaphore::new(0));
        let (reader, abort_handle) = PrefetchChunkReader::open(
            Some(spawn_initial_arrow_decode(int64_batch_base64(vec![1]))),
            VecDeque::from([chunk_at("stalled")]),
            StalledChunkDownloader {
                gate: Arc::clone(&gate),
            },
            ArrowChunkParser,
            &single_threaded_prefetch(),
        )
        .await
        .expect("fetcher should open on inline initial");
        let prefetch_task = abort_handle.clone();
        let mut fetcher = AsyncArrowBatchFetcher::new(reader, Some(abort_handle), None);

        fetcher
            .next_batch()
            .await
            .expect("initial batch should succeed")
            .expect("initial batch should exist");
        tokio::task::yield_now().await;
        assert!(
            !prefetch_task.is_finished(),
            "the prefetch should still be awaiting the stalled download"
        );

        drop(fetcher);

        await_task_finished(
            &prefetch_task,
            "dropping the fetcher must abandon the prefetch without waiting for the download",
        )
        .await;
        assert_eq!(
            gate.available_permits(),
            0,
            "the stalled download should never have been released"
        );
    }
}
