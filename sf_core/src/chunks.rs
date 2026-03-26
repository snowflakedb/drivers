use std::collections::{HashMap, VecDeque};
use std::io;
use std::str::FromStr;
use std::sync::Arc;

use crate::arrow_utils::convert_string_rowset_to_arrow_reader;
use crate::compression::{CompressionError, decompress_data};
use crate::query_types::RowType;
use arrow::array::{RecordBatch, RecordBatchReader};
use arrow::datatypes::{Fields, Schema, SchemaRef};
use arrow::error::ArrowError;
use arrow_ipc::reader::StreamReader;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use futures::stream::{self, StreamExt};
use reqwest::Client;
use reqwest::header::{self, HeaderMap, HeaderName, HeaderValue};
use snafu::{Location, OptionExt, ResultExt, Snafu};

const MAX_CHUNK_DECOMPRESSION_RETRIES: u32 = 2;
pub const DEFAULT_PREFETCH_THREADS: usize = 8;

#[derive(Debug)]
pub struct ChunkDownloadData {
    pub url: String,
    pub headers: HashMap<String, String>,
}

impl ChunkDownloadData {
    pub fn new(chunk_url: &str, chunk_headers: &HashMap<String, String>) -> Self {
        Self {
            url: chunk_url.to_string(),
            headers: chunk_headers.clone(),
        }
    }
}

/// Reads Arrow record batches from one or more Snowflake result-set chunks.
///
/// For multi-chunk results, a background task downloads chunks in parallel,
/// parses the Arrow IPC data, and sends ready-to-consume `RecordBatch`es
/// through a bounded channel. This overlaps downloading, parsing, and
/// consumption across chunks.
///
/// # Sync iteration constraint
///
/// The `Iterator` implementation uses `blocking_recv()` on the prefetch channel.
/// This **must not** be called from within an active tokio runtime context
/// (e.g., inside a `Runtime::block_on` or from a spawned task), as doing so
/// will panic (current-thread runtime) or deadlock (multi-thread runtime).
///
/// All current consumers (ODBC `fetch`, C API wrappers, tests) iterate the
/// reader from a plain synchronous context, which is correct.
pub struct ChunkReader {
    schema: SchemaRef,
    /// Inline stream for single-chunk results (no background task needed).
    current_stream: Option<StreamReader<io::Cursor<Vec<u8>>>>,
    /// Pre-parsed batches from the background prefetch pipeline (multi-chunk).
    batch_rx: Option<tokio::sync::mpsc::Receiver<Result<RecordBatch, ArrowError>>>,
}

impl ChunkReader {
    pub async fn multi_chunk(
        initial_base64_opt: Option<&str>,
        mut rest: VecDeque<ChunkDownloadData>,
        client: Client,
        prefetch_concurrency: usize,
    ) -> Result<Self, ChunkError> {
        let initial_bytes = if let Some(initial) = initial_base64_opt {
            BASE64.decode(initial).context(Base64DecodingSnafu)?
        } else {
            let first = rest.pop_front().context(InitialChunkMissingSnafu)?;
            get_chunk_data(&client, &first).await?
        };
        let cursor = io::Cursor::new(initial_bytes.clone());
        let schema_reader = StreamReader::try_new(cursor, None).context(ChunkReadingSnafu)?;
        let schema = schema_reader.schema().clone();

        let (tx, rx) = tokio::sync::mpsc::channel(2);
        tokio::spawn(prefetch_batches(
            initial_bytes,
            rest,
            client,
            tx,
            prefetch_concurrency,
        ));

        Ok(Self {
            schema,
            current_stream: None,
            batch_rx: Some(rx),
        })
    }

    pub fn single_chunk(base64: &str) -> Result<Self, ChunkError> {
        let bytes = BASE64.decode(base64).context(Base64DecodingSnafu)?;
        let cursor = io::Cursor::new(bytes);
        let reader = StreamReader::try_new(cursor, None).context(ChunkReadingSnafu)?;
        Ok(Self {
            schema: reader.schema().clone(),
            current_stream: Some(reader),
            batch_rx: None,
        })
    }

    pub fn empty() -> Self {
        Self {
            schema: Arc::new(Schema::new(Fields::empty())),
            current_stream: None,
            batch_rx: None,
        }
    }
}

/// Background task that downloads, parses, and sends `RecordBatch`es through
/// `tx`. The first chunk's raw bytes are parsed alongside the remaining chunks,
/// which are downloaded with up to `concurrency` HTTP requests in flight.
/// Backpressure is provided by the bounded channel.
async fn prefetch_batches(
    first_chunk_bytes: Vec<u8>,
    rest: VecDeque<ChunkDownloadData>,
    client: Client,
    tx: tokio::sync::mpsc::Sender<Result<RecordBatch, ArrowError>>,
    concurrency: usize,
) {
    stream::once(async { Ok(first_chunk_bytes) })
        .chain(
            stream::iter(rest)
                .map(move |chunk| {
                    let client = client.clone();
                    async move { get_chunk_data(&client, &chunk).await }
                })
                .buffered(concurrency),
        )
        .map(|chunk_result| match chunk_result {
            Ok(data) => {
                let cursor = io::Cursor::new(data);
                match StreamReader::try_new(cursor, None) {
                    Ok(reader) => stream::iter(reader).left_stream(),
                    Err(e) => stream::iter(vec![Err(e)]).right_stream(),
                }
            }
            Err(e) => stream::iter(vec![Err(ArrowError::IpcError(e.to_string()))]).right_stream(),
        })
        .flatten()
        .for_each(|batch| {
            let tx = tx.clone();
            async move {
                let _ = tx.send(batch).await;
            }
        })
        .await;
}

