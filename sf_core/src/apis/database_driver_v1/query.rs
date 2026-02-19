use crate::arrow_utils::ArrowUtilsError;
use crate::arrow_utils::{
    boxed_arrow_reader, convert_string_rowset_to_arrow_reader, create_schema,
};
use crate::chunks::{ChunkError, ChunkReader};
use crate::file_manager;
use crate::file_manager::{DownloadResult, UploadResult, download_files, upload_files};
use crate::query_types::RowType;
use crate::rest;
use arrow::array::{Array, Int64Array, RecordBatchReader, StringArray};
use arrow::error::ArrowError;
use reqwest::Client;
use rest::snowflake::query_response::{self, QueryResponseError};
use snafu::{Location, ResultExt, Snafu};
use std::sync::Arc;

const PUT_GET_ROWSET_TEXT_LENGTH: u64 = 10000;
const PUT_GET_ROWSET_FIXED_LENGTH: u64 = 64;

pub async fn process_query_response(
    data: &query_response::Data,
    http_client: &Client,
) -> Result<Box<dyn RecordBatchReader + Send>, QueryResponseProcessingError> {
    match data.command {
        Some(ref command) => perform_put_get(command.clone(), data).await,
        None => read_batches(data.to_rowset_data(), http_client)
            .await
            .context(BatchReadingSnafu),
    }
}

async fn perform_put_get(
    command: String,
    data: &query_response::Data,
) -> Result<Box<dyn RecordBatchReader + Send>, QueryResponseProcessingError> {
    match command.as_str() {
        "UPLOAD" => {
            let file_upload_data = data
                .to_file_upload_data()
                .context(FileTransferPreparationSnafu)?;
            let upload_results = upload_files(&file_upload_data)
                .await
                .context(FileUploadSnafu)?;
            upload_results_reader(upload_results).context(UploadResultsConversionSnafu)
        }
        "DOWNLOAD" => {
            let file_download_data = data
                .to_file_download_data()
                .context(FileTransferPreparationSnafu)?;
            let download_results = download_files(file_download_data)
                .await
                .context(FileDownloadSnafu)?;
            download_results_reader(download_results).context(DownloadResultsConversionSnafu)
        }
        _ => UnsupportedCommandSnafu {
            command: command.to_string(),
        }
        .fail(),
    }
}

async fn read_batches<'a>(
    data: query_response::RowsetData<'a>,
    http_client: &Client,
) -> Result<Box<dyn RecordBatchReader + Send>, ReadBatchesError> {
    match data {
        query_response::RowsetData::ArrowSingleChunk { chunk_base64 } => {
            let reader_result =
                ChunkReader::single_chunk(chunk_base64).context(ChunkReadingSnafu)?;

            Ok(Box::new(reader_result))
        }
        query_response::RowsetData::ArrowMultiChunk {
            initial_base64_opt,
            chunk_download_data,
        } => {
            // Handle chunk download case without base64 data
            let reader_result = ChunkReader::multi_chunk(
                initial_base64_opt,
                chunk_download_data.into(),
                http_client.clone(),
            )
            .await
            .context(ChunkReadingSnafu)?;

            Ok(Box::new(reader_result))
        }
        query_response::RowsetData::JsonRowset { rowset, rowtype } => {
            let row_types = rowtype
                .iter()
                .map(|rt| rt.try_into())
                .collect::<Result<Vec<_>, _>>()
                .context(RowTypeParsingSnafu)?;

            // Validate column counts before converting
            if !rowset.is_empty() {
                let num_columns_rowset = rowset.first().unwrap().len();
                let num_columns_rowtype = row_types.len();
                if num_columns_rowset != num_columns_rowtype {
                    return ColumnCountMismatchSnafu {
                        rowtype_count: num_columns_rowtype,
                        rowset_count: num_columns_rowset,
                    }
                    .fail();
                }
            }
            convert_string_rowset_to_arrow_reader(rowset, &row_types).context(RowsetConversionSnafu)
        }
        query_response::RowsetData::NoData => {
            // No rowset or rowtype found, return empty reader
            let reader = ChunkReader::empty();
            Ok(Box::new(reader))
        }
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
    let row_types: Vec<RowType> = vec![
        build_generic_text_rowtype("source"),
        build_generic_text_rowtype("target"),
        build_generic_fixed_rowtype("source_size"),
        build_generic_fixed_rowtype("target_size"),
        build_generic_text_rowtype("source_compression"),
        build_generic_text_rowtype("target_compression"),
        build_generic_text_rowtype("status"),
        build_generic_text_rowtype("message"),
    ];
    let schema = create_schema(&row_types).expect("Failed to create schema from RowTypes");

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
    let row_types: Vec<RowType> = vec![
        build_generic_text_rowtype("file"),
        build_generic_fixed_rowtype("size"),
        build_generic_text_rowtype("status"),
        build_generic_text_rowtype("message"),
    ];
    let schema = create_schema(&row_types).expect("Failed to create schema from RowTypes");

    let columns: Vec<Arc<dyn Array>> = vec![
        string_array!(download_results, file),
        int64_array!(download_results, size),
        string_array!(download_results, status),
        string_array!(download_results, message),
    ];

    boxed_arrow_reader(schema, columns)
}

fn build_generic_text_rowtype(name: &str) -> RowType {
    RowType::text(
        name,
        false,
        PUT_GET_ROWSET_TEXT_LENGTH,
        PUT_GET_ROWSET_TEXT_LENGTH,
    )
}

fn build_generic_fixed_rowtype(name: &str) -> RowType {
    RowType::fixed_with_scale_zero(name, false, PUT_GET_ROWSET_FIXED_LENGTH)
}

#[derive(Debug, Snafu)]
pub enum QueryResponseProcessingError {
    #[snafu(display("Failed to convert upload results to Arrow format"))]
    UploadResultsConversion {
        source: ArrowError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to convert download results to Arrow format"))]
    DownloadResultsConversion {
        source: ArrowError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to upload files"))]
    FileUpload {
        source: file_manager::FileManagerError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to download files"))]
    FileDownload {
        source: file_manager::FileManagerError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read batches from query response"))]
    BatchReading {
        source: ReadBatchesError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Unsupported command in query response: {command}"))]
    UnsupportedCommand {
        command: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to prepare file transfer data"))]
    FileTransferPreparation {
        source: QueryResponseError,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Snafu)]
pub enum ReadBatchesError {
    #[snafu(display(
        "Column count mismatch: rowtype has {rowtype_count} columns, but rowset has {rowset_count} columns"
    ))]
    ColumnCountMismatch {
        rowtype_count: usize,
        rowset_count: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Rowset or rowtype not found in the response"))]
    MissingRowsetOrRowtype {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse rowtype"))]
    RowTypeParsing {
        source: QueryResponseError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to decode base64 rowset"))]
    Base64Decoding {
        source: base64::DecodeError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read chunks"))]
    ChunkReading {
        source: ChunkError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to convert rowset to Arrow format"))]
    RowsetConversion {
        source: ArrowUtilsError,
        #[snafu(implicit)]
        location: Location,
    },
}
