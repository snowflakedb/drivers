use crate::api::api_utils::{cstr_to_string, utf16_to_string};
use crate::api::error::{
    ArrowArrayStreamReaderCreationSnafu, DisconnectedSnafu, InvalidBufferLengthSnafu,
    InvalidCursorStateSnafu, InvalidHandleSnafu, InvalidParameterNumberSnafu, JsonBindingSnafu,
    NoMoreDataSnafu, Required,
};
use crate::api::{ConnectionState, OdbcResult, ParameterBinding, StatementState, stmt_from_handle};
use crate::cdata_types::CDataType;
use crate::conversion::Binding;
use crate::write_json::odbc_bindings_to_json;
use arrow::array::RecordBatchReader;
use arrow::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use odbc_sys as sql;
use sf_core::protobuf::apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf::generated::database_driver_v1::{
    ArrowArrayStreamPtr, BinaryDataPtr, ConnectionGetParameterRequest, ConnectionHandle,
    QueryBindings, StatementExecuteQueryRequest, StatementExecuteQueryResponse,
    StatementPrepareRequest, StatementSetSqlQueryRequest, query_bindings,
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
            conn_handle,
        } => {
            DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
                stmt_handle: Some(stmt.stmt_handle),
                query: statement_text.to_string(),
            })?;

            let (bindings, _json_owner) = apply_parameter_bindings(&stmt.parameter_bindings)?;

            let response =
                DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
                    stmt_handle: Some(stmt.stmt_handle),
                    bindings,
                });

            tracing::info!("exec_direct: response={:?}", response);
            let response = response?;

            update_numeric_settings(conn_handle, &mut stmt.conn.numeric_settings);
            stmt.state = create_execute_state(response)?.into();
            if let StatementState::Executed { reader, .. } = stmt.state.as_ref() {
                stmt.ird.desc_count = reader.schema().fields().len() as sql::SmallInt;
            }
            Ok(())
        }
        ConnectionState::Disconnected => {
            tracing::error!("exec_direct: connection is disconnected");
            DisconnectedSnafu.fail()
        }
    }
}

use crate::conversion::NumericSettings;

fn update_numeric_settings(conn_handle: &ConnectionHandle, settings: &mut NumericSettings) {
    if let Ok(resp) =
        DatabaseDriverClient::connection_get_parameter(ConnectionGetParameterRequest {
            conn_handle: Some(*conn_handle),
            key: "ODBC_TREAT_DECIMAL_AS_INT".to_string(),
        })
        && let Some(value) = resp.value
    {
        let bool_value = value.eq_ignore_ascii_case("true");
        settings.treat_decimal_as_int = bool_value;
        tracing::info!(
            "Server parameter ODBC_TREAT_DECIMAL_AS_INT = {}",
            bool_value
        );
    }

    if let Ok(resp) =
        DatabaseDriverClient::connection_get_parameter(ConnectionGetParameterRequest {
            conn_handle: Some(*conn_handle),
            key: "ODBC_TREAT_BIG_NUMBER_AS_STRING".to_string(),
        })
        && let Some(value) = resp.value
    {
        let bool_value = value.eq_ignore_ascii_case("true");
        settings.treat_big_number_as_string = bool_value;
        tracing::info!(
            "Server parameter ODBC_TREAT_BIG_NUMBER_AS_STRING = {}",
            bool_value
        );
    }
}

pub fn prepare_n(
    statement_handle: sql::Handle,
    statement_text: *const sql::Char,
    text_length: sql::Integer,
) -> OdbcResult<()> {
    let query = cstr_to_string(statement_text, text_length)?;
    prepare(statement_handle, &query)
}

pub fn prepare_w(
    statement_handle: sql::Handle,
    statement_text: *const sql::WChar,
    text_length: sql::Integer,
) -> OdbcResult<()> {
    let query = utf16_to_string(statement_text, text_length)?;
    prepare(statement_handle, &query)
}

