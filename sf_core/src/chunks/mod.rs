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
use arrow::array::{RecordBatch, RecordBatchIterator, RecordBatchReader};
use arrow::datatypes::{Field, Fields, Schema, SchemaRef};
use arrow_ipc::reader::StreamReader;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
pub use error::ChunkError;
use error::*;
pub(crate) use error::{ArrowIpcEncodeSnafu, ChunkReadSnafu};
pub use json_parser::convert_string_rowset_to_arrow_reader;
use prefetch::{
    ArrowChunkParser, HttpChunkDownloader, JsonChunkParser, ParseChunk, PrefetchChunkReader,
};
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use snafu::{OptionExt, ResultExt, ensure};

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
    json_chunks_reader(
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
    let initial_reader = get_initial_chunk_reader(
        initial_base64_opt,
        &mut chunk_download_data,
        &client,
        ChunkFormatKind::ArrowIpc,
        &[],
    )
    .await?;
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

/// Builds the initial reader from either the inline base64 chunk (always Arrow
/// IPC) or by popping and fetching the first remote chunk. Remote chunks are
/// parsed according to `remote_format`: Arrow IPC goes through `StreamReader`,
/// JSON goes through `JsonChunkParser`.
async fn get_initial_chunk_reader(
    initial_base64_opt: Option<&str>,
    chunk_download_data: &mut VecDeque<ChunkDownloadData>,
    client: &Client,
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
        let bytes = get_chunk_data(client.clone(), first).await?;
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
                initial_base64_opt.as_deref(),
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
                HttpError::ResponseTooLarge { .. } => UnsuccessfulHttpStatusCodeSnafu {
                    status: reqwest::StatusCode::PAYLOAD_TOO_LARGE,
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
            .context(ChunkDecompressionSnafu)?;
        Ok(decompressed)
    } else {
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

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
        params.insert("CLIENT_PREFETCH_THREADS".to_string(), "8".to_string());
        params.insert("CLIENT_MEMORY_LIMIT".to_string(), "2048".to_string());

        let config = PrefetchConfig::from_session_params(&params);
        assert_eq!(config.prefetch_threads, 8);
        assert_eq!(config.memory_limit_mb, 2048);
    }

    #[test]
    fn prefetch_config_from_session_params_falls_back_on_invalid_values() {
        let mut params = HashMap::new();
        params.insert(
            "CLIENT_PREFETCH_THREADS".to_string(),
            "not-a-number".to_string(),
        );
        params.insert("CLIENT_MEMORY_LIMIT".to_string(), "-1".to_string());

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
        let decompressed = maybe_decompress_gzip(plain.clone()).expect("plain bytes should pass");
        assert_eq!(decompressed, plain);
    }

    #[test]
    fn maybe_decompress_gzip_inflates_gzip_payload() {
        let plain = b"[ \"1\" ], [ \"2\" ]";
        let compressed = gzip_compress(plain);
        let decompressed =
            maybe_decompress_gzip(compressed).expect("gzip payload should decompress");
        assert_eq!(decompressed, plain);
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
}
