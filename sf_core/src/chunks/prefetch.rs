pub use super::arrow_parser::ArrowChunkParser;
pub use super::http_downloader::HttpChunkDownloader;
pub use super::json_parser::JsonChunkParser;

use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::Arc;

use arrow::array::{RecordBatch, RecordBatchReader};
use arrow::datatypes::SchemaRef;
use arrow::error::ArrowError;
use snafu::ResultExt;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use super::{ChunkDownloadData, ChunkError, ChunkReadingSnafu, PrefetchConfig};

pub trait DownloadChunk: Send + Sync + Clone + 'static {
    fn download_chunk(
        &self,
        chunk: ChunkDownloadData,
    ) -> impl Future<Output = Result<Vec<u8>, ArrowError>> + Send;
}

pub trait ParseChunk: Send + Sync + Clone + 'static {
    fn parse_chunk(&self, data: Vec<u8>) -> Result<Vec<RecordBatch>, ArrowError>;
}

/// Tracks a megabyte-granularity memory budget for prefetched chunks.
///
/// Cheap to clone (inner state is behind [`Arc`]). Hand out [`MemoryTicket`]s
/// via [`acquire`](Self::acquire). Each ticket reserves `n` MB and
/// automatically returns them to the budget when dropped.
///
/// When a chunk is larger than the entire budget, [`acquire`](Self::acquire)
/// requests all available permits, effectively waiting for exclusive access.
/// This prevents deadlock on oversized chunks.
///
/// Backed by a [`tokio::sync::Semaphore`] (1 permit = 1 MB) which provides
/// FIFO-fair async waiting with no polling loops.
#[derive(Clone)]
pub(crate) struct MemoryBudget {
    semaphore: Arc<Semaphore>,
    limit_mb: u32,
}

/// RAII guard for a memory reservation.
///
/// Dropping the ticket returns its permits to the parent [`MemoryBudget`].
#[allow(dead_code)]
pub(crate) struct MemoryTicket(Option<OwnedSemaphorePermit>);

impl MemoryTicket {
    fn empty() -> Self {
        Self(None)
    }
}

impl MemoryBudget {
    fn new(limit_mb: u64) -> Self {
        let limit_mb = u32::try_from(limit_mb).unwrap_or(u32::MAX);
        Self {
            semaphore: Arc::new(Semaphore::new(limit_mb as usize)),
            limit_mb,
        }
    }

    /// Wait until `mb` worth of permits are available, then reserve them.
    ///
    /// If `mb` exceeds the total budget, all permits are acquired instead,
    /// which waits for exclusive access (no other tickets outstanding).
    async fn acquire(&self, mb: u64) -> MemoryTicket {
        if self.limit_mb == 0 {
            return MemoryTicket(None);
        }

        let permits = u32::try_from(mb).unwrap_or(u32::MAX).min(self.limit_mb);

        let permit = self
            .semaphore
            .clone()
            .acquire_many_owned(permits)
            .await
            .expect("semaphore is never closed");

        MemoryTicket(Some(permit))
    }
}

/// Channel message carrying all record batches from a single chunk.
///
/// The ticket keeps the memory reservation alive; dropping it releases
/// the bytes back to the budget. Initial (inline) batches use an empty ticket.
struct Chunk {
    batches: VecDeque<RecordBatch>,
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
    ) -> Result<Box<dyn RecordBatchReader + Send>, ChunkError> {
        let schema = initial.schema();
        let initial = initial
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .context(ChunkReadingSnafu)?;

        let prefetch_concurrency = config.prefetch_threads;
        let (tx, rx) = tokio::sync::mpsc::channel(prefetch_concurrency);
        let memory_budget = MemoryBudget::new(config.memory_limit_mb);

        tokio::spawn(Self::prefetch_batches(
            downloader,
            parser,
            chunks,
            initial,
            tx,
            prefetch_concurrency,
            memory_budget,
        ));

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
    ) -> Result<(), SendError<Result<Chunk, ArrowError>>> {
        let send = |msg: Result<Chunk, ArrowError>| {
            let tx = &tx;
            async move {
                if let Err(e) = tx.send(msg).await {
                    tracing::error!("Failed to send result to channel: {e:?}");
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
                chunk_tasks.push_back(tokio::task::spawn(get_chunk(d, p, data, ticket)));
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
                chunk_tasks.push_back(tokio::task::spawn(get_chunk(d, p, data, ticket)));
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
) -> Result<Chunk, ArrowError> {
    let bytes = downloader.download_chunk(data).await?;
    let batches = parser.parse_chunk(bytes)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn available(budget: &MemoryBudget) -> usize {
        budget.semaphore.available_permits()
    }

    #[tokio::test]
    async fn unlimited_budget() {
        let budget = MemoryBudget::new(0);
        let _ticket = budget.acquire(1_000_000).await;
    }

    #[tokio::test]
    async fn first_acquire_exceeds_limit() {
        let budget = MemoryBudget::new(100);
        let ticket = budget.acquire(200).await;
        assert_eq!(available(&budget), 0);
        drop(ticket);
        assert_eq!(available(&budget), 100);
    }

    #[tokio::test]
    async fn within_limit() {
        let budget = MemoryBudget::new(1000);
        let _t1 = budget.acquire(500).await;
        let _t2 = budget.acquire(400).await;
        assert_eq!(available(&budget), 100);
    }

    #[tokio::test]
    async fn ticket_drop_frees_capacity() {
        let budget = MemoryBudget::new(1000);
        let ticket = budget.acquire(600).await;
        assert_eq!(available(&budget), 400);
        drop(ticket);
        assert_eq!(available(&budget), 1000);
    }

    #[tokio::test]
    async fn acquire_wakes_on_ticket_drop() {
        let budget = MemoryBudget::new(100);
        let ticket = budget.acquire(90).await;

        let budget_clone = budget.clone();
        let handle = tokio::spawn(async move { budget_clone.acquire(50).await });

        tokio::task::yield_now().await;
        drop(ticket);

        let _acquired = tokio::time::timeout(std::time::Duration::from_secs(1), handle)
            .await
            .expect("acquire should complete after ticket drop")
            .unwrap();
    }

    #[tokio::test]
    async fn no_outstanding_tickets_allows_exceeding_again() {
        let budget = MemoryBudget::new(100);
        let ticket = budget.acquire(200).await;
        assert_eq!(available(&budget), 0);
        drop(ticket);

        let ticket = budget.acquire(150).await;
        assert_eq!(available(&budget), 0);
        drop(ticket);
        assert_eq!(available(&budget), 100);
    }
}
