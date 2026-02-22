use crate::api::api_utils::{cstr_to_string, utf16_to_string};
use crate::api::error::{
    ArrowArrayStreamReaderCreationSnafu, ArrowBindingSnafu, DisconnectedSnafu,
    InvalidParameterNumberSnafu, Required,
};
use crate::api::{ConnectionState, OdbcResult, ParameterBinding, StatementState, stmt_from_handle};
use crate::cdata_types::CDataType;
use crate::conversion::Binding;
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
                });

            tracing::info!("exec_direct: response={:?}", response);
            let response = response?;

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
                    bindings: None,
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

const STATEMENT_TYPE_ID_MANAGE_PATS: i64 = 0x6244;

fn is_pat_statement(statement_type_id: i64) -> bool {
    statement_type_id == STATEMENT_TYPE_ID_MANAGE_PATS
}

fn is_ddl_statement(statement_type_id: i64) -> bool {
    tracing::debug!("is_ddl_statement: statement_type_id={}", statement_type_id);
    if statement_type_id == STATEMENT_TYPE_ID_MANAGE_PATS {
        return false;
    }
    (0x6000..0x7000).contains(&statement_type_id)
}

fn has_result_set(statement_type_id: i64) -> bool {
    is_ddl_statement(statement_type_id) && !is_pat_statement(statement_type_id)
}

