use crate::arrow_utils::ArrowUtilsError;
use crate::arrow_utils::{
    boxed_arrow_reader, convert_string_rowset_to_arrow_reader, create_schema,
};
use crate::chunks::ChunkReader;
use crate::file_manager;
use crate::file_manager::{DownloadResult, UploadResult, download_files, upload_files};
use crate::query_types::RowType;
use crate::rest;
use arrow::array::{Array, Int64Array, StringArray};
use arrow::error::ArrowError;
use arrow::record_batch::{RecordBatch, RecordBatchIterator, RecordBatchReader};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rest::snowflake::query_response::{self, QueryResponseError};
use snafu::{Location, OptionExt, ResultExt, Snafu};
use std::sync::Arc;

const PUT_GET_ROWSET_TEXT_LENGTH: u64 = 10000;
const PUT_GET_ROWSET_FIXED_LENGTH: u64 = 64;
const MAX_PARALLEL_CHUNK_DOWNLOADS: usize = 8;

/// Parse a chunk of JSON data into rows
fn parse_json_chunk(
    chunk_data: &[u8],
    idx: usize,
) -> Result<Vec<Vec<Option<String>>>, ReadBatchesError> {
    use std::io::Write;

    // Check if chunk is Arrow IPC or JSON
    let is_arrow = chunk_data.len() >= 6 && &chunk_data[0..6] == b"ARROW1";

    if is_arrow {
        // TODO: Handle Arrow IPC chunks
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/rust_debug.log")
            .and_then(|mut f| {
                writeln!(
                    f,
                    "WARNING: Arrow IPC chunks not yet implemented for JSON rowsets"
                )
            });
        Ok(vec![])
    } else {
        // Chunk format is comma-separated rows without outer array brackets
        // e.g.: ["12288"],\n["12289"],\n...["205531"],
        // We need to wrap it in array brackets and remove trailing comma
        let mut json_to_parse = Vec::with_capacity(chunk_data.len() + 2);
        json_to_parse.push(b'[');

        // Remove trailing comma and newline if present
        let data_to_add = if chunk_data.ends_with(b",\n") {
            &chunk_data[..chunk_data.len() - 2]
        } else if chunk_data.ends_with(b",") {
            &chunk_data[..chunk_data.len() - 1]
        } else {
            chunk_data
        };

        json_to_parse.extend_from_slice(data_to_add);
        json_to_parse.push(b']');

        // Parse as JSON array of rows
        let parsed: Vec<Vec<serde_json::Value>> = serde_json::from_slice(&json_to_parse)
            .map_err(|e| {
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/rust_debug.log")
                    .and_then(|mut f| writeln!(f, "ERROR parsing chunk {}: {}", idx, e));
                ArrowError::JsonError(format!("Failed to parse chunk {} as rowset: {}", idx, e))
            })
            .context(ChunkReadingSnafu)?;

        Ok(json_rowset_to_strings(&parsed))
    }
}

fn convert_row_types(
    row_types: &[query_response::RowType],
) -> Result<Vec<RowType>, ReadBatchesError> {
    row_types
        .iter()
        .map(|rt| rt.try_into())
        .collect::<Result<Vec<_>, _>>()
        .context(RowTypeParsingSnafu)
}

fn json_rowset_to_strings(rowset: &[Vec<serde_json::Value>]) -> Vec<Vec<Option<String>>> {
    rowset
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| match value {
                    serde_json::Value::Null => None,
                    serde_json::Value::String(s) => Some(s.clone()),
                    _ => Some(value.to_string()),
                })
                .collect()
        })
        .collect()
}

pub async fn process_query_response(
    data: &query_response::Data,
) -> Result<Box<dyn RecordBatchReader + Send>, QueryResponseProcessingError> {
    use std::io::Write;
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rust_debug.log")
        .and_then(|mut f| {
            writeln!(
                f,
                "DEBUG process_query_response: command={:?}",
                data.command
            )
        });

    match data.command {
        Some(ref command) => perform_put_get(command.clone(), data).await,
        None => read_batches(data).await.context(BatchReadingSnafu),
    }
}

