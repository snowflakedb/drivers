mod arrow_parser;
mod async_arrow_batch_fetcher;
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
use std::time::Duration;

use crate::apis::database_driver_v1::Setting;
use crate::config::retry::RetryPolicy;
use crate::config::settings::Settings;
use crate::http::retry::{HttpContext, HttpError, execute_with_retry, next_delay_ms};
use crate::query_types::RowType;
use crate::rest::snowflake::query_response::Chunk;
use arrow::array::{RecordBatch, RecordBatchIterator, RecordBatchReader};
use arrow::datatypes::{Field, Fields, Schema, SchemaRef};
use arrow_ipc::reader::StreamReader;
pub(crate) use async_arrow_batch_fetcher::AsyncArrowBatchFetcher;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
#[cfg(feature = "protobuf")]
pub(crate) use error::ArrowIpcEncodeSnafu;
pub use error::ChunkError;
pub(crate) use error::ChunkReadSnafu;
use error::*;
pub use json_parser::convert_string_rowset_to_arrow_reader;
pub(crate) use prefetch::PrefetchChunkReader;
use prefetch::{ArrowChunkParser, HttpChunkDownloader, JsonChunkParser, ParseChunk};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Method};
use snafu::{IntoError, OptionExt, ResultExt, ensure};
use tokio::task::AbortHandle;

pub const DEFAULT_PREFETCH_THREADS: usize = 4;
pub const DEFAULT_MEMORY_LIMIT_MB: u32 = 1536;

/// Configuration for the chunk prefetch pipeline.
#[derive(Debug, Clone)]
pub struct PrefetchConfig {
    /// Number of concurrent chunk download+parse tasks.
    pub prefetch_threads: usize,
    /// Memory budget in MB for buffered chunks. 0 means unlimited.
    pub memory_limit_mb: u32,
    /// How long a chunk GET may receive no data, while waiting for the response
    /// or between body frames, before the whole GET is retried. `None` means no
    /// inactivity timer.
    pub chunk_read_timeout: Option<Duration>,
}

impl Default for PrefetchConfig {
    fn default() -> Self {
        Self {
            prefetch_threads: DEFAULT_PREFETCH_THREADS,
            memory_limit_mb: DEFAULT_MEMORY_LIMIT_MB,
            chunk_read_timeout: None,
        }
    }
}

