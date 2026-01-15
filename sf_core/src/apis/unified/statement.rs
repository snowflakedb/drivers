//! Statement execution for the unified driver API.

use super::connection::Connection;
use super::error::*;
use super::global_state::{CONN_HANDLE_MANAGER, STMT_HANDLE_MANAGER};
use crate::arrow_utils::{
    boxed_arrow_reader, convert_arrow_record_batches_to_arrow_reader,
    convert_string_rowset_to_arrow_reader,
};
use crate::config::settings::Setting;
use crate::handle_manager::Handle;
use crate::query_types::RowType as InternalRowType;
#[allow(unused_imports)]
use crate::rest::{BindParameter, QueryExecutionMode, QueryResponse, SnowflakeRestClient};
use crate::runtime::block_on;
use arrow::array::RecordBatch;
#[cfg(feature = "native")]
use arrow::ffi::{FFI_ArrowArray, FFI_ArrowSchema};
#[cfg(feature = "native")]
use arrow::ffi_stream::FFI_ArrowArrayStream;
use arrow::record_batch::RecordBatchReader;
use snafu::ResultExt;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Statement state.
#[derive(Debug, Clone)]
pub enum StatementState {
    Initialized,
    Prepared,
    Executed,
}

/// Statement container.
pub struct Statement {
    pub state: StatementState,
    pub settings: HashMap<String, Setting>,
    pub query: Option<String>,
    /// Parameter bindings for query execution (JSON-based, works for both native and WASM)
    pub bind_parameters: Option<HashMap<String, BindParameter>>,
    pub conn: Arc<Mutex<Connection>>,
}

impl Statement {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Statement {
            settings: HashMap::new(),
            state: StatementState::Initialized,
            query: None,
            bind_parameters: None,
            conn,
        }
    }
}

/// Result of executing a query.
#[cfg(feature = "native")]
pub struct ExecuteResult {
    pub stream: Box<FFI_ArrowArrayStream>,
    pub rows_affected: i64,
}

/// Result of executing a query (WASM version - uses RecordBatchReader for serialization).
#[cfg(all(feature = "wasm", not(feature = "native")))]
pub struct ExecuteResult {
    pub stream: Box<dyn RecordBatchReader + Send>,
    pub rows_affected: i64,
}

/// Create a new statement handle.
pub fn statement_new(conn_handle: Handle) -> Result<Handle, ApiError> {
    match CONN_HANDLE_MANAGER.get_obj(conn_handle) {
        Some(conn_ptr) => {
            let stmt = Mutex::new(Statement::new(conn_ptr));
            let handle = STMT_HANDLE_MANAGER.add_handle(stmt);
            Ok(handle)
        }
        None => InvalidArgumentSnafu {
            argument: "Connection handle not found".to_string(),
        }
        .fail(),
    }
}

/// Release a statement handle.
pub fn statement_release(stmt_handle: Handle) -> Result<(), ApiError> {
    match STMT_HANDLE_MANAGER.delete_handle(stmt_handle) {
        true => Ok(()),
        false => InvalidArgumentSnafu {
            argument: "Failed to release statement handle".to_string(),
        }
        .fail(),
    }
}

/// Set an option on a statement handle.
pub fn statement_set_option(handle: Handle, key: String, value: Setting) -> Result<(), ApiError> {
    match STMT_HANDLE_MANAGER.get_obj(handle) {
        Some(stmt_ptr) => {
            let mut stmt = stmt_ptr
                .lock()
                .map_err(|_| StatementLockingSnafu {}.build())?;
            stmt.settings.insert(key, value);
            Ok(())
        }
        None => InvalidArgumentSnafu {
            argument: "Statement handle not found".to_string(),
        }
        .fail(),
    }
}

/// Set the SQL query on a statement handle.
pub fn statement_set_sql_query(stmt_handle: Handle, query: String) -> Result<(), ApiError> {
    match STMT_HANDLE_MANAGER.get_obj(stmt_handle) {
        Some(stmt_ptr) => {
            let mut stmt = stmt_ptr
                .lock()
                .map_err(|_| StatementLockingSnafu {}.build())?;
            stmt.query = Some(query);
            Ok(())
        }
        None => InvalidArgumentSnafu {
            argument: "Statement handle not found".to_string(),
        }
        .fail(),
    }
}