async fn perform_put_get(
    command: String,
    data: &query_response::Data,
) -> Result<Box<dyn RecordBatchReader + Send>, QueryResponseProcessingError> {
    use std::io::Write;
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rust_debug.log")
        .and_then(|mut f| writeln!(f, "DEBUG perform_put_get: command={}", command));

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
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/rust_debug.log")
                .and_then(|mut f| {
                    writeln!(f, "DEBUG perform_put_get: Starting DOWNLOAD operation")
                });

            let file_download_data = data
                .to_file_download_data()
                .context(FileTransferPreparationSnafu)?;

            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/rust_debug.log")
                .and_then(|mut f| {
                    writeln!(
                        f,
                        "DEBUG perform_put_get: Calling download_files with {} files",
                        file_download_data.src_locations.len()
                    )
                });

            let download_results = download_files(file_download_data)
                .await
                .context(FileDownloadSnafu)?;

            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/rust_debug.log")
                .and_then(|mut f| {
                    writeln!(
                        f,
                        "DEBUG perform_put_get: download_files completed, got {} results",
                        download_results.len()
                    )
                });

            download_results_reader(download_results).context(DownloadResultsConversionSnafu)
        }
        _ => UnsupportedCommandSnafu {
            command: command.to_string(),
        }
        .fail(),
    }
}

