pub use super::arrow_parser::ArrowChunkParser;
pub use super::http_downloader::HttpChunkDownloader;
pub use super::json_parser::JsonChunkParser;

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use arrow::array::{RecordBatch, RecordBatchReader};
use arrow::datatypes::{Fields, Schema, SchemaRef};
use arrow::error::ArrowError;
use futures::Stream;
use snafu::{OptionExt, ResultExt};
use tokio::sync::Semaphore;
use tokio::sync::mpsc::error::SendError;
use tokio::task::{AbortHandle, JoinHandle};
use tracing::instrument::WithSubscriber;

use super::batch_budget::{BatchBudget, BatchTicket};
use super::memory_budget::{MemoryBudget, MemoryTicket};
use super::{
    ChunkDownloadData, ChunkError, ChunkReadSnafu, MissingInitialChunkSnafu, PrefetchConfig,
    SpawnBlockingSnafu,
};
use crate::log_foreign_error;

pub trait DownloadChunk: Send + Sync + Clone + 'static {
    fn download_chunk(
        &self,
        chunk: ChunkDownloadData,
    ) -> impl Future<Output = Result<Vec<u8>, ArrowError>> + Send;
}

pub trait ParseChunk: Send + Sync + Clone + 'static {
    fn parse_chunk(&self, data: Vec<u8>) -> Result<Vec<RecordBatch>, ArrowError>;
}

/// Channel message carrying all record batches from a single chunk.
///
/// The memory ticket keeps the reservation alive until the chunk is drained.
/// The batch ticket accounts for every batch handed to the consumer, including
/// inline batches, and releases capacity as batches are yielded.
struct Chunk {
    batches: VecDeque<RecordBatch>,
    #[allow(dead_code)]
    ticket: MemoryTicket,
    batch_ticket: BatchTicket,
}

/// Prefetching chunk reader that downloads and parses chunks in the background.
///
/// [`Iterator`] and [`RecordBatchReader`] wait on the prefetch channel with
/// [`tokio::sync::mpsc::Receiver::blocking_recv`]. [`Stream`] waits with
/// [`tokio::sync::mpsc::Receiver::poll_recv`] and is the in-runtime consumer
/// used by the async Arrow batch fetcher. Both APIs drain the same channel, so
/// a given reader is consumed through one of them.
///
/// Dropping the reader aborts the coordinator and the chunk downloads it still
/// owns. A coordinator parked in a download or memory-budget wait is not woken
/// by dropping the channel receiver alone.
pub struct PrefetchChunkReader {
    schema: SchemaRef,
    batch_rx: tokio::sync::mpsc::Receiver<Result<Chunk, ArrowError>>,
    /// Buffered batches from the current chunk, paired with the ticket that
    /// keeps the memory reservation alive until all batches are yielded.
    current: Option<Chunk>,
    abort_handle: AbortHandle,
}

type ChunkTask = AbortOnDrop<Result<Chunk, ArrowError>>;
pub(crate) type InitialChunkTask = JoinHandle<Result<(SchemaRef, Vec<RecordBatch>), ChunkError>>;

/// Aborts the task on drop. [`JoinHandle`]'s own drop detaches it, which leaves
/// a chunk download running after the prefetch that spawned it is gone.
struct AbortOnDrop<T> {
    handle: JoinHandle<T>,
}

impl<T> AbortOnDrop<T> {
    fn new(handle: JoinHandle<T>) -> Self {
        Self { handle }
    }
}

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

impl<T> Future for AbortOnDrop<T> {
    type Output = Result<T, tokio::task::JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().handle).poll(cx)
    }
}

