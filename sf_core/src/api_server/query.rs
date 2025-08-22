use crate::arrow_utils::{boxed_arrow_reader, convert_string_rowset_to_arrow_reader};
use crate::chunks::ChunkReader;
use crate::file_manager;
use crate::file_manager::{DownloadResult, UploadResult, download_files, upload_files};
use crate::rest;
use crate::rest::RestError;
use arrow::array::{Array, Int64Array, RecordBatchReader, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::error::ArrowError;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rest::snowflake::query_response;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum QueryResponseProcessingError {
    #[error("Arrow processing error: {0}")]
    Arrow(#[from] ArrowError),
    #[error("File manager error: {0}")]
    FileManager(#[from] file_manager::FileManagerError),
    #[error("Rest error: {0}")]
    Rest(#[from] RestError),
}

pub async fn process_query_response(
    data: &query_response::Data,
) -> Result<Box<dyn RecordBatchReader + Send>, QueryResponseProcessingError> {
    match data.command {
        Some(ref command) => perform_put_get(command.clone(), data).await,
        None => read_batches(data),
    }
}

async fn perform_put_get(
    command: String,
    data: &query_response::Data,
) -> Result<Box<dyn RecordBatchReader + Send>, QueryResponseProcessingError> {
    match command.as_str() {
        "UPLOAD" => {
            let file_upload_data = data.to_file_upload_data()?;
            let upload_results = upload_files(&file_upload_data).await?;
            upload_results_reader(upload_results).map_err(|e| e.into())
        }
        "DOWNLOAD" => {
            let file_download_data = data.to_file_download_data()?;
            let download_results = download_files(file_download_data).await?;
            download_results_reader(download_results).map_err(|e| e.into())
        }
        _ => Err(RestError::InvalidSnowflakeResponse(
            "Unsupported command in query response".to_string(),
        )
        .into()),
    }
}

fn read_batches(
    data: &query_response::Data,
) -> Result<Box<dyn RecordBatchReader + Send>, QueryResponseProcessingError> {
    if let Some(rowset_base64) = &data.rowset_base64 {
        let rowset_bytes = BASE64.decode(rowset_base64).map_err(|e| {
            RestError::InvalidSnowflakeResponse(format!("Failed to decode base64 rowset: {e}"))
        })?;

        let reader_result = if let Some(chunk_download_data) = data.to_chunk_download_data() {
            ChunkReader::multi_chunk(rowset_bytes, chunk_download_data)
        } else {
            ChunkReader::single_chunk(rowset_bytes)
        }?;

        Ok(Box::new(reader_result))
    } else if let (Some(rowset), Some(rowtype)) = (&data.rowset, &data.row_type) {
        // Validate column counts before converting
        if !rowset.is_empty() {
            let num_columns_rowset = rowset.first().unwrap().len();
            let num_columns_rowtype = rowtype.len();
            if num_columns_rowset != num_columns_rowtype {
                return Err(RestError::InvalidSnowflakeResponse(format!(
                    "RowType count ({num_columns_rowtype}) doesn't match column count ({num_columns_rowset})"
                )).into());
            }
        }
        convert_string_rowset_to_arrow_reader(rowset, rowtype).map_err(|e| {
            RestError::InvalidSnowflakeResponse(format!("Failed to convert rowset: {e}")).into()
        })
    } else {
        Err(RestError::InvalidSnowflakeResponse(
            "Rowset or rowtype not found in the response".to_string(),
        )
        .into())
    }
}

/// Helper macro to create string arrays from field accessors
macro_rules! string_array {
    ($data:expr, $field:ident) => {
        Arc::new(StringArray::from(
            $data.iter().map(|r| r.$field.as_str()).collect::<Vec<_>>(),
        ))
    };
}

/// Helper macro to create int64 arrays from field accessors
macro_rules! int64_array {
    ($data:expr, $field:ident) => {
        Arc::new(Int64Array::from(
            $data.iter().map(|r| r.$field).collect::<Vec<_>>(),
        ))
    };
}

/// Converts upload results to Arrow format
pub fn upload_results_reader(
    upload_results: Vec<UploadResult>,
) -> Result<Box<dyn RecordBatchReader + Send>, ArrowError> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("source", DataType::Utf8, false),
        Field::new("target", DataType::Utf8, false),
        Field::new("source_size", DataType::Int64, false),
        Field::new("target_size", DataType::Int64, false),
        Field::new("source_compression", DataType::Utf8, false),
        Field::new("target_compression", DataType::Utf8, false),
        Field::new("status", DataType::Utf8, false),
        Field::new("message", DataType::Utf8, false),
    ]));

    let columns: Vec<Arc<dyn Array>> = vec![
        string_array!(upload_results, source),
        string_array!(upload_results, target),
        int64_array!(upload_results, source_size),
        int64_array!(upload_results, target_size),
        string_array!(upload_results, source_compression),
        string_array!(upload_results, target_compression),
        string_array!(upload_results, status),
        string_array!(upload_results, message),
    ];

    boxed_arrow_reader(schema, columns)
}

/// Converts download results to Arrow format
pub fn download_results_reader(
    download_results: Vec<DownloadResult>,
) -> Result<Box<dyn RecordBatchReader + Send>, ArrowError> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("file", DataType::Utf8, false),
        Field::new("size", DataType::Int64, false),
        Field::new("status", DataType::Utf8, false),
        Field::new("message", DataType::Utf8, false),
    ]));

    let columns: Vec<Arc<dyn Array>> = vec![
        string_array!(download_results, file),
        int64_array!(download_results, size),
        string_array!(download_results, status),
        string_array!(download_results, message),
    ];

    boxed_arrow_reader(schema, columns)
}
