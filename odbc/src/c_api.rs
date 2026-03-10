//! ODBC C API functions
//!
//! This module provides the C API interface for ODBC functions.
//! All string encoding/decoding happens here at the FFI boundary:
//! - Input strings: decoded from locale encoding (ANSI) or UTF-16 (W)
//! - Output strings: encoded to locale encoding (ANSI) or UTF-16 (W)
//! - The api module works exclusively with Rust `String` / `&str`.

#![allow(non_snake_case)]

use crate::api::diagnostic::DiagRecData;
use crate::api::error::EncodingSnafu;
use crate::api::{
    self, ColAttributeResult, ConnectAttrValue, DiagFieldValue, InfoValue, ToSqlReturn,
};
use crate::cdata_types::CDataType;
use crate::conversion::warning::Warning;
use odbc_sys as sql;
use snafu::ResultExt;

/// Decode an optional narrow C string: NULL pointers yield `None`,
/// since ODBC allows NULL for optional string parameters.
fn decode_optional_char(ptr: *const u8, length: i32) -> api::OdbcResult<Option<String>> {
    if ptr.is_null() {
        return Ok(None);
    }
    unsafe { crate::encoding::decode_char(ptr, length) }
        .context(EncodingSnafu)
        .map(Some)
}

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
    let result = unsafe { crate::encoding::decode_char(statement_text, text_length) }
        .context(EncodingSnafu)
        .and_then(|query| api::statement::exec_direct(statement_handle, &query));
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
    let result = unsafe { crate::encoding::decode_wchar(statement_text, text_length) }
        .context(EncodingSnafu)
        .and_then(|query| api::statement::exec_direct(statement_handle, &query));
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
    option: sql::FreeStmtOption,
) -> sql::RetCode {
    api::statement::free_stmt(statement_handle, option).to_sql_code()
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
    let result: api::OdbcResult<()> = (|| {
        let server = unsafe { crate::encoding::decode_char(server_name, name_length1 as i32) }
            .context(EncodingSnafu)?;
        let user = decode_optional_char(user_name, name_length2 as i32)?;
        let auth = decode_optional_char(authentication, name_length3 as i32)?;
        api::connection::connect(connection_handle, &server, user.as_deref(), auth.as_deref())
    })();
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
pub unsafe extern "C" fn SQLGetInfo(
    connection_handle: sql::Handle,
    info_type: sql::USmallInt,
    info_value_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result: api::OdbcResult<()> = (|| {
        let info = api::connection::get_info(connection_handle, info_type)?;
        match info {
            InfoValue::USmallInt(val) => unsafe {
                if !info_value_ptr.is_null() {
                    *(info_value_ptr as *mut u16) = val;
                }
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<u16>() as sql::SmallInt;
                }
            },
            InfoValue::UInteger(val) => unsafe {
                if !info_value_ptr.is_null() {
                    *(info_value_ptr as *mut u32) = val;
                }
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<u32>() as sql::SmallInt;
                }
            },
            InfoValue::String(s) => {
                let mut full_len: i32 = 0;
                unsafe {
                    crate::encoding::write_char_to_buffer(
                        &s,
                        info_value_ptr as *mut u8,
                        buffer_length as i32,
                        &mut full_len,
                    )
                }
                .context(EncodingSnafu)?;
                if !string_length_ptr.is_null() {
                    unsafe { *string_length_ptr = full_len as sql::SmallInt };
                }
            }
        }
        Ok(())
    })();
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
    let result: api::OdbcResult<()> = (|| {
        let string_value = if let Some(attr) = api::ConnectionAttribute::from_raw(attribute)
            && attr.is_string_type()
        {
            if value.is_null() {
                Some(String::new())
            } else {
                Some(
                    unsafe { crate::encoding::decode_char(value as *const u8, string_length) }
                        .context(EncodingSnafu)?,
                )
            }
        } else {
            None
        };
        api::connection::set_connect_attr(
            connection_handle,
            attribute,
            value,
            string_value.as_deref(),
        )
    })();
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
    let result: api::OdbcResult<()> = (|| {
        let attr_value = api::connection::get_connect_attr(connection_handle, attribute)?;
        match attr_value {
            ConnectAttrValue::ULen(val) => unsafe {
                if !value.is_null() {
                    *(value as *mut sql::ULen) = val;
                }
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<sql::ULen>() as sql::Integer;
                }
            },
            ConnectAttrValue::String(s) => {
                let truncated = unsafe {
                    crate::encoding::write_char_to_buffer(
                        &s,
                        value as *mut u8,
                        buffer_length,
                        string_length_ptr,
                    )
                }
                .context(EncodingSnafu)?;
                if truncated {
                    warnings.push(Warning::StringDataTruncated);
                }
            }
        }
        Ok(())
    })();
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
    let result =
        unsafe { crate::encoding::decode_char(in_connection_string, in_string_length as i32) }
            .context(EncodingSnafu)
            .and_then(|cs| api::connection::driver_connect(connection_handle, &cs));
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
    let result: api::OdbcResult<()> = (|| {
        let attr_result =
            api::utils::col_attribute(statement_handle, column_number, field_identifier)?;
        match attr_result {
            ColAttributeResult::Numeric(val) => {
                if !numeric_attribute_ptr.is_null() {
                    unsafe { std::ptr::write(numeric_attribute_ptr, val) };
                }
            }
            ColAttributeResult::String(s) => {
                let mut full_len: i32 = 0;
                unsafe {
                    crate::encoding::write_char_to_buffer(
                        &s,
                        character_attribute_ptr as *mut u8,
                        buffer_length as i32,
                        &mut full_len,
                    )
                }
                .context(EncodingSnafu)?;
                if !string_length_ptr.is_null() {
                    unsafe { *string_length_ptr = full_len as sql::SmallInt };
                }
            }
        }
        Ok(())
    })();
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
    let result: api::OdbcResult<()> = (|| {
        let name = api::utils::describe_col(
            statement_handle,
            column_number,
            data_type_ptr,
            column_size_ptr,
            decimal_digits_ptr,
            nullable_ptr,
        )?;

        let mut full_len: i32 = 0;
        let truncated = unsafe {
            crate::encoding::write_char_to_buffer(
                &name,
                column_name,
                buffer_length as i32,
                &mut full_len,
            )
        }
        .context(EncodingSnafu)?;
        if !name_length_ptr.is_null() {
            unsafe { *name_length_ptr = full_len as sql::SmallInt };
        }
        if truncated {
            warnings.push(Warning::StringDataTruncated);
        }
        Ok(())
    })();
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
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = unsafe { crate::encoding::decode_char(statement_text, text_length) }
        .context(EncodingSnafu)
        .and_then(|query| api::statement::prepare(statement_handle, &query));
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
    let result = unsafe { crate::encoding::decode_wchar(statement_text, text_length) }
        .context(EncodingSnafu)
        .and_then(|query| api::statement::prepare(statement_handle, &query));
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
    let result: api::OdbcResult<()> = (|| {
        let diag = api::diagnostic::get_diag_rec(handle_type, handle, rec_number)?;
        write_diag_rec_to_buffers(
            &diag,
            sql_state,
            native_error_ptr,
            message_text,
            buffer_length,
            text_length_ptr,
            &mut warnings,
        )
    })();
    result.to_sql_code_with_warnings(&warnings)
}