impl PrefetchChunkReader {
    /// Starts background download and parse and returns the reader plus an
    /// [`AbortHandle`] for the coordinator task. Dropping the reader aborts
    /// that task, and so does aborting the returned handle. The async batch
    /// fetcher keeps the handle and aborts it on drop.
    pub async fn open<D: DownloadChunk, P: ParseChunk>(
        initial: Option<InitialChunkTask>,
        chunks: VecDeque<ChunkDownloadData>,
        downloader: D,
        parser: P,
        config: &PrefetchConfig,
    ) -> Result<(Self, AbortHandle), ChunkError> {
        let initial = initial.map(AbortOnDrop::new);
        let prefetch_concurrency = config.prefetch_threads.max(1);
        let (tx, rx) = tokio::sync::mpsc::channel(prefetch_concurrency);
        let batch_budget = config
            .result_queue_size
            .filter(|&limit| limit > 0)
            .map(BatchBudget::new);
        let mut pipeline = ChunkPipeline::new(
            downloader,
            parser,
            chunks,
            prefetch_concurrency,
            config.memory_limit_mb,
        );
        pipeline.fill().await;

        let (schema, head_chunk) = match initial {
            Some(initial_chunk_task) => {
                let (schema, batches) = initial_chunk_task.await.context(SpawnBlockingSnafu)??;
                let chunk = if batches.is_empty() {
                    None
                } else {
                    Some(Chunk {
                        batches: VecDeque::from(batches),
                        ticket: MemoryTicket::empty(),
                        batch_ticket: BatchTicket::empty(),
                    })
                };
                (schema, chunk)
            }
            None => {
                let task = pipeline.next_task().context(MissingInitialChunkSnafu)?;
                let chunk = task
                    .await
                    .context(SpawnBlockingSnafu)?
                    .context(ChunkReadSnafu)?;
                let schema = chunk
                    .batches
                    .front()
                    .map(RecordBatch::schema)
                    .unwrap_or_else(|| Arc::new(Schema::new(Fields::empty())));
                (schema, Some(chunk))
            }
        };

        let join = tokio::spawn(
            async move {
                if let Some(chunk) = head_chunk {
                    send_result(&tx, Ok(chunk), batch_budget.as_ref()).await?;
                }
                pipeline.drain(&tx, batch_budget.as_ref()).await
            }
            .with_current_subscriber(),
        );
        let abort_handle = join.abort_handle();

        Ok((
            Self {
                schema,
                batch_rx: rx,
                current: None,
                abort_handle: abort_handle.clone(),
            },
            abort_handle,
        ))
    }
}

impl Drop for PrefetchChunkReader {
    fn drop(&mut self) {
        self.abort_handle.abort();
    }
}

async fn send_result(
    tx: &tokio::sync::mpsc::Sender<Result<Chunk, ArrowError>>,
    mut msg: Result<Chunk, ArrowError>,
    budget: Option<&BatchBudget>,
) -> Result<(), SendError<Result<Chunk, ArrowError>>> {
    if let Some((budget, batches)) = budget.zip(msg.as_ref().ok().map(|chunk| chunk.batches.len()))
    {
        let ticket = tokio::select! {
            biased;
            _ = tx.closed() => return Err(SendError(msg)),
            ticket = budget.acquire(batches) => ticket,
        };
        if let Ok(chunk) = &mut msg {
            chunk.batch_ticket = ticket;
        }
    }
    if let Err(e) = tx.send(msg).await {
        log_foreign_error!(e, "Failed to send result to channel");
        return Err(e);
    }
    Ok(())
}

struct ChunkPipeline<D, P> {
    downloader: D,
    parser: P,
    pending: VecDeque<ChunkDownloadData>,
    tasks: VecDeque<ChunkTask>,
    /// Twice the download concurrency, so a freed download slot goes to the
    /// next queued chunk without waiting for the head.
    window: usize,
    budget: MemoryBudget,
    /// Slots go to tasks in the order they first poll, not spawn order, so on a
    /// multi-threaded runtime a later chunk can occasionally start before the
    /// head, delaying it by at most one download round.
    downloads: Arc<Semaphore>,
}

impl<D: DownloadChunk, P: ParseChunk> ChunkPipeline<D, P> {
    fn new(
        downloader: D,
        parser: P,
        pending: VecDeque<ChunkDownloadData>,
        concurrency: usize,
        memory_limit_mb: u32,
    ) -> Self {
        Self {
            downloader,
            parser,
            pending,
            tasks: VecDeque::new(),
            window: concurrency * 2,
            budget: MemoryBudget::new(memory_limit_mb),
            downloads: Arc::new(Semaphore::new(concurrency)),
        }
    }

    fn next_task(&mut self) -> Option<ChunkTask> {
        self.tasks.pop_front()
    }

    /// Spawn download+parse tasks from the front of `pending` until the window is full.
    ///
    /// Budget is awaited only when no task is queued: queued tasks may hold
    /// tickets that are released only after they are sent, so waiting behind
    /// them would deadlock.
    async fn fill(&mut self) {
        while self.tasks.len() < self.window {
            let Some(data) = self.pending.pop_front() else {
                break;
            };
            let mb = data.estimated_memory_mb();
            let ticket = if self.tasks.is_empty() {
                self.budget.acquire(mb).await
            } else if let Some(ticket) = self.budget.try_acquire(mb) {
                ticket
            } else {
                self.pending.push_front(data);
                break;
            };
            self.tasks.push_back(AbortOnDrop::new(tokio::spawn(
                get_chunk(
                    self.downloader.clone(),
                    self.parser.clone(),
                    data,
                    ticket,
                    self.downloads.clone(),
                )
                .with_current_subscriber(),
            )));
        }
    }

