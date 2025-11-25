use std::collections::{HashMap, VecDeque};
use std::io;
use std::str::FromStr;

use crate::compression::{CompressionError, decompress_data};
use arrow::array::{RecordBatch, RecordBatchReader};
use arrow::datatypes::SchemaRef;
use arrow::error::ArrowError;
use arrow_ipc::reader::StreamReader;
use once_cell::sync::Lazy;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use snafu::{Location, ResultExt, Snafu};

static CHUNK_HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

pub struct ChunkDownloadData {
    url: String,
    headers: HashMap<String, String>,
}

impl ChunkDownloadData {
    pub fn new(chunk_url: &str, chunk_headers: &HashMap<String, String>) -> Self {
        Self {
            url: chunk_url.to_string(),
            headers: chunk_headers.clone(),
        }
    }
}
pub struct ChunkReader {
    rest: VecDeque<ChunkDownloadData>,
    schema: SchemaRef,
    current_stream: Option<StreamReader<io::Cursor<Vec<u8>>>>,
}

impl ChunkReader {
    pub async fn multi_chunk(
        initial: Vec<u8>,
        mut rest: VecDeque<ChunkDownloadData>,
    ) -> Result<Self, ChunkError> {
        let initial = if initial.is_empty() {
            get_chunk_data(&rest.pop_front().unwrap()).await?
        } else {
            initial
        };
        let cursor = io::Cursor::new(initial);
        let reader = StreamReader::try_new(cursor, None).context(ChunkReadingSnafu)?;
        let schema = reader.schema().clone();
        Ok(Self {
            rest,
            schema,
            current_stream: Some(reader),
        })
    }

    pub fn single_chunk(initial: Vec<u8>) -> Result<Self, ChunkError> {
        let cursor = io::Cursor::new(initial);
        let reader = StreamReader::try_new(cursor, None).context(ChunkReadingSnafu)?;
        Ok(Self {
            rest: VecDeque::new(),
            schema: reader.schema().clone(),
            current_stream: Some(reader),
        })
    }
}

impl Iterator for ChunkReader {
    type Item = Result<RecordBatch, ArrowError>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(mut current_stream) = self.current_stream.take() {
            let next_batch = current_stream.next();
            if next_batch.is_some() {
                self.current_stream = Some(current_stream);
                return next_batch;
            }
            if let Some(chunk) = self.rest.pop_front() {
                let chunk_data_result = get_chunk_data_sync(&chunk);
                if let Err(e) = chunk_data_result {
                    return Some(Err(ArrowError::IpcError(e.to_string())));
                }
                let data = chunk_data_result.unwrap();
                let cursor = io::Cursor::new(data);
                let reader = match StreamReader::try_new(cursor, None) {
                    Ok(r) => r,
                    Err(e) => return Some(Err(e)),
                };
                self.current_stream = Some(reader);
            }
        }
        None
    }
}

impl RecordBatchReader for ChunkReader {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

pub fn get_chunk_data_sync(chunk: &ChunkDownloadData) -> Result<Vec<u8>, ChunkError> {
    // TODO: Find a better way of managing tokio runtimes
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async { get_chunk_data(chunk).await })
}

pub async fn get_chunk_data(chunk: &ChunkDownloadData) -> Result<Vec<u8>, ChunkError> {
    let url = chunk.url.clone();
    let client = CHUNK_HTTP_CLIENT.clone();
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

    let build = || client.get(url.clone()).headers(headers.clone());
    let response = match execute_with_retry(build, &ctx, &policy, |r| async move { Ok(r) }).await {
        Ok(r) => r,
        Err(e) => {
            return Err(match e {
                HttpError::Transport { source, .. } => ChunkError::Communication {
                    source,
                    location: Location::new(file!(), line!(), column!()),
                },
                HttpError::DeadlineExceeded { .. } | HttpError::RetryAfterExceeded { .. } => {
                    ChunkError::UnsuccessfulResponseHTTP {
                        status: reqwest::StatusCode::REQUEST_TIMEOUT,
                        location: Location::new(file!(), line!(), column!()),
                    }
                }
                HttpError::MaxAttempts { last_status, .. } => {
                    ChunkError::UnsuccessfulResponseHTTP {
                        status: last_status,
                        location: Location::new(file!(), line!(), column!()),
                    }
                }
            });
        }
    };

    if !response.status().is_success() {
        UnsuccessfulResponseHTTPSnafu {
            status: response.status(),
        }
        .fail()?;
    }
    tracing::debug!("Chunk response: {:?}", response);
    let body = if response.headers().get("Content-Encoding")
        == Some(&HeaderValue::from_str("gzip").unwrap())
    {
        tracing::debug!("Decompressing chunk data");
        let compressed_body = response.bytes().await.context(CommunicationSnafu)?;
        decompress_data(compressed_body.to_vec()).context(DecompressionSnafu)?
    } else {
        response.bytes().await.context(CommunicationSnafu)?.to_vec()
    };

    Ok(body)
}

#[derive(Snafu, Debug)]
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
    #[snafu(display("Failed to decompress chunk data"))]
    Decompression {
        source: CompressionError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read chunk data"))]
    ChunkReading {
        source: ArrowError,
        #[snafu(implicit)]
        location: Location,
    },
}
