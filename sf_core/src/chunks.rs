use std::collections::{HashMap, VecDeque};
use std::io;
use std::str::FromStr;

use crate::compression::{CompressionError, decompress_data};
use arrow::array::{RecordBatch, RecordBatchReader};
use arrow::datatypes::SchemaRef;
use arrow::error::ArrowError;
use arrow_ipc::reader::StreamReader;
use reqwest::Client;
use reqwest::header::{self, HeaderMap, HeaderName, HeaderValue};
use snafu::{Location, ResultExt, Snafu};

const MAX_CHUNK_DECOMPRESSION_RETRIES: u32 = 2;

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
    client: Option<Client>,
}

impl ChunkReader {
    pub async fn multi_chunk(
        initial: Vec<u8>,
        mut rest: VecDeque<ChunkDownloadData>,
        client: Client,
    ) -> Result<Self, ChunkError> {
        let initial = if initial.is_empty() {
            get_chunk_data(&client, &rest.pop_front().unwrap()).await?
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
            client: Some(client),
        })
    }

    pub fn single_chunk(initial: Vec<u8>) -> Result<Self, ChunkError> {
        let cursor = io::Cursor::new(initial);
        let reader = StreamReader::try_new(cursor, None).context(ChunkReadingSnafu)?;
        Ok(Self {
            rest: VecDeque::new(),
            schema: reader.schema().clone(),
            current_stream: Some(reader),
            client: None,
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
                let client = match &self.client {
                    Some(client) => client,
                    None => {
                        return Some(Err(ArrowError::IpcError(
                            "chunk reader missing HTTP client".to_string(),
                        )));
                    }
                };
                let chunk_data_result = get_chunk_data_sync(client, &chunk);
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

pub fn get_chunk_data_sync(
    client: &Client,
    chunk: &ChunkDownloadData,
) -> Result<Vec<u8>, ChunkError> {
    // TODO: Find a better way of managing tokio runtimes
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async { get_chunk_data(client, chunk).await })
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
