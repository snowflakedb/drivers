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

use super::{ChunkDownloadData, ChunkError, ChunkReadingSnafu};

pub trait DownloadChunk: Send + Sync + Clone + 'static {
    fn download_chunk(
        &self,
        chunk: ChunkDownloadData,
    ) -> impl Future<Output = Result<Vec<u8>, ArrowError>> + Send;
}

pub trait ParseChunk: Send + Sync + Clone + 'static {
    fn parse_chunk(&self, data: Vec<u8>) -> Result<Vec<RecordBatch>, ArrowError>;
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
    batch_rx: tokio::sync::mpsc::Receiver<Result<RecordBatch, ArrowError>>,
    phantom: PhantomData<(D, P)>,
}

impl<D: DownloadChunk, P: ParseChunk> PrefetchChunkReader<D, P> {
    pub async fn reader<R: RecordBatchReader + Send>(
        initial: R,
        chunks: VecDeque<ChunkDownloadData>,
        downloader: D,
        parser: P,
        prefetch_concurrency: usize,
    ) -> Result<Box<dyn RecordBatchReader + Send>, ChunkError> {
        let schema = initial.schema();
        let initial = initial
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .context(ChunkReadingSnafu)?;
        let (tx, rx) = tokio::sync::mpsc::channel(prefetch_concurrency);

        tokio::spawn(Self::prefetch_batches(
            downloader,
            parser,
            chunks,
            initial,
            tx,
            prefetch_concurrency,
        ));

        Ok(Box::new(Self {
            schema,
            batch_rx: rx,
            phantom: PhantomData,
        }))
    }

    async fn prefetch_batches(
        downloader: D,
        parser: P,
        mut chunks: VecDeque<ChunkDownloadData>,
        initial: Vec<RecordBatch>,
        tx: tokio::sync::mpsc::Sender<Result<RecordBatch, ArrowError>>,
        prefetch_concurrency: usize,
    ) -> Result<(), SendError<Result<RecordBatch, ArrowError>>> {
        let send_result = |result: Result<RecordBatch, ArrowError>| async {
            if let Err(e) = tx.send(result).await {
                tracing::error!("Failed to send result to channel for: {e:?}");
                return Err(e);
            }
            Ok(())
        };
        let mut chunk_tasks = VecDeque::new();
        for _ in 0..prefetch_concurrency {
            let downloader = downloader.clone();
            let parser = parser.clone();
            if let Some(data) = chunks.pop_front() {
                chunk_tasks.push_back(tokio::task::spawn(async move {
                    let bytes = downloader.download_chunk(data).await?;
                    parser.parse_chunk(bytes)
                }));
            }
        }
        for chunk in initial {
            send_result(Ok(chunk)).await?;
        }
        while let Some(task) = chunk_tasks.pop_front() {
            let prefetch_batch_result = task.await;
            if let Err(e) = prefetch_batch_result {
                return send_result(Err(ArrowError::ExternalError(Box::new(e)))).await;
            }
            let batches = prefetch_batch_result.unwrap();
            let downloader_ = downloader.clone();
            let parser_ = parser.clone();
            if let Some(data) = chunks.pop_front() {
                chunk_tasks.push_back(tokio::task::spawn(async move {
                    let bytes = downloader_.download_chunk(data).await?;
                    parser_.parse_chunk(bytes)
                }));
            }
            match batches {
                Ok(batches) => {
                    for batch in batches {
                        send_result(Ok(batch)).await?;
                    }
                }
                Err(e) => {
                    return send_result(Err(e)).await;
                }
            }
        }
        Ok(())
    }
}

impl<D: DownloadChunk + 'static, P: ParseChunk + 'static> Iterator for PrefetchChunkReader<D, P> {
    type Item = Result<RecordBatch, ArrowError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.batch_rx.blocking_recv()
    }
}

impl<D: DownloadChunk + 'static, P: ParseChunk + 'static> RecordBatchReader
    for PrefetchChunkReader<D, P>
{
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}