/// Prepare a statement for execution.
pub fn statement_prepare(_stmt_handle: Handle) -> Result<(), ApiError> {
    // No preparation needed for now
    Ok(())
}

/// Bind parameters to a statement using FFI pointers.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers to FFI_ArrowSchema and FFI_ArrowArray.
#[cfg(feature = "native")]
pub unsafe fn statement_bind_ffi(
    _stmt_handle: Handle,
    _schema: *mut FFI_ArrowSchema,
    _array: *mut FFI_ArrowArray,
) -> Result<(), ApiError> {
    // FFI parameter binding not fully supported yet
    InvalidArgumentSnafu {
        argument: "FFI parameter binding not yet fully implemented".to_string(),
    }
    .fail()
}

/// Bind parameters to a statement using JSON-encoded key-value pairs.
/// Each parameter is a JSON object with "type" and "value" fields.
/// This is the common path for both native and WASM builds when using the unified driver.
pub fn statement_bind_stream(
    stmt_handle: Handle,
    values: &[u8], // JSON-encoded parameter bindings
) -> Result<(), ApiError> {
    let bindings: HashMap<String, BindParameter> = serde_json::from_slice(values).map_err(|e| {
        InvalidArgumentSnafu {
            argument: format!("Failed to parse parameter bindings: {}", e),
        }
        .build()
    })?;

    match STMT_HANDLE_MANAGER.get_obj(stmt_handle) {
        Some(stmt_ptr) => {
            let mut stmt = stmt_ptr
                .lock()
                .map_err(|_| StatementLockingSnafu {}.build())?;
            stmt.bind_parameters = Some(bindings);
            Ok(())
        }
        None => InvalidArgumentSnafu {
            argument: "Statement handle not found".to_string(),
        }
        .fail(),
    }
}

/// Execute a query and return results.
pub fn statement_execute_query(stmt_handle: Handle) -> Result<ExecuteResult, ApiError> {
    let stmt_ptr = STMT_HANDLE_MANAGER.get_obj(stmt_handle).ok_or_else(|| {
        InvalidArgumentSnafu {
            argument: "Statement handle not found".to_string(),
        }
        .build()
    })?;

    let mut stmt = stmt_ptr
        .lock()
        .map_err(|_| StatementLockingSnafu {}.build())?;

    let query_str = stmt.query.take().ok_or_else(|| {
        InvalidArgumentSnafu {
            argument: "Query not set".to_string(),
        }
        .build()
    })?;

    // Get bind parameters
    #[cfg(all(feature = "wasm", not(feature = "native")))]
    let bindings = stmt.bind_parameters.take();
    #[cfg(feature = "native")]
    let bindings: Option<HashMap<String, BindParameter>> = None; // TODO: convert from RecordBatch

    // Check for async execution option
    let async_execution = stmt
        .settings
        .get("adbc.snowflake.query.async_execution")
        .map(|s| matches!(s, Setting::String(v) if v == "true"))
        .unwrap_or(false);

    let execution_mode = if async_execution {
        QueryExecutionMode::Async
    } else {
        QueryExecutionMode::Blocking
    };

    // Execute query and get response with client reference for chunk downloading
    let conn = stmt
        .conn
        .lock()
        .map_err(|_| ConnectionLockingSnafu {}.build())?;

    let client = conn
        .client
        .as_ref()
        .ok_or_else(|| ConnectionNotInitializedSnafu {}.build())?;

    // Execute query using the REST client with bindings and execution mode
    let response = block_on(client.query_with_mode(&query_str, bindings, execution_mode))
        .context(QuerySnafu)?;

    // Check for query failure
    if !response.success {
        return QueryResponseProcessingSnafu {
            message: format!(
                "Query failed: {} (code: {})",
                response.message.as_deref().unwrap_or("Unknown error"),
                response.code.as_deref().unwrap_or("?")
            ),
        }
        .fail();
    }

    // Check for chunked results that need downloading
    let has_chunks = response
        .data
        .as_ref()
        .map(|d| d.chunks.as_ref().map(|c| !c.is_empty()).unwrap_or(false))
        .unwrap_or(false);

    // If we have chunks, download them all
    let all_batches = if has_chunks {
        download_all_chunks(&response, client.as_ref())?
    } else {
        Vec::new()
    };

    drop(conn); // Release the lock before processing

    stmt.state = StatementState::Executed;

    #[cfg(feature = "native")]
    {
        // Native: return FFI stream
        let stream = if has_chunks {
            convert_batches_to_ffi_stream(all_batches)?
        } else {
            convert_query_response_to_ffi_stream(&response)?
        };
        Ok(ExecuteResult {
            stream: Box::new(stream),
            rows_affected: response.data.as_ref().and_then(|d| d.total).unwrap_or(0),
        })
    }

    #[cfg(all(feature = "wasm", not(feature = "native")))]
    {
        // WASM: return reader for IPC serialization
        let stream = if has_chunks {
            convert_batches_to_reader(all_batches)?
        } else {
            convert_query_response_to_reader(&response)?
        };
        Ok(ExecuteResult {
            stream,
            rows_affected: response.data.as_ref().and_then(|d| d.total).unwrap_or(0),
        })
    }
}

