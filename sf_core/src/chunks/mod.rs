mod arrow_parser;
mod error;
mod http_downloader;
mod json_parser;
pub mod mock;
pub mod prefetch;

use std::collections::{HashMap, VecDeque};
use std::io;
use std::str::FromStr;
use std::sync::Arc;

use crate::query_types::RowType;
use crate::rest::snowflake::query_response::Chunk;
use arrow::array::{RecordBatchIterator, RecordBatchReader};
use arrow::datatypes::{Fields, Schema};
use arrow_ipc::reader::StreamReader;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
pub use error::ChunkError;
use error::*;
pub use json_parser::convert_string_rowset_to_arrow_reader;
use prefetch::{ArrowChunkParser, HttpChunkDownloader, JsonChunkParser, PrefetchChunkReader};
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use snafu::{OptionExt, ResultExt};

pub const DEFAULT_PREFETCH_THREADS: usize = 8;

pub async fn json_prefetch_reader(
    initial_rowset: &[Vec<Option<String>>],
    row_types: Vec<RowType>,
    chunk_download_data: Vec<ChunkDownloadData>,
    client: Client,
    prefetch_concurrency: usize,
) -> Result<Box<dyn RecordBatchReader + Send>, ChunkError> {
    let initial_reader = convert_string_rowset_to_arrow_reader(initial_rowset, &row_types)?;
    let downloader = HttpChunkDownloader { client };
    let parser = JsonChunkParser {
        row_types: row_types.clone(),
    };
    PrefetchChunkReader::reader(
        initial_reader,
        chunk_download_data.into(),
        downloader,
        parser,
        prefetch_concurrency,
    )
    .await
}

pub async fn arrow_prefetch_reader(
    initial_base64_opt: Option<&str>,
    mut chunk_download_data: VecDeque<ChunkDownloadData>,
    client: Client,
    prefetch_concurrency: usize,
) -> Result<Box<dyn RecordBatchReader + Send>, ChunkError> {
    let initial_reader = if let Some(initial_base64) = initial_base64_opt {
        let bytes = BASE64.decode(initial_base64).context(Base64DecodingSnafu)?;
        let cursor = io::Cursor::new(bytes);
        StreamReader::try_new(cursor, None).context(ChunkReadingSnafu)?
    } else {
        let first = chunk_download_data
            .pop_front()
            .context(InitialChunkMissingSnafu)?;
        let bytes = get_chunk_data(client.clone(), first).await?;
        let cursor = io::Cursor::new(bytes);
        StreamReader::try_new(cursor, None).context(ChunkReadingSnafu)?
    };
    let downloader = HttpChunkDownloader { client };
    let parser = ArrowChunkParser;
    PrefetchChunkReader::reader(
        initial_reader,
        chunk_download_data,
        downloader,
        parser,
        prefetch_concurrency,
    )
    .await
}

pub fn single_chunk_reader(base64: &str) -> Result<Box<dyn RecordBatchReader + Send>, ChunkError> {
    let bytes = BASE64.decode(base64).context(Base64DecodingSnafu)?;
    let cursor = io::Cursor::new(bytes);
    let reader = StreamReader::try_new(cursor, None).context(ChunkReadingSnafu)?;
    Ok(Box::new(reader))
}

pub fn schema_only_reader(
    rowtype: &[RowType],
) -> Result<Box<dyn RecordBatchReader + Send>, ChunkError> {
    convert_string_rowset_to_arrow_reader(&[], rowtype)
}

pub fn empty_reader() -> Box<dyn RecordBatchReader + Send> {
    Box::new(RecordBatchIterator::new(
        vec![],
        Arc::new(Schema::new(Fields::empty())),
    ))
}

#[derive(Debug, Clone)]
pub struct ChunkDownloadData {
    pub url: String,
    pub row_count: i32,
    pub uncompressed_size: i64,
    pub compressed_size: i64,
    pub headers: HashMap<String, String>,
}

impl ChunkDownloadData {
    pub fn new(chunk: &Chunk, chunk_headers: &HashMap<String, String>) -> Self {
        Self {
            url: chunk.url.to_string(),
            row_count: chunk.row_count,
            uncompressed_size: chunk.uncompressed_size,
            compressed_size: chunk.compressed_size,
            headers: chunk_headers.clone(),
        }
    }
}

/// Downloads chunk data from the given URL.
///
/// When reqwest's `gzip` feature handles `Content-Encoding: gzip` transparently
/// the returned bytes are already decompressed. Some cloud providers (notably
/// GCS on GCP) may serve gzip-compressed data without setting that header, so
/// we detect the gzip magic bytes and decompress explicitly when needed.
pub async fn get_chunk_data(
    client: Client,
    chunk: ChunkDownloadData,
) -> Result<Vec<u8>, ChunkError> {
    let url = &chunk.url;
    let mut headers = HeaderMap::new();
    for (key, value) in chunk.headers.iter() {
        let header_name = HeaderName::from_str(key).context(HeaderNameSnafu { key })?;
        let header_value = HeaderValue::from_str(value).context(HeaderValueSnafu { key })?;
        headers.insert(header_name, header_value);
    }
    use crate::config::retry::RetryPolicy;
    use crate::http::retry::{HttpContext, HttpError, execute_with_retry};
    use reqwest::Method;

    let policy = RetryPolicy::default();
    let ctx = HttpContext::new(Method::GET, url.clone()).with_idempotent(true);

    let response = match execute_with_retry(
        || client.get(url.clone()).headers(headers.clone()),
        &ctx,
        &policy,
        |r| async move { Ok(r) },
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return match e {
                HttpError::Transport { source, .. } => Err(source).context(CommunicationSnafu),
                HttpError::DeadlineExceeded { .. } | HttpError::RetryAfterExceeded { .. } => {
                    UnsuccessfulHttpStatusCodeSnafu {
                        status: reqwest::StatusCode::REQUEST_TIMEOUT,
                    }
                    .fail()
                }
                HttpError::MaxAttempts { last_status, .. } => UnsuccessfulHttpStatusCodeSnafu {
                    status: last_status,
                }
                .fail(),
            };
        }
    };

    if !response.status().is_success() {
        UnsuccessfulHttpStatusCodeSnafu {
            status: response.status(),
        }
        .fail()?;
    }

    let body = response.bytes().await.context(CommunicationSnafu)?;
    let bytes = body.to_vec();
    maybe_decompress_gzip(bytes)
}

const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

fn maybe_decompress_gzip(data: Vec<u8>) -> Result<Vec<u8>, ChunkError> {
    if data.len() >= 2 && data[..2] == GZIP_MAGIC {
        use flate2::bufread::GzDecoder;
        use std::io::Read as _;
        let mut decoder = GzDecoder::new(&data[..]);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| ChunkError::ChunkDecompression {
                source: e,
                location: snafu::Location::new(file!(), line!(), 0),
            })?;
        Ok(decompressed)
    } else {
        Ok(data)
    }
}