impl Iterator for ChunkReader {
    type Item = Result<RecordBatch, ArrowError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref mut stream) = self.current_stream {
            match stream.next() {
                some @ Some(_) => return some,
                None => self.current_stream = None,
            }
        }
        if let Some(ref mut rx) = self.batch_rx {
            return rx.blocking_recv();
        }
        None
    }
}

impl RecordBatchReader for ChunkReader {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

/// Reads Arrow record batches from JSON result-set chunks, converting each
/// chunk from JSON rows to Arrow on demand.
///
/// Mirrors `ChunkReader`'s prefetch pattern: a background task downloads
/// chunks concurrently, parses JSON, converts to Arrow, and sends
/// `RecordBatch`es through a bounded channel.
///
/// See `ChunkReader` for the sync-iteration constraint.
pub struct JsonChunkReader {
    schema: SchemaRef,
    batch_rx: tokio::sync::mpsc::Receiver<Result<RecordBatch, ArrowError>>,
}

impl JsonChunkReader {
    pub async fn new(
        initial_rowset: &[Vec<Option<String>>],
        row_types: Vec<RowType>,
        chunk_download_data: Vec<ChunkDownloadData>,
        client: Client,
        prefetch_concurrency: usize,
    ) -> Result<Self, ChunkError> {
        let reader = convert_string_rowset_to_arrow_reader(initial_rowset, &row_types)
            .map_err(|e| ArrowError::ExternalError(Box::new(e)))
            .context(ChunkReadingSnafu)?;
        let schema = reader.schema();
        let initial_batches: Vec<RecordBatch> = reader
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .context(ChunkReadingSnafu)?;

        let (tx, rx) = tokio::sync::mpsc::channel(2);
        tokio::spawn(prefetch_json_batches(
            initial_batches,
            chunk_download_data.into(),
            row_types,
            client,
            tx,
            prefetch_concurrency,
        ));

        Ok(Self {
            schema,
            batch_rx: rx,
        })
    }
}

/// Background task that sends the initial rowset batches and downloads
/// remaining JSON chunks concurrently, converting each to Arrow and sending
/// through `tx`.
async fn prefetch_json_batches(
    initial_batches: Vec<RecordBatch>,
    rest: VecDeque<ChunkDownloadData>,
    row_types: Vec<RowType>,
    client: Client,
    tx: tokio::sync::mpsc::Sender<Result<RecordBatch, ArrowError>>,
    concurrency: usize,
) {
    for batch in initial_batches {
        if tx.send(Ok(batch)).await.is_err() {
            return;
        }
    }

    let row_types = Arc::new(row_types);

    let mut chunk_stream = stream::iter(rest)
        .map(move |chunk| {
            let client = client.clone();
            let row_types = Arc::clone(&row_types);
            async move { download_and_parse_json_chunk(&client, &chunk, &row_types).await }
        })
        .buffered(concurrency)
        .map(|result| match result {
            Ok(batches) => stream::iter(batches.into_iter().map(Ok)).left_stream(),
            Err(e) => stream::iter(vec![Err(e)]).right_stream(),
        })
        .flatten();

    while let Some(result) = chunk_stream.next().await {
        if tx.send(result).await.is_err() {
            return;
        }
    }
}

async fn download_and_parse_json_chunk(
    client: &Client,
    chunk: &ChunkDownloadData,
    row_types: &[RowType],
) -> Result<Vec<RecordBatch>, ArrowError> {
    let chunk_bytes = get_chunk_data(client, chunk)
        .await
        .map_err(|e| ArrowError::ExternalError(Box::new(e)))?;

    // Chunk data is comma-separated row arrays without outer brackets,
    // e.g.: [null,"a",...],\n["b","c",...],\n...
    // Wrap in [] to make it a valid JSON array.
    let mut wrapped = Vec::with_capacity(chunk_bytes.len() + 2);
    wrapped.push(b'[');
    wrapped.extend_from_slice(&chunk_bytes);
    wrapped.push(b']');

    let chunk_rows: Vec<Vec<Option<String>>> =
        serde_json::from_slice(&wrapped).map_err(|e| ArrowError::ExternalError(Box::new(e)))?;

    let expected_cols = row_types.len();
    if let Some((row_idx, row)) = chunk_rows
        .iter()
        .enumerate()
        .find(|(_, r)| r.len() != expected_cols)
    {
        return Err(ArrowError::InvalidArgumentError(format!(
            "JSON chunk row {row_idx} has {} columns, expected {expected_cols}",
            row.len()
        )));
    }

    let reader = convert_string_rowset_to_arrow_reader(&chunk_rows, row_types)
        .map_err(|e| ArrowError::ExternalError(Box::new(e)))?;

    reader.into_iter().collect::<Result<Vec<_>, _>>()
}

impl Iterator for JsonChunkReader {
    type Item = Result<RecordBatch, ArrowError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.batch_rx.blocking_recv()
    }
}