/// Download all chunks from a query response.
fn download_all_chunks(
    response: &QueryResponse,
    client: &dyn SnowflakeRestClient,
) -> Result<Vec<RecordBatch>, ApiError> {
    let data = response.data.as_ref().ok_or_else(|| {
        QueryResponseProcessingSnafu {
            message: "No data in response".to_string(),
        }
        .build()
    })?;

    let chunks = data.chunks.as_ref().ok_or_else(|| {
        QueryResponseProcessingSnafu {
            message: "No chunks in response".to_string(),
        }
        .build()
    })?;

    let chunk_headers = data.chunk_headers.clone().unwrap_or_default();
    let mut all_batches = Vec::new();

    // First, add inline data if present (rowset or rowset_base64)
    if let Some(inline_reader) = get_inline_data(response)? {
        let inline_batches: Result<Vec<_>, _> = inline_reader.collect();
        let inline_batches = inline_batches.map_err(|e| {
            QueryResponseProcessingSnafu {
                message: format!("Failed to read inline batches: {}", e),
            }
            .build()
        })?;
        all_batches.extend(inline_batches);
    }

    // Download each chunk
    for (idx, chunk) in chunks.iter().enumerate() {
        let chunk_data =
            block_on(client.download_chunk(&chunk.url, &chunk_headers)).map_err(|e| {
                QueryResponseProcessingSnafu {
                    message: format!("Failed to download chunk {}: {}", idx, e),
                }
                .build()
            })?;

        // Parse Arrow IPC data
        let cursor = std::io::Cursor::new(chunk_data);
        let reader = arrow::ipc::reader::StreamReader::try_new(cursor, None).map_err(|e| {
            QueryResponseProcessingSnafu {
                message: format!("Failed to parse chunk {} Arrow IPC: {}", idx, e),
            }
            .build()
        })?;

        let batches: Result<Vec<RecordBatch>, _> = reader.collect();
        let batches = batches.map_err(|e| {
            QueryResponseProcessingSnafu {
                message: format!("Failed to read chunk {} batches: {}", idx, e),
            }
            .build()
        })?;

        all_batches.extend(batches);
    }

    Ok(all_batches)
}

