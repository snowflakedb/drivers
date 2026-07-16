mod arrow_parser;
mod error;
mod http_downloader;
mod json_parser;
mod memory_budget;
pub mod mock;
pub mod prefetch;

use std::collections::{HashMap, VecDeque};
use std::io;
use std::str::FromStr;
use std::sync::Arc;

use crate::query_types::RowType;
use crate::rest::snowflake::query_response::Chunk;
use arrow::array::{RecordBatchIterator, RecordBatchReader};
use arrow::datatypes::{Field, Fields, Schema, SchemaRef};
use arrow_ipc::reader::StreamReader;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
pub use error::ChunkError;
use error::*;
pub(crate) use error::{ArrowIpcEncodeSnafu, ChunkReadSnafu};
pub use json_parser::convert_string_rowset_to_arrow_reader;
use prefetch::{ArrowChunkParser, HttpChunkDownloader, JsonChunkParser, PrefetchChunkReader};
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use snafu::{OptionExt, ResultExt};

pub const DEFAULT_PREFETCH_THREADS: usize = 4;
pub const DEFAULT_MEMORY_LIMIT_MB: u32 = 1536;

/// Configuration for the chunk prefetch pipeline.
#[derive(Debug, Clone)]
pub struct PrefetchConfig {
    /// Number of concurrent chunk download+parse tasks.
    pub prefetch_threads: usize,
    /// Memory budget in MB for buffered chunks. 0 means unlimited.
    pub memory_limit_mb: u32,
}

impl Default for PrefetchConfig {
    fn default() -> Self {
        Self {
            prefetch_threads: DEFAULT_PREFETCH_THREADS,
            memory_limit_mb: DEFAULT_MEMORY_LIMIT_MB,
        }
    }
}

impl PrefetchConfig {
    /// Resolve from a session parameters map, falling back to defaults for
    /// missing or unparseable values.
    pub fn from_session_params(params: &HashMap<String, String>) -> Self {
        let prefetch_threads = params
            .get("CLIENT_PREFETCH_THREADS")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(DEFAULT_PREFETCH_THREADS);
        let memory_limit_mb = params
            .get("CLIENT_MEMORY_LIMIT")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(DEFAULT_MEMORY_LIMIT_MB);
        Self {
            prefetch_threads,
            memory_limit_mb,
        }
    }
}

pub async fn json_prefetch_reader(
    initial_rowset: &[Vec<Option<String>>],
    row_types: Vec<RowType>,
    chunk_download_data: Vec<ChunkDownloadData>,
    client: Client,
    config: &PrefetchConfig,
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
        config,
    )
    .await
}

pub async fn arrow_prefetch_reader(
    initial_base64_opt: Option<&str>,
    mut chunk_download_data: VecDeque<ChunkDownloadData>,
    client: Client,
    config: &PrefetchConfig,
    nullable_flags: Option<&[bool]>,
) -> Result<Box<dyn RecordBatchReader + Send>, ChunkError> {
    let initial_reader = if let Some(initial_base64) = initial_base64_opt {
        let bytes = BASE64.decode(initial_base64).context(Base64DecodeSnafu)?;
        let cursor = io::Cursor::new(bytes);
        StreamReader::try_new(cursor, None).context(ChunkReadSnafu)?
    } else {
        let first = chunk_download_data
            .pop_front()
            .context(MissingInitialChunkSnafu)?;
        let bytes = get_chunk_data(client.clone(), first).await?;
        let cursor = io::Cursor::new(bytes);
        StreamReader::try_new(cursor, None).context(ChunkReadSnafu)?
    };
    let downloader = HttpChunkDownloader { client };
    let parser = ArrowChunkParser;
    let reader = PrefetchChunkReader::reader(
        initial_reader,
        chunk_download_data,
        downloader,
        parser,
        config,
    )
    .await?;
    Ok(maybe_inject_nullable(reader, nullable_flags))
}

pub fn single_chunk_reader(
    base64: &str,
    nullable_flags: Option<&[bool]>,
) -> Result<Box<dyn RecordBatchReader + Send>, ChunkError> {
    let bytes = BASE64.decode(base64).context(Base64DecodeSnafu)?;
    let cursor = io::Cursor::new(bytes);
    let reader = StreamReader::try_new(cursor, None).context(ChunkReadSnafu)?;
    let boxed: Box<dyn RecordBatchReader + Send> = Box::new(reader);
    Ok(maybe_inject_nullable(boxed, nullable_flags))
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

/// Overrides the schema returned by a reader without touching the underlying batches.
struct SchemaOverrideReader {
    inner: Box<dyn RecordBatchReader + Send>,
    schema: SchemaRef,
}

impl RecordBatchReader for SchemaOverrideReader {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

impl Iterator for SchemaOverrideReader {
    type Item = Result<arrow::record_batch::RecordBatch, arrow::error::ArrowError>;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// Injects `"nullable"` metadata into each Arrow field that doesn't already have it.
/// Returns the reader unchanged if no injection is needed.
fn maybe_inject_nullable(
    reader: Box<dyn RecordBatchReader + Send>,
    nullable_flags: Option<&[bool]>,
) -> Box<dyn RecordBatchReader + Send> {
    let Some(flags) = nullable_flags else {
        return reader;
    };
    let schema = reader.schema();
    if flags.is_empty() || flags.len() != schema.fields().len() {
        return reader;
    }
    let needs_injection = schema
        .fields()
        .iter()
        .any(|f| !f.metadata().contains_key("nullable"));
    if !needs_injection {
        return reader;
    }
    let new_fields: Vec<Field> = schema
        .fields()
        .iter()
        .zip(flags.iter())
        .map(|(field, &nullable)| {
            if field.metadata().contains_key("nullable") {
                field.as_ref().clone()
            } else {
                let mut metadata = field.metadata().clone();
                metadata.insert("nullable".to_string(), nullable.to_string());
                field.as_ref().clone().with_metadata(metadata)
            }
        })
        .collect();
    let new_schema = Arc::new(Schema::new_with_metadata(
        new_fields,
        schema.metadata().clone(),
    ));
    Box::new(SchemaOverrideReader {
        inner: reader,
        schema: new_schema,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkFormatKind {
    ArrowIpc,
    Json,
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

    /// Estimates in-memory size after decompression and Arrow conversion.
    /// Uses 1.5x uncompressed size as a heuristic for Arrow overhead.
    pub fn estimated_memory_mb(&self) -> u32 {
        const BYTES_PER_MB: u64 = 1024 * 1024;
        let bytes = (self.uncompressed_size.max(0) as u64) * 3 / 2;
        ((bytes / BYTES_PER_MB).max(1)) as u32
    }
}

#[derive(Debug)]
pub struct InitialChunkData {
    pub rowset_base64: String,
    pub row_count: i32,
    pub uncompressed_size: i64,
    pub compressed_size: i64,
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
    // gzip inflate is CPU-bound; run it on the blocking pool so a large chunk
    // body doesn't stall this runtime worker.
    tokio::task::spawn_blocking(move || maybe_decompress_gzip(bytes))
        .await
        .context(SpawnBlockingSnafu)?
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
