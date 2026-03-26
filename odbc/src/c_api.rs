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
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result = api::connection::set_connect_attr::<Narrow>(
        connection_handle,
        attribute,
        value,
        string_length,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    result.to_sql_code()
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
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result = api::connection::set_connect_attr::<Wide>(
        connection_handle,
        attribute,
        value,
        string_length,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    result.to_sql_code()
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
pub unsafe extern "C" fn SQLGetConnectAttrW(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
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
pub unsafe extern "C" fn SQLFetchScroll(
    statement_handle: sql::Handle,
    fetch_orientation: sql::SmallInt,
    _fetch_offset: sql::Len,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let mut warnings = vec![];
    let result = api::data::fetch_scroll(statement_handle, fetch_orientation, &mut warnings);
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
    let result = api::utils::col_attribute(
        statement_handle,
        column_number,
        field_identifier,
        character_attribute_ptr,
        buffer_length,
        string_length_ptr,
        numeric_attribute_ptr,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
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
    let result = api::utils::col_attribute(
        statement_handle,
        column_number,
        field_identifier,
        character_attribute_ptr,
        buffer_length,
        string_length_ptr,
        numeric_attribute_ptr,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
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
pub unsafe extern "C" fn SQLMoreResults(statement_handle: sql::Handle) -> sql::RetCode {
    if statement_handle.is_null() {
        return sql::SqlReturn::INVALID_HANDLE.0;
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    // TODO: Implement proper SQLMoreResults functionality (multiple result sets).
    // For now, close the cursor as if SQLFreeStmt(SQL_CLOSE) was called, per ODBC spec.
    let result = api::statement::free_stmt(statement_handle, api::FreeStmtOption::Close);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    if result.is_err() {
        return result.to_sql_code();
    }
    sql::SqlReturn::NO_DATA.0
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