/// Get inline data from response (if any).
fn get_inline_data(
    response: &QueryResponse,
) -> Result<Option<Box<dyn RecordBatchReader + Send>>, ApiError> {
    let data = match response.data.as_ref() {
        Some(d) => d,
        None => return Ok(None),
    };

    // Check for inline rowset (JSON format)
    if let (Some(rowset), Some(rowtype)) = (&data.rowset, &data.rowtype) {
        if rowset.is_empty() {
            return Ok(None);
        }

        let row_types: Vec<InternalRowType> = rowtype
            .iter()
            .map(|rt| {
                let type_lower = rt.data_type.to_lowercase();
                if type_lower.contains("fixed")
                    || type_lower.contains("number")
                    || type_lower.contains("int")
                    || type_lower.contains("decimal")
                    || type_lower.contains("float")
                    || type_lower.contains("double")
                    || type_lower.contains("real")
                {
                    InternalRowType::Fixed {
                        name: rt.name.clone(),
                        nullable: rt.nullable,
                        precision: rt.precision.unwrap_or(38) as u64,
                        scale: rt.scale.unwrap_or(0) as u64,
                    }
                } else {
                    InternalRowType::Text {
                        name: rt.name.clone(),
                        nullable: rt.nullable,
                        length: rt.length.unwrap_or(16777216) as u64,
                        byte_length: rt.byte_length.unwrap_or(rt.length.unwrap_or(16777216)) as u64,
                    }
                }
            })
            .collect();

        let string_rowset: Vec<Vec<String>> = rowset
            .iter()
            .map(|row| {
                row.iter()
                    .map(|val| match val {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Null => String::new(),
                        other => other.to_string(),
                    })
                    .collect()
            })
            .collect();

        let reader =
            convert_string_rowset_to_arrow_reader(&string_rowset, &row_types).map_err(|e| {
                QueryResponseProcessingSnafu {
                    message: format!("Failed to convert inline rowset: {}", e),
                }
                .build()
            })?;

        return Ok(Some(reader));
    }

    // Check for base64-encoded Arrow data
    if let Some(ref rowset_base64) = data.rowset_base64 {
        use base64::Engine;
        let arrow_bytes = base64::engine::general_purpose::STANDARD
            .decode(rowset_base64)
            .map_err(|e| {
                QueryResponseProcessingSnafu {
                    message: format!("Failed to decode rowset base64: {}", e),
                }
                .build()
            })?;

        if arrow_bytes.is_empty() {
            return Ok(None);
        }

        let cursor = std::io::Cursor::new(arrow_bytes);
        let reader = arrow::ipc::reader::StreamReader::try_new(cursor, None).map_err(|e| {
            QueryResponseProcessingSnafu {
                message: format!("Failed to parse Arrow IPC stream: {}", e),
            }
            .build()
        })?;

        let batches: Result<Vec<RecordBatch>, _> = reader.collect();
        let batches = batches.map_err(|e| {
            QueryResponseProcessingSnafu {
                message: format!("Failed to read Arrow batches: {}", e),
            }
            .build()
        })?;

        if batches.is_empty() {
            return Ok(None);
        }

        let schema = batches[0].schema();
        let reader =
            convert_arrow_record_batches_to_arrow_reader(schema, batches).map_err(|e| {
                QueryResponseProcessingSnafu {
                    message: format!("Failed to create Arrow reader: {}", e),
                }
                .build()
            })?;

        return Ok(Some(reader));
    }

    Ok(None)
}

/// Convert batches to reader (WASM).
fn convert_batches_to_reader(
    batches: Vec<RecordBatch>,
) -> Result<Box<dyn RecordBatchReader + Send>, ApiError> {
    if batches.is_empty() {
        let empty_schema = Arc::new(arrow::datatypes::Schema::empty());
        return boxed_arrow_reader(empty_schema, vec![]).map_err(|e| {
            QueryResponseProcessingSnafu {
                message: format!("Failed to create empty reader: {}", e),
            }
            .build()
        });
    }

    let schema = batches[0].schema();
    convert_arrow_record_batches_to_arrow_reader(schema, batches).map_err(|e| {
        QueryResponseProcessingSnafu {
            message: format!("Failed to create Arrow reader: {}", e),
        }
        .build()
    })
}

/// Convert batches to FFI stream (native only).
#[cfg(feature = "native")]
fn convert_batches_to_ffi_stream(
    batches: Vec<RecordBatch>,
) -> Result<FFI_ArrowArrayStream, ApiError> {
    let reader = convert_batches_to_reader(batches)?;
    Ok(FFI_ArrowArrayStream::new(reader))
}