impl RecordBatchReader for JsonChunkReader {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

pub async fn get_chunk_data(
    client: &Client,
    chunk: &ChunkDownloadData,
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

    let mut decompress_attempt = 0;
    loop {
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
                        UnsuccessfulResponseHTTPSnafu {
                            status: reqwest::StatusCode::REQUEST_TIMEOUT,
                        }
                        .fail()
                    }
                    HttpError::MaxAttempts { last_status, .. } => UnsuccessfulResponseHTTPSnafu {
                        status: last_status,
                    }
                    .fail(),
                };
            }
        };

        if !response.status().is_success() {
            UnsuccessfulResponseHTTPSnafu {
                status: response.status(),
            }
            .fail()?;
        }

        let encoding_header = response.headers().get(header::CONTENT_ENCODING).cloned();
        let body = response.bytes().await.context(CommunicationSnafu)?.to_vec();

        match decode_chunk_body(body, encoding_header.as_ref()) {
            Ok(decoded) => return Ok(decoded),
            Err(err @ ChunkError::Decompression { .. }) => {
                if decompress_attempt >= MAX_CHUNK_DECOMPRESSION_RETRIES {
                    return Err(err);
                }
                decompress_attempt += 1;
                tracing::warn!(
                    attempt = decompress_attempt,
                    url = %url,
                    "Chunk decompression failed, retrying"
                );
                continue;
            }
            Err(err) => return Err(err),
        }
    }
}

fn decode_chunk_body(body: Vec<u8>, encoding: Option<&HeaderValue>) -> Result<Vec<u8>, ChunkError> {
    let Some(value) = encoding else {
        return Ok(body);
    };

    let encoding_str = value.to_str().context(ContentEncodingHeaderSnafu)?;

    let mut data = body;
    for token in encoding_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        if token.eq_ignore_ascii_case("identity") {
            continue;
        }
        if token.eq_ignore_ascii_case("gzip") {
            data = decompress_data(&data).context(DecompressionSnafu)?;
            continue;
        }
        return UnsupportedEncodingSnafu {
            encoding: token.to_string(),
        }
        .fail();
    }

    Ok(data)
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
pub enum ChunkError {
    #[snafu(display("Invalid header name for {key}"))]
    HeaderName {
        key: String,
        source: reqwest::header::InvalidHeaderName,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid header value for {key}"))]
    HeaderValue {
        key: String,
        source: reqwest::header::InvalidHeaderValue,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to communicate with Snowflake to get chunk data"))]
    Communication {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Snowflake responded with non-successful HTTP status"))]
    UnsuccessfulResponseHTTP {
        status: reqwest::StatusCode,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to decompress chunk data: {source}"))]
    Decompression {
        source: CompressionError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Content-Encoding header is not valid UTF-8"))]
    ContentEncodingHeader {
        source: reqwest::header::ToStrError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Unsupported content encoding: {encoding}"))]
    UnsupportedEncoding {
        encoding: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read chunk data"))]
    ChunkReading {
        source: ArrowError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to decode base64 data"))]
    Base64Decoding {
        source: base64::DecodeError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("No initial inline data and no remote chunks available to derive schema"))]
    InitialChunkMissing {
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compression::compress_data;

    fn header(value: &str) -> HeaderValue {
        HeaderValue::from_str(value).expect("valid header value")
    }

    #[test]
    fn decode_chunk_body_identity_returns_original() {
        let source = b"hello world".to_vec();
        let identity = header("identity");
        let decoded =
            decode_chunk_body(source.clone(), Some(&identity)).expect("identity succeeds");
        assert_eq!(decoded, source);
    }

    #[test]
    fn decode_chunk_body_supports_gzip() {
        let payload = b"payload".to_vec();
        let compressed = compress_data(payload.clone()).expect("compression succeeds");
        let gzip = header("gzip");
        let decoded = decode_chunk_body(compressed, Some(&gzip)).expect("gzip decodes");
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_chunk_body_supports_mixed_encodings() {
        let payload = b"abc123".to_vec();
        let compressed = compress_data(payload.clone()).expect("compression succeeds");
        let gzip_identity = header(" gzip , identity ");
        let decoded =
            decode_chunk_body(compressed, Some(&gzip_identity)).expect("mixed encodings decode");
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_chunk_body_rejects_unsupported_encoding() {
        let data = b"bytes".to_vec();
        let unsupported = header("br");
        let err = decode_chunk_body(data, Some(&unsupported)).expect_err("br unsupported");
        match err {
            ChunkError::UnsupportedEncoding { encoding, .. } => assert_eq!(encoding, "br"),
            other => panic!("expected unsupported-encoding error, got {other:?}"),
        }
    }
}
