use snafu::{OptionExt, ResultExt};
use std::sync::{Mutex, MutexGuard};

use super::Handle;
use super::error::*;
use super::global_state::{CONN_HANDLE_MANAGER, STMT_HANDLE_MANAGER};
use crate::apis::database_driver_v1::query::process_query_response;
use crate::{
    arrow_utils,
    config::{rest_parameters::QueryParameters, settings::Setting},
    rest::snowflake::{cancel_query_with_client, snowflake_query_with_client},
};

use arrow::{
    array::{Array, Int32Array, Int64Array, StringArray, StructArray},
    datatypes::{DataType, Schema},
    error::ArrowError,
    ffi::{FFI_ArrowArray, FFI_ArrowSchema},
    ffi_stream::FFI_ArrowArrayStream,
    record_batch::{RecordBatch, RecordBatchIterator, RecordBatchReader},
};
use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};
use snafu::Snafu;
use std::{collections::HashMap, sync::Arc};

use super::connection::Connection;
use crate::rest::snowflake::query_request;

pub fn statement_new(conn_handle: Handle) -> Result<Handle, ApiError> {
    let handle = conn_handle;
    match CONN_HANDLE_MANAGER.get_obj(handle) {
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

pub fn statement_release(stmt_handle: Handle) -> Result<(), ApiError> {
    match STMT_HANDLE_MANAGER.delete_handle(stmt_handle) {
        true => Ok(()),
        false => InvalidArgumentSnafu {
            argument: "Failed to release statement handle".to_string(),
        }
        .fail(),
    }
}

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

pub fn statement_set_multi_statement_count(handle: Handle, count: usize) -> Result<(), ApiError> {
    tracing::info!(
        "statement_set_multi_statement_count: handle={:?}, count={}",
        handle,
        count
    );
    match STMT_HANDLE_MANAGER.get_obj(handle) {
        Some(stmt_ptr) => {
            let mut stmt = stmt_ptr
                .lock()
                .map_err(|_| StatementLockingSnafu {}.build())?;
            stmt.multi_statement_count = count;
            tracing::info!(
                "statement_set_multi_statement_count: successfully set count to {}",
                count
            );
            Ok(())
        }
        None => {
            tracing::error!("statement_set_multi_statement_count: Statement handle not found");
            InvalidArgumentSnafu {
                argument: "Statement handle not found".to_string(),
            }
            .fail()
        }
    }
}

pub fn statement_set_sql_query(stmt_handle: Handle, query: String) -> Result<(), ApiError> {
    let handle = stmt_handle;
    match STMT_HANDLE_MANAGER.get_obj(handle) {
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

pub fn statement_prepare(stmt_handle: Handle) -> Result<(), ApiError> {
    // Reset statement state to Initialized to allow parameter binding
    with_statement(stmt_handle, |mut stmt| {
        stmt.state = StatementState::Initialized;
        stmt.parameter_bindings = None;
        Ok(())
    })
}

fn with_statement<T>(
    handle: Handle,
    f: impl FnOnce(MutexGuard<Statement>) -> Result<T, ApiError>,
) -> Result<T, ApiError> {
    let stmt = STMT_HANDLE_MANAGER.get_obj(handle).ok_or_else(|| {
        InvalidArgumentSnafu {
            argument: "Statement handle not found".to_string(),
        }
        .build()
    })?;
    let guard = stmt.lock().map_err(|_| {
        InvalidArgumentSnafu {
            argument: "Statement cannot be locked".to_string(),
        }
        .build()
    })?;
    f(guard)
}

/// # Safety
///
/// This function is unsafe because it dereferences raw pointers to FFI_ArrowSchema and FFI_ArrowArray.
/// The caller must ensure that:
/// - The pointers are valid and properly aligned
/// - The pointers point to valid FFI_ArrowSchema and FFI_ArrowArray structs
/// - The structs referenced by the pointers will not be freed by the caller
/// - No other code is concurrently modifying the memory referenced by these pointers
pub unsafe fn statement_bind(
    stmt_handle: Handle,
    schema: *mut FFI_ArrowSchema,
    array: *mut FFI_ArrowArray,
) -> Result<(), ApiError> {
    let schema = unsafe { FFI_ArrowSchema::from_raw(schema) };
    let array = unsafe { FFI_ArrowArray::from_raw(array) };
    let array = unsafe { arrow::ffi::from_ffi(array, &schema) }.map_err(|_| {
        InvalidArgumentSnafu {
            argument: "Failed to convert ArrowArray".to_string(),
        }
        .build()
    })?;
    let record_batch = RecordBatch::from(StructArray::from(array));
    with_statement(stmt_handle, |mut stmt| {
        stmt.bind_parameters(record_batch).map_err(|e| {
            tracing::error!("Failed to bind parameters: {:?}", e);
            InvalidArgumentSnafu {
                argument: format!("Failed to bind parameters: {}", e),
            }
            .build()
        })
    })
}

pub struct ExecuteResult {
    pub stream: Box<FFI_ArrowArrayStream>,
    pub rows_affected: i64,
    pub query_id: Option<String>,
    pub child_result_ids: Vec<String>,
}

fn empty_arrow_stream(schema: Arc<Schema>) -> Box<FFI_ArrowArrayStream> {
    let batch = RecordBatch::new_empty(schema.clone());
    let iter = vec![Ok(batch)].into_iter();
    let reader =
        Box::new(RecordBatchIterator::new(iter, schema)) as Box<dyn RecordBatchReader + Send>;
    Box::new(FFI_ArrowArrayStream::new(reader))
}

pub fn statement_execute_query(
    stmt_handle: Handle,
    describe_only: bool,
) -> Result<ExecuteResult, ApiError> {
    let handle = stmt_handle;
    let stmt_ptr = STMT_HANDLE_MANAGER.get_obj(handle).ok_or_else(|| {
        InvalidArgumentSnafu {
            argument: "Statement handle not found".to_string(),
        }
        .build()
    })?;

    let mut stmt = stmt_ptr
        .lock()
        .map_err(|_| StatementLockingSnafu {}.build())?;
    let query = stmt.query.clone().ok_or_else(|| {
        InvalidArgumentSnafu {
            argument: "Query not found".to_string(),
        }
        .build()
    })?;

    // Create a blocking runtime for the async operations
    let rt = tokio::runtime::Runtime::new().context(RuntimeCreationSnafu)?;

    let (query_parameters, session_token, http_client, session_timezone, force_json_rowset) = {
        let conn = stmt
            .conn
            .lock()
            .map_err(|_| ConnectionLockingSnafu {}.build())?;
        (
            QueryParameters::from_settings(&conn.settings).context(ConfigurationSnafu)?,
            conn.session_token.clone().ok_or_else(|| {
                InvalidArgumentSnafu {
                    argument: "Session token not found".to_string(),
                }
                .build()
            })?,
            conn.http_client.clone(),
            conn.session_timezone.clone(),
            conn.force_json_rowset,
        )
    };

    let bindings = stmt.get_query_parameter_bindings().map_err(|e| {
        tracing::error!("Failed to get query parameter bindings: {:?}", e);
        InvalidArgumentSnafu {
            argument: format!("Failed to get query parameter bindings: {}", e),
        }
        .build()
    })?;

    // Get multi-statement count if set
    // Note: multi_statement_count is initialized to 0, which means "not set"
    // When explicitly set via SQLSetStmtAttr, it will be > 0
    // We always pass it to force multi-statement support
    let multi_statement_count = if stmt.multi_statement_count > 0 {
        tracing::info!(
            "statement_execute_query: using explicit multi_statement_count={}",
            stmt.multi_statement_count
        );
        Some(stmt.multi_statement_count)
    } else {
        // Force multi-statement support by passing 0
        tracing::info!(
            "statement_execute_query: forcing multi_statement_count=0 to enable multi-statement support"
        );
        Some(0)
    };

    if force_json_rowset
        && !query
            .to_ascii_uppercase()
            .contains("ODBC_QUERY_RESULT_FORMAT")
    {
        apply_result_format_override(
            &rt,
            &http_client,
            query_parameters.clone(),
            session_token.clone(),
            session_timezone.clone(),
            true,
        );
    }

    let response = rt
        .block_on(snowflake_query_with_client(
            &http_client,
            query_parameters.clone(),
            session_token.clone(),
            query.clone(),
            bindings,
            multi_statement_count,
            describe_only,
            session_timezone.clone(),
            force_json_rowset,
        ))
        .context(QueryExecutionSnafu)?;
    let new_timezone = parse_session_timezone(&query);
    let new_timestamp_mapping = parse_client_timestamp_mapping(&query);
    let mut format_override_action: Option<bool> = None;
    if new_timezone.is_some() || new_timestamp_mapping.is_some() {
        if let Ok(mut conn_guard) = stmt.conn.lock() {
            if let Some(new_timezone) = new_timezone.clone() {
                tracing::info!(
                    "statement_execute_query: updating session timezone to '{}'",
                    new_timezone
                );
                conn_guard.session_timezone = Some(new_timezone);
            }
            if let Some(mapping) = new_timestamp_mapping.clone() {
                let enable_json = mapping.eq_ignore_ascii_case("TIMESTAMP_NTZ");
                tracing::info!(
                    "statement_execute_query: CLIENT_TIMESTAMP_TYPE_MAPPING set to '{}'",
                    mapping
                );
                format_override_action = Some(enable_json);
                conn_guard.force_json_rowset = enable_json;
                conn_guard.json_rowset_override_applied = enable_json;
            }
        } else {
            tracing::warn!(
                "statement_execute_query: failed to lock connection for session setting updates"
            );
        }
    }
    if let Some(enable_json) = format_override_action {
        apply_result_format_override(
            &rt,
            &http_client,
            query_parameters.clone(),
            session_token.clone(),
            session_timezone.clone(),
            enable_json,
        );
    }

    if describe_only {
        let schema = response
            .data
            .row_type
            .as_ref()
            .and_then(|row_types| {
                let converted = row_types
                    .iter()
                    .map(|rt| rt.try_into())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|err| {
                        tracing::warn!(
                            "statement_execute_query: failed to convert row type for describe-only response: {err:?}"
                        );
                        err
                    })
                    .ok()?;
                arrow_utils::create_schema(&converted)
                    .map_err(|err| {
                        tracing::warn!(
                            "statement_execute_query: failed to create Arrow schema for describe-only response: {err:?}"
                        );
                        err
                    })
                    .ok()
            })
            .unwrap_or_else(|| Arc::new(Schema::empty()));

        return Ok(ExecuteResult {
            stream: empty_arrow_stream(schema.clone()),
            rows_affected: -1, // No rows affected for describe-only
            query_id: response.data.query_id.clone(),
            child_result_ids: Vec::new(),
        });
    }
    let query_id = response.data.query_id.clone();

    // Parse child result IDs for multi-statement queries
    let child_result_ids: Vec<String> = response
        .data
        .result_ids
        .as_ref()
        .map(|ids| {
            ids.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    tracing::debug!("Multi-statement child result IDs: {:?}", child_result_ids);

    let response_reader = rt
        .block_on(process_query_response(&response.data))
        .context(QueryResponseProcessingSnafu)?;

    let rowset_stream = Box::new(FFI_ArrowArrayStream::new(response_reader));

    // Serialize pointer into integer
    stmt.state = StatementState::Executed;
    stmt.current_query_id = query_id.clone();

    // Use 'total' or 'returned' field for row count when available
    // For DDL statements (BEGIN, COMMIT, CREATE, etc.), return -1
    let command = response.data.command.as_deref().unwrap_or("");
    let is_ddl = matches!(
        command.to_uppercase().as_str(),
        "" | "BEGIN"
            | "COMMIT"
            | "ROLLBACK"
            | "CREATE"
            | "DROP"
            | "ALTER"
            | "TRUNCATE"
            | "USE"
            | "SET"
    );

    let rows_affected = if is_ddl {
        -1
    } else {
        response.data.total.or(response.data.returned).unwrap_or(-1)
    };

    tracing::debug!(
        "statement_execute_query: command='{}', is_ddl={}, total={:?}, returned={:?}, rows_affected={}",
        command,
        is_ddl,
        response.data.total,
        response.data.returned,
        rows_affected
    );

    let execute_result = ExecuteResult {
        stream: rowset_stream,
        rows_affected,
        query_id,
        child_result_ids,
    };
    stmt.state = StatementState::Initialized;
    stmt.parameter_bindings = None;
    Ok(execute_result)
}

/// Fetch a child result by query ID (for multi-statement queries)
pub fn fetch_child_result(
    stmt_handle: Handle,
    child_query_id: &str,
) -> Result<ExecuteResult, ApiError> {
    let handle = stmt_handle;
    let stmt_ptr = STMT_HANDLE_MANAGER.get_obj(handle).ok_or_else(|| {
        InvalidArgumentSnafu {
            argument: "Statement handle not found".to_string(),
        }
        .build()
    })?;

    let stmt = stmt_ptr
        .lock()
        .map_err(|_| StatementLockingSnafu {}.build())?;

    let (query_parameters, session_token, http_client) = {
        let conn = stmt
            .conn
            .lock()
            .map_err(|_| ConnectionLockingSnafu {}.build())?;
        (
            QueryParameters::from_settings(&conn.settings).context(ConfigurationSnafu)?,
            conn.session_token
                .clone()
                .context(MissingSessionTokenSnafu)?,
            conn.http_client.clone(),
        )
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context(TokioRuntimeSnafu)?;

    // Fetch the child result using the result path
    let result_path = format!("/queries/{child_query_id}/result");
    let response = rt
        .block_on(crate::rest::snowflake::fetch_child_query_result(
            &http_client,
            &query_parameters.server_url,
            &session_token,
            &result_path,
        ))
        .context(QueryExecutionSnafu)?;

    let response_reader = rt
        .block_on(process_query_response(&response.data))
        .context(QueryResponseProcessingSnafu)?;

    let rowset_stream = Box::new(FFI_ArrowArrayStream::new(response_reader));

    // Use 'total' or 'returned' field for row count when available
    // Statement type IDs from Snowflake (observed values):
    // - 21504 (0x5400): BEGIN
    // - 13056 (0x3300): DELETE
    // - 4352 (0x1100): INSERT
    // DML types return actual row count, others return -1
    let statement_type_id = response.data.statement_type_id.unwrap_or(0);

    // DML statement types that should return actual row count
    // Based on observed values from Snowflake:
    // - 12544 (0x3100): INSERT
    // - 13056 (0x3300): DELETE
    // - 12800 (0x3200): UPDATE (assumed)
    // - 13312 (0x3400): MERGE (assumed)
    let is_dml = matches!(
        statement_type_id,
        4352 | 4608 | 4864 | 5120 | 5376 |  // Standard DML IDs
        12544 | 12800 | 13056 | 13312 | 13568 | 13824 // Observed DML IDs (0x3100-0x3600)
    );

    // Check rowset for DML - it may contain the actual affected count
    let rowset_count = response
        .data
        .rowset
        .as_ref()
        .and_then(|rs| rs.first())
        .and_then(|row| row.first())
        .and_then(|val| match val {
            serde_json::Value::String(s) => s.parse::<i64>().ok(),
            serde_json::Value::Number(num) => num.as_i64(),
            _ => None,
        });

    let rows_affected = if is_dml {
        // For DML, the rowset often contains the actual count as the first value
        rowset_count
            .or(response.data.total)
            .or(response.data.returned)
            .unwrap_or(-1)
    } else {
        -1 // DDL, SELECT, or other statements
    };

    tracing::debug!(
        "fetch_child_result: statement_type_id={} (0x{:X}), is_dml={}, total={:?}, returned={:?}, rowset_count={:?}, rows_affected={}",
        statement_type_id,
        statement_type_id,
        is_dml,
        response.data.total,
        response.data.returned,
        rowset_count,
        rows_affected
    );

    Ok(ExecuteResult {
        stream: rowset_stream,
        rows_affected,
        query_id: response.data.query_id.clone(),
        child_result_ids: Vec::new(),
    })
}

pub fn statement_cancel(stmt_handle: Handle) -> Result<(), ApiError> {
    with_statement(stmt_handle, |mut stmt| {
        let Some(query_id) = stmt.current_query_id.clone() else {
            tracing::info!("statement_cancel: no active query to cancel");
            return Ok(());
        };

        let (query_parameters, session_token, http_client) = {
            let conn = stmt
                .conn
                .lock()
                .map_err(|_| ConnectionLockingSnafu {}.build())?;
            (
                QueryParameters::from_settings(&conn.settings).context(ConfigurationSnafu)?,
                conn.session_token.clone().ok_or_else(|| {
                    InvalidArgumentSnafu {
                        argument: "Session token not found".to_string(),
                    }
                    .build()
                })?,
                conn.http_client.clone(),
            )
        };

        let rt = tokio::runtime::Runtime::new().context(RuntimeCreationSnafu)?;
        rt.block_on(cancel_query_with_client(
            &http_client,
            query_parameters,
            session_token,
            &query_id,
        ))
        .context(LoginSnafu)?;

        stmt.current_query_id = None;
        stmt.state = StatementState::Initialized;
        Ok(())
    })
}

fn parameters_from_record_batch(
    record_batch: &RecordBatch,
) -> Result<HashMap<String, query_request::BindParameter>, StatementError> {
    let mut parameters = HashMap::new();
    let num_rows = record_batch.num_rows();

    for i in 0..record_batch.num_columns() {
        let column = record_batch.column(i);
        let field = record_batch.schema().field(i).clone();
        let logical_type = field
            .metadata()
            .get("logicalType")
            .cloned()
            .unwrap_or_else(|| "TEXT".to_string());
        match column.data_type() {
            DataType::Int32 => {
                let arr = column.as_any().downcast_ref::<Int32Array>().unwrap();
                let json_value = if num_rows == 1 {
                    serde_json::Value::String(arr.value(0).to_string())
                } else {
                    serde_json::Value::Array(
                        (0..num_rows)
                            .map(|row| {
                                if arr.is_null(row) {
                                    serde_json::Value::Null
                                } else {
                                    serde_json::Value::String(arr.value(row).to_string())
                                }
                            })
                            .collect(),
                    )
                };
                parameters.insert(
                    format!("{}", i + 1),
                    query_request::BindParameter {
                        type_: "FIXED".to_string(),
                        value: json_value,
                        format: None,
                        schema: None,
                    },
                );
            }
            DataType::Utf8 => {
                let arr = column.as_any().downcast_ref::<StringArray>().unwrap();
                let json_value = if num_rows == 1 {
                    serde_json::Value::String(arr.value(0).to_string())
                } else {
                    serde_json::Value::Array(
                        (0..num_rows)
                            .map(|row| {
                                if arr.is_null(row) {
                                    serde_json::Value::Null
                                } else {
                                    serde_json::Value::String(arr.value(row).to_string())
                                }
                            })
                            .collect(),
                    )
                };
                parameters.insert(
                    format!("{}", i + 1),
                    query_request::BindParameter {
                        type_: "TEXT".to_string(),
                        value: json_value,
                        format: None,
                        schema: None,
                    },
                );
            }
            DataType::Float64 => {
                let arr = column
                    .as_any()
                    .downcast_ref::<arrow::array::Float64Array>()
                    .unwrap();
                let json_value = if num_rows == 1 {
                    serde_json::Value::String(arr.value(0).to_string())
                } else {
                    serde_json::Value::Array(
                        (0..num_rows)
                            .map(|row| {
                                if arr.is_null(row) {
                                    serde_json::Value::Null
                                } else {
                                    serde_json::Value::String(arr.value(row).to_string())
                                }
                            })
                            .collect(),
                    )
                };
                parameters.insert(
                    format!("{}", i + 1),
                    query_request::BindParameter {
                        type_: "REAL".to_string(),
                        value: json_value,
                        format: None,
                        schema: None,
                    },
                );
            }
            DataType::Timestamp(_, _) => {
                let arr = column
                    .as_any()
                    .downcast_ref::<arrow::array::TimestampNanosecondArray>()
                    .unwrap();
                let type_name = match logical_type.as_str() {
                    "TIMESTAMP_NTZ" => "TIMESTAMP_NTZ",
                    "TIMESTAMP_TZ" => "TIMESTAMP_TZ",
                    _ => "TIMESTAMP_LTZ",
                };
                let format_ts = |value: i64| -> String {
                    let secs = value.div_euclid(1_000_000_000);
                    let nanos = value.rem_euclid(1_000_000_000) as u32;
                    format!("{secs}.{nanos:09}")
                };
                let json_value = if num_rows == 1 {
                    let value = arr.value(0);
                    serde_json::Value::String(format_ts(value))
                } else {
                    serde_json::Value::Array(
                        (0..num_rows)
                            .map(|row| {
                                if arr.is_null(row) {
                                    serde_json::Value::Null
                                } else {
                                    let value = arr.value(row);
                                    serde_json::Value::String(format_ts(value))
                                }
                            })
                            .collect(),
                    )
                };
                if let serde_json::Value::String(val) = &json_value {
                    tracing::debug!("bind param {} TIMESTAMP value {}", i + 1, val);
                } else if let serde_json::Value::Array(arr) = &json_value {
                    if let Some(serde_json::Value::String(val)) = arr.first() {
                        tracing::debug!("bind param {} TIMESTAMP first array value {}", i + 1, val);
                    }
                }
                parameters.insert(
                    format!("{}", i + 1),
                    query_request::BindParameter {
                        type_: type_name.to_string(),
                        value: json_value,
                        format: None,
                        schema: None,
                    },
                );
            }
            DataType::Date32 => {
                let arr = column
                    .as_any()
                    .downcast_ref::<arrow::array::Date32Array>()
                    .unwrap();
                let json_value = if num_rows == 1 {
                    if arr.is_null(0) {
                        serde_json::Value::Null
                    } else {
                        let days = arr.value(0);
                        serde_json::Value::String(days.to_string())
                    }
                } else {
                    serde_json::Value::Array(
                        (0..num_rows)
                            .map(|row| {
                                if arr.is_null(row) {
                                    serde_json::Value::Null
                                } else {
                                    let days = arr.value(row);
                                    serde_json::Value::String(days.to_string())
                                }
                            })
                            .collect(),
                    )
                };
                parameters.insert(
                    format!("{}", i + 1),
                    query_request::BindParameter {
                        type_: "DATE".to_string(),
                        value: json_value,
                        format: None,
                        schema: None,
                    },
                );
            }
            DataType::Time64(arrow::datatypes::TimeUnit::Nanosecond) => {
                let arr = column
                    .as_any()
                    .downcast_ref::<arrow::array::Time64NanosecondArray>()
                    .unwrap();
                let json_value = if num_rows == 1 {
                    // Time64 is nanoseconds since midnight
                    serde_json::Value::String(arr.value(0).to_string())
                } else {
                    serde_json::Value::Array(
                        (0..num_rows)
                            .map(|row| {
                                if arr.is_null(row) {
                                    serde_json::Value::Null
                                } else {
                                    serde_json::Value::String(arr.value(row).to_string())
                                }
                            })
                            .collect(),
                    )
                };
                parameters.insert(
                    format!("{}", i + 1),
                    query_request::BindParameter {
                        type_: "TIME".to_string(),
                        value: json_value,
                        format: None,
                        schema: None,
                    },
                );
            }
            DataType::Struct(_) => {
                let struct_array = column.as_any().downcast_ref::<StructArray>().unwrap();
                let type_name = match logical_type.as_str() {
                    "TIMESTAMP_NTZ" => "TIMESTAMP_NTZ",
                    "TIMESTAMP_TZ" => "TIMESTAMP_TZ",
                    _ => "TIMESTAMP_LTZ",
                };

                if let (Some(epoch_col), Some(fraction_col)) = (
                    struct_array.column_by_name("epoch"),
                    struct_array.column_by_name("fraction"),
                ) {
                    let epoch_arr = epoch_col.as_any().downcast_ref::<Int64Array>().unwrap();
                    let fraction_arr = fraction_col.as_any().downcast_ref::<Int32Array>().unwrap();

                    let to_json_value = |row: usize| {
                        if struct_array.is_null(row)
                            || epoch_arr.is_null(row)
                            || fraction_arr.is_null(row)
                        {
                            serde_json::Value::Null
                        } else {
                            let secs = epoch_arr.value(row);
                            let nanos = fraction_arr.value(row).max(0) as u32;
                            serde_json::Value::String(format_timestamp_struct_value(secs, nanos))
                        }
                    };

                    let json_value = if num_rows == 1 {
                        to_json_value(0)
                    } else {
                        serde_json::Value::Array(
                            (0..num_rows).map(|row| to_json_value(row)).collect(),
                        )
                    };

                    if let serde_json::Value::String(val) = &json_value {
                        tracing::debug!("bind param {} TIMESTAMP struct value {}", i + 1, val);
                        eprintln!("DEBUG bind param {} TIMESTAMP struct value {}", i + 1, val);
                    } else if let serde_json::Value::Array(arr) = &json_value {
                        if let Some(serde_json::Value::String(val)) = arr.first() {
                            tracing::debug!(
                                "bind param {} TIMESTAMP struct first array value {}",
                                i + 1,
                                val
                            );
                            eprintln!(
                                "DEBUG bind param {} TIMESTAMP struct first array value {}",
                                i + 1,
                                val
                            );
                        }
                    }

                    parameters.insert(
                        format!("{}", i + 1),
                        query_request::BindParameter {
                            type_: type_name.to_string(),
                            value: json_value,
                            format: None,
                            schema: None,
                        },
                    );
                } else {
                    UnsupportedBindParameterTypeSnafu {
                        type_: column.data_type().to_string(),
                    }
                    .fail()?;
                }
            }
            _ => {
                UnsupportedBindParameterTypeSnafu {
                    type_: column.data_type().to_string(),
                }
                .fail()?;
            }
        }
    }
    Ok(parameters)
}

fn format_timestamp_ntz_iso(secs: i64, nanos: u32) -> String {
    if let Some(dt) = chrono::NaiveDateTime::from_timestamp_opt(secs, nanos) {
        let year_str = format_year(dt.year());
        format!(
            "{year_str}-{:02}-{:02} {:02}:{:02}:{:02}.{:09}",
            dt.month(),
            dt.day(),
            dt.hour(),
            dt.minute(),
            dt.second(),
            nanos
        )
    } else {
        format!("{secs}.{nanos:09}")
    }
}

fn format_year(year: i32) -> String {
    let adjusted = if year > 0 { year } else { year - 1 };
    if adjusted >= 0 {
        format!("{adjusted:04}")
    } else {
        format!("-{:04}", (-adjusted))
    }
}

fn format_timestamp_struct_value(secs: i64, nanos: u32) -> String {
    let total = (secs as i128) * 1_000_000_000i128 + nanos as i128;
    let whole = total.div_euclid(1_000_000_000);
    let fractional = total.rem_euclid(1_000_000_000);
    format!("{whole}.{fractional:09}")
}

fn parse_session_timezone(query: &str) -> Option<String> {
    let upper = query.to_ascii_uppercase();
    if !upper.contains("ALTER SESSION") || !upper.contains("TIMEZONE") {
        return None;
    }

    let timezone_pos = upper.find("TIMEZONE")?;
    let slice = &query[timezone_pos..];
    let eq_idx = slice.find('=')?;
    let after_eq = slice[eq_idx + 1..].trim_start();

    let value = if let Some(rest) = after_eq.strip_prefix('\'') {
        let end = rest.find('\'')?;
        &rest[..end]
    } else if let Some(rest) = after_eq.strip_prefix('"') {
        let end = rest.find('"')?;
        &rest[..end]
    } else {
        let end = after_eq
            .find(|c: char| c == ' ' || c == ';' || c == '\n')
            .unwrap_or(after_eq.len());
        &after_eq[..end]
    };

    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_client_timestamp_mapping(query: &str) -> Option<String> {
    let lower = query.to_ascii_lowercase();
    if !lower.contains("alter session") || !lower.contains("client_timestamp_type_mapping") {
        return None;
    }

    let after_equals = query.split('=').nth(1)?.trim();
    let cleaned = after_equals.trim_end_matches(';').trim();

    if let Some(rest) = cleaned.strip_prefix('\'') {
        let end = rest.find('\'')?;
        Some(rest[..end].to_string())
    } else if let Some(rest) = cleaned.strip_prefix('"') {
        let end = rest.find('"')?;
        Some(rest[..end].to_string())
    } else {
        let token_end = cleaned
            .find(|c: char| c.is_whitespace() || c == ';')
            .unwrap_or(cleaned.len());
        let token = &cleaned[..token_end];
        if token.is_empty() {
            None
        } else {
            Some(token.trim_matches('\'').trim_matches('"').to_string())
        }
    }
}

fn apply_result_format_override(
    rt: &tokio::runtime::Runtime,
    http_client: &reqwest::Client,
    query_parameters: QueryParameters,
    session_token: String,
    session_timezone: Option<String>,
    enable_json: bool,
) {
    let alter_stmt = if enable_json {
        "alter session set ODBC_QUERY_RESULT_FORMAT='JSON', GO_QUERY_RESULT_FORMAT='JSON'"
    } else {
        "alter session set ODBC_QUERY_RESULT_FORMAT='ARROW', GO_QUERY_RESULT_FORMAT='ARROW'"
    };
    if let Err(err) = rt.block_on(snowflake_query_with_client(
        http_client,
        query_parameters,
        session_token,
        alter_stmt.to_string(),
        None,
        None,
        false,
        session_timezone,
        enable_json,
    )) {
        tracing::warn!(
            error = %err,
            "Failed to apply ODBC_QUERY_RESULT_FORMAT override via '{alter_stmt}'"
        );
    }
}

pub struct Statement {
    pub state: StatementState,
    pub settings: HashMap<String, Setting>,
    pub query: Option<String>,
    pub parameter_bindings: Option<RecordBatch>,
    pub conn: Arc<Mutex<Connection>>,
    pub current_query_id: Option<String>,
    pub multi_statement_count: usize,
}

#[derive(Debug, Clone)]
pub enum StatementState {
    Initialized,
    Executed,
}

impl Statement {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Statement {
            settings: HashMap::new(),
            state: StatementState::Initialized,
            query: None,
            parameter_bindings: None,
            conn,
            current_query_id: None,
            multi_statement_count: 0,
        }
    }

    pub fn bind_parameters(&mut self, record_batch: RecordBatch) -> Result<(), StatementError> {
        match self.state {
            StatementState::Initialized => {
                self.parameter_bindings = Some(record_batch);
            }
            _ => {
                InvalidStateTransitionSnafu {
                    msg: format!("Cannot bind parameters in state={:?}", self.state),
                }
                .fail()?;
            }
        }
        Ok(())
    }

    pub fn get_query_parameter_bindings(
        &self,
    ) -> Result<Option<HashMap<String, query_request::BindParameter>>, StatementError> {
        match self.parameter_bindings.as_ref() {
            Some(parameters) => Ok(Some(parameters_from_record_batch(parameters)?)),
            None => Ok(None),
        }
    }
}

#[derive(Snafu, Debug)]
pub enum StatementError {
    #[snafu(display("Unsupported bind parameter type: {type_}"))]
    UnsupportedBindParameterType {
        type_: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Invalid state transition: {msg}"))]
    InvalidStateTransition {
        msg: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
