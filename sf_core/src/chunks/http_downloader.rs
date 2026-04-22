use arrow::error::ArrowError;
use reqwest::Client;

use super::prefetch::DownloadChunk;
use super::{ChunkDownloadData, get_chunk_data};

#[derive(Clone)]
pub struct HttpChunkDownloader {
    pub client: Client,
}

impl DownloadChunk for HttpChunkDownloader {
    #[tracing::instrument(
        name = "core_chunk_download",
        target = "sf_core::perf",
        level = "trace",
        skip_all
    )]
    async fn download_chunk(&self, chunk: ChunkDownloadData) -> Result<Vec<u8>, ArrowError> {
        get_chunk_data(self.client.clone(), chunk)
            .await
            .map_err(|e| ArrowError::ExternalError(Box::new(e)))
    }
}