/// Convert a query response to an Arrow RecordBatchReader.
/// Note: This only handles inline data (non-chunked responses).
/// For chunked responses, use download_all_chunks + convert_batches_to_reader.
fn convert_query_response_to_reader(
    response: &QueryResponse,
) -> Result<Box<dyn RecordBatchReader + Send>, ApiError> {
    let data = response.data.as_ref().ok_or_else(|| {
        QueryResponseProcessingSnafu {
            message: "No data in query response".to_string(),
        }
        .build()
    })?;

    // Check for rowset (inline JSON results)
    if let (Some(rowset), Some(rowtype)) = (&data.rowset, &data.rowtype) {
        // Convert rowtype to our internal RowType enum
        let row_types: Vec<InternalRowType> = rowtype
            .iter()
            .map(|rt| {
                let type_lower = rt.data_type.to_lowercase();
                if type_lower.contains("fixed")
                    || type_lower.contains("number")
                    || type_lower.contains("int")
                    || type_lower.contains("decimal")
                    || type_lower.contains("float")
                    || type_lower.contains("double")
                    || type_lower.contains("real")
                {
                    InternalRowType::Fixed {
                        name: rt.name.clone(),
                        nullable: rt.nullable,
                        precision: rt.precision.unwrap_or(38) as u64,
                        scale: rt.scale.unwrap_or(0) as u64,
                    }
                } else {
                    InternalRowType::Text {
                        name: rt.name.clone(),
                        nullable: rt.nullable,
                        length: rt.length.unwrap_or(16777216) as u64,
                        byte_length: rt.byte_length.unwrap_or(rt.length.unwrap_or(16777216)) as u64,
                    }
                }
            })
            .collect();

        // Convert JSON rowset to string rowset
        let string_rowset: Vec<Vec<String>> = rowset
            .iter()
            .map(|row| {
                row.iter()
                    .map(|val| match val {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Null => String::new(),
                        other => other.to_string(),
                    })
                    .collect()
            })
            .collect();

        // Convert string rowset to Arrow
        let reader =
            convert_string_rowset_to_arrow_reader(&string_rowset, &row_types).map_err(|e| {
                QueryResponseProcessingSnafu {
                    message: format!("Failed to convert rowset: {}", e),
                }
                .build()
            })?;

        Ok(reader)
    } else if let Some(ref rowset_base64) = data.rowset_base64 {
        // Handle base64-encoded Arrow rowset
        use base64::Engine;
        let arrow_bytes = base64::engine::general_purpose::STANDARD
            .decode(rowset_base64)
            .map_err(|e| {
                QueryResponseProcessingSnafu {
                    message: format!("Failed to decode rowset base64: {}", e),
                }
                .build()
            })?;

        // Parse as Arrow IPC stream
        let cursor = std::io::Cursor::new(arrow_bytes);
        let reader = arrow::ipc::reader::StreamReader::try_new(cursor, None).map_err(|e| {
            QueryResponseProcessingSnafu {
                message: format!("Failed to parse Arrow IPC stream: {}", e),
            }
            .build()
        })?;

        // Convert to record batches
        let batches: Result<Vec<RecordBatch>, _> = reader.collect();
        let batches = batches.map_err(|e| {
            QueryResponseProcessingSnafu {
                message: format!("Failed to read Arrow batches: {}", e),
            }
            .build()
        })?;

        if batches.is_empty() {
            // Return empty stream
            let empty_schema = Arc::new(arrow::datatypes::Schema::empty());
            let reader = boxed_arrow_reader(empty_schema, vec![]).map_err(|e| {
                QueryResponseProcessingSnafu {
                    message: format!("Failed to create empty reader: {}", e),
                }
                .build()
            })?;
            Ok(reader)
        } else {
            let schema = batches[0].schema();
            let reader =
                convert_arrow_record_batches_to_arrow_reader(schema, batches).map_err(|e| {
                    QueryResponseProcessingSnafu {
                        message: format!("Failed to create Arrow reader: {}", e),
                    }
                    .build()
                })?;
            Ok(reader)
        }
    } else {
        // No inline rowset - return empty reader (chunked data is handled separately)
        let empty_schema = Arc::new(arrow::datatypes::Schema::empty());
        boxed_arrow_reader(empty_schema, vec![]).map_err(|e| {
            QueryResponseProcessingSnafu {
                message: format!("Failed to create empty reader: {}", e),
            }
            .build()
        })
    }
}

/// Convert a query response to an Arrow FFI stream (native only).
#[cfg(feature = "native")]
fn convert_query_response_to_ffi_stream(
    response: &QueryResponse,
) -> Result<FFI_ArrowArrayStream, ApiError> {
    let reader = convert_query_response_to_reader(response)?;
    Ok(FFI_ArrowArrayStream::new(reader))
}
