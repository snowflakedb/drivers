use std::path::PathBuf;

use arrow::error::ArrowError;

use super::ChunkDownloadData;
use super::prefetch::DownloadChunk;

#[derive(Clone)]
pub struct FileChunkDownloader;

impl DownloadChunk for FileChunkDownloader {
    async fn download_chunk(&self, chunk: ChunkDownloadData) -> Result<Vec<u8>, ArrowError> {
        let path = PathBuf::from(&chunk.url);
        tokio::fs::read(&path)
            .await
            .map_err(|e| ArrowError::ExternalError(Box::new(e)))
    }
}
