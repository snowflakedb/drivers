use crate::api::api_utils::cstr_to_string;
use crate::api::error::{
    ArrowArrayStreamReaderCreationSnafu, ArrowBindingSnafu, DisconnectedSnafu,
    InvalidParameterNumberSnafu, Required,
};
use crate::api::{ConnectionState, OdbcResult, ParameterBinding, StatementState, stmt_from_handle};
use crate::cdata_types::CDataType;
use crate::write_arrow::odbc_bindings_to_arrow_bindings;
use arrow::ffi::{FFI_ArrowArray, FFI_ArrowSchema};
use arrow::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use odbc_sys as sql;
use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf_gen::database_driver_v1::{
    ArrowArrayPtr, ArrowSchemaPtr, StatementBindRequest, StatementExecuteQueryRequest,
    StatementExecuteQueryResponse, StatementPrepareRequest, StatementSetSqlQueryRequest,
};
use snafu::ResultExt;
use tracing;

fn protobuf_from_ffi_arrow_array(raw: *mut FFI_ArrowArray) -> ArrowArrayPtr {
    let len = std::mem::size_of::<*mut FFI_ArrowArray>();
    let buf_ptr = std::ptr::addr_of!(raw) as *const u8;
    let slice = unsafe { std::slice::from_raw_parts(buf_ptr, len) };
    let vec = slice.to_vec();
    ArrowArrayPtr { value: vec }
}

fn protobuf_from_ffi_arrow_schema(raw: *mut FFI_ArrowSchema) -> ArrowSchemaPtr {
    let len = std::mem::size_of::<*mut FFI_ArrowSchema>();
    let buf_ptr = std::ptr::addr_of!(raw) as *const u8;
    let slice = unsafe { std::slice::from_raw_parts(buf_ptr, len) };
    let vec = slice.to_vec();
    ArrowSchemaPtr { value: vec }
}

/// Execute a SQL statement directly
pub fn exec_direct(
    statement_handle: sql::Handle,
    statement_text: *const sql::Char,
    text_length: sql::Integer,
) -> OdbcResult<()> {
    let stmt = stmt_from_handle(statement_handle);
    tracing::debug!("exec_direct: statement_handle={:?}", statement_handle);

    match &mut stmt.conn.state {
        ConnectionState::Connected {
            db_handle: _,
            conn_handle: _,
        } => {
            let query = cstr_to_string(statement_text, text_length)?;

            DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
                stmt_handle: Some(stmt.stmt_handle),
                query,
            })?;

            let response =
                DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
                    stmt_handle: Some(stmt.stmt_handle),
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
            // If there are bound parameters, we should bind them to the statement
            if !stmt.parameter_bindings.is_empty() {
                tracing::info!(
                    "execute: Found {} bound parameters",
                    stmt.parameter_bindings.len()
                );

                let (schema, array) = odbc_bindings_to_arrow_bindings(&stmt.parameter_bindings)
                    .context(ArrowBindingSnafu {})?;

                // Bind parameters to statement
                DatabaseDriverClient::statement_bind(StatementBindRequest {
                    stmt_handle: Some(stmt.stmt_handle),
                    schema: Some(protobuf_from_ffi_arrow_schema(Box::into_raw(schema))),
                    array: Some(protobuf_from_ffi_arrow_array(Box::into_raw(array))),
                })?;

                tracing::info!("Successfully bound parameters");
            }

            // Execute the prepared statement
            let response =
                DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
                    stmt_handle: Some(stmt.stmt_handle),
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
