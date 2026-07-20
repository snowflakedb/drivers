pub use super::arrow_parser::ArrowChunkParser;
pub use super::http_downloader::HttpChunkDownloader;
pub use super::json_parser::JsonChunkParser;

use std::collections::VecDeque;
use std::marker::PhantomData;

use arrow::array::{RecordBatch, RecordBatchReader};
use arrow::datatypes::SchemaRef;
use arrow::error::ArrowError;
use snafu::ResultExt;
use tokio::sync::mpsc::error::SendError;
use tracing::instrument::WithSubscriber;

use super::memory_budget::{MemoryBudget, MemoryTicket};
use super::{ChunkDownloadData, ChunkError, ChunkReadSnafu, PrefetchConfig};
use crate::log_foreign_error;

pub trait DownloadChunk: Send + Sync + Clone + 'static {
    fn download_chunk(
        &self,
        chunk: ChunkDownloadData,
        cancel: tokio_util::sync::CancellationToken,
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
/// # Safety
///
/// This reader uses [`tokio::sync::mpsc::Receiver::blocking_recv`] in its
/// [`Iterator`] implementation. It **must not** be iterated from within an
/// active Tokio runtime (e.g. inside `tokio::spawn`, `block_on`, or an
/// `async` block), as this will deadlock or panic. Consume the iterator from
/// a synchronous context or from a dedicated blocking thread
/// (e.g. [`tokio::task::spawn_blocking`]).
pub struct PrefetchChunkReader<D: DownloadChunk, P: ParseChunk> {
    schema: SchemaRef,
    batch_rx: tokio::sync::mpsc::Receiver<Result<Chunk, ArrowError>>,
    /// Buffered batches from the current chunk, paired with the ticket that
    /// keeps the memory reservation alive until all batches are yielded.
    current: Option<Chunk>,
    phantom: PhantomData<(D, P)>,
}

impl<D: DownloadChunk, P: ParseChunk> PrefetchChunkReader<D, P> {
    pub async fn reader<R: RecordBatchReader + Send>(
        initial: R,
        chunks: VecDeque<ChunkDownloadData>,
        downloader: D,
        parser: P,
        config: &PrefetchConfig,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<Box<dyn RecordBatchReader + Send>, ChunkError> {
        let schema = initial.schema();
        let initial = initial
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .context(ChunkReadSnafu)?;

        let prefetch_concurrency = config.prefetch_threads;
        let (tx, rx) = tokio::sync::mpsc::channel(prefetch_concurrency);
        let memory_budget = MemoryBudget::new(config.memory_limit_mb);

        tokio::spawn(
            Self::prefetch_batches(
                downloader,
                parser,
                chunks,
                initial,
                tx,
                prefetch_concurrency,
                memory_budget,
                cancel,
            )
            .with_current_subscriber(),
        );

        Ok(Box::new(Self {
            schema,
            batch_rx: rx,
            current: None,
            phantom: PhantomData,
        }))
    }

    async fn prefetch_batches(
        downloader: D,
        parser: P,
        mut chunks: VecDeque<ChunkDownloadData>,
        initial: Vec<RecordBatch>,
        tx: tokio::sync::mpsc::Sender<Result<Chunk, ArrowError>>,
        prefetch_concurrency: usize,
        memory_budget: MemoryBudget,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<(), SendError<Result<Chunk, ArrowError>>> {
        let send = |msg: Result<Chunk, ArrowError>| {
            let tx = &tx;
            async move {
                if let Err(e) = tx.send(msg).await {
                    log_foreign_error!(e, "Failed to send result to channel");
                    return Err(e);
                }
                Ok(())
            }
        };

        if !initial.is_empty() {
            send(Ok(Chunk {
                batches: VecDeque::from(initial),
                ticket: MemoryTicket::empty(),
            }))
            .await?;
        }

        let mut chunk_tasks: VecDeque<tokio::task::JoinHandle<Result<Chunk, ArrowError>>> =
            VecDeque::new();

        for _ in 0..prefetch_concurrency {
            if let Some(data) = chunks.pop_front() {
                let estimate = data.estimated_memory_mb();
                let ticket = memory_budget.acquire(estimate).await;

                let d = downloader.clone();
                let p = parser.clone();
                let task_cancel = cancel.clone();
                chunk_tasks.push_back(tokio::task::spawn(
                    get_chunk(d, p, data, ticket, task_cancel).with_current_subscriber(),
                ));
            }
        }

        while let Some(task) = chunk_tasks.pop_front() {
            match task.await {
                Err(e) => {
                    return send(Err(ArrowError::ExternalError(Box::new(e)))).await;
                }
                Ok(Err(e)) => {
                    return send(Err(e)).await;
                }
                Ok(Ok(chunk)) => {
                    send(Ok(chunk)).await?;
                }
            }

            if let Some(data) = chunks.pop_front() {
                let next_estimate = data.estimated_memory_mb();
                let ticket = memory_budget.acquire(next_estimate).await;

                let d = downloader.clone();
                let p = parser.clone();
                let task_cancel = cancel.clone();
                chunk_tasks.push_back(tokio::task::spawn(
                    get_chunk(d, p, data, ticket, task_cancel).with_current_subscriber(),
                ));
            }
        }

        Ok(())
    }
}

async fn get_chunk(
    downloader: impl DownloadChunk,
    parser: impl ParseChunk,
    data: ChunkDownloadData,
    ticket: MemoryTicket,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<Chunk, ArrowError> {
    let bytes = downloader.download_chunk(data, cancel).await?;
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

impl<D: DownloadChunk + 'static, P: ParseChunk + 'static> Iterator for PrefetchChunkReader<D, P> {
    type Item = Result<RecordBatch, ArrowError>;

    #[tracing::instrument(
        name = "core_batch_wait",
        target = "sf_core::perf",
        level = "trace",
        skip_all
    )]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ref mut chunk) = self.current {
                if let Some(batch) = chunk.batches.pop_front() {
                    return Some(Ok(batch));
                }
                self.current = None;
            }

            match self.batch_rx.blocking_recv() {
                Some(Ok(chunk)) => {
                    self.current = Some(chunk);
                }
                Some(Err(e)) => return Some(Err(e)),
                None => return None,
            }
        }
    }
}

impl<D: DownloadChunk + 'static, P: ParseChunk + 'static> RecordBatchReader
    for PrefetchChunkReader<D, P>
{
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}
