use crate::api::api_utils::{cstr_to_string, utf16_to_string};
use crate::api::error::{
    ArrowArrayStreamReaderCreationSnafu, BindParametersSnafu, DisconnectedSnafu,
    InvalidParameterNumberSnafu, Required,
};
use crate::api::{ConnectionState, OdbcResult, ParameterBinding, StatementState, stmt_from_handle};
use crate::cdata_types::CDataType;
use crate::conversion::Binding;
use crate::json_binding;
use arrow::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use odbc_sys as sql;
use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf_gen::database_driver_v1::{
    QueryBindings, StatementExecuteQueryRequest, StatementExecuteQueryResponse,
    StatementPrepareRequest, StatementSetSqlQueryRequest, StringPtr, query_bindings,
};
use snafu::ResultExt;
use tracing;

pub fn exec_direct_n(
    statement_handle: sql::Handle,
    statement_text: *const sql::Char,
    text_length: sql::Integer,
) -> OdbcResult<()> {
    let query = cstr_to_string(statement_text, text_length)?;
    exec_direct(statement_handle, &query)
}

pub fn exec_direct_w(
    statement_handle: sql::Handle,
    statement_text: *const sql::WChar,
    text_length: sql::Integer,
) -> OdbcResult<()> {
    let query = utf16_to_string(statement_text, text_length)?;
    exec_direct(statement_handle, &query)
}

/// Execute a SQL statement directly
pub fn exec_direct(statement_handle: sql::Handle, statement_text: &str) -> OdbcResult<()> {
    let stmt = stmt_from_handle(statement_handle);
    tracing::debug!("exec_direct: statement_handle={:?}", statement_handle);

    match &mut stmt.conn.state {
        ConnectionState::Connected {
            db_handle: _,
            conn_handle: _,
        } => {
            DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
                stmt_handle: Some(stmt.stmt_handle),
                query: statement_text.to_string(),
            })?;

            let response =
                DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
                    stmt_handle: Some(stmt.stmt_handle),
                    bindings: None,
                })?;

            stmt.state = create_execute_state(response)?.into();
            Ok(())
        }
        ConnectionState::Disconnected => {
            tracing::error!("exec_direct: connection is disconnected");
            DisconnectedSnafu.fail()
        }
    }
}

/// Prepare a SQL statement
pub fn prepare(
    statement_handle: sql::Handle,
    statement_text: *const sql::Char,
    text_length: sql::Integer,
) -> OdbcResult<()> {
    tracing::debug!("prepare: statement_handle={:?}", statement_handle);
    let stmt = stmt_from_handle(statement_handle);

    match &mut stmt.conn.state {
        ConnectionState::Connected {
            db_handle: _,
            conn_handle: _,
        } => {
            let query = cstr_to_string(statement_text, text_length)?;
            tracing::debug!("prepare: query = {}", query);

            // Set the SQL query for the statement
            DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
                stmt_handle: Some(stmt.stmt_handle),
                query,
            })?;

            // Call the prepare method on the statement
            DatabaseDriverClient::statement_prepare(StatementPrepareRequest {
                stmt_handle: Some(stmt.stmt_handle),
            })?;

            tracing::info!("prepare: Successfully prepared statement");
            Ok(())
        }
        ConnectionState::Disconnected => {
            tracing::error!("prepare: connection is disconnected");
            DisconnectedSnafu.fail()
        }
    }
}

