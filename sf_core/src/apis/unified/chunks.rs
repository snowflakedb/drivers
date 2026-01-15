//! Portable chunk downloading for large query results.
//!
//! This module handles downloading Arrow IPC chunks from presigned S3 URLs
//! using the portable HTTP client abstraction.

use crate::http::{HttpClient, HttpRequest};
use crate::rest::ChunkInfo;
use arrow::array::RecordBatch;
use arrow::datatypes::SchemaRef;
use arrow::error::ArrowError;
use arrow::ipc::reader::StreamReader;
use std::collections::HashMap;
use std::io::Cursor;

/// Download data for a single chunk.
pub struct ChunkDownload {
    pub url: String,
    pub headers: HashMap<String, String>,
}

impl ChunkDownload {
    pub fn new(chunk: &ChunkInfo, headers: &HashMap<String, String>) -> Self {
        Self {
            url: chunk.url.clone(),
            headers: headers.clone(),
        }
    }
}

/// Download a chunk using the portable HTTP client.
pub async fn download_chunk<C: HttpClient>(
    client: &C,
    chunk: &ChunkDownload,
) -> Result<Vec<u8>, ChunkDownloadError> {
    let mut request = HttpRequest::get(&chunk.url);
    for (key, value) in &chunk.headers {
        request = request.header(key, value);
    }

    let response = client
        .request(request)
        .await
        .map_err(|e| ChunkDownloadError::Http {
            message: format!("Failed to download chunk: {}", e),
        })?;

    if !response.status.is_success() {
        return Err(ChunkDownloadError::Http {
            message: format!(
                "Chunk download failed with status {}: {}",
                response.status.0,
                String::from_utf8_lossy(&response.body)
            ),
        });
    }

    // Handle gzip decompression if needed
    let content_encoding = response.header("content-encoding");

    let body = if content_encoding
        .map(|s| s.contains("gzip"))
        .unwrap_or(false)
    {
        decompress_gzip(&response.body)?
    } else {
        response.body
    };

    Ok(body)
}

/// Decompress gzip data.
fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>, ChunkDownloadError> {
    use std::io::Read;
    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| ChunkDownloadError::Decompression {
            message: format!("Failed to decompress chunk: {}", e),
        })?;
    Ok(decompressed)
}

/// Parse Arrow IPC data into record batches.
pub fn parse_arrow_ipc(data: Vec<u8>) -> Result<(SchemaRef, Vec<RecordBatch>), ChunkDownloadError> {
    let cursor = Cursor::new(data);
    let reader = StreamReader::try_new(cursor, None).map_err(|e| ChunkDownloadError::Arrow {
        message: format!("Failed to create Arrow reader: {}", e),
    })?;

    let schema = reader.schema().clone();
    let batches: Result<Vec<RecordBatch>, ArrowError> = reader.collect();
    let batches = batches.map_err(|e| ChunkDownloadError::Arrow {
        message: format!("Failed to read Arrow batches: {}", e),
    })?;

    Ok((schema, batches))
}

/// Errors that can occur during chunk downloading.
#[derive(Debug)]
pub enum ChunkDownloadError {
    Http { message: String },
    Decompression { message: String },
    Arrow { message: String },
}

impl std::fmt::Display for ChunkDownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkDownloadError::Http { message } => write!(f, "HTTP error: {}", message),
            ChunkDownloadError::Decompression { message } => {
                write!(f, "Decompression error: {}", message)
            }
            ChunkDownloadError::Arrow { message } => write!(f, "Arrow error: {}", message),
        }
    }
}

impl std::error::Error for ChunkDownloadError {}