impl PrefetchConfig {
    /// Resolve from a session parameters map, falling back to defaults for
    /// missing or unparseable values.
    pub fn from_session_params(params: &HashMap<String, Setting>) -> Self {
        let prefetch_threads = params
            .get_u64("CLIENT_PREFETCH_THREADS")
            .and_then(|v| usize::try_from(v).ok())
            .unwrap_or(DEFAULT_PREFETCH_THREADS);
        let memory_limit_mb = params
            .get_u64("CLIENT_MEMORY_LIMIT")
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(DEFAULT_MEMORY_LIMIT_MB);
        Self {
            prefetch_threads,
            memory_limit_mb,
            chunk_read_timeout: None,
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
    let (reader, _abort) = json_prefetch(
        initial_rowset,
        row_types,
        chunk_download_data,
        client,
        config,
    )
    .await?;
    Ok(Box::new(reader))
}

pub(crate) async fn json_prefetch(
    initial_rowset: &[Vec<Option<String>>],
    row_types: Vec<RowType>,
    chunk_download_data: Vec<ChunkDownloadData>,
    client: Client,
    config: &PrefetchConfig,
) -> Result<(PrefetchChunkReader, AbortHandle), ChunkError> {
    let initial_reader = convert_string_rowset_to_arrow_reader(initial_rowset, &row_types)?;
    open_json_prefetch(
        initial_reader,
        row_types,
        chunk_download_data,
        client,
        config,
    )
    .await
}

async fn json_chunks_reader(
    initial_reader: Box<dyn RecordBatchReader + Send>,
    row_types: Vec<RowType>,
    chunk_download_data: Vec<ChunkDownloadData>,
    client: Client,
    config: &PrefetchConfig,
) -> Result<Box<dyn RecordBatchReader + Send>, ChunkError> {
    let (reader, _abort) = open_json_prefetch(
        initial_reader,
        row_types,
        chunk_download_data,
        client,
        config,
    )
    .await?;
    Ok(Box::new(reader))
}

async fn open_json_prefetch(
    initial_reader: Box<dyn RecordBatchReader + Send>,
    row_types: Vec<RowType>,
    chunk_download_data: Vec<ChunkDownloadData>,
    client: Client,
    config: &PrefetchConfig,
) -> Result<(PrefetchChunkReader, AbortHandle), ChunkError> {
    let initial_future =
        tokio::task::spawn_blocking(move || drain_reader_to_batches(initial_reader));
    PrefetchChunkReader::open(
        Some(initial_future),
        chunk_download_data.into(),
        HttpChunkDownloader {
            client,
            read_timeout: config.chunk_read_timeout,
        },
        JsonChunkParser { row_types },
        config,
    )
    .await
}

pub(crate) fn drain_reader_to_batches(
    reader: impl RecordBatchReader,
) -> Result<(SchemaRef, Vec<RecordBatch>), ChunkError> {
    let schema = reader.schema();
    let batches = reader
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .context(ChunkReadSnafu)?;
    Ok((schema, batches))
}

fn spawn_initial_arrow_decode(
    b64: String,
) -> tokio::task::JoinHandle<Result<(SchemaRef, Vec<RecordBatch>), ChunkError>> {
    tokio::task::spawn_blocking(move || {
        let bytes = BASE64.decode(b64).context(Base64DecodeSnafu)?;
        let reader = StreamReader::try_new(io::Cursor::new(bytes), None).context(ChunkReadSnafu)?;
        drain_reader_to_batches(reader)
    })
}

pub async fn arrow_prefetch_reader(
    initial_base64_opt: Option<String>,
    chunk_download_data: VecDeque<ChunkDownloadData>,
    client: Client,
    config: &PrefetchConfig,
    nullable_flags: Option<&[bool]>,
) -> Result<Box<dyn RecordBatchReader + Send>, ChunkError> {
    let (reader, _abort) =
        arrow_prefetch(initial_base64_opt, chunk_download_data, client, config).await?;
    Ok(maybe_inject_nullable(Box::new(reader), nullable_flags))
}

pub(crate) async fn arrow_prefetch(
    initial_base64_opt: Option<String>,
    chunk_download_data: VecDeque<ChunkDownloadData>,
    client: Client,
    config: &PrefetchConfig,
) -> Result<(PrefetchChunkReader, AbortHandle), ChunkError> {
    PrefetchChunkReader::open(
        initial_base64_opt.map(spawn_initial_arrow_decode),
        chunk_download_data,
        HttpChunkDownloader {
            client,
            read_timeout: config.chunk_read_timeout,
        },
        ArrowChunkParser,
        config,
    )
    .await
}

/// Builds the initial reader from either the inline base64 chunk (always Arrow
/// IPC) or by popping and fetching the first remote chunk. Remote chunks are
/// parsed according to `remote_format`: Arrow IPC goes through `StreamReader`,
/// JSON goes through `JsonChunkParser`.
async fn get_initial_chunk_reader(
    initial_base64_opt: Option<&str>,
    chunk_download_data: &mut VecDeque<ChunkDownloadData>,
    client: &Client,
    read_timeout: Option<Duration>,
    remote_format: ChunkFormatKind,
    row_types: &[RowType],
) -> Result<Box<dyn RecordBatchReader + Send>, ChunkError> {
    if let Some(initial_base64) = initial_base64_opt {
        let bytes = BASE64.decode(initial_base64).context(Base64DecodeSnafu)?;
        let cursor = io::Cursor::new(bytes);
        Ok(Box::new(
            StreamReader::try_new(cursor, None).context(ChunkReadSnafu)?,
        ))
    } else {
        let first = chunk_download_data
            .pop_front()
            .context(MissingInitialChunkSnafu)?;
        let bytes = get_chunk_data(client.clone(), first, read_timeout).await?;
        match remote_format {
            ChunkFormatKind::ArrowIpc => {
                let cursor = io::Cursor::new(bytes);
                Ok(Box::new(
                    StreamReader::try_new(cursor, None).context(ChunkReadSnafu)?,
                ))
            }
            ChunkFormatKind::Json => {
                let parser = JsonChunkParser {
                    row_types: row_types.to_vec(),
                };
                let batches = tokio::task::spawn_blocking(move || parser.parse_chunk(bytes))
                    .await
                    .context(SpawnBlockingSnafu)?
                    .context(ChunkReadSnafu)?;
                let schema = batches
                    .first()
                    .map(RecordBatch::schema)
                    .unwrap_or_else(|| Arc::new(Schema::new(Fields::empty())));
                Ok(Box::new(RecordBatchIterator::new(
                    batches.into_iter().map(Ok::<_, arrow::error::ArrowError>),
                    schema,
                )))
            }
        }
    }
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

pub(crate) fn override_reader_schema(
    reader: Box<dyn RecordBatchReader + Send>,
    schema: SchemaRef,
) -> Box<dyn RecordBatchReader + Send> {
    Box::new(SchemaOverrideReader {
        inner: reader,
        schema,
    })
}

pub(crate) fn inject_nullable_schema(
    schema: SchemaRef,
    nullable_flags: Option<&[bool]>,
) -> SchemaRef {
    let Some(flags) = nullable_flags else {
        return schema;
    };
    if flags.is_empty() || flags.len() != schema.fields().len() {
        return schema;
    }
    let needs_injection = schema
        .fields()
        .iter()
        .any(|f| !f.metadata().contains_key("nullable"));
    if !needs_injection {
        return schema;
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
    Arc::new(Schema::new_with_metadata(
        new_fields,
        schema.metadata().clone(),
    ))
}

/// Injects `"nullable"` metadata into each Arrow field that doesn't already have it.
/// Returns the reader unchanged if no injection is needed.
pub(crate) fn maybe_inject_nullable(
    reader: Box<dyn RecordBatchReader + Send>,
    nullable_flags: Option<&[bool]>,
) -> Box<dyn RecordBatchReader + Send> {
    let schema = reader.schema();
    let injected = inject_nullable_schema(schema.clone(), nullable_flags);
    if Arc::ptr_eq(&injected, &schema) {
        return reader;
    }
    override_reader_schema(reader, injected)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkFormatKind {
    ArrowIpc,
    Json,
}

#[derive(Debug, Clone)]
pub enum FetchChunkInput {
    Inline(String),
    Remote(ChunkDownloadData),
}

pub async fn fetch_chunks_reader(
    chunks: Vec<FetchChunkInput>,
    chunk_format: ChunkFormatKind,
    row_types: Vec<RowType>,
    nullable_flags: &[bool],
    client: Client,
    prefetch_config: &PrefetchConfig,
) -> Result<Box<dyn RecordBatchReader + Send>, ChunkError> {
    let mut initial_base64_opt = None;
    let mut remote_chunks = VecDeque::new();

    for chunk in chunks {
        match chunk {
            FetchChunkInput::Inline(bytes) => {
                // The server is expected to send at most one inline chunk.
                ensure!(initial_base64_opt.is_none(), MultipleInlineChunksSnafu);
                initial_base64_opt = Some(bytes)
            }
            FetchChunkInput::Remote(chunk_download_data) => {
                remote_chunks.push_back(chunk_download_data)
            }
        }
    }

    match chunk_format {
        ChunkFormatKind::ArrowIpc => {
            arrow_prefetch_reader(
                initial_base64_opt,
                remote_chunks,
                client,
                prefetch_config,
                Some(nullable_flags),
            )
            .await
        }
        ChunkFormatKind::Json => {
            let initial_chunk_reader = get_initial_chunk_reader(
                initial_base64_opt.as_deref(),
                &mut remote_chunks,
                &client,
                prefetch_config.chunk_read_timeout,
                ChunkFormatKind::Json,
                &row_types,
            )
            .await?;
            json_chunks_reader(
                initial_chunk_reader,
                row_types,
                Vec::from(remote_chunks),
                client,
                prefetch_config,
            )
            .await
        }
    }
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
///
/// With a `read_timeout`, each attempt may wait that long for the response
/// headers and for each body frame. A body that stalls or breaks off retries
/// the whole GET with backoff, up to the retry policy's `max_attempts`. Each of
/// those attempts also runs `execute_with_retry` with the same policy, so a mix
/// of status or header failures and body stalls can issue up to `max_attempts`
/// squared GETs. Callers pass [`RetryPolicy::default`]: the connection's
/// `retry_max_attempts`, `request_timeout`, and `retry_timeout` do not bound
/// this download. Without a `read_timeout`, there is no inactivity timer and a
/// body failure is not retried.
pub async fn get_chunk_data(
    client: Client,
    chunk: ChunkDownloadData,
    read_timeout: Option<Duration>,
) -> Result<Vec<u8>, ChunkError> {
    get_chunk_data_with_policy(client, chunk, read_timeout, &RetryPolicy::default()).await
}

async fn get_chunk_data_with_policy(
    client: Client,
    chunk: ChunkDownloadData,
    read_timeout: Option<Duration>,
    policy: &RetryPolicy,
) -> Result<Vec<u8>, ChunkError> {
    let mut headers = HeaderMap::new();
    for (key, value) in chunk.headers.iter() {
        let header_name = HeaderName::from_str(key).context(HeaderNameSnafu { key })?;
        let header_value = HeaderValue::from_str(value).context(HeaderValueSnafu { key })?;
        headers.insert(header_name, header_value);
    }

    let mut http_ctx = HttpContext::new(Method::GET, chunk.url.clone()).with_idempotent(true);
    if let Some(timeout) = read_timeout {
        http_ctx = http_ctx.with_response_timeout(timeout);
    }
    let max_attempts = if read_timeout.is_some() {
        policy.max_attempts
    } else {
        1
    };
    let uncompressed_size = usize::try_from(chunk.uncompressed_size).unwrap_or(0);
    let build_request = || client.get(chunk.url.as_str()).headers(headers.clone());

    let mut sleep_ms = policy.backoff.base.as_millis() as f64;
    let mut attempt = 1;
    let bytes = loop {
        match download_chunk_body(
            &build_request,
            &http_ctx,
            policy,
            uncompressed_size,
            read_timeout,
        )
        .await
        {
            Ok(bytes) => break bytes,
            Err(ChunkAttemptError::Retryable(error)) if attempt < max_attempts => {
                let reason = if matches!(error, ChunkError::ChunkReadTimeout { .. }) {
                    "no data before the read timeout"
                } else {
                    "transport error while reading the body"
                };
                tracing::warn!(
                    attempt,
                    path = chunk.url.split(['?', '#']).next().unwrap_or(""),
                    reason,
                    "Retrying chunk download"
                );
                sleep_ms = next_delay_ms(sleep_ms, &policy.backoff);
                tokio::time::sleep(Duration::from_millis(sleep_ms as u64)).await;
                attempt += 1;
            }
            Err(ChunkAttemptError::Retryable(error) | ChunkAttemptError::Fatal(error)) => {
                return Err(error);
            }
        }
    };
    // gzip inflate is CPU-bound; run it on the blocking pool so a large chunk
    // body doesn't stall this runtime worker.
    tokio::task::spawn_blocking(move || maybe_decompress_gzip(bytes, uncompressed_size))
        .await
        .context(SpawnBlockingSnafu)?
}

enum ChunkAttemptError {
    Retryable(ChunkError),
    Fatal(ChunkError),
}

async fn download_chunk_body<B>(
    build_request: B,
    http_ctx: &HttpContext,
    policy: &RetryPolicy,
    uncompressed_size: usize,
    read_timeout: Option<Duration>,
) -> Result<Vec<u8>, ChunkAttemptError>
where
    B: Fn() -> reqwest::RequestBuilder,
{
    use ChunkAttemptError::{Fatal, Retryable};

    let mut response =
        match execute_with_retry(build_request, http_ctx, policy, |r| async move { Ok(r) }).await {
            Ok(r) => r,
            Err(e) => {
                let error = match e {
                    HttpError::Transport { source, .. } => CommunicationSnafu.into_error(source),
                    HttpError::ResponseTimeout { timeout, .. } => ChunkReadTimeoutSnafu {
                        read_timeout: timeout,
                    }
                    .build(),
                    HttpError::DeadlineExceeded { .. } | HttpError::RetryAfterExceeded { .. } => {
                        UnsuccessfulHttpStatusCodeSnafu {
                            status: reqwest::StatusCode::REQUEST_TIMEOUT,
                        }
                        .build()
                    }
                    HttpError::MaxAttempts { last_status, .. } => UnsuccessfulHttpStatusCodeSnafu {
                        status: last_status,
                    }
                    .build(),
                    HttpError::ResponseTooLarge { .. } => UnsuccessfulHttpStatusCodeSnafu {
                        status: reqwest::StatusCode::PAYLOAD_TOO_LARGE,
                    }
                    .build(),
                };
                return Err(Fatal(error));
            }
        };

    if !response.status().is_success() {
        return Err(Fatal(
            UnsuccessfulHttpStatusCodeSnafu {
                status: response.status(),
            }
            .build(),
        ));
    }

    // One buffer for the body. reqwest drops `content_length()` after it
    // inflates `Content-Encoding: gzip`, so the reservation falls back to
    // `uncompressed_size`. A raw gzip body (GCS) still reports its compressed
    // length. Both figures are advertisements: the reservation is capped, the
    // read stops at `MAX_CHUNK_BYTES`, and spare capacity is released before
    // return because the Arrow decoder keeps the allocation.
    if let Some(len) = response.content_length() {
        ensure_chunk_content_length(len).map_err(Fatal)?;
    }
    let capacity = chunk_reservation(
        response
            .content_length()
            .and_then(|len| usize::try_from(len).ok())
            .unwrap_or(uncompressed_size),
    );
    let mut bytes = Vec::with_capacity(capacity);
    loop {
        let next = match read_timeout {
            Some(timeout) => tokio::time::timeout(timeout, response.chunk())
                .await
                .map_err(|_| {
                    Retryable(
                        ChunkReadTimeoutSnafu {
                            read_timeout: timeout,
                        }
                        .build(),
                    )
                })?,
            None => response.chunk().await,
        };
        let Some(frame) = next.context(CommunicationSnafu).map_err(Retryable)? else {
            return Ok(bytes);
        };
        append_chunk_bytes(&mut bytes, &frame).map_err(Fatal)?;
    }
}

/// First allocation taken from an advertised length. Larger bodies grow from
/// here up to [`MAX_CHUNK_BYTES`].
const MAX_CHUNK_RESERVATION: usize = 8 * 1024 * 1024;

/// Largest chunk body accepted on the wire and after gzip inflate.
const MAX_CHUNK_BYTES: usize = 1024 * 1024 * 1024;

const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

fn chunk_reservation(advertised: usize) -> usize {
    advertised.min(MAX_CHUNK_RESERVATION)
}

fn ensure_chunk_content_length(len: u64) -> Result<(), ChunkError> {
    if len > MAX_CHUNK_BYTES as u64 {
        ChunkTooLargeSnafu {
            size: len,
            max_size: MAX_CHUNK_BYTES,
        }
        .fail()
    } else {
        Ok(())
    }
}

fn append_chunk_bytes(buf: &mut Vec<u8>, frame: &[u8]) -> Result<(), ChunkError> {
    append_limited(buf, frame, MAX_CHUNK_BYTES)
}

fn append_limited(buf: &mut Vec<u8>, frame: &[u8], max_bytes: usize) -> Result<(), ChunkError> {
    let new_len = buf.len().saturating_add(frame.len());
    if new_len > max_bytes {
        return ChunkTooLargeSnafu {
            size: new_len as u64,
            max_size: max_bytes,
        }
        .fail();
    }
    buf.extend_from_slice(frame);
    Ok(())
}

/// `Buffer::from_vec` retains this allocation for the life of the record batches.
fn release_excess_capacity(buf: &mut Vec<u8>) {
    let len = buf.len();
    let spare = buf.capacity().saturating_sub(len);
    if spare > len.max(64 * 1024) {
        buf.shrink_to_fit();
    }
}

fn maybe_decompress_gzip(
    mut data: Vec<u8>,
    uncompressed_size: usize,
) -> Result<Vec<u8>, ChunkError> {
    if data.len() >= 2 && data[..2] == GZIP_MAGIC {
        inflate_gzip(&data, chunk_reservation(uncompressed_size), MAX_CHUNK_BYTES)
    } else {
        release_excess_capacity(&mut data);
        Ok(data)
    }
}

fn inflate_gzip(data: &[u8], reservation: usize, max_bytes: usize) -> Result<Vec<u8>, ChunkError> {
    use flate2::bufread::GzDecoder;
    use std::io::Read as _;

    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::with_capacity(reservation.min(max_bytes));
    let mut tmp = [0u8; 64 * 1024];
    loop {
        let n = decoder.read(&mut tmp).context(ChunkDecompressionSnafu)?;
        if n == 0 {
            break;
        }
        append_limited(&mut decompressed, &tmp[..n], max_bytes)?;
    }
    release_excess_capacity(&mut decompressed);
    Ok(decompressed)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use arrow::array::{Array, ArrayRef, Int64Array, RecordBatch, StringArray};
    use arrow::datatypes::DataType;
    use arrow::ipc::writer::StreamWriter;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::query_types::RowType;

    use super::*;

    fn encode_arrow_ipc_base64(schema: SchemaRef, batches: &[RecordBatch]) -> String {
        let mut buf = Vec::new();
        {
            let mut writer = StreamWriter::try_new(&mut buf, schema.as_ref())
                .expect("StreamWriter should accept schema");
            for batch in batches {
                writer
                    .write(batch)
                    .expect("StreamWriter should accept batch");
            }
            writer.finish().expect("StreamWriter should finish");
        }
        BASE64.encode(buf)
    }

    fn gzip_compress(data: &[u8]) -> Vec<u8> {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(data)
            .expect("gzip encoder should accept input");
        encoder.finish().expect("gzip encoder should finish")
    }

    fn drain_reader(reader: Box<dyn RecordBatchReader + Send>) -> Vec<RecordBatch> {
        reader
            .collect::<Result<Vec<_>, _>>()
            .expect("draining reader should succeed")
    }

    #[test]
    fn prefetch_config_from_session_params_uses_defaults_when_missing() {
        let config = PrefetchConfig::from_session_params(&HashMap::new());
        assert_eq!(config.prefetch_threads, DEFAULT_PREFETCH_THREADS);
        assert_eq!(config.memory_limit_mb, DEFAULT_MEMORY_LIMIT_MB);
    }

    #[test]
    fn prefetch_config_from_session_params_parses_valid_values() {
        let mut params = HashMap::new();
        params.insert("CLIENT_PREFETCH_THREADS".to_string(), Setting::Int(8));
        params.insert("CLIENT_MEMORY_LIMIT".to_string(), Setting::Int(2048));

        let config = PrefetchConfig::from_session_params(&params);
        assert_eq!(config.prefetch_threads, 8);
        assert_eq!(config.memory_limit_mb, 2048);
    }

    #[test]
    fn prefetch_config_from_session_params_parses_string_values() {
        let mut params = HashMap::new();
        params.insert(
            "CLIENT_PREFETCH_THREADS".to_string(),
            Setting::String("8".to_string()),
        );
        params.insert(
            "CLIENT_MEMORY_LIMIT".to_string(),
            Setting::String("2048".to_string()),
        );

        let config = PrefetchConfig::from_session_params(&params);
        assert_eq!(config.prefetch_threads, 8);
        assert_eq!(config.memory_limit_mb, 2048);
    }

    #[test]
    fn prefetch_config_from_session_params_falls_back_on_invalid_values() {
        let mut params = HashMap::new();
        params.insert(
            "CLIENT_PREFETCH_THREADS".to_string(),
            Setting::String("not-a-number".to_string()),
        );
        params.insert("CLIENT_MEMORY_LIMIT".to_string(), Setting::Int(-1));

        let config = PrefetchConfig::from_session_params(&params);
        assert_eq!(config.prefetch_threads, DEFAULT_PREFETCH_THREADS);
        assert_eq!(config.memory_limit_mb, DEFAULT_MEMORY_LIMIT_MB);
    }

    #[test]
    fn chunk_download_data_estimated_memory_mb_applies_overhead() {
        let chunk = ChunkDownloadData {
            url: String::new(),
            row_count: 1,
            uncompressed_size: 2 * 1024 * 1024,
            compressed_size: 0,
            headers: HashMap::new(),
        };
        assert_eq!(chunk.estimated_memory_mb(), 3);
    }

    #[test]
    fn chunk_download_data_estimated_memory_mb_minimum_one() {
        let chunk = ChunkDownloadData {
            url: String::new(),
            row_count: 0,
            uncompressed_size: 0,
            compressed_size: 0,
            headers: HashMap::new(),
        };
        assert_eq!(chunk.estimated_memory_mb(), 1);
    }

    #[test]
    fn maybe_decompress_gzip_leaves_plain_bytes_unchanged() {
        let plain = b"plain chunk body".to_vec();
        let decompressed =
            maybe_decompress_gzip(plain.clone(), plain.len()).expect("plain bytes should pass");
        assert_eq!(decompressed, plain);
    }

    #[test]
    fn maybe_decompress_gzip_inflates_gzip_payload() {
        let plain = b"[ \"1\" ], [ \"2\" ]";
        let compressed = gzip_compress(plain);
        // The size hint only pre-allocates; a wrong hint must not change output.
        for hint in [0, 1, plain.len(), plain.len() * 4] {
            let decompressed = maybe_decompress_gzip(compressed.clone(), hint)
                .expect("gzip payload should decompress");
            assert_eq!(decompressed, plain);
        }
    }

    #[test]
    fn chunk_reservation_caps_an_advertised_length() {
        assert_eq!(chunk_reservation(0), 0);
        assert_eq!(chunk_reservation(1024), 1024);
        assert_eq!(chunk_reservation(usize::MAX), MAX_CHUNK_RESERVATION);
    }

    #[test]
    fn ensure_chunk_content_length_rejects_an_advertised_length_over_the_ceiling() {
        let err = ensure_chunk_content_length(MAX_CHUNK_BYTES as u64 + 1).unwrap_err();
        assert!(matches!(
            err,
            ChunkError::ChunkTooLarge { max_size, .. } if max_size == MAX_CHUNK_BYTES
        ));
        assert!(ensure_chunk_content_length(MAX_CHUNK_BYTES as u64).is_ok());
    }

    #[test]
    fn append_limited_rejects_a_frame_past_the_max() {
        let mut buf = vec![0u8; 4];
        let err = append_limited(&mut buf, &[1, 2], 5).unwrap_err();
        assert!(matches!(
            err,
            ChunkError::ChunkTooLarge {
                size: 6,
                max_size: 5,
                ..
            }
        ));
        assert_eq!(buf.len(), 4);
    }

    #[test]
    fn inflate_gzip_rejects_output_past_the_max() {
        let plain = vec![b'a'; 8 * 1024];
        let compressed = gzip_compress(&plain);
        let err = inflate_gzip(&compressed, plain.len(), 64).unwrap_err();
        assert!(matches!(err, ChunkError::ChunkTooLarge { .. }));
    }

    #[test]
    fn maybe_decompress_gzip_releases_spare_capacity_from_a_high_hint() {
        let plain = vec![b'x'; 64 * 1024];
        let compressed = gzip_compress(&plain);
        let decompressed = maybe_decompress_gzip(compressed, plain.len() * 8)
            .expect("gzip payload should decompress");
        assert_eq!(decompressed, plain);
        assert!(
            decompressed.capacity() <= decompressed.len().saturating_mul(2),
            "retained capacity {}",
            decompressed.capacity()
        );
    }

    #[test]
    fn empty_reader_returns_empty_schema_and_no_batches() {
        let mut reader = empty_reader();
        assert_eq!(reader.schema().fields().len(), 0);
        assert!(reader.next().is_none());
    }

    #[test]
    fn schema_only_reader_builds_schema_without_rows() {
        let row_types = vec![
            RowType::fixed("id", false, 10, 0),
            RowType::text("name", true, 64, 256),
        ];
        let reader = schema_only_reader(&row_types).expect("schema-only reader should build");
        let schema = reader.schema();
        assert_eq!(schema.fields().len(), 2);
        assert_eq!(schema.field(0).name(), "id");
        assert_eq!(schema.field(0).data_type(), &DataType::Int64);
        assert_eq!(schema.field(1).name(), "name");
        assert_eq!(schema.field(1).data_type(), &DataType::Utf8);
        let batches = drain_reader(reader);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].num_rows(), 0);
        assert_eq!(batches[0].schema().fields().len(), 2);
    }

    #[test]
    fn single_chunk_reader_decodes_arrow_ipc_batch() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
        ]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int64Array::from(vec![10, 20])) as ArrayRef,
                Arc::new(StringArray::from(vec![Some("a"), None])) as ArrayRef,
            ],
        )
        .expect("RecordBatch should build");
        let base64 = encode_arrow_ipc_base64(schema, &[batch]);