async fn read_batches(
    data: &query_response::Data,
) -> Result<Box<dyn RecordBatchReader + Send>, ReadBatchesError> {
    use std::io::Write;
    if let Some(rowset_base64) = data.rowset_base64.as_ref().filter(|s| !s.is_empty()) {
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/rust_debug.log")
            .and_then(|mut f| {
                writeln!(
                    f,
                    "DEBUG read_batches: Using rowset_base64 path (Arrow IPC), base64 len = {}, total={:?}, returned={:?}",
                    rowset_base64.len(),
                    data.total,
                    data.returned
                )
            });
        let rowset_bytes = BASE64.decode(rowset_base64).context(Base64DecodingSnafu)?;
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/rust_debug.log")
            .and_then(|mut f| {
                writeln!(
                    f,
                    "DEBUG read_batches: Decoded {} bytes from base64",
                    rowset_bytes.len()
                )
            });

        let row_types = data.row_type.as_deref();

        if rowset_bytes.is_empty()
            && data
                .rowset
                .as_ref()
                .map(|rows| rows.is_empty())
                .unwrap_or(true)
        {
            let schema = if let Some(row_types) = row_types {
                let converted_row_types = convert_row_types(row_types)?;
                create_schema(&converted_row_types).context(RowsetConversionSnafu)?
            } else {
                Arc::new(arrow::datatypes::Schema::empty())
            };
            let batch = RecordBatch::new_empty(schema.clone());
            let reader = RecordBatchIterator::new(vec![Ok(batch)], schema);
            return Ok(Box::new(reader));
        } else {
            let reader_result = if let Some(chunk_download_data) = data.to_chunk_download_data() {
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/rust_debug.log")
                    .and_then(|mut f| {
                        writeln!(
                            f,
                            "DEBUG read_batches: Found {} additional chunks to download",
                            chunk_download_data.len()
                        )
                    });
                ChunkReader::multi_chunk(rowset_bytes, chunk_download_data, row_types)
            } else {
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/rust_debug.log")
                    .and_then(|mut f| {
                        writeln!(f, "DEBUG read_batches: Single chunk, no additional chunks")
                    });
                ChunkReader::single_chunk(rowset_bytes, row_types)
            }
            .context(ChunkReadingSnafu)?;

            return Ok(Box::new(reader_result));
        }
    }

    if let (Some(rowset), Some(rowtype)) = (&data.rowset, &data.row_type) {
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/rust_debug.log")
            .and_then(|mut f| {
                writeln!(
                    f,
                    "DEBUG read_batches: JSON rowset first row = {:?}",
                    rowset.first()
                )
            });
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/rust_debug.log")
            .and_then(|mut f| {
                for (idx, row) in rowset.iter().enumerate().take(5) {
                    writeln!(f, "DEBUG json rowset row {} = {:?}", idx, row)?;
                }
                Ok(())
            });
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/rust_debug.log")
        {
            for rt in rowtype {
                let _ = writeln!(
                    file,
                    "ROWTYPE raw: name={:?} type={:?} logicalType={:?} precision={:?} scale={:?}",
                    rt.name, rt.type_, rt.logical_type, rt.precision, rt.scale
                );
            }
        }

        let row_types = convert_row_types(rowtype)?;

        // Check if there are additional chunks to download
        let mut base_rows = json_rowset_to_strings(rowset);

        if let Some(chunk_download_data) = data.to_chunk_download_data() {
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/rust_debug.log")
                .and_then(|mut f| writeln!(f, "DEBUG read_batches: JSON rowset with {} initial rows and {} additional chunks", base_rows.len(), chunk_download_data.len()));

            // Combine initial rowset with additional chunks
            // Download and parse chunks in parallel batches
            let chunks_vec = chunk_download_data.clone();
            for batch_start in (0..chunks_vec.len()).step_by(MAX_PARALLEL_CHUNK_DOWNLOADS) {
                let batch_end = (batch_start + MAX_PARALLEL_CHUNK_DOWNLOADS).min(chunks_vec.len());

                // Create tasks for parallel download
                let download_tasks: Vec<_> = (batch_start..batch_end)
                    .map(|global_idx| {
                        let chunk = chunks_vec[global_idx].clone();
                        tokio::spawn(async move {
                            match crate::chunks::get_chunk_data(&chunk).await {
                                Ok(chunk_data) => match parse_json_chunk(&chunk_data, global_idx) {
                                    Ok(chunk_rowset) => Ok((global_idx, chunk_rowset)),
                                    Err(e) => Err(format!(
                                        "Parse error for chunk {}: {:?}",
                                        global_idx, e
                                    )),
                                },
                                Err(e) => {
                                    Err(format!("Download error for chunk {}: {:?}", global_idx, e))
                                }
                            }
                        })
                    })
                    .collect();

                // Wait for all tasks in this batch
                let mut results = Vec::with_capacity(download_tasks.len());
                for task in download_tasks {
                    let task_result = task
                        .await
                        .map_err(|e| ArrowError::IpcError(format!("Task join error: {}", e)))
                        .context(ChunkReadingSnafu)?;
                    let chunk_result = task_result
                        .map_err(|e| ArrowError::IpcError(e))
                        .context(ChunkReadingSnafu)?;
                    results.push(chunk_result);
                }

                // Add results in order
                for (idx, chunk_rowset) in results {
                    let _ = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/rust_debug.log")
                        .and_then(|mut f| {
                            writeln!(
                                f,
                                "DEBUG read_batches: Downloaded chunk {} with {} rows",
                                idx,
                                chunk_rowset.len()
                            )
                        });
                    base_rows.extend(chunk_rowset);
                }
            }

            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/rust_debug.log")
                .and_then(|mut f| {
                    writeln!(
                        f,
                        "DEBUG read_batches: Total rows after combining chunks: {}",
                        base_rows.len()
                    )
                });
        } else {
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/rust_debug.log")
                .and_then(|mut f| writeln!(f, "DEBUG read_batches: Using rowset/rowtype path (JSON), rowset has {} rows, no additional chunks", base_rows.len()));
        }

        let all_rows = base_rows;

        // Validate column counts before converting
        if !all_rows.is_empty() {
            let num_columns_rowset = all_rows.first().unwrap().len();
            let num_columns_rowtype = row_types.len();
            if num_columns_rowset != num_columns_rowtype {
                return ColumnCountMismatchSnafu {
                    rowtype_count: num_columns_rowtype,
                    rowset_count: num_columns_rowset,
                }
                .fail();
            }
        }
        convert_string_rowset_to_arrow_reader(&all_rows, &row_types).context(RowsetConversionSnafu)
    } else if data
        .rowset
        .as_ref()
        .map(|rows| rows.is_empty())
        .unwrap_or(true)
    {
        let schema = if let Some(rowtype) = data.row_type.as_ref() {
            let row_types = convert_row_types(rowtype)?;
            create_schema(&row_types).context(RowsetConversionSnafu)?
        } else {
            Arc::new(arrow::datatypes::Schema::empty())
        };
        let batch = RecordBatch::new_empty(schema.clone());
        let reader = RecordBatchIterator::new(vec![Ok(batch)], schema);
        Ok(Box::new(reader))
    } else {
        MissingRowsetOrRowtypeSnafu.fail()
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
        source: ArrowError,
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
