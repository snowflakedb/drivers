use std::collections::{HashMap, VecDeque};
use std::io;
use std::str::FromStr;

use arrow::array::{RecordBatch, RecordBatchReader};
use arrow::datatypes::SchemaRef;
use arrow::error::ArrowError;
use arrow_ipc::reader::StreamReader;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::compression::decompress_data;
use crate::rest::RestError;

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
    pub fn multi_chunk(initial: Vec<u8>, rest: Vec<ChunkDownloadData>) -> Result<Self, ArrowError> {
        let cursor = io::Cursor::new(initial);
        let reader = StreamReader::try_new(cursor, None)?;
        let schema = reader.schema().clone();
        Ok(Self {
            rest: rest.into(),
            schema,
            current_stream: Some(reader),
        })
    }
    pub fn single_chunk(initial: Vec<u8>) -> Result<Self, ArrowError> {
        let cursor = io::Cursor::new(initial);
        let reader = StreamReader::try_new(cursor, None)?;
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

// TODO Should we return RestError here?
pub fn get_chunk_data_sync(chunk: &ChunkDownloadData) -> Result<Vec<u8>, RestError> {
    // TODO: Find a better way of managing tokio runtimes
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async { get_chunk_data(chunk).await })
}

pub async fn get_chunk_data(chunk: &ChunkDownloadData) -> Result<Vec<u8>, RestError> {
    let url = chunk.url.clone();
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    for (key, value) in chunk.headers.iter() {
        let header_name = HeaderName::from_str(key)
            .map_err(|e| RestError::Internal(format!("Invalid header name '{key}': {e}")))?;
        let header_value = HeaderValue::from_str(value)
            .map_err(|e| RestError::Internal(format!("Invalid header value for '{key}': {e}")))?;
        headers.insert(header_name, header_value);
    }
    let response = client
        .get(url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| RestError::Internal(format!("Failed to get chunk data: {e}")))?;
    if !response.status().is_success() {
        return Err(RestError::Internal(format!(
            "Failed to get chunk data: {}",
            response.status()
        )));
    }
    tracing::debug!("Chunk response: {:?}", response);
    let body = if response.headers().get("Content-Encoding")
        == Some(&HeaderValue::from_str("gzip").unwrap())
    {
        tracing::debug!("Decompressing chunk data");
        let compressed_body = response
            .bytes()
            .await
            .map_err(|e| RestError::Internal(format!("Failed to get chunk data: {e}")))?;
        decompress_data(compressed_body.to_vec())
            .map_err(|e| RestError::Internal(format!("Failed to decompress chunk data: {e}")))?
    } else {
        response
            .bytes()
            .await
            .map_err(|e| RestError::Internal(format!("Failed to get chunk data: {e}")))?
            .to_vec()
    };

    Ok(body)
}