/// Write a `DiagRecData` to the output buffers provided by the caller.
fn write_diag_rec_to_buffers(
    diag: &DiagRecData,
    sql_state: *mut sql::Char,
    native_error_ptr: *mut sql::Integer,
    message_text: *mut sql::Char,
    buffer_length: sql::SmallInt,
    text_length_ptr: *mut sql::SmallInt,
    warnings: &mut Vec<Warning>,
) -> api::OdbcResult<()> {
    if !sql_state.is_null() {
        let state_str = diag.sql_state.as_str();
        let state_bytes = state_str.as_bytes();
        let len = std::cmp::min(state_bytes.len(), 5);
        unsafe {
            std::ptr::copy_nonoverlapping(state_bytes.as_ptr(), sql_state, len);
            *sql_state.add(len) = 0;
        }
    }
    if !native_error_ptr.is_null() {
        unsafe { std::ptr::write(native_error_ptr, diag.native_error) };
    }

    let mut full_len: i32 = 0;
    let truncated = unsafe {
        crate::encoding::write_char_to_buffer(
            &diag.message_text,
            message_text,
            buffer_length as i32,
            &mut full_len,
        )
    }
    .context(EncodingSnafu)?;
    if !text_length_ptr.is_null() {
        unsafe { *text_length_ptr = full_len as sql::SmallInt };
    }
    if truncated {
        warnings.push(Warning::StringDataTruncated);
    }
    Ok(())
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
    let result: api::OdbcResult<()> = (|| {
        let field_value =
            api::diagnostic::get_diag_field(handle_type, handle, rec_number, diag_identifier)?;
        match field_value {
            DiagFieldValue::Integer(val) => unsafe {
                std::ptr::write(diag_info_ptr as *mut sql::Integer, val);
            },
            DiagFieldValue::Len(val) => unsafe {
                std::ptr::write(diag_info_ptr as *mut sql::Len, val);
            },
            DiagFieldValue::RetCode(val) => unsafe {
                std::ptr::write(diag_info_ptr as *mut sql::RetCode, val);
            },
            DiagFieldValue::String(s) => {
                let mut full_len: i32 = 0;
                unsafe {
                    crate::encoding::write_char_to_buffer(
                        &s,
                        diag_info_ptr as *mut u8,
                        buffer_length as i32,
                        &mut full_len,
                    )
                }
                .context(EncodingSnafu)?;
                if !string_length_ptr.is_null() {
                    unsafe { *string_length_ptr = full_len as sql::SmallInt };
                }
            }
        }
        Ok(())
    })();
    result.to_sql_code()
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
pub unsafe extern "C" fn SQLGetStmtAttr(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    api::statement::get_stmt_attr(
        statement_handle,
        attribute,
        value_ptr,
        buffer_length,
        string_length_ptr,
    )
    .to_sql_code()
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