/// Execute a prepared statement
pub fn execute(statement_handle: sql::Handle) -> OdbcResult<()> {
    tracing::debug!("execute: statement_handle={:?}", statement_handle);
    let stmt = stmt_from_handle(statement_handle);

    match &mut stmt.conn.state {
        ConnectionState::Connected {
            db_handle: _,
            conn_handle: _,
        } => {
            // If there are bound parameters, serialize them to JSON
            let bindings = if !stmt.parameter_bindings.is_empty() {
                tracing::info!(
                    "execute: Found {} bound parameters",
                    stmt.parameter_bindings.len()
                );

                // Serialize bindings to JSON
                let json_str =
                    json_binding::serialize_bindings(&stmt.parameter_bindings).map_err(|e| {
                        BindParametersSnafu {
                            parameters: format!("Failed to serialize bindings: {}", e),
                        }
                        .build()
                    })?;

                if json_str.is_empty() {
                    None
                } else {
                    // Store JSON string in statement to prevent deallocation
                    stmt.json_binding_data = Some(json_str);

                    // Get pointer to the stored JSON string
                    let json_bytes = stmt.json_binding_data.as_ref().unwrap().as_bytes();
                    let ptr_value = json_bytes.as_ptr() as usize;

                    // Convert pointer to 8-byte little-endian representation
                    let ptr_bytes = ptr_value.to_le_bytes().to_vec();

                    // Create StringPtr with memory pointer
                    let string_ptr = StringPtr {
                        value: ptr_bytes,
                        length: json_bytes.len() as i64,
                    };

                    // Create QueryBindings with JSON binding type
                    Some(QueryBindings {
                        binding_type: Some(query_bindings::BindingType::Json(string_ptr)),
                    })
                }
            } else {
                None
            };

            // Execute the prepared statement with bindings
            let response =
                DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
                    stmt_handle: Some(stmt.stmt_handle),
                    bindings,
                })?;

            tracing::info!("execute: Successfully executed statement");
            stmt.state = create_execute_state(response)?.into();
            Ok(())
        }
        ConnectionState::Disconnected => {
            tracing::error!("execute: connection is disconnected");
            DisconnectedSnafu.fail()
        }
    }
}

fn create_execute_state(response: StatementExecuteQueryResponse) -> OdbcResult<StatementState> {
    let result = response.result.required("Execute result is required")?;
    let stream_ptr: *mut FFI_ArrowArrayStream =
        result.stream.required("Stream is required")?.into();
    let stream = unsafe { FFI_ArrowArrayStream::from_raw(stream_ptr) };
    let reader =
        ArrowArrayStreamReader::try_new(stream).context(ArrowArrayStreamReaderCreationSnafu {})?;
    let rows_affected = result.rows_affected;
    Ok(StatementState::Executed {
        reader,
        rows_affected,
    })
}

/// Bind a parameter to a prepared statement
#[allow(clippy::too_many_arguments)]
pub fn bind_parameter(
    statement_handle: sql::Handle,
    parameter_number: sql::USmallInt,
    input_output_type: sql::ParamType,
    value_type: CDataType,
    parameter_type: sql::SqlDataType,
    _column_size: sql::ULen,
    _decimal_digits: sql::SmallInt,
    parameter_value_ptr: sql::Pointer,
    buffer_length: sql::Len,
    str_len_or_ind_ptr: *mut sql::Len,
) -> OdbcResult<()> {
    // TODO handle input_output_type
    tracing::debug!(
        "bind_parameter: parameter_number={}, input_output_type={:?}, value_type={:?}, parameter_type={:?}",
        parameter_number,
        input_output_type,
        value_type,
        parameter_type
    );

    if parameter_number == 0 {
        tracing::error!("bind_parameter: parameter_number cannot be 0");
        return InvalidParameterNumberSnafu.fail();
    }

    let stmt = stmt_from_handle(statement_handle);

    let binding = ParameterBinding {
        parameter_type,
        value_type,
        parameter_value_ptr,
        buffer_length,
        str_len_or_ind_ptr,
    };

    // Store the binding
    stmt.parameter_bindings.insert(parameter_number, binding);

    tracing::info!(
        "bind_parameter: Successfully bound parameter {}",
        parameter_number
    );
    Ok(())
}

/// Bind a column to a statement
pub fn bind_col(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    target_type: CDataType,
    target_value_ptr: sql::Pointer,
    buffer_length: sql::Len,
    str_len_or_ind_ptr: *mut sql::Len,
) -> OdbcResult<()> {
    tracing::debug!(
        "bind_col: statement_handle={:?}, column_number={}, target_type={:?}",
        statement_handle,
        column_number,
        target_type
    );

    let stmt = stmt_from_handle(statement_handle);

    stmt.column_bindings.insert(
        column_number,
        Binding {
            target_type,
            target_value_ptr,
            buffer_length,
            str_len_or_ind_ptr,
        },
    );
    Ok(())
}
