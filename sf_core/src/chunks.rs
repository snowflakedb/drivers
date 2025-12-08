use std::collections::{HashMap, VecDeque};
use std::io;
use std::str::FromStr;
use std::sync::Arc;

use crate::compression::{CompressionError, decompress_data};
use crate::rest::snowflake::query_response::RowType as RestRowType;
use arrow::array::{
    Array, Decimal128Array, Decimal128Builder, Int64Array, RecordBatch, RecordBatchReader,
};
use arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use arrow::error::ArrowError;
use arrow_ipc::reader::StreamReader;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use snafu::{Location, ResultExt, Snafu};

#[derive(Clone)]
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

/// Converts INT64 fields with scale>0 metadata to Decimal128 type
/// This fixes Snowflake's Arrow IPC which encodes decimals as INT64+metadata
fn convert_int64_to_decimal_schema(schema: &Schema) -> SchemaRef {
    use std::io::Write;
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rust_debug.log")
        .and_then(|mut f| {
            writeln!(
                f,
                "DEBUG convert_int64_to_decimal_schema: processing schema with {} fields",
                schema.fields().len()
            )
        });

    let fields: Vec<Field> = schema
        .fields()
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/rust_debug.log")
                .and_then(|mut f| {
                    writeln!(
                        f,
                        "  Field {}: name={}, type={:?}",
                        i,
                        field.name(),
                        field.data_type()
                    )
                });

            // Check if this is an INT64 field with scale metadata
            if matches!(field.data_type(), DataType::Int64) {
                let metadata = field.metadata();
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/rust_debug.log")
                    .and_then(|mut f| writeln!(f, "    INT64 field, metadata: {:?}", metadata));

                if let Some(logical_type) = metadata.get("logicalType") {
                    let upper = logical_type.to_ascii_uppercase();
                    if matches!(
                        upper.as_str(),
                        "TIME" | "TIMESTAMP_NTZ" | "TIMESTAMP_LTZ" | "TIMESTAMP_TZ"
                    ) {
                        return (**field).clone();
                    }
                }

                if let (Some(scale_str), Some(precision_str)) =
                    (metadata.get("scale"), metadata.get("precision"))
                {
                    if let (Ok(scale), Ok(precision)) =
                        (scale_str.parse::<i8>(), precision_str.parse::<u8>())
                    {
                        let _ = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("/tmp/rust_debug.log")
                            .and_then(|mut f| {
                                writeln!(f, "    scale={}, precision={}", scale, precision)
                            });

                        // Convert to Decimal128 if:
                        // 1. scale > 0 (has fractional part), OR
                        // 2. precision > 18 (too big for int64, even if scale=0)
                        if scale > 0 || precision > 18 {
                            // Ensure precision is at least 1 (Arrow requires precision between 1 and 38)
                            // Also ensure precision >= scale (required for Decimal128)
                            let precision = std::cmp::max(std::cmp::max(1, precision), scale as u8);
                            let _ = std::fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open("/tmp/rust_debug.log")
                                .and_then(|mut f| {
                                    writeln!(
                                        f,
                                        "    CONVERTING to Decimal128({}, {})",
                                        precision, scale
                                    )
                                });

                            return Field::new(
                                field.name(),
                                DataType::Decimal128(precision, scale),
                                field.is_nullable(),
                            )
                            .with_metadata(metadata.clone());
                        }
                    }
                }
            }
            (**field).clone()
        })
        .collect();

    Arc::new(Schema::new_with_metadata(fields, schema.metadata().clone()))
}

fn apply_row_type_metadata(schema: &SchemaRef, row_types: Option<&[RestRowType]>) -> SchemaRef {
    let Some(row_types) = row_types else {
        return schema.clone();
    };

    if row_types.len() != schema.fields().len() {
        tracing::warn!(
            "apply_row_type_metadata: row_types len {} != schema fields len {}",
            row_types.len(),
            schema.fields().len()
        );
        return schema.clone();
    }

    let new_fields: Vec<Field> = schema
        .fields()
        .iter()
        .enumerate()
        .map(|(idx, field)| {
            let mut metadata = field.metadata().clone();
            if let Some(ext_name) = row_types[idx].ext_type_name.as_ref() {
                metadata.insert("extTypeName".to_string(), ext_name.to_ascii_uppercase());
            }
            if let Some(type_name) = row_types[idx].type_.as_ref() {
                metadata.insert("logicalType".to_string(), type_name.to_ascii_uppercase());
            }
            if let Some(scale) = row_types[idx].scale {
                metadata
                    .entry("scale".to_string())
                    .or_insert_with(|| scale.to_string());
            }
            if let Some(precision) = row_types[idx].precision {
                metadata
                    .entry("precision".to_string())
                    .or_insert_with(|| precision.to_string());
            }
            Field::new(field.name(), field.data_type().clone(), field.is_nullable())
                .with_metadata(metadata)
        })
        .collect();

    Arc::new(Schema::new_with_metadata(
        new_fields,
        schema.metadata().clone(),
    ))
}

