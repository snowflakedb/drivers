//! ODBC C API functions
//!
//! This module provides the C API interface for ODBC functions.

use crate::api::{self, ToSqlReturn};
use crate::cdata_types::CDataType;
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
    api::statement::exec_direct(statement_handle, statement_text, text_length).to_sql_code()
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
pub unsafe extern "C" fn SQLConnect(
    connection_handle: sql::Handle,
    server_name: *const sql::Char,
    name_length1: sql::SmallInt,
    user_name: *const sql::Char,
    name_length2: sql::SmallInt,
    authentication: *const sql::Char,
    name_length3: sql::SmallInt,
) -> sql::RetCode {
    let result = api::connection::connect(
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
    _string_length: sql::SmallInt,
) -> sql::RetCode {
    api::environment::set_env_attribute(environment_handle, attribute, value).to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetEnvAttr(
    environment_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    _string_length: sql::SmallInt,
) -> sql::RetCode {
    api::environment::get_env_attribute(environment_handle, attribute, value).to_sql_code()
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
    let result =
        api::connection::driver_connect(connection_handle, in_connection_string, in_string_length);
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
    api::data::fetch(statement_handle).to_sql_code()
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
    api::data::get_data(
        statement_handle,
        col_or_param_num,
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
pub unsafe extern "C" fn SQLNumResultCols(
    statement_handle: sql::Handle,
    column_count_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    api::utils::num_result_cols(statement_handle, column_count_ptr).to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLRowCount(
    statement_handle: sql::Handle,
    row_count_ptr: *mut sql::Len,
) -> sql::RetCode {
    api::utils::row_count(statement_handle, row_count_ptr).to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLBindParameter(
    statement_handle: sql::Handle,
    parameter_number: sql::USmallInt,
    input_output_type: sql::ParamType,
    value_type: CDataType,
    parameter_type: sql::SqlDataType,
    column_size: sql::ULen,
    decimal_digits: sql::SmallInt,
    parameter_value_ptr: sql::Pointer,
    buffer_length: sql::Len,
    str_len_or_ind_ptr: *mut sql::Len,
) -> sql::RetCode {
    api::statement::bind_parameter(
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
    )
    .to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLPrepare(
    statement_handle: sql::Handle,
    statement_text: *const sql::Char,
    text_length: sql::Integer,
) -> sql::RetCode {
    api::statement::prepare(statement_handle, statement_text, text_length).to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLExecute(statement_handle: sql::Handle) -> sql::RetCode {
    api::statement::execute(statement_handle).to_sql_code()
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
    unsafe {
        api::diagnostic::get_diag_rec(
            handle_type,
            handle,
            rec_number,
            sql_state,
            native_error_ptr,
            message_text,
            buffer_length,
            text_length_ptr,
        )
        .to_sql_code()
    }
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
    api::diagnostic::get_diag_field(
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
