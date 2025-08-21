use crate::api::{
    ConnectionState, OdbcError, OdbcResult, ParameterBinding, StatementState, stmt_from_handle,
};
use crate::cdata_types::CDataType;
use crate::write_arrow::odbc_bindings_to_arrow_bindings;
use arrow::ffi::{FFI_ArrowArray, FFI_ArrowSchema};
use odbc_sys as sql;
use sf_core::thrift_gen::database_driver_v1::{ArrowArrayPtr, ArrowSchemaPtr};
use tracing;

fn thrift_from_ffi_arrow_array(raw: *mut FFI_ArrowArray) -> ArrowArrayPtr {
    let len = std::mem::size_of::<*mut FFI_ArrowArray>();
    let buf_ptr = std::ptr::addr_of!(raw) as *const u8;
    let slice = unsafe { std::slice::from_raw_parts(buf_ptr, len) };
    let vec = slice.to_vec();
    ArrowArrayPtr { value: vec }
}

fn thrift_from_ffi_arrow_schema(raw: *mut FFI_ArrowSchema) -> ArrowSchemaPtr {
    let len = std::mem::size_of::<*mut FFI_ArrowSchema>();
    let buf_ptr = std::ptr::addr_of!(raw) as *const u8;
    let slice = unsafe { std::slice::from_raw_parts(buf_ptr, len) };
    let vec = slice.to_vec();
    ArrowSchemaPtr { value: vec }
}

/// Convert text pointer to String
fn text_to_string(text: *const sql::Char, length: sql::Integer) -> Result<String, OdbcError> {
    if length == sql::NTS as i32 {
        let result = unsafe { std::ffi::CStr::from_ptr(text as *const i8).to_str() };
        match result {
            Ok(s) => Ok(s.to_string()),
            Err(e) => {
                tracing::error!("text_to_string: error converting text to string: {}", e);
                Err(OdbcError::TextConversion(format!(
                    "Failed to convert text: {e}"
                )))
            }
        }
    } else {
        let text_slice = unsafe { std::slice::from_raw_parts(text, length as usize) };
        String::from_utf8(text_slice.to_vec())
            .map_err(|e| OdbcError::TextConversion(format!("Failed to convert UTF-8: {e}")))
    }
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
            client,
            db_handle: _,
            conn_handle: _,
        } => {
            let query = text_to_string(statement_text, text_length)?;

            client
                .statement_set_sql_query(stmt.stmt_handle.clone(), query)
                .map_err(|e| OdbcError::SetSqlQuery(format!("{e:?}")))?;

            let result = client
                .statement_execute_query(stmt.stmt_handle.clone())
                .map_err(|e| OdbcError::ExecuteStatement(format!("{e:?}")))?;

            stmt.state = StatementState::Executed { result };
            Ok(())
        }
        ConnectionState::Disconnected => {
            tracing::error!("exec_direct: connection is disconnected");
            Err(OdbcError::Disconnected)
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
            client,
            db_handle: _,
            conn_handle: _,
        } => {
            let query = text_to_string(statement_text, text_length)?;
            tracing::debug!("prepare: query = {}", query);

            // Set the SQL query for the statement
            client
                .statement_set_sql_query(stmt.stmt_handle.clone(), query)
                .map_err(|e| OdbcError::SetSqlQuery(format!("{e:?}")))?;

            // Call the prepare method on the statement
            client
                .statement_prepare(stmt.stmt_handle.clone())
                .map_err(|e| OdbcError::PrepareStatement(format!("{e:?}")))?;

            tracing::info!("prepare: Successfully prepared statement");
            Ok(())
        }
        ConnectionState::Disconnected => {
            tracing::error!("prepare: connection is disconnected");
            Err(OdbcError::Disconnected)
        }
    }
}

/// Execute a prepared statement
pub fn execute(statement_handle: sql::Handle) -> OdbcResult<()> {
    tracing::debug!("execute: statement_handle={:?}", statement_handle);
    let stmt = stmt_from_handle(statement_handle);

    match &mut stmt.conn.state {
        ConnectionState::Connected {
            client,
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
                    .map_err(|e| OdbcError::ParameterBinding(format!("{e:?}")))?;

                // Bind parameters to statement
                client
                    .statement_bind(
                        stmt.stmt_handle.clone(),
                        thrift_from_ffi_arrow_schema(Box::into_raw(schema)),
                        thrift_from_ffi_arrow_array(Box::into_raw(array)),
                    )
                    .map_err(|e| OdbcError::BindParameters(format!("{e:?}")))?;

                tracing::info!("Successfully bound parameters");
            }

            // Execute the prepared statement
            let result = client
                .statement_execute_query(stmt.stmt_handle.clone())
                .map_err(|e| OdbcError::ExecuteStatement(format!("{e:?}")))?;

            tracing::info!("execute: Successfully executed statement");
            stmt.state = StatementState::Executed { result };
            Ok(())
        }
        ConnectionState::Disconnected => {
            tracing::error!("execute: connection is disconnected");
            Err(OdbcError::Disconnected)
        }
    }
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
        return Err(OdbcError::InvalidParameterNumber);
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