    /// Await tasks in order, send results, and refill the window after each completion.
    async fn drain(
        &mut self,
        tx: &tokio::sync::mpsc::Sender<Result<Chunk, ArrowError>>,
        batch_budget: Option<&BatchBudget>,
    ) -> Result<(), SendError<Result<Chunk, ArrowError>>> {
        while let Some(task) = self.next_task() {
            let result = task
                .await
                .unwrap_or_else(|e| Err(ArrowError::ExternalError(Box::new(e))));
            let is_err = result.is_err();
            send_result(tx, result, batch_budget).await?;
            if is_err {
                return Ok(());
            }
            self.fill().await;
        }
        Ok(())
    }
}

async fn get_chunk(
    downloader: impl DownloadChunk,
    parser: impl ParseChunk,
    data: ChunkDownloadData,
    ticket: MemoryTicket,
    downloads: Arc<Semaphore>,
) -> Result<Chunk, ArrowError> {
    let permit = downloads
        .acquire_owned()
        .await
        .map_err(|e| ArrowError::ExternalError(Box::new(e)))?;
    let bytes = downloader.download_chunk(data).await?;
    drop(permit);
    // Arrow IPC / JSON→Arrow decode is CPU-bound; run it on the blocking pool so
    // it doesn't occupy this runtime worker (result chunks are routinely multi-MB).
    let batches = tokio::task::spawn_blocking(move || parser.parse_chunk(bytes))
        .await
        .map_err(|e| ArrowError::ExternalError(Box::new(e)))??;
    Ok(Chunk {
        batches: batches.into(),
        ticket,
        batch_ticket: BatchTicket::empty(),
    })
}

/// # Panics
///
/// [`Iterator::next`] panics when the calling thread is inside a Tokio runtime.
impl Iterator for PrefetchChunkReader {
    type Item = Result<RecordBatch, ArrowError>;