fn create_execute_state(response: StatementExecuteQueryResponse) -> OdbcResult<StatementState> {
    tracing::debug!("create_execute_state: response={:?}", response);
    let result = response.result.required("Execute result is required")?;
    let stream_ptr: *mut FFI_ArrowArrayStream =
        result.stream.required("Stream is required")?.into();
    let stream = unsafe { FFI_ArrowArrayStream::from_raw(stream_ptr) };
    let reader =
        ArrowArrayStreamReader::try_new(stream).context(ArrowArrayStreamReaderCreationSnafu {})?;
    let rows_affected = result.rows_affected;
    if let Some(statement_type_id) = result.statement_type_id
        && has_result_set(statement_type_id)
    {
        return Ok(StatementState::NoResultSet);
    }
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

/// Free statement resources based on the option
pub fn free_stmt(statement_handle: sql::Handle, option: sql::FreeStmtOption) -> OdbcResult<()> {
    tracing::debug!("free_stmt: statement_handle={statement_handle:?}, option={option:?}");

    let stmt = stmt_from_handle(statement_handle);

    match option {
        sql::FreeStmtOption::Close => {
            tracing::info!("free_stmt: Closing cursor");
            stmt.state = StatementState::Created.into();
            stmt.get_data_state = None;
        }
        sql::FreeStmtOption::Unbind => {
            tracing::info!("free_stmt: Unbinding all columns");
            stmt.ard.unbind_all();
        }
        sql::FreeStmtOption::ResetParams => {
            tracing::info!("free_stmt: Resetting all parameters");
            stmt.parameter_bindings.clear();
        }
    }

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

    // Per ODBC specification, if target_value_ptr is null, unbind the column
    if target_value_ptr.is_null() {
        tracing::debug!("bind_col: unbinding column {}", column_number);
        stmt.ard.bindings.remove(&column_number);
    } else {
        stmt.ard.bindings.insert(
            column_number,
            Binding {
                target_type,
                target_value_ptr,
                buffer_length,
                str_len_or_ind_ptr,
                precision: None,
                scale: None,
            },
        );
    }
    Ok(())
}

/// Set a statement attribute value
pub fn set_stmt_attr(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    _string_length: sql::Integer,
) -> OdbcResult<()> {
    use crate::api::StmtAttr;

    tracing::debug!(
        "set_stmt_attr: statement_handle={:?}, attribute={}, value_ptr={:?}",
        statement_handle,
        attribute,
        value_ptr
    );

    let attr = StmtAttr::try_from(attribute)?;
    let stmt = stmt_from_handle(statement_handle);

    match attr {
        StmtAttr::UseBookmarks => {
            tracing::debug!("set_stmt_attr: UseBookmarks (ignored, bookmarks not supported)");
            Ok(())
        }
        StmtAttr::RowArraySize => {
            let size = value_ptr as usize;
            tracing::debug!("set_stmt_attr: RowArraySize = {}", size);
            let effective_size = if size == 0 {
                tracing::warn!("set_stmt_attr: RowArraySize value 0 is invalid; coercing to 1");
                1
            } else {
                size
            };
            stmt.ard.array_size = effective_size;
            Ok(())
        }
        StmtAttr::RowStatusPtr => {
            let ptr = value_ptr as *mut u16;
            tracing::debug!("set_stmt_attr: RowStatusPtr = {:?}", ptr);
            stmt.ird.array_status_ptr = ptr;
            Ok(())
        }
        StmtAttr::RowsFetchedPtr => {
            let ptr = value_ptr as *mut sql::ULen;
            tracing::debug!("set_stmt_attr: RowsFetchedPtr = {:?}", ptr);
            stmt.ird.rows_processed_ptr = ptr;
            Ok(())
        }
        StmtAttr::RowBindType => {
            let raw_bind_type = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: RowBindType (raw) = {}", raw_bind_type);
            stmt.ard.bind_type = raw_bind_type;
            Ok(())
        }
        StmtAttr::RowBindOffsetPtr => {
            let ptr = value_ptr as *mut sql::Len;
            tracing::debug!("set_stmt_attr: RowBindOffsetPtr = {:?}", ptr);
            stmt.ard.bind_offset_ptr = ptr;
            Ok(())
        }
        _ => {
            tracing::warn!("set_stmt_attr: unsupported attribute {:?}", attr);
            crate::api::error::UnsupportedAttributeSnafu { attribute }.fail()
        }
    }
}

/// Get a statement attribute value
pub fn get_stmt_attr(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    _buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> OdbcResult<()> {
    use crate::api::StmtAttr;

    tracing::debug!(
        "get_stmt_attr: statement_handle={:?}, attribute={}",
        statement_handle,
        attribute
    );

    let attr = StmtAttr::try_from(attribute)?;
    let stmt = stmt_from_handle(statement_handle);

    match attr {
        StmtAttr::AppRowDesc => {
            let ard_ptr = &mut stmt.ard as *mut crate::api::ArdDescriptor as sql::Handle;
            unsafe {
                *(value_ptr as *mut sql::Handle) = ard_ptr;
            }
            Ok(())
        }
        StmtAttr::ImpRowDesc => {
            let ird_ptr = &mut stmt.ird as *mut crate::api::IrdDescriptor as sql::Handle;
            unsafe {
                *(value_ptr as *mut sql::Handle) = ird_ptr;
            }
            Ok(())
        }
        StmtAttr::RowArraySize => {
            unsafe {
                *(value_ptr as *mut sql::ULen) = stmt.ard.array_size as sql::ULen;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        StmtAttr::RowStatusPtr => {
            unsafe {
                *(value_ptr as *mut *mut u16) = stmt.ird.array_status_ptr;
            }
            Ok(())
        }
        StmtAttr::RowsFetchedPtr => {
            unsafe {
                *(value_ptr as *mut *mut sql::ULen) = stmt.ird.rows_processed_ptr;
            }
            Ok(())
        }
        StmtAttr::RowBindType => {
            unsafe {
                *(value_ptr as *mut sql::ULen) = stmt.ard.bind_type;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        StmtAttr::RowBindOffsetPtr => {
            unsafe {
                *(value_ptr as *mut *mut sql::Len) = stmt.ard.bind_offset_ptr;
            }
            Ok(())
        }
        _ => {
            tracing::warn!("get_stmt_attr: unsupported attribute {:?}", attr);
            crate::api::error::UnknownAttributeSnafu { attribute }.fail()
        }
    }
}
