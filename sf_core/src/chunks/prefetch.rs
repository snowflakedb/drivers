pub use super::arrow_parser::ArrowChunkParser;
pub use super::http_downloader::HttpChunkDownloader;
pub use super::json_parser::JsonChunkParser;

use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use arrow::array::{RecordBatch, RecordBatchReader};
use arrow::datatypes::SchemaRef;
use arrow::error::ArrowError;
use snafu::ResultExt;
use tokio::sync::Notify;
use tokio::sync::mpsc::error::SendError;

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

pub(crate) struct MemoryBudget {
    committed: AtomicU64,
    limit: u64,
    notify: Notify,
}

impl MemoryBudget {
    fn new(limit: u64) -> Self {
        Self {
            committed: AtomicU64::new(0),
            limit,
            notify: Notify::new(),
        }
    }

    fn try_commit(&self, estimate: u64, is_first: bool) -> bool {
        if self.limit == 0 || is_first {
            self.committed.fetch_add(estimate, Ordering::Relaxed);
            return true;
        }
        let current = self.committed.load(Ordering::Relaxed);
        if current + estimate <= self.limit {
            self.committed.fetch_add(estimate, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    fn commit(&self, estimate: u64) {
        self.committed.fetch_add(estimate, Ordering::Relaxed);
    }

    fn release(&self, estimate: u64) {
        self.committed.fetch_sub(estimate, Ordering::Relaxed);
        self.notify.notify_one();
    }

    async fn wait_for_capacity(&self, estimate: u64) {
        loop {
            let current = self.committed.load(Ordering::Relaxed);
            if current + estimate <= self.limit {
                return;
            }
            self.notify.notified().await;
        }
    }
}

/// Channel message: a RecordBatch plus an optional memory-release marker.
/// When `release_bytes` is `Some`, the consumer must release that many bytes
/// from the memory budget after receiving this batch.
type BatchMsg = (RecordBatch, Option<u64>);

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
    batch_rx: tokio::sync::mpsc::Receiver<Result<BatchMsg, ArrowError>>,
    memory_budget: Arc<MemoryBudget>,
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
        let memory_budget = Arc::new(MemoryBudget::new(config.memory_limit_bytes));

        tokio::spawn(Self::prefetch_batches(
            downloader,
            parser,
            chunks,
            initial,
            tx,
            prefetch_concurrency,
            Arc::clone(&memory_budget),
        ));

        Ok(Box::new(Self {
            schema,
            batch_rx: rx,
            memory_budget,
            phantom: PhantomData,
        }))
    }

    async fn prefetch_batches(
        downloader: D,
        parser: P,
        mut chunks: VecDeque<ChunkDownloadData>,
        initial: Vec<RecordBatch>,
        tx: tokio::sync::mpsc::Sender<Result<BatchMsg, ArrowError>>,
        prefetch_concurrency: usize,
        memory_budget: Arc<MemoryBudget>,
    ) -> Result<(), SendError<Result<BatchMsg, ArrowError>>> {
        let send_result = |result: Result<BatchMsg, ArrowError>| {
            let tx = &tx;
            async move {
                if let Err(e) = tx.send(result).await {
                    tracing::error!("Failed to send result to channel: {e:?}");
                    return Err(e);
                }
                Ok(())
            }
        };

        // Send initial rowset batches (already in memory, no budget tracking needed)
        for batch in initial {
            send_result(Ok((batch, None))).await?;
        }

        let mut chunk_tasks: VecDeque<(
            tokio::task::JoinHandle<Result<Vec<RecordBatch>, ArrowError>>,
            u64,
        )> = VecDeque::new();
        let mut is_first_remote_chunk = true;

        // Fill initial concurrency window
        for _ in 0..prefetch_concurrency {
            if let Some(data) = chunks.pop_front() {
                let estimate = data.estimated_memory_bytes();
                if !memory_budget.try_commit(estimate, is_first_remote_chunk) {
                    memory_budget.wait_for_capacity(estimate).await;
                    memory_budget.commit(estimate);
                }
                is_first_remote_chunk = false;

                let d = downloader.clone();
                let p = parser.clone();
                chunk_tasks.push_back((
                    tokio::task::spawn(async move {
                        let bytes = d.download_chunk(data).await?;
                        p.parse_chunk(bytes)
                    }),
                    estimate,
                ));
            }
        }

        while let Some((task, estimate)) = chunk_tasks.pop_front() {
            let prefetch_batch_result = task.await;
            if let Err(e) = prefetch_batch_result {
                memory_budget.release(estimate);
                return send_result(Err(ArrowError::ExternalError(Box::new(e)))).await;
            }

            match prefetch_batch_result.unwrap() {
                Ok(batches) => {
                    let batch_count = batches.len();
                    for (i, batch) in batches.into_iter().enumerate() {
                        let release = if i == batch_count - 1 {
                            Some(estimate)
                        } else {
                            None
                        };
                        send_result(Ok((batch, release))).await?;
                    }
                }
                Err(e) => {
                    memory_budget.release(estimate);
                    return send_result(Err(e)).await;
                }
            }

            // Spawn replacement task (rolling window refill)
            if let Some(data) = chunks.pop_front() {
                let next_estimate = data.estimated_memory_bytes();
                if !memory_budget.try_commit(next_estimate, false) {
                    memory_budget.wait_for_capacity(next_estimate).await;
                    memory_budget.commit(next_estimate);
                }

                let d = downloader.clone();
                let p = parser.clone();
                chunk_tasks.push_back((
                    tokio::task::spawn(async move {
                        let bytes = d.download_chunk(data).await?;
                        p.parse_chunk(bytes)
                    }),
                    next_estimate,
                ));
            }
        }

        Ok(())
    }
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
        match self.batch_rx.blocking_recv() {
            Some(Ok((batch, release))) => {
                if let Some(bytes) = release {
                    self.memory_budget.release(bytes);
                }
                Some(Ok(batch))
            }
            Some(Err(e)) => Some(Err(e)),
            None => None,
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

    #[test]
    fn memory_budget_unlimited() {
        let budget = MemoryBudget::new(0);
        assert!(budget.try_commit(u64::MAX, false));
    }

    #[test]
    fn memory_budget_first_chunk_escape() {
        let budget = MemoryBudget::new(100);
        assert!(budget.try_commit(200, true));
    }

    #[test]
    fn memory_budget_within_limit() {
        let budget = MemoryBudget::new(1000);
        assert!(budget.try_commit(500, false));
        assert!(budget.try_commit(400, false));
        assert!(!budget.try_commit(200, false));
    }

    #[test]
    fn memory_budget_release_frees_capacity() {
        let budget = MemoryBudget::new(1000);
        assert!(budget.try_commit(600, false));
        assert!(!budget.try_commit(500, false));
        budget.release(600);
        assert!(budget.try_commit(500, false));
    }

    #[tokio::test]
    async fn memory_budget_wait_wakes_on_release() {
        let budget = Arc::new(MemoryBudget::new(100));
        budget.commit(90);

        let budget_clone = Arc::clone(&budget);
        let handle = tokio::spawn(async move {
            budget_clone.wait_for_capacity(50).await;
        });

        tokio::task::yield_now().await;
        budget.release(80);

        tokio::time::timeout(std::time::Duration::from_secs(1), handle)
            .await
            .expect("wait_for_capacity should complete after release")
            .unwrap();
    }
}