        let reader = single_chunk_reader(&base64, None).expect("arrow chunk should decode");
        let batches = drain_reader(reader);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].num_rows(), 2);

        let ids = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .expect("id column should be Int64");
        assert_eq!(ids.value(0), 10);
        assert_eq!(ids.value(1), 20);
    }

    #[test]
    fn single_chunk_reader_injects_nullable_metadata() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("nullable_col", DataType::Utf8, true),
            Field::new("required_col", DataType::Int64, false),
        ]));
        let batch = RecordBatch::new_empty(schema.clone());
        let base64 = encode_arrow_ipc_base64(schema, &[batch]);

        let reader =
            single_chunk_reader(&base64, Some(&[true, false])).expect("arrow chunk should decode");
        let schema = reader.schema();
        assert_eq!(
            schema.field(0).metadata().get("nullable"),
            Some(&"true".to_string())
        );
        assert_eq!(
            schema.field(1).metadata().get("nullable"),
            Some(&"false".to_string())
        );
    }

    #[test]
    fn single_chunk_reader_rejects_invalid_base64() {
        let result = single_chunk_reader("!!!not-base64!!!", None);
        assert!(matches!(result, Err(ChunkError::Base64Decode { .. })));
    }

    #[tokio::test]
    async fn json_prefetch_reader_reads_initial_rowset() {
        let initial_rowset = vec![vec![Some("1".to_string())], vec![Some("2".to_string())]];
        let row_types = vec![RowType::fixed("id", false, 10, 0)];

        let reader = json_prefetch_reader(
            &initial_rowset,
            row_types,
            vec![],
            Client::new(),
            &PrefetchConfig::default(),
        )
        .await
        .expect("initial JSON rowset should convert");

        let batches = tokio::task::spawn_blocking(move || drain_reader(reader))
            .await
            .expect("spawn_blocking should join");
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 2);
    }

    #[tokio::test]
    async fn fetch_chunks_reader_parses_inline_arrow_chunk() {
        let schema = Arc::new(Schema::new(vec![Field::new("n", DataType::Int64, false)]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(Int64Array::from(vec![42])) as ArrayRef],
        )
        .expect("RecordBatch should build");
        let inline = encode_arrow_ipc_base64(schema, &[batch]);

        let reader = fetch_chunks_reader(
            vec![FetchChunkInput::Inline(inline)],
            ChunkFormatKind::ArrowIpc,
            vec![],
            &[false],
            Client::new(),
            &PrefetchConfig::default(),
        )
        .await
        .expect("inline Arrow chunk should decode");

        let batches = tokio::task::spawn_blocking(move || drain_reader(reader))
            .await
            .expect("spawn_blocking should join");
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 1);
        let ids = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .expect("n column should be Int64");
        assert_eq!(ids.value(0), 42);
    }

    #[tokio::test]
    async fn fetch_chunks_reader_parses_remote_only_arrow_chunks() {
        let schema = Arc::new(Schema::new(vec![Field::new("n", DataType::Int64, false)]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(Int64Array::from(vec![7, 8])) as ArrayRef],
        )
        .expect("RecordBatch should build");
        let arrow_body = {
            let mut buf = Vec::new();
            {
                let mut writer = StreamWriter::try_new(&mut buf, schema.as_ref())
                    .expect("StreamWriter should accept schema");
                writer
                    .write(&batch)
                    .expect("StreamWriter should accept batch");
                writer.finish().expect("StreamWriter should finish");
            }
            buf
        };

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(arrow_body))
            .mount(&server)
            .await;

        let chunks = vec![FetchChunkInput::Remote(ChunkDownloadData {
            url: server.uri(),
            row_count: 2,
            uncompressed_size: 64,
            compressed_size: 64,
            headers: HashMap::new(),
        })];

        let reader = fetch_chunks_reader(
            chunks,
            ChunkFormatKind::ArrowIpc,
            vec![],
            &[false],
            Client::new(),
            &PrefetchConfig::default(),
        )
        .await
        .expect("remote-only Arrow fetch should succeed");

        let batches = tokio::task::spawn_blocking(move || drain_reader(reader))
            .await
            .expect("spawn_blocking should join");
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 2);
    }

    /// Each body shape must arrive intact whatever the reported size. A high
    /// hint pre-allocates and is released; it does not stay in the returned buffer.
    #[tokio::test]
    async fn get_chunk_data_reads_plain_encoded_and_raw_gzip_bodies() {
        // Large enough to arrive in several frames.
        let plain: Vec<u8> = (0..300_000u32).flat_map(|i| i.to_le_bytes()).collect();
        let compressed = gzip_compress(&plain);
        let bodies = [
            (
                "plain",
                ResponseTemplate::new(200).set_body_bytes(plain.clone()),
            ),
            (
                "content-encoding gzip",
                ResponseTemplate::new(200)
                    .insert_header("Content-Encoding", "gzip")
                    .set_body_bytes(compressed.clone()),
            ),
            (
                "raw gzip",
                ResponseTemplate::new(200).set_body_bytes(compressed.clone()),
            ),
        ];
        for (name, response) in bodies {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .respond_with(response)
                .mount(&server)
                .await;
            for uncompressed_size in [0, 1, plain.len() as i64, plain.len() as i64 * 4] {
                let chunk = ChunkDownloadData {
                    url: server.uri(),
                    row_count: 1,
                    uncompressed_size,
                    compressed_size: compressed.len() as i64,
                    headers: HashMap::new(),
                };
                let bytes = get_chunk_data(Client::new(), chunk, None)
                    .await
                    .unwrap_or_else(|e| panic!("{name}: download failed: {e:?}"));
                assert_eq!(bytes, plain, "{name} with size {uncompressed_size}");
                assert!(
                    bytes.capacity() <= bytes.len().saturating_mul(2),
                    "{name} with size {uncompressed_size} retained capacity {}",
                    bytes.capacity()
                );
            }
        }
    }

    #[tokio::test]
    async fn fetch_chunks_reader_parses_remote_only_json_chunks() {
        let server = MockServer::start().await;
        let json_body = br#"[ "1" ], [ "2" ]"#;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(json_body, "application/json"))
            .mount(&server)
            .await;

        let row_types = vec![RowType::fixed("id", false, 10, 0)];
        let chunks = vec![FetchChunkInput::Remote(ChunkDownloadData {
            url: server.uri(),
            row_count: 2,
            uncompressed_size: json_body.len() as i64,
            compressed_size: json_body.len() as i64,
            headers: HashMap::new(),
        })];

        let reader = fetch_chunks_reader(
            chunks,
            ChunkFormatKind::Json,
            row_types,
            &[false],
            Client::new(),
            &PrefetchConfig::default(),
        )
        .await
        .expect("remote-only JSON fetch should succeed");

        let batches = tokio::task::spawn_blocking(move || {
            reader
                .collect::<Result<Vec<_>, _>>()
                .expect("draining reader should succeed")
        })
        .await
        .expect("spawn_blocking should join");

        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 2);
        let data_batch = batches
            .iter()
            .find(|batch| batch.num_rows() > 0)
            .expect("expected a non-empty batch");
        let ids = data_batch
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .expect("id column should be Int64");
        assert_eq!(ids.value(0), 1);
        assert_eq!(ids.value(1), 2);
    }

    #[tokio::test]
    async fn fetch_chunks_reader_rejects_multiple_inline_chunks() {
        let chunks = vec![
            FetchChunkInput::Inline("first".to_string()),
            FetchChunkInput::Inline("second".to_string()),
        ];

        let result = fetch_chunks_reader(
            chunks,
            ChunkFormatKind::Json,
            Vec::new(),
            &[],
            Client::new(),
            &PrefetchConfig::default(),
        )
        .await;

        assert!(matches!(
            result,
            Err(ChunkError::MultipleInlineChunks { .. })
        ));
    }

    #[derive(Clone, Copy)]
    enum Serve {
        StallBeforeHeaders,
        StallMidBody,
        CloseMidBody,
        Full,
    }

    /// Serves connection `n` according to `plan[n]`, repeating the last entry.
    /// A stalled connection stays open and sends nothing more.
    async fn chunk_server(plan: Vec<Serve>, body: Vec<u8>) -> (String, Arc<AtomicUsize>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/chunk", listener.local_addr().unwrap());
        let accepted = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&accepted);
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let index = counter.fetch_add(1, Ordering::SeqCst);
                let serve = plan[index.min(plan.len() - 1)];
                let body = body.clone();
                tokio::spawn(async move {
                    if !read_request_head(&mut socket).await {
                        return;
                    }
                    let head = format!("HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n", body.len());
                    match serve {
                        Serve::StallBeforeHeaders => {}
                        Serve::StallMidBody => {
                            let _ = socket.write_all(head.as_bytes()).await;
                            let _ = socket.write_all(&body[..body.len() / 2]).await;
                        }
                        Serve::CloseMidBody => {
                            let _ = socket.write_all(head.as_bytes()).await;
                            let _ = socket.write_all(&body[..body.len() / 2]).await;
                            return;
                        }
                        Serve::Full => {
                            let _ = socket.write_all(head.as_bytes()).await;
                            let _ = socket.write_all(&body).await;
                            return;
                        }
                    }
                    std::future::pending::<()>().await;
                });
            }
        });
        (url, accepted)
    }

    async fn read_request_head(socket: &mut tokio::net::TcpStream) -> bool {
        let mut request = Vec::new();
        let mut buf = [0u8; 1024];
        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
            match socket.read(&mut buf).await {
                Ok(0) | Err(_) => return false,
                Ok(n) => request.extend_from_slice(&buf[..n]),
            }
        }
        true
    }

    fn served_chunk(url: String, body: &[u8]) -> ChunkDownloadData {
        ChunkDownloadData {
            url,
            row_count: 1,
            uncompressed_size: body.len() as i64,
            compressed_size: body.len() as i64,
            headers: HashMap::new(),
        }
    }

    const TEST_READ_TIMEOUT: Duration = Duration::from_millis(300);
    const TEST_MAX_ATTEMPTS: u32 = 3;

    fn fixed_backoff_policy(delay: Duration) -> RetryPolicy {
        RetryPolicy {
            max_attempts: TEST_MAX_ATTEMPTS,
            backoff: crate::config::retry::BackoffConfig {
                base: delay,
                factor: 1.0,
                cap: delay,
                jitter: crate::config::retry::Jitter::None,
            },
            ..RetryPolicy::default()
        }
    }

    fn served_body() -> Vec<u8> {
        (0..64_000u32).flat_map(|i| i.to_le_bytes()).collect()
    }

    async fn fetch_served(
        url: String,
        body: &[u8],
        read_timeout: Option<Duration>,
    ) -> Result<Vec<u8>, ChunkError> {
        get_chunk_data_with_policy(
            Client::new(),
            served_chunk(url, body),
            read_timeout,
            &fixed_backoff_policy(Duration::ZERO),
        )
        .await
    }

    #[tokio::test]
    async fn get_chunk_data_retries_a_response_that_stalls_mid_body() {
        let body = served_body();
        let (url, accepted) =
            chunk_server(vec![Serve::StallMidBody, Serve::Full], body.clone()).await;

        let bytes = fetch_served(url, &body, Some(TEST_READ_TIMEOUT))
            .await
            .expect("the retried GET should return the full body");

        assert_eq!(bytes, body);
        assert_eq!(accepted.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn get_chunk_data_retries_a_response_that_never_sends_headers() {
        let body = served_body();
        let (url, accepted) =
            chunk_server(vec![Serve::StallBeforeHeaders, Serve::Full], body.clone()).await;

        let bytes = fetch_served(url, &body, Some(TEST_READ_TIMEOUT))
            .await
            .expect("the retried GET should return the full body");

        assert_eq!(bytes, body);
        assert_eq!(accepted.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn get_chunk_data_fails_with_a_read_timeout_once_every_body_stalls() {
        let body = served_body();
        let (url, accepted) = chunk_server(vec![Serve::StallMidBody], body.clone()).await;

        let error = fetch_served(url, &body, Some(TEST_READ_TIMEOUT))
            .await
            .expect_err("a body that never finishes should fail the download");

        assert!(
            matches!(error, ChunkError::ChunkReadTimeout { read_timeout, .. } if read_timeout == TEST_READ_TIMEOUT),
            "unexpected error: {error:?}"
        );
        assert_eq!(accepted.load(Ordering::SeqCst), TEST_MAX_ATTEMPTS as usize);
    }

    #[tokio::test]
    async fn get_chunk_data_fails_with_a_read_timeout_once_every_response_wait_stalls() {
        let body = served_body();
        let (url, accepted) = chunk_server(vec![Serve::StallBeforeHeaders], body.clone()).await;

        let error = fetch_served(url, &body, Some(TEST_READ_TIMEOUT))
            .await
            .expect_err("a response that never arrives should fail the download");

        assert!(
            matches!(error, ChunkError::ChunkReadTimeout { read_timeout, .. } if read_timeout == TEST_READ_TIMEOUT),
            "unexpected error: {error:?}"
        );
        assert_eq!(accepted.load(Ordering::SeqCst), TEST_MAX_ATTEMPTS as usize);
    }

    #[tokio::test]
    async fn get_chunk_data_keeps_a_slow_body_that_keeps_arriving() {
        let body = served_body();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/chunk", listener.local_addr().unwrap());
        let served = body.clone();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            assert!(read_request_head(&mut socket).await);
            let head = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n",
                served.len()
            );
            socket.write_all(head.as_bytes()).await.unwrap();
            for part in served.chunks(served.len() / 4) {
                tokio::time::sleep(Duration::from_millis(100)).await;
                socket.write_all(part).await.unwrap();
            }
        });

        let bytes = fetch_served(url, &body, Some(Duration::from_secs(1)))
            .await
            .expect("a body that keeps arriving should not time out");

        assert_eq!(bytes, body);
        server
            .await
            .expect("the server should serve one connection");
    }

    #[tokio::test]
    async fn get_chunk_data_backoff_between_status_retries_does_not_count_as_inactivity() {
        let body = served_body();
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
            .mount(&server)
            .await;

        let bytes = get_chunk_data_with_policy(
            Client::new(),
            served_chunk(format!("{}/chunk", server.uri()), &body),
            Some(TEST_READ_TIMEOUT),
            &fixed_backoff_policy(TEST_READ_TIMEOUT * 2),
        )
        .await
        .expect("a 503 followed by a backoff longer than the read timeout should still succeed");

        assert_eq!(bytes, body);
        assert_eq!(server.received_requests().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn get_chunk_data_without_read_timeout_does_not_retry_a_broken_body() {
        let body = served_body();
        let (url, accepted) =
            chunk_server(vec![Serve::CloseMidBody, Serve::Full], body.clone()).await;

        let error = fetch_served(url, &body, None)
            .await
            .expect_err("a body cut short should fail when no read timeout is set");

        assert!(
            matches!(error, ChunkError::Communication { .. }),
            "unexpected error: {error:?}"
        );
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn get_chunk_data_with_read_timeout_retries_a_broken_body() {
        let body = served_body();
        let (url, accepted) =
            chunk_server(vec![Serve::CloseMidBody, Serve::Full], body.clone()).await;

        let bytes = fetch_served(url, &body, Some(TEST_READ_TIMEOUT))
            .await
            .expect("the retried GET should return the full body");

        assert_eq!(bytes, body);
        assert_eq!(accepted.load(Ordering::SeqCst), 2);
    }
}