/// Converts a RecordBatch with INT64 columns to use Decimal128 where needed
fn convert_batch_int64_to_decimal(
    batch: RecordBatch,
    target_schema: &SchemaRef,
) -> Result<RecordBatch, ArrowError> {
    let original_schema = batch.schema();

    // Check if any conversion is needed
    let needs_conversion = original_schema
        .fields()
        .iter()
        .zip(target_schema.fields().iter())
        .any(|(orig, target)| !orig.data_type().equals_datatype(target.data_type()));

    if !needs_conversion {
        return Ok(batch);
    }

    // Convert columns as needed
    let new_columns: Result<Vec<Arc<dyn Array>>, ArrowError> = batch
        .columns()
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let target_type = target_schema.field(i).data_type();

            // If original is Int64 and target is Decimal128, convert
            if matches!(col.data_type(), DataType::Int64)
                && matches!(target_type, DataType::Decimal128(_, _))
            {
                if let DataType::Decimal128(precision, scale) = target_type {
                    let int64_array = col.as_any().downcast_ref::<Int64Array>().unwrap();

                    // Use builder to properly handle nulls
                    let mut builder = Decimal128Builder::with_capacity(int64_array.len())
                        .with_precision_and_scale(*precision, *scale)?;

                    for i in 0..int64_array.len() {
                        if int64_array.is_null(i) {
                            builder.append_null();
                        } else {
                            builder.append_value(int64_array.value(i) as i128);
                        }
                    }

                    Ok(Arc::new(builder.finish()) as Arc<dyn Array>)
                } else {
                    Ok(col.clone())
                }
            } else {
                Ok(col.clone())
            }
        })
        .collect();

    RecordBatch::try_new(target_schema.clone(), new_columns?)
}

pub struct ChunkReader {
    rest: VecDeque<ChunkDownloadData>,
    schema: SchemaRef,
    current_stream: Option<StreamReader<io::Cursor<Vec<u8>>>>,
}

impl ChunkReader {
    pub fn multi_chunk(
        initial: Vec<u8>,
        rest: Vec<ChunkDownloadData>,
        row_types: Option<&[RestRowType]>,
    ) -> Result<Self, ArrowError> {
        let cursor = io::Cursor::new(initial);
        let reader = StreamReader::try_new(cursor, None)?;
        let original_schema = reader.schema().clone();
        let schema = convert_int64_to_decimal_schema(&original_schema);
        let schema = apply_row_type_metadata(&schema, row_types);
        Ok(Self {
            rest: rest.into(),
            schema,
            current_stream: Some(reader),
        })
    }
    pub fn single_chunk(
        initial: Vec<u8>,
        row_types: Option<&[RestRowType]>,
    ) -> Result<Self, ArrowError> {
        tracing::debug!(
            "ChunkReader::single_chunk: initial data size = {} bytes",
            initial.len()
        );
        if initial.len() < 10 {
            tracing::error!(
                "ChunkReader::single_chunk: initial data too small, bytes = {:?}",
                &initial
            );
        } else {
            tracing::debug!(
                "ChunkReader::single_chunk: first 10 bytes = {:?}",
                &initial[..10]
            );
        }
        let cursor = io::Cursor::new(initial);
        let reader = match StreamReader::try_new(cursor, None) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(
                    "ChunkReader::single_chunk: StreamReader::try_new failed: {}",
                    e
                );
                return Err(e);
            }
        };
        let original_schema = reader.schema().clone();
        let schema = convert_int64_to_decimal_schema(&original_schema);
        let schema = apply_row_type_metadata(&schema, row_types);
        Ok(Self {
            rest: VecDeque::new(),
            schema,
            current_stream: Some(reader),
        })
    }
}

impl Iterator for ChunkReader {
    type Item = Result<RecordBatch, ArrowError>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(mut current_stream) = self.current_stream.take() {
            let next_batch = current_stream.next();
            if let Some(batch_result) = next_batch {
                self.current_stream = Some(current_stream);
                // Convert INT64+scale to Decimal128 if needed
                return Some(
                    batch_result
                        .and_then(|batch| convert_batch_int64_to_decimal(batch, &self.schema)),
                );
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
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    for (key, value) in chunk.headers.iter() {
        let header_name = HeaderName::from_str(key).context(HeaderNameSnafu { key })?;
        let header_value = HeaderValue::from_str(value).context(HeaderValueSnafu { key })?;
        headers.insert(header_name, header_value);
    }
    let response = client
        .get(url)
        .headers(headers)
        .send()
        .await
        .context(CommunicationSnafu)?;

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
}