fn reader_from_protobuf_stream(stream: ArrowArrayStreamPtr) -> OdbcResult<ArrowArrayStreamReader> {
    let stream_ptr: *mut FFI_ArrowArrayStream = stream.into();
    let stream = unsafe { FFI_ArrowArrayStream::from_raw(stream_ptr) };
    let reader =
        ArrowArrayStreamReader::try_new(stream).context(ArrowArrayStreamReaderCreationSnafu {})?;
    Ok(reader)
}

/// Prepare a SQL statement
pub fn prepare(statement_handle: sql::Handle, query: &str) -> OdbcResult<()> {
    if statement_handle.is_null() {
        return InvalidHandleSnafu.fail();
    }
    if query.is_empty() {
        return InvalidBufferLengthSnafu { length: 0i64 }.fail();
    }
    tracing::debug!("prepare: statement_handle={:?}", statement_handle);
    let stmt = stmt_from_handle(statement_handle);

    if matches!(
        stmt.state.as_ref(),
        StatementState::Executed { .. } | StatementState::Fetching { .. } | StatementState::Done
    ) {
        tracing::error!("prepare: cursor is already open");
        return InvalidCursorStateSnafu.fail();
    }

    match &mut stmt.conn.state {
        ConnectionState::Connected {
            db_handle: _,
            conn_handle: _,
        } => {
            tracing::debug!("prepare: query = {query}");

            DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
                stmt_handle: Some(stmt.stmt_handle),
                query: query.to_string(),
            })?;

            let prepare_result =
                DatabaseDriverClient::statement_prepare(StatementPrepareRequest {
                    stmt_handle: Some(stmt.stmt_handle),
                })?;

            let result = prepare_result.result.required("Result is required")?;
            let stream_ptr = result.stream.required("Stream is required")?;
            let reader = reader_from_protobuf_stream(stream_ptr)?;
            stmt.ird.desc_count = reader.schema().fields().len() as sql::SmallInt;
            stmt.state.set(StatementState::Prepared { reader });
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

    if matches!(
        stmt.state.as_ref(),
        StatementState::Executed { .. } | StatementState::Fetching { .. } | StatementState::Done
    ) {
        tracing::error!("execute: cursor is already open");
        return InvalidCursorStateSnafu.fail();
    }

    match &mut stmt.conn.state {
        ConnectionState::Connected {
            db_handle: _,
            conn_handle,
        } => {
            let (bindings, _json_owner) = apply_parameter_bindings(&stmt.parameter_bindings)?;

            let response =
                DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
                    stmt_handle: Some(stmt.stmt_handle),
                    bindings,
                })?;

            tracing::info!("execute: Successfully executed statement");
            update_numeric_settings(conn_handle, &mut stmt.conn.numeric_settings);

            let statement_type_id = response.result.as_ref().and_then(|r| r.statement_type_id);
            let rows_affected = response.result.as_ref().and_then(|r| r.rows_affected);

            let execute_state = create_execute_state(response)?;

            // DML that affected 0 rows returns SQL_NO_DATA per ODBC spec.
            if is_dml_statement_type(statement_type_id) && Some(0) == rows_affected {
                stmt.state = StatementState::NoResultSet.into();
                stmt.ird.desc_count = 0;
                return NoMoreDataSnafu.fail();
            }

            stmt.state = execute_state.into();
            if let StatementState::Executed { reader, .. } = stmt.state.as_ref() {
                stmt.ird.desc_count = reader.schema().fields().len() as sql::SmallInt;
            }
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

fn is_dml_statement_type(statement_type_id: Option<i64>) -> bool {
    statement_type_id.is_some_and(|id| (0x3000..0x4000).contains(&id))
}

fn has_result_set(statement_type_id: i64) -> bool {
    is_ddl_statement(statement_type_id) && !is_pat_statement(statement_type_id)
}

fn create_execute_state(response: StatementExecuteQueryResponse) -> OdbcResult<StatementState> {
    tracing::debug!("create_execute_state: response={:?}", response);
    let result = response.result.required("Execute result is required")?;
    let stream = result.stream.required("Stream is required")?;
    let reader = reader_from_protobuf_stream(stream)?;
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

/// Build JSON query bindings from ODBC parameter bindings.
///
/// Returns `(bindings, json_owner)`. The caller **must** keep `json_owner` alive
/// until after the bindings have been consumed by `statement_execute_query`,
/// because `BinaryDataPtr` holds a raw pointer into the owned `String`.
fn apply_parameter_bindings(
    parameter_bindings: &std::collections::HashMap<u16, ParameterBinding>,
) -> OdbcResult<(Option<QueryBindings>, Option<String>)> {
    if parameter_bindings.is_empty() {
        return Ok((None, None));
    }
    tracing::info!(
        "apply_parameter_bindings: Found {} bound parameters",
        parameter_bindings.len()
    );

    let json_string = odbc_bindings_to_json(parameter_bindings).context(JsonBindingSnafu {})?;

    let json_data_ptr = json_string.as_bytes().as_ptr() as u64;
    let json_data_len = json_string.len();

    let binary_data_ptr = BinaryDataPtr {
        value: json_data_ptr.to_le_bytes().to_vec(),
        length: json_data_len as i64,
    };

    let bindings = QueryBindings {
        binding_type: Some(query_bindings::BindingType::Json(binary_data_ptr)),
    };

    tracing::info!("apply_parameter_bindings: Successfully bound parameters");

    Ok((Some(bindings), Some(json_string)))
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
            stmt.used_extended_fetch = false;
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
                octet_length_ptr: str_len_or_ind_ptr,
                indicator_ptr: str_len_or_ind_ptr,
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
    warnings: &mut crate::conversion::warning::Warnings,
) -> OdbcResult<()> {
    use crate::api::{CursorType, StmtAttr};
    use crate::conversion::warning::Warning;

    tracing::debug!(
        "set_stmt_attr: statement_handle={:?}, attribute={}, value_ptr={:?}",
        statement_handle,
        attribute,
        value_ptr
    );

    let attr = StmtAttr::try_from(attribute)?;
    let stmt = stmt_from_handle(statement_handle);

    match attr {
        StmtAttr::CursorType => {
            let raw = value_ptr as sql::ULen;
            let requested = CursorType::try_from(raw)?;
            tracing::debug!("set_stmt_attr: CursorType requested = {requested:?}");
            if requested != CursorType::ForwardOnly {
                stmt.cursor_type = CursorType::ForwardOnly;
                warnings.push(Warning::OptionValueChanged);
            } else {
                stmt.cursor_type = CursorType::ForwardOnly;
            }
            Ok(())
        }
        StmtAttr::MaxLength => {
            let length = value_ptr as sql::ULen;
            tracing::debug!("set_stmt_attr: MaxLength = {}", length);
            stmt.max_length = length;
            Ok(())
        }
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

    tracing::debug!("get_stmt_attr: attribute={}", attribute);

    let attr = StmtAttr::try_from(attribute)?;
    let stmt = stmt_from_handle(statement_handle);

    match attr {
        StmtAttr::CursorType => {
            unsafe {
                std::ptr::write_unaligned(
                    value_ptr as *mut sql::ULen,
                    stmt.cursor_type as sql::ULen,
                );
                if !string_length_ptr.is_null() {
                    std::ptr::write_unaligned(
                        string_length_ptr,
                        std::mem::size_of::<sql::ULen>() as sql::Integer,
                    );
                }
            }
            Ok(())
        }
        StmtAttr::MaxLength => {
            unsafe {
                *(value_ptr as *mut sql::ULen) = stmt.max_length;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
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
        StmtAttr::AppParamDesc => {
            let apd_ptr = &mut stmt.apd as *mut crate::api::ArdDescriptor as sql::Handle;
            unsafe {
                *(value_ptr as *mut sql::Handle) = apd_ptr;
            }
            Ok(())
        }
        StmtAttr::ImpParamDesc => {
            let ipd_ptr = &mut stmt.ipd as *mut crate::api::IrdDescriptor as sql::Handle;
            unsafe {
                *(value_ptr as *mut sql::Handle) = ipd_ptr;
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
