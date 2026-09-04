pub use super::arrow_parser::ArrowChunkParser;
pub use super::http_downloader::HttpChunkDownloader;
pub use super::json_parser::JsonChunkParser;

use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use arrow::array::{RecordBatch, RecordBatchReader};
use arrow::datatypes::{Fields, Schema, SchemaRef};
use arrow::error::ArrowError;
use futures::Stream;
use snafu::{OptionExt, ResultExt};
use tokio::sync::mpsc::error::SendError;
use tokio::task::{AbortHandle, JoinHandle};
use tracing::instrument::WithSubscriber;

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
/// The ticket keeps the memory reservation alive; dropping it releases
/// the bytes back to the budget. Initial (inline) batches use an empty ticket.
struct Chunk {
    batches: VecDeque<RecordBatch>,
    #[allow(dead_code)]
    ticket: MemoryTicket,
}

/// Prefetching chunk reader that downloads and parses chunks in the background.
///
/// [`Iterator`] and [`RecordBatchReader`] wait on the prefetch channel with
/// [`tokio::sync::mpsc::Receiver::blocking_recv`]. [`Stream`] waits with
/// [`tokio::sync::mpsc::Receiver::poll_recv`] and is the in-runtime consumer
/// used by the async Arrow batch fetcher. Both APIs drain the same channel, so
/// a given reader is consumed through one of them.
pub struct PrefetchChunkReader {
    schema: SchemaRef,
    batch_rx: tokio::sync::mpsc::Receiver<Result<Chunk, ArrowError>>,
    /// Buffered batches from the current chunk, paired with the ticket that
    /// keeps the memory reservation alive until all batches are yielded.
    current: Option<Chunk>,
}

type ChunkTask = JoinHandle<Result<Chunk, ArrowError>>;
pub(crate) type InitialChunkTask = JoinHandle<Result<(SchemaRef, Vec<RecordBatch>), ChunkError>>;

impl PrefetchChunkReader {
    /// Starts background download and parse and returns the reader plus an
    /// [`AbortHandle`] for the coordinator task. The async batch fetcher aborts
    /// that handle on drop; a coordinator parked in a download or memory-budget
    /// wait is not woken by dropping the channel receiver alone.
    pub async fn open<D: DownloadChunk, P: ParseChunk>(
        initial: Option<InitialChunkTask>,
        mut chunks: VecDeque<ChunkDownloadData>,
        downloader: D,
        parser: P,
        config: &PrefetchConfig,
    ) -> Result<(Self, AbortHandle), ChunkError> {
        let prefetch_concurrency = config.prefetch_threads.max(1);
        let (tx, rx) = tokio::sync::mpsc::channel(prefetch_concurrency);
        let memory_budget = MemoryBudget::new(config.memory_limit_mb);

        let mut tasks: VecDeque<ChunkTask> = VecDeque::new();
        fill_window(
            &downloader,
            &parser,
            &mut chunks,
            &mut tasks,
            &memory_budget,
            prefetch_concurrency,
        )
        .await;

        let (schema, head_chunk) = match initial {
            Some(initial_chunk_task) => {
                let (schema, batches) = initial_chunk_task.await.context(SpawnBlockingSnafu)??;
                let chunk = if batches.is_empty() {
                    None
                } else {
                    Some(Chunk {
                        batches: VecDeque::from(batches),
                        ticket: MemoryTicket::empty(),
                    })
                };
                (schema, chunk)
            }
            None => {
                let task = tasks.pop_front().context(MissingInitialChunkSnafu)?;
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
                    send_result(&tx, Ok(chunk)).await?;
                }
                drain_window(
                    &downloader,
                    &parser,
                    &mut chunks,
                    &mut tasks,
                    &memory_budget,
                    &tx,
                )
                .await
            }
            .with_current_subscriber(),
        );

        Ok((
            Self {
                schema,
                batch_rx: rx,
                current: None,
            },
            join.abort_handle(),
        ))
    }
}

async fn send_result(
    tx: &tokio::sync::mpsc::Sender<Result<Chunk, ArrowError>>,
    msg: Result<Chunk, ArrowError>,
) -> Result<(), SendError<Result<Chunk, ArrowError>>> {
    if let Err(e) = tx.send(msg).await {
        log_foreign_error!(e, "Failed to send result to channel");
        return Err(e);
    }
    Ok(())
}

/// Spawn up to `concurrency` download+parse tasks from the front of `chunks`.
async fn fill_window(
    downloader: &impl DownloadChunk,
    parser: &impl ParseChunk,
    chunks: &mut VecDeque<ChunkDownloadData>,
    tasks: &mut VecDeque<ChunkTask>,
    budget: &MemoryBudget,
    concurrency: usize,
) {
    while tasks.len() < concurrency {
        let Some(data) = chunks.pop_front() else {
            break;
        };
        let ticket = budget.acquire(data.estimated_memory_mb()).await;
        tasks.push_back(tokio::spawn(
            get_chunk(downloader.clone(), parser.clone(), data, ticket).with_current_subscriber(),
        ));
    }
}

/// Await tasks in order, send results, and refill the window after each completion.
async fn drain_window(
    downloader: &impl DownloadChunk,
    parser: &impl ParseChunk,
    chunks: &mut VecDeque<ChunkDownloadData>,
    tasks: &mut VecDeque<ChunkTask>,
    budget: &MemoryBudget,
    tx: &tokio::sync::mpsc::Sender<Result<Chunk, ArrowError>>,
) -> Result<(), SendError<Result<Chunk, ArrowError>>> {
    while let Some(task) = tasks.pop_front() {
        let result = task
            .await
            .unwrap_or_else(|e| Err(ArrowError::ExternalError(Box::new(e))));
        let is_err = result.is_err();
        send_result(tx, result).await?;
        if is_err {
            return Ok(());
        }
        if let Some(data) = chunks.pop_front() {
            let ticket = budget.acquire(data.estimated_memory_mb()).await;
            tasks.push_back(tokio::spawn(
                get_chunk(downloader.clone(), parser.clone(), data, ticket)
                    .with_current_subscriber(),
            ));
        }
    }
    Ok(())
}

async fn get_chunk(
    downloader: impl DownloadChunk,
    parser: impl ParseChunk,
    data: ChunkDownloadData,
    ticket: MemoryTicket,
) -> Result<Chunk, ArrowError> {
    let bytes = downloader.download_chunk(data).await?;
    // Arrow IPC / JSON→Arrow decode is CPU-bound; run it on the blocking pool so
    // it doesn't occupy this runtime worker (result chunks are routinely multi-MB).
    let batches = tokio::task::spawn_blocking(move || parser.parse_chunk(bytes))
        .await
        .map_err(|e| ArrowError::ExternalError(Box::new(e)))??;
    Ok(Chunk {
        batches: batches.into(),
        ticket,
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