    #[tracing::instrument(
        name = "core_batch_wait",
        target = "sf_core::perf",
        level = "trace",
        skip_all
    )]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(batch) = pop_current_batch(&mut self.current) {
                return Some(Ok(batch));
            }
            match take_channel_chunk(self.batch_rx.blocking_recv()) {
                Ok(Some(chunk)) => self.current = Some(chunk),
                Ok(None) => return None,
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

impl RecordBatchReader for PrefetchChunkReader {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

impl Stream for PrefetchChunkReader {
    type Item = Result<RecordBatch, ArrowError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            if let Some(batch) = pop_current_batch(&mut this.current) {
                return Poll::Ready(Some(Ok(batch)));
            }
            match this.batch_rx.poll_recv(cx) {
                Poll::Ready(msg) => match take_channel_chunk(msg) {
                    Ok(Some(chunk)) => this.current = Some(chunk),
                    Ok(None) => return Poll::Ready(None),
                    Err(e) => return Poll::Ready(Some(Err(e))),
                },
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

fn pop_current_batch(current: &mut Option<Chunk>) -> Option<RecordBatch> {
    let chunk = current.as_mut()?;
    if let Some(batch) = chunk.batches.pop_front() {
        chunk.batch_ticket.batch_consumed(chunk.batches.len());
        return Some(batch);
    }
    *current = None;
    None
}

fn take_channel_chunk(msg: Option<Result<Chunk, ArrowError>>) -> Result<Option<Chunk>, ArrowError> {
    match msg {
        Some(Ok(chunk)) => Ok(Some(chunk)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use arrow::array::{ArrayRef, Int64Array};
    use arrow::datatypes::{DataType, Field};
    use futures::StreamExt;
    use tokio::sync::{Semaphore, mpsc};

    use super::*;

    #[derive(Clone, Default)]
    struct DelayedDownloader {
        state: Arc<Mutex<DownloadState>>,
    }

    #[derive(Default)]
    struct DownloadState {
        active: usize,
        max_active: usize,
        finished: Vec<usize>,
    }

    impl DownloadChunk for DelayedDownloader {
        async fn download_chunk(&self, chunk: ChunkDownloadData) -> Result<Vec<u8>, ArrowError> {
            let index: usize = chunk.url.parse().unwrap();
            {
                let mut state = self.state.lock().unwrap();
                state.active += 1;
                state.max_active = state.max_active.max(state.active);
            }
            let delay = if index == 0 { 100 } else { 10 };
            tokio::time::sleep(Duration::from_millis(delay)).await;
            let mut state = self.state.lock().unwrap();
            state.active -= 1;
            state.finished.push(index);
            Ok(Vec::new())
        }
    }

    #[derive(Clone)]
    struct MockDownloader;

    impl DownloadChunk for MockDownloader {
        async fn download_chunk(&self, chunk: ChunkDownloadData) -> Result<Vec<u8>, ArrowError> {
            Ok(chunk.url.into_bytes())
        }
    }

    #[derive(Clone)]
    struct EmptyParser;

    impl ParseChunk for EmptyParser {
        fn parse_chunk(&self, _data: Vec<u8>) -> Result<Vec<RecordBatch>, ArrowError> {
            Ok(Vec::new())
        }
    }

    fn chunk(index: usize) -> ChunkDownloadData {
        ChunkDownloadData {
            url: index.to_string(),
            row_count: 1,
            uncompressed_size: 1,
            compressed_size: 1,
            headers: HashMap::new(),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn slow_head_chunk_does_not_idle_the_other_download_slots() {
        let downloader = DelayedDownloader::default();
        let config = PrefetchConfig {
            prefetch_threads: 2,
            memory_limit_mb: 0,
            ..PrefetchConfig::default()
        };

        let (reader, _abort) = PrefetchChunkReader::open(
            None,
            (0..6).map(chunk).collect(),
            downloader.clone(),
            EmptyParser,
            &config,
        )
        .await
        .unwrap();
        drop(reader);

        let state = downloader.state.lock().unwrap();
        assert_eq!(state.finished.first(), Some(&1));
        let head = state.finished.iter().position(|&i| i == 0).unwrap();
        assert!(
            head >= 3,
            "chunks 1..=3 download while chunk 0 is in flight: {:?}",
            state.finished
        );
        assert_eq!(state.max_active, 2);
    }

    #[tokio::test(start_paused = true)]
    async fn window_deeper_than_memory_budget_respects_budget() {
        let downloader = DelayedDownloader::default();
        let config = PrefetchConfig {
            prefetch_threads: 2,
            memory_limit_mb: 2,
            ..PrefetchConfig::default()
        };

        let (mut reader, _abort) = tokio::time::timeout(
            Duration::from_secs(10),
            PrefetchChunkReader::open(
                None,
                (0..6).map(chunk).collect(),
                downloader.clone(),
                EmptyParser,
                &config,
            ),
        )
        .await
        .expect("open must not wait for budget held by unconsumed chunks")
        .unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(downloader.state.lock().unwrap().finished.len(), 2);

        tokio::time::timeout(Duration::from_secs(10), async {
            while futures::StreamExt::next(&mut reader).await.is_some() {}
        })
        .await
        .expect("all chunks arrive once consumed");
        assert_eq!(downloader.state.lock().unwrap().finished.len(), 6);
    }

    #[derive(Clone)]
    struct MockParser {
        schema: SchemaRef,
        batches_per_chunk: usize,
        produced: Arc<AtomicUsize>,
        parsed: mpsc::UnboundedSender<()>,
    }

    impl ParseChunk for MockParser {
        fn parse_chunk(&self, data: Vec<u8>) -> Result<Vec<RecordBatch>, ArrowError> {
            let id = std::str::from_utf8(&data).unwrap().parse::<i64>().unwrap();
            let batch = RecordBatch::try_new(
                self.schema.clone(),
                vec![Arc::new(Int64Array::from(vec![id])) as ArrayRef],
            )?;
            let batches = vec![batch; self.batches_per_chunk];
            self.produced.fetch_add(batches.len(), Ordering::SeqCst);
            self.parsed.send(()).unwrap();
            Ok(batches)
        }
    }

    #[derive(Clone)]
    struct StalledDownloader {
        started: mpsc::UnboundedSender<()>,
        gate: Arc<Semaphore>,
    }

    impl DownloadChunk for StalledDownloader {
        async fn download_chunk(&self, _chunk: ChunkDownloadData) -> Result<Vec<u8>, ArrowError> {
            self.started.send(()).unwrap();
            let _permit = self.gate.acquire().await.unwrap();
            Ok(b"1".to_vec())
        }
    }

    fn schema() -> SchemaRef {
        Arc::new(Schema::new(vec![Field::new("n", DataType::Int64, false)]))
    }

    fn batch(schema: SchemaRef, n: i64) -> RecordBatch {
        RecordBatch::try_new(
            schema,
            vec![Arc::new(Int64Array::from(vec![n])) as ArrayRef],
        )
        .unwrap()
    }

    fn initial(schema: SchemaRef, count: usize) -> InitialChunkTask {
        tokio::spawn(async move { Ok((schema.clone(), vec![batch(schema, 0); count])) })
    }

    fn remote(id: usize) -> ChunkDownloadData {
        ChunkDownloadData {
            url: id.to_string(),
            row_count: 1,
            compressed_size: 1,
            uncompressed_size: 1,
            headers: Default::default(),
        }
    }

    fn parser(
        schema: SchemaRef,
        batches_per_chunk: usize,
    ) -> (MockParser, mpsc::UnboundedReceiver<()>, Arc<AtomicUsize>) {
        let (parsed, rx) = mpsc::unbounded_channel();
        let produced = Arc::new(AtomicUsize::new(0));
        (
            MockParser {
                schema,
                batches_per_chunk,
                produced: produced.clone(),
                parsed,
            },
            rx,
            produced,
        )
    }

    async fn wait_for_channel(reader: &PrefetchChunkReader) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while reader.batch_rx.is_empty() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("coordinator should hand off a chunk");
    }

    async fn assert_channel_stays_empty(reader: &PrefetchChunkReader) {
        let next_chunk = async {
            while reader.batch_rx.is_empty() {
                tokio::task::yield_now().await;
            }
        };
        assert!(
            tokio::time::timeout(Duration::from_millis(40), next_chunk)
                .await
                .is_err(),
            "prefetch handed off a chunk before its batches fitted the queue limit"
        );
    }

    async fn next_value(reader: &mut PrefetchChunkReader) -> i64 {
        let batch = tokio::time::timeout(Duration::from_secs(2), StreamExt::next(reader))
            .await
            .expect("the next batch should arrive")
            .expect("a batch should exist")
            .expect("a batch should parse");
        batch
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .value(0)
    }

    #[tokio::test]
    async fn unset_queue_preserves_prefetch_concurrency_and_channel_capacity() {
        let schema = schema();
        let (parser, _parsed, _produced) = parser(schema.clone(), 1);
        let gate = Arc::new(Semaphore::new(0));
        let (started, mut started_rx) = mpsc::unbounded_channel();
        let config = PrefetchConfig {
            prefetch_threads: 3,
            memory_limit_mb: 0,
            result_queue_size: None,
            ..PrefetchConfig::default()
        };
        let (reader, abort) = PrefetchChunkReader::open(
            Some(initial(schema, 1)),
            (1..=4).map(remote).collect(),
            StalledDownloader { started, gate },
            parser,
            &config,
        )
        .await
        .unwrap();

        for _ in 0..3 {
            tokio::time::timeout(Duration::from_secs(2), started_rx.recv())
                .await
                .expect("three concurrent downloads should start")
                .expect("download notifications should stay open");
        }
        assert_eq!(reader.batch_rx.max_capacity(), 3);
        assert!(
            started_rx.try_recv().is_err(),
            "the fourth download exceeds the concurrency limit"
        );
        abort.abort();
    }

    #[tokio::test]
    async fn zero_queue_size_leaves_batch_handoff_unbounded() {
        let schema = schema();
        let (parser, _parsed, _produced) = parser(schema.clone(), 1);
        let config = PrefetchConfig {
            result_queue_size: Some(0),
            ..PrefetchConfig::default()
        };
        let (mut reader, _abort) = PrefetchChunkReader::open(
            Some(initial(schema, 2)),
            [remote(1)].into(),
            MockDownloader,
            parser,
            &config,
        )
        .await
        .unwrap();
        for value in [0, 0, 1] {
            assert_eq!(next_value(&mut reader).await, value);
        }
        assert!(StreamExt::next(&mut reader).await.is_none());
    }

    #[tokio::test]
    async fn batch_queue_blocks_chunk_handoff_until_stream_yields_batches() {
        let schema = schema();
        let (parser, mut parsed, produced) = parser(schema.clone(), 2);
        let config = PrefetchConfig {
            prefetch_threads: 1,
            memory_limit_mb: 0,
            result_queue_size: Some(2),
            ..PrefetchConfig::default()
        };
        let (mut reader, abort) = PrefetchChunkReader::open(
            Some(initial(schema, 2)),
            [remote(1), remote(2)].into(),
            MockDownloader,
            parser,
            &config,
        )
        .await
        .unwrap();
        wait_for_channel(&reader).await;
        for _ in 0..2 {
            tokio::time::timeout(Duration::from_secs(2), parsed.recv())
                .await
                .unwrap()
                .unwrap();
        }
        assert_eq!(produced.load(Ordering::SeqCst), 4);
        assert_eq!(reader.batch_rx.len(), 1);

        assert_eq!(next_value(&mut reader).await, 0);
        assert_channel_stays_empty(&reader).await;
        assert_eq!(next_value(&mut reader).await, 0);
        wait_for_channel(&reader).await;
        assert_eq!(produced.load(Ordering::SeqCst), 4);
        assert_eq!(reader.batch_rx.len(), 1);

        assert_eq!(next_value(&mut reader).await, 1);
        assert_channel_stays_empty(&reader).await;
        assert_eq!(next_value(&mut reader).await, 1);
        assert_eq!(next_value(&mut reader).await, 2);
        assert_eq!(next_value(&mut reader).await, 2);
        assert!(StreamExt::next(&mut reader).await.is_none());
        abort.abort();
    }

    #[tokio::test]
    async fn oversized_chunk_is_admitted_and_does_not_deadlock_next_chunk() {
        let schema = schema();
        let (parser, _parsed, _produced) = parser(schema.clone(), 1);
        let config = PrefetchConfig {
            prefetch_threads: 1,
            memory_limit_mb: 0,
            result_queue_size: Some(2),
            ..PrefetchConfig::default()
        };
        let (mut reader, abort) = PrefetchChunkReader::open(
            Some(initial(schema, 4)),
            [remote(1)].into(),
            MockDownloader,
            parser,
            &config,
        )
        .await
        .unwrap();
        for value in [0, 0, 0, 0, 1] {
            assert_eq!(next_value(&mut reader).await, value);
        }
        abort.abort();
    }

    #[tokio::test]
    async fn blocking_reader_releases_capacity_before_draining_current_chunk() {
        let schema = schema();
        let (parser, _parsed, _produced) = parser(schema.clone(), 1);
        let config = PrefetchConfig {
            prefetch_threads: 1,
            memory_limit_mb: 0,
            result_queue_size: Some(2),
            ..PrefetchConfig::default()
        };
        let (mut reader, _abort) = PrefetchChunkReader::open(
            Some(initial(schema, 2)),
            [remote(1)].into(),
            MockDownloader,
            parser,
            &config,
        )
        .await
        .unwrap();

        reader = tokio::task::spawn_blocking(move || {
            Iterator::next(&mut reader).unwrap().unwrap();
            reader
        })
        .await
        .unwrap();
        assert_eq!(reader.current.as_ref().unwrap().batches.len(), 1);
        // The first chunk is still held by the reader, so this handoff needs
        // the permit released for its first batch, not a dropped chunk.
        wait_for_channel(&reader).await;

        let values = tokio::task::spawn_blocking(move || {
            Iterator::map(reader, |batch| {
                batch
                    .unwrap()
                    .column(0)
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap()
                    .value(0)
            })
            .collect::<Vec<_>>()
        })
        .await
        .unwrap();
        assert_eq!(values, [0, 1]);
    }

    #[tokio::test]
    async fn closing_receiver_wakes_coordinator_waiting_for_batch_capacity() {
        let schema = schema();
        let (parser, mut parsed, _produced) = parser(schema.clone(), 1);
        let config = PrefetchConfig {
            prefetch_threads: 1,
            memory_limit_mb: 0,
            result_queue_size: Some(1),
            ..PrefetchConfig::default()
        };
        let (mut reader, abort) = PrefetchChunkReader::open(
            Some(initial(schema, 1)),
            [remote(1)].into(),
            MockDownloader,
            parser,
            &config,
        )
        .await
        .unwrap();
        wait_for_channel(&reader).await;
        tokio::time::timeout(Duration::from_secs(2), parsed.recv())
            .await
            .unwrap()
            .unwrap();
        reader.batch_rx.close();
        tokio::time::timeout(Duration::from_secs(2), async {
            while !abort.is_finished() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("closing the channel must cancel the batch-budget wait");
    }
}
