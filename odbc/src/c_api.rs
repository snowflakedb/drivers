//! ODBC C API functions
//!
//! This module provides the C API interface for ODBC functions.

#![allow(non_snake_case)]

use crate::api::CDataType;
use crate::api::{self, Narrow, ToSqlReturn, Wide};
use odbc_sys as sql;

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLAllocEnv(output_handle: *mut sql::Handle) -> sql::RetCode {
    api::handle_allocation::sql_alloc_handle(sql::HandleType::Env, 0 as sql::Handle, output_handle)
        .to_sql_code()
}
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLAllocConnect(
    environment_handle: sql::Handle,
    output_handle: *mut sql::Handle,
) -> sql::RetCode {
    api::handle_allocation::sql_alloc_handle(
        sql::HandleType::Dbc,
        environment_handle,
        output_handle,
    )
    .to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLAllocHandle(
    handle_type: sql::HandleType,
    input_handle: sql::Handle,
    output_handle: *mut sql::Handle,
) -> sql::RetCode {
    api::handle_allocation::sql_alloc_handle(handle_type, input_handle, output_handle).to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLExecDirect(
    statement_handle: sql::Handle,
    statement_text: *const sql::Char,
    text_length: sql::Integer,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result =
        api::statement::exec_direct::<Narrow>(statement_handle, statement_text, text_length);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLExecDirectW(
    statement_handle: sql::Handle,
    statement_text: *const sql::WChar,
    text_length: sql::Integer,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::exec_direct::<Wide>(statement_handle, statement_text, text_length);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLFreeHandle(
    handle_type: sql::HandleType,
    handle: sql::Handle,
) -> sql::RetCode {
    api::handle_allocation::sql_free_handle(handle_type, handle).to_sql_code()
}
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLFreeStmt(
    statement_handle: sql::Handle,
    option: sql::USmallInt,
) -> sql::RetCode {
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::FreeStmtOption::try_from(option)
        .and_then(|opt| api::statement::free_stmt(statement_handle, opt));
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLCloseCursor(statement_handle: sql::Handle) -> sql::RetCode {
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::close_cursor(statement_handle);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
///
/// ODBC allows SQLCancel to be called from a different thread.
/// `cancel()` accesses the `Statement` via `stmt_from_handle()` — the
/// same pattern used by every other C API entry point. Cross-thread
/// calls therefore create concurrent `&mut Statement` references, which
/// is a pre-existing codebase-wide aliasing issue. A future handle
/// manager will introduce proper interior mutability to eliminate this UB.
///
/// This function does not modify statement diagnostics. Any diagnostic
/// information related to cancellation must be produced by the executing
/// thread (e.g., returning HY008) or by code that can ensure exclusive
/// access to the statement.
///
/// NOTE: On Unix platforms, the Driver Manager (unixODBC/iODBC) updates its
/// internal state machine to close the cursor after SQLCancel, even though
/// this function is a no-op. Subsequent SQLFetch calls are rejected by the
/// DM with HY010 before reaching the driver. This is an ODBC 2.x behavior
/// that both Unix DMs implement regardless of SQL_ATTR_ODBC_VERSION.
///
/// KNOWN LIMITATION: Same-thread SQLCancel (for no-op / synchronous
/// cancel) also skips clearing stale diagnostics, which diverges from the
/// driver's pattern of clearing diagnostics on entry for every API call.
/// This means `SQLGetDiagRec` may return records from a previous call
/// after a successful `SQLCancel`. We accept this because we currently
/// cannot distinguish same-thread vs cross-thread callers.
///
/// TODO(SNOW-3258918, SNOW-3258919): When async or DAE cancel is
/// implemented, add thread-ID tracking to distinguish same-thread vs
/// cross-thread. Same-thread cancel must clear_diag_info and post its own
/// diagnostic records per spec. Only cross-thread cancel skips diagnostics.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLCancel(statement_handle: sql::Handle) -> sql::RetCode {
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    let result = api::statement::cancel(statement_handle);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
///
/// `SQLCancelHandle` is the ODBC 3.8 generalization of `SQLCancel` that
/// accepts both statement and connection handles.
///
/// For **SQL_HANDLE_STMT** the behavior is identical to `SQLCancel`:
/// this calls `api::statement::cancel(handle)`, which follows the same
/// handle-to-statement path as `SQLCancel` and therefore has the same
/// aliasing caveat described there. Diagnostics are not touched
/// (same reasoning as `SQLCancel`).
///
/// For **SQL_HANDLE_DBC** this is currently a no-op returning SUCCESS.
/// Connection-level cancel (async connect, cross-thread
/// `SQLDriverConnect`) will be implemented after the connection state
/// machine is hardened (SNOW-3307201).
///
/// **SQL_HANDLE_ENV** and **SQL_HANDLE_DESC** return `SQL_ERROR` with
/// SQLSTATE HY092 per the ODBC 3.8 spec. Any truly unknown handle
/// type returns `SQL_INVALID_HANDLE`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLCancelHandle(
    handle_type: sql::HandleType,
    handle: sql::Handle,
) -> sql::RetCode {
    if handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    match handle_type {
        sql::HandleType::Stmt => {
            let result = api::statement::cancel(handle);
            result.to_sql_code()
        }
        // TODO(SNOW-3307201): implement connection-level cancel after
        // the connection state machine is hardened.
        sql::HandleType::Dbc => {
            api::diagnostic::clear_diag_info(handle_type, handle);
            sql::SqlReturn::SUCCESS.0
        }
        sql::HandleType::Env | sql::HandleType::Desc => {
            api::diagnostic::clear_diag_info(handle_type, handle);
            let result: api::OdbcResult<()> = api::error::InvalidHandleTypeSnafu {
                handle_type: handle_type as i16,
            }
            .fail();
            api::diagnostic::set_diag_info_from_result(handle_type, handle, &result);
            result.to_sql_code()
        }
        _ => sql::SqlReturn::INVALID_HANDLE.0,
    }
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLConnect(
    connection_handle: sql::Handle,
    server_name: *const sql::Char,
    name_length1: sql::SmallInt,
    user_name: *const sql::Char,
    name_length2: sql::SmallInt,
    authentication: *const sql::Char,
    name_length3: sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result = api::connection::connect::<Narrow>(
        connection_handle,
        server_name,
        name_length1,
        user_name,
        name_length2,
        authentication,
        name_length3,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLConnectW(
    connection_handle: sql::Handle,
    server_name: *const sql::WChar,
    name_length1: sql::SmallInt,
    user_name: *const sql::WChar,
    name_length2: sql::SmallInt,
    authentication: *const sql::WChar,
    name_length3: sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result = api::connection::connect::<Wide>(
        connection_handle,
        server_name,
        name_length1,
        user_name,
        name_length2,
        authentication,
        name_length3,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLSetEnvAttr(
    environment_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    string_length: sql::Integer,
) -> sql::RetCode {
    if environment_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Env, environment_handle);
    let result =
        api::environment::set_env_attribute(environment_handle, attribute, value, string_length);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Env, environment_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetEnvAttr(
    environment_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    if environment_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Env, environment_handle);
    let result = api::environment::get_env_attribute(
        environment_handle,
        attribute,
        value,
        buffer_length,
        string_length_ptr,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Env, environment_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetInfo(
    connection_handle: sql::Handle,
    info_type: sql::USmallInt,
    info_value_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result = api::connection::get_info::<Narrow>(
        connection_handle,
        info_type,
        info_value_ptr,
        buffer_length,
        string_length_ptr,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetInfoW(
    connection_handle: sql::Handle,
    info_type: sql::USmallInt,
    info_value_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result = api::connection::get_info::<Wide>(
        connection_handle,
        info_type,
        info_value_ptr,
        buffer_length,
        string_length_ptr,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLSetConnectAttr(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    string_length: sql::Integer,
) -> sql::RetCode {
    if connection_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result = api::connection::set_connect_attr::<Narrow>(
        connection_handle,
        attribute,
        value,
        string_length,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Dbc,
        connection_handle,
        &warnings,
    );
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLSetConnectAttrW(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    string_length: sql::Integer,
) -> sql::RetCode {
    if connection_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result = api::connection::set_connect_attr::<Wide>(
        connection_handle,
        attribute,
        value,
        string_length,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Dbc,
        connection_handle,
        &warnings,
    );
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetConnectAttr(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    if connection_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result = api::connection::get_connect_attr::<Narrow>(
        connection_handle,
        attribute,
        value,
        buffer_length,
        string_length_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Dbc,
        connection_handle,
        &warnings,
    );
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetConnectAttrW(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    if connection_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result = api::connection::get_connect_attr::<Wide>(
        connection_handle,
        attribute,
        value,
        buffer_length,
        string_length_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Dbc,
        connection_handle,
        &warnings,
    );
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLDriverConnect(
    connection_handle: sql::Handle,
    _window_handle: sql::Handle,
    in_connection_string: *const sql::Char,
    in_string_length: sql::SmallInt,
    _out_connection_string: *mut sql::Char,
    _out_string_length: *mut sql::SmallInt,
    _driver_completion: sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result = api::connection::driver_connect::<Narrow>(
        connection_handle,
        in_connection_string,
        in_string_length,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLDriverConnectW(
    connection_handle: sql::Handle,
    _window_handle: sql::Handle,
    in_connection_string: *const sql::WChar,
    in_string_length: sql::SmallInt,
    _out_connection_string: *mut sql::WChar,
    _out_string_length: *mut sql::SmallInt,
    _driver_completion: sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result = api::connection::driver_connect::<Wide>(
        connection_handle,
        in_connection_string,
        in_string_length,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLDisconnect(connection_handle: sql::Handle) -> sql::RetCode {
    api::connection::disconnect(connection_handle).to_sql_code()
}
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLFetch(statement_handle: sql::Handle) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::data::fetch(statement_handle, &mut warnings);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLFetchScroll(
    statement_handle: sql::Handle,
    fetch_orientation: sql::SmallInt,
    _fetch_offset: sql::Len,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::data::fetch_scroll(statement_handle, fetch_orientation, &mut warnings);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLExtendedFetch(
    statement_handle: sql::Handle,
    fetch_orientation: sql::SmallInt,
    fetch_offset: sql::Len,
    row_count_ptr: *mut sql::ULen,
    row_status_ptr: *mut sql::USmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::data::extended_fetch(
        statement_handle,
        fetch_orientation,
        fetch_offset,
        row_count_ptr,
        row_status_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetData(
    statement_handle: sql::Handle,
    col_or_param_num: sql::USmallInt,
    target_type: CDataType,
    target_value_ptr: sql::Pointer,
    buffer_length: sql::Len,
    str_len_or_ind_ptr: *mut sql::Len,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::data::get_data(
        statement_handle,
        col_or_param_num,
        target_type,
        target_value_ptr,
        buffer_length,
        str_len_or_ind_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLColAttribute(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    field_identifier: sql::USmallInt,
    character_attribute_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    numeric_attribute_ptr: *mut sql::Len,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::utils::col_attribute::<Narrow>(
        statement_handle,
        column_number,
        field_identifier,
        character_attribute_ptr as *mut sql::Char,
        buffer_length,
        string_length_ptr,
        numeric_attribute_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLColAttributeW(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    field_identifier: sql::USmallInt,
    character_attribute_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    numeric_attribute_ptr: *mut sql::Len,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::utils::col_attribute::<Wide>(
        statement_handle,
        column_number,
        field_identifier,
        character_attribute_ptr as *mut sql::WChar,
        buffer_length,
        string_length_ptr,
        numeric_attribute_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLDescribeCol(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    column_name: *mut sql::Char,
    buffer_length: sql::SmallInt,
    name_length_ptr: *mut sql::SmallInt,
    data_type_ptr: *mut sql::SmallInt,
    column_size_ptr: *mut sql::ULen,
    decimal_digits_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::utils::describe_col::<Narrow>(
        statement_handle,
        column_number,
        column_name,
        buffer_length,
        name_length_ptr,
        data_type_ptr,
        column_size_ptr,
        decimal_digits_ptr,
        nullable_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLDescribeColW(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    column_name: *mut sql::WChar,
    buffer_length: sql::SmallInt,
    name_length_ptr: *mut sql::SmallInt,
    data_type_ptr: *mut sql::SmallInt,
    column_size_ptr: *mut sql::ULen,
    decimal_digits_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::utils::describe_col::<Wide>(
        statement_handle,
        column_number,
        column_name,
        buffer_length,
        name_length_ptr,
        data_type_ptr,
        column_size_ptr,
        decimal_digits_ptr,
        nullable_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLNumResultCols(
    statement_handle: sql::Handle,
    column_count_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::utils::num_result_cols(statement_handle, column_count_ptr);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLNumParams(
    statement_handle: sql::Handle,
    param_count_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::num_params(statement_handle, param_count_ptr);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLDescribeParam(
    statement_handle: sql::Handle,
    parameter_number: sql::USmallInt,
    data_type_ptr: *mut sql::SmallInt,
    parameter_size_ptr: *mut sql::ULen,
    decimal_digits_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::describe_param(
        statement_handle,
        parameter_number,
        data_type_ptr,
        parameter_size_ptr,
        decimal_digits_ptr,
        nullable_ptr,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLRowCount(
    statement_handle: sql::Handle,
    row_count_ptr: *mut sql::Len,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::utils::row_count(statement_handle, row_count_ptr);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLBindParameter(
    statement_handle: sql::Handle,
    parameter_number: sql::USmallInt,
    input_output_type: sql::SmallInt,
    value_type: sql::SmallInt,
    parameter_type: sql::SmallInt,
    column_size: sql::ULen,
    decimal_digits: sql::SmallInt,
    parameter_value_ptr: sql::Pointer,
    buffer_length: sql::Len,
    str_len_or_ind_ptr: *mut sql::Len,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::bind_parameter(
        statement_handle,
        parameter_number,
        input_output_type,
        value_type,
        parameter_type,
        column_size,
        decimal_digits,
        parameter_value_ptr,
        buffer_length,
        str_len_or_ind_ptr,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLPrepare(
    statement_handle: sql::Handle,
    statement_text: *const sql::Char,
    text_length: sql::Integer,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::prepare::<Narrow>(statement_handle, statement_text, text_length);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLPrepareW(
    statement_handle: sql::Handle,
    statement_text: *const sql::WChar,
    text_length: sql::Integer,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::prepare::<Wide>(statement_handle, statement_text, text_length);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLExecute(statement_handle: sql::Handle) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::execute(statement_handle);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetDiagRec(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    rec_number: sql::SmallInt,
    sql_state: *mut sql::Char,
    native_error_ptr: *mut sql::Integer,
    message_text: *mut sql::Char,
    buffer_length: sql::SmallInt,
    text_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    let mut warnings = vec![];
    let result = unsafe {
        api::diagnostic::get_diag_rec::<Narrow>(
            handle_type,
            handle,
            rec_number,
            sql_state,
            native_error_ptr,
            message_text,
            buffer_length,
            text_length_ptr,
            &mut warnings,
        )
    };
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetDiagRecW(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    rec_number: sql::SmallInt,
    sql_state: *mut sql::WChar,
    native_error_ptr: *mut sql::Integer,
    message_text: *mut sql::WChar,
    buffer_length: sql::SmallInt,
    text_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    let mut warnings = vec![];
    let result = unsafe {
        api::diagnostic::get_diag_rec::<Wide>(
            handle_type,
            handle,
            rec_number,
            sql_state,
            native_error_ptr,
            message_text,
            buffer_length,
            text_length_ptr,
            &mut warnings,
        )
    };
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetDiagField(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    rec_number: sql::SmallInt,
    diag_identifier: sql::SmallInt,
    diag_info_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::get_diag_field::<Narrow>(
        handle_type,
        handle,
        rec_number,
        diag_identifier,
        diag_info_ptr,
        buffer_length,
        string_length_ptr,
    )
    .to_sql_code()
}
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetDiagFieldW(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    rec_number: sql::SmallInt,
    diag_identifier: sql::SmallInt,
    diag_info_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::get_diag_field::<Wide>(
        handle_type,
        handle,
        rec_number,
        diag_identifier,
        diag_info_ptr,
        buffer_length,
        string_length_ptr,
    )
    .to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLBindCol(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    target_type: CDataType,
    target_value_ptr: sql::Pointer,
    buffer_length: sql::Len,
    str_len_or_ind_ptr: *mut sql::Len,
) -> sql::RetCode {
    api::statement::bind_col(
        statement_handle,
        column_number,
        target_type,
        target_value_ptr,
        buffer_length,
        str_len_or_ind_ptr,
    )
    .to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLSetStmtAttr(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    string_length: sql::Integer,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::statement::set_stmt_attr(
        statement_handle,
        attribute,
        value_ptr,
        string_length,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLSetStmtAttrW(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    string_length: sql::Integer,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::statement::set_stmt_attr(
        statement_handle,
        attribute,
        value_ptr,
        string_length,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetStmtAttr(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::statement::get_stmt_attr::<Narrow>(
        statement_handle,
        attribute,
        value_ptr,
        buffer_length,
        string_length_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetStmtAttrW(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::statement::get_stmt_attr::<Wide>(
        statement_handle,
        attribute,
        value_ptr,
        buffer_length,
        string_length_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Stmt,
        statement_handle,
        &warnings,
    );
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLMoreResults(statement_handle: sql::Handle) -> sql::RetCode {
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::more_results(statement_handle);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLNativeSql(
    connection_handle: sql::Handle,
    in_statement_text: *const sql::Char,
    text_length1: sql::Integer,
    out_statement_text: *mut sql::Char,
    buffer_length: sql::Integer,
    text_length2_ptr: *mut sql::Integer,
) -> sql::RetCode {
    if connection_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result = api::connection::native_sql::<Narrow>(
        connection_handle,
        in_statement_text,
        text_length1,
        out_statement_text,
        buffer_length,
        text_length2_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Dbc,
        connection_handle,
        &warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLNativeSqlW(
    connection_handle: sql::Handle,
    in_statement_text: *const sql::WChar,
    text_length1: sql::Integer,
    out_statement_text: *mut sql::WChar,
    buffer_length: sql::Integer,
    text_length2_ptr: *mut sql::Integer,
) -> sql::RetCode {
    if connection_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result = api::connection::native_sql::<Wide>(
        connection_handle,
        in_statement_text,
        text_length1,
        out_statement_text,
        buffer_length,
        text_length2_ptr,
        &mut warnings,
    );
    api::diagnostic::set_diag_info_from_warnings(
        sql::HandleType::Dbc,
        connection_handle,
        &warnings,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    result.to_sql_code_with_warnings(&warnings)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetDescField(
    descriptor_handle: sql::Handle,
    rec_number: sql::SmallInt,
    field_identifier: sql::SmallInt,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    api::descriptor::get_desc_field(
        descriptor_handle,
        rec_number,
        field_identifier,
        value_ptr,
        buffer_length,
        string_length_ptr,
    )
    .to_sql_code()
}
/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetDescFieldW(
    descriptor_handle: sql::Handle,
    rec_number: sql::SmallInt,
    field_identifier: sql::SmallInt,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    api::descriptor::get_desc_field(
        descriptor_handle,
        rec_number,
        field_identifier,
        value_ptr,
        buffer_length,
        string_length_ptr,
    )
    .to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLSetDescField(
    descriptor_handle: sql::Handle,
    rec_number: sql::SmallInt,
    field_identifier: sql::SmallInt,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
) -> sql::RetCode {
    api::descriptor::set_desc_field(
        descriptor_handle,
        rec_number,
        field_identifier,
        value_ptr,
        buffer_length,
    )
    .to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLSetDescFieldW(
    descriptor_handle: sql::Handle,
    rec_number: sql::SmallInt,
    field_identifier: sql::SmallInt,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
) -> sql::RetCode {
    api::descriptor::set_desc_field(
        descriptor_handle,
        rec_number,
        field_identifier,
        value_ptr,
        buffer_length,
    )
    .to_sql_code()
}

// ============================================================================
// DllMain — capture module handle for dialog resources
// ============================================================================

#[cfg(target_os = "windows")]
pub(crate) static DLL_HINSTANCE: std::sync::atomic::AtomicPtr<core::ffi::c_void> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(
    h_instance: *mut core::ffi::c_void,
    reason: u32,
    _reserved: *mut core::ffi::c_void,
) -> i32 {
    const DLL_PROCESS_ATTACH: u32 = 1;
    if reason == DLL_PROCESS_ATTACH {
        DLL_HINSTANCE.store(h_instance, std::sync::atomic::Ordering::Relaxed);
    }
    1 // TRUE
}

// ============================================================================
// Setup DLL API — ConfigDriver / ConfigDSN
//
// These functions are called by the ODBC Installer DLL, not the
// Driver Manager. They allow the ODBC Administrator UI to add, modify, and
// remove DSNs for this driver.
// ============================================================================

#[cfg(target_os = "windows")]
mod setup {
    use std::ptr;

    #[link(
        name = "odbccp32",
        kind = "raw-dylib",
        import_name_type = "undecorated"
    )]
    unsafe extern "system" {
        fn SQLWriteDSNToIniW(lpszDSN: *const u16, lpszDriver: *const u16) -> i32;
        fn SQLRemoveDSNFromIniW(lpszDSN: *const u16) -> i32;
        fn SQLWritePrivateProfileStringW(
            lpszSection: *const u16,
            lpszEntry: *const u16,
            lpszString: *const u16,
            lpszFilename: *const u16,
        ) -> i32;
    }

    const ODBC_ADD_DSN: u16 = 1;
    const ODBC_CONFIG_DSN: u16 = 2;
    const ODBC_REMOVE_DSN: u16 = 3;

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Parse a double-null-terminated wide attribute string into key-value pairs.
    unsafe fn parse_attributes_w(attrs: *const u16) -> Vec<(String, String)> {
        let mut result = Vec::new();
        if attrs.is_null() {
            return result;
        }
        let mut p = attrs;
        loop {
            if unsafe { *p } == 0 {
                break;
            }
            let start = p;
            let mut len = 0usize;
            while unsafe { *p } != 0 {
                len += 1;
                p = unsafe { p.add(1) };
            }
            let slice = unsafe { std::slice::from_raw_parts(start, len) };
            let s = String::from_utf16_lossy(slice);
            if let Some((k, v)) = s.split_once('=') {
                result.push((k.to_string(), v.to_string()));
            }
            p = unsafe { p.add(1) };
        }
        result
    }

    unsafe fn parse_attributes_a(attrs: *const u8) -> Vec<(String, String)> {
        let mut result = Vec::new();
        if attrs.is_null() {
            return result;
        }
        let mut p = attrs;
        loop {
            if unsafe { *p } == 0 {
                break;
            }
            let start = p;
            let mut len = 0usize;
            while unsafe { *p } != 0 {
                len += 1;
                p = unsafe { p.add(1) };
            }
            let slice = unsafe { std::slice::from_raw_parts(start, len) };
            let s = String::from_utf8_lossy(slice).into_owned();
            if let Some((k, v)) = s.split_once('=') {
                result.push((k.to_string(), v.to_string()));
            }
            p = unsafe { p.add(1) };
        }
        result
    }

    fn find_dsn(attrs: &[(String, String)]) -> Option<&str> {
        attrs
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("DSN"))
            .map(|(_, v)| v.as_str())
    }

    unsafe fn write_dsn_silent(dsn: &str, driver: &str, attrs: &[(String, String)]) -> bool {
        let dsn_w = to_wide(dsn);
        let driver_w = to_wide(driver);
        if unsafe { SQLWriteDSNToIniW(dsn_w.as_ptr(), driver_w.as_ptr()) } == 0 {
            return false;
        }
        let odbc_ini = to_wide("odbc.ini");
        for (key, value) in attrs {
            if key.eq_ignore_ascii_case("DSN") || key.eq_ignore_ascii_case("PWD") {
                continue;
            }
            let key_w = to_wide(key);
            let val_w = to_wide(value);
            unsafe {
                SQLWritePrivateProfileStringW(
                    dsn_w.as_ptr(),
                    key_w.as_ptr(),
                    val_w.as_ptr(),
                    odbc_ini.as_ptr(),
                );
            }
        }
        true
    }

    unsafe fn config_dsn_impl(
        hwnd_parent: *mut core::ffi::c_void,
        f_request: u16,
        driver: &str,
        attrs: &[(String, String)],
    ) -> bool {
        match f_request {
            ODBC_REMOVE_DSN => {
                let Some(dsn) = find_dsn(attrs) else {
                    return false;
                };
                let dsn_w = to_wide(dsn);
                unsafe { SQLRemoveDSNFromIniW(dsn_w.as_ptr()) != 0 }
            }
            ODBC_ADD_DSN | ODBC_CONFIG_DSN => {
                let dsn = find_dsn(attrs).unwrap_or("");
                let is_add = f_request == ODBC_ADD_DSN;

                // Show dialog when a parent window is provided (ODBC Administrator).
                // Fall back to silent mode for programmatic DSN creation (null hwnd).
                if !hwnd_parent.is_null() {
                    unsafe {
                        crate::setup_dialog::show_config_dialog(
                            hwnd_parent,
                            is_add,
                            driver,
                            dsn,
                            attrs,
                        )
                    }
                } else if !dsn.is_empty() {
                    unsafe { write_dsn_silent(dsn, driver, attrs) }
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// # Safety
    /// Called by the ODBC Installer DLL (Unicode variant).
    #[unsafe(no_mangle)]
    pub unsafe extern "system" fn ConfigDSNW(
        hwnd_parent: *mut core::ffi::c_void,
        f_request: u16,
        lpsz_driver: *const u16,
        lpsz_attributes: *const u16,
    ) -> i32 {
        let driver = if lpsz_driver.is_null() {
            String::new()
        } else {
            let mut len = 0;
            let mut p = lpsz_driver;
            while unsafe { *p } != 0 {
                len += 1;
                p = unsafe { p.add(1) };
            }
            String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(lpsz_driver, len) })
        };
        let attrs = unsafe { parse_attributes_w(lpsz_attributes) };
        i32::from(unsafe { config_dsn_impl(hwnd_parent, f_request, &driver, &attrs) })
    }

    /// # Safety
    /// Called by the ODBC Installer DLL (ANSI variant).
    #[unsafe(no_mangle)]
    pub unsafe extern "system" fn ConfigDSN(
        hwnd_parent: *mut core::ffi::c_void,
        f_request: u16,
        lpsz_driver: *const u8,
        lpsz_attributes: *const u8,
    ) -> i32 {
        let driver = if lpsz_driver.is_null() {
            String::new()
        } else {
            let mut len = 0;
            let mut p = lpsz_driver;
            while unsafe { *p } != 0 {
                len += 1;
                p = unsafe { p.add(1) };
            }
            String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(lpsz_driver, len) })
                .into_owned()
        };
        let attrs = unsafe { parse_attributes_a(lpsz_attributes) };
        i32::from(unsafe { config_dsn_impl(hwnd_parent, f_request, &driver, &attrs) })
    }

    /// # Safety
    /// Called by the ODBC Installer DLL for driver-level install/remove hooks.
    #[unsafe(no_mangle)]
    pub unsafe extern "system" fn ConfigDriver(
        _hwnd_parent: *mut core::ffi::c_void,
        _f_request: u16,
        _lpsz_driver: *const u8,
        _lpsz_args: *const u8,
        _lpsz_msg: *mut u8,
        _cb_msg_max: u16,
        _pcb_msg_out: *mut u16,
    ) -> i32 {
        if !_pcb_msg_out.is_null() {
            unsafe { ptr::write(_pcb_msg_out, 0) };
        }
        1 // TRUE
    }
}
