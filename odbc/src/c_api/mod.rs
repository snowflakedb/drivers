//! ODBC C API functions
//!
//! This module provides the C API interface for ODBC functions.
//! All string encoding/decoding happens here at the FFI boundary:
//! - Input strings: decoded from locale encoding (ANSI) or UTF-16 (W)
//! - Output strings: encoded to locale encoding (ANSI) or UTF-16 (W)
//! - The api module works exclusively with Rust `String` / `&str`.

#![allow(non_snake_case)]

mod write;

use crate::api::error::EncodingSnafu;
use crate::api::{self, FieldValue, ToSqlReturn};
use crate::cdata_types::CDataType;
use odbc_sys as sql;
use snafu::ResultExt;

#[cfg(not(windows))]
use write::{write_char_to_buffer, write_diag_rec_to_buffers, write_field_value};
use write::{write_diag_rec_to_buffers_w, write_field_value_w, write_wchar_to_buffer};

#[cfg(not(windows))]
fn decode_optional_char(ptr: *const u8, length: i32) -> api::OdbcResult<Option<String>> {
    if ptr.is_null() {
        return Ok(None);
    }
    unsafe { crate::encoding::decode_char(ptr, length) }
        .context(EncodingSnafu)
        .map(Some)
}

fn decode_optional_wchar(ptr: *const u16, length: i32) -> api::OdbcResult<Option<String>> {
    if ptr.is_null() {
        return Ok(None);
    }
    unsafe { crate::encoding::decode_wchar(ptr, length) }
        .context(EncodingSnafu)
        .map(Some)
}

#[cfg(not(windows))]
fn parse_connect_attr_value(
    attr: api::ConnectionAttribute,
    value_ptr: sql::Pointer,
    string_length: sql::Integer,
) -> api::OdbcResult<api::AttributeValue> {
    match attr.attribute_type() {
        api::AttributeType::String => {
            if value_ptr.is_null() {
                return Ok(api::AttributeValue::String(String::new()));
            }
            let s = unsafe { crate::encoding::decode_char(value_ptr as *const u8, string_length) }
                .context(EncodingSnafu)?;
            Ok(api::AttributeValue::String(s))
        }
        api::AttributeType::Int => Ok(api::AttributeValue::Int(value_ptr as usize)),
        api::AttributeType::None => Ok(api::AttributeValue::None),
    }
}

fn parse_connect_attr_value_w(
    attr: api::ConnectionAttribute,
    value_ptr: sql::Pointer,
    string_length: sql::Integer,
) -> api::OdbcResult<api::AttributeValue> {
    match attr.attribute_type() {
        api::AttributeType::String => {
            if value_ptr.is_null() {
                return Ok(api::AttributeValue::String(String::new()));
            }
            let s =
                unsafe { crate::encoding::decode_wchar(value_ptr as *const u16, string_length) }
                    .context(EncodingSnafu)?;
            Ok(api::AttributeValue::String(s))
        }
        api::AttributeType::Int => Ok(api::AttributeValue::Int(value_ptr as usize)),
        api::AttributeType::None => Ok(api::AttributeValue::None),
    }
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
#[cfg(not(windows))]
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
#[cfg(not(windows))]
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
pub unsafe extern "C" fn SQLConnectW(
    connection_handle: sql::Handle,
    server_name: *const sql::WChar,
    name_length1: sql::SmallInt,
    user_name: *const sql::WChar,
    name_length2: sql::SmallInt,
    authentication: *const sql::WChar,
    name_length3: sql::SmallInt,
) -> sql::RetCode {
    let result: api::OdbcResult<()> = (|| {
        let server = unsafe { crate::encoding::decode_wchar(server_name, name_length1 as i32) }
            .context(EncodingSnafu)?;
        let user = decode_optional_wchar(user_name, name_length2 as i32)?;
        let auth = decode_optional_wchar(authentication, name_length3 as i32)?;
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
#[cfg(not(windows))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetInfo(
    connection_handle: sql::Handle,
    info_type: sql::USmallInt,
    info_value_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result: api::OdbcResult<()> = (|| {
        let info = api::connection::get_info(connection_handle, info_type)?;
        unsafe {
            write_field_value(
                info,
                info_value_ptr,
                buffer_length,
                string_length_ptr,
                &mut warnings,
            )
        }
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
pub unsafe extern "C" fn SQLGetInfoW(
    connection_handle: sql::Handle,
    info_type: sql::USmallInt,
    info_value_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result: api::OdbcResult<()> = (|| {
        let info = api::connection::get_info(connection_handle, info_type)?;
        unsafe {
            write_field_value_w(
                info,
                info_value_ptr,
                buffer_length,
                string_length_ptr,
                &mut warnings,
            )
        }
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
#[cfg(not(windows))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLSetConnectAttr(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    string_length: sql::Integer,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result: api::OdbcResult<()> = (|| {
        let attr = match api::ConnectionAttribute::try_from(attribute) {
            Ok(a) => a,
            Err(_) if !api::ConnectionAttribute::is_snowflake_custom(attribute) => {
                tracing::debug!("SQLSetConnectAttr: ignoring standard attribute {attribute}");
                return Ok(());
            }
            Err(e) => return Err(e),
        };
        let attr_value = parse_connect_attr_value(attr, value, string_length)?;
        api::connection::set_connect_attr(connection_handle, attr, attr_value)
    })();
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
    let result: api::OdbcResult<()> = (|| {
        let attr = match api::ConnectionAttribute::try_from(attribute) {
            Ok(a) => a,
            Err(_) if !api::ConnectionAttribute::is_snowflake_custom(attribute) => {
                tracing::debug!("SQLSetConnectAttrW: ignoring standard attribute {attribute}");
                return Ok(());
            }
            Err(e) => return Err(e),
        };
        let attr_value = parse_connect_attr_value_w(attr, value, string_length)?;
        api::connection::set_connect_attr(connection_handle, attr, attr_value)
    })();
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[cfg(not(windows))]
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
        let attr = api::ConnectionAttribute::try_from(attribute)?;
        let attr_value = api::connection::get_connect_attr(connection_handle, attr)?;
        unsafe {
            write_field_value(
                attr_value.into(),
                value,
                buffer_length,
                string_length_ptr,
                &mut warnings,
            )
        }
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
pub unsafe extern "C" fn SQLGetConnectAttrW(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let mut warnings = vec![];
    let result: api::OdbcResult<()> = (|| {
        let attr = api::ConnectionAttribute::try_from(attribute)?;
        let attr_value = api::connection::get_connect_attr(connection_handle, attr)?;
        let buf_len = buffer_length as isize;
        let mut out_len: isize = 0;
        unsafe {
            write_field_value_w(
                attr_value.into(),
                value,
                buf_len,
                &mut out_len,
                &mut warnings,
            )
        }?;
        if !string_length_ptr.is_null() {
            unsafe { std::ptr::write(string_length_ptr, out_len as sql::Integer) };
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
#[cfg(not(windows))]
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
    let result =
        unsafe { crate::encoding::decode_wchar(in_connection_string, in_string_length as i32) }
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
#[cfg(not(windows))]
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
    let result: api::OdbcResult<()> = (|| {
        let attr_result =
            api::utils::col_attribute(statement_handle, column_number, field_identifier)?;
        match &attr_result {
            FieldValue::String(_) => unsafe {
                write_field_value(
                    attr_result,
                    character_attribute_ptr,
                    buffer_length,
                    string_length_ptr,
                    &mut warnings,
                )?;
            },
            _ => unsafe {
                write_field_value(
                    attr_result,
                    numeric_attribute_ptr as sql::Pointer,
                    0,
                    string_length_ptr,
                    &mut warnings,
                )?;
            },
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
    let result: api::OdbcResult<()> = (|| {
        let attr_result =
            api::utils::col_attribute(statement_handle, column_number, field_identifier)?;
        match &attr_result {
            FieldValue::String(_) => unsafe {
                write_field_value_w(
                    attr_result,
                    character_attribute_ptr,
                    buffer_length,
                    string_length_ptr,
                    &mut warnings,
                )?;
            },
            _ => unsafe {
                write_field_value_w(
                    attr_result,
                    numeric_attribute_ptr as sql::Pointer,
                    0i16,
                    string_length_ptr,
                    &mut warnings,
                )?;
            },
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
#[cfg(not(windows))]
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

        unsafe {
            write_char_to_buffer(
                &name,
                column_name,
                buffer_length,
                name_length_ptr,
                &mut warnings,
            )
        }
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
    let result: api::OdbcResult<()> = (|| {
        let name = api::utils::describe_col(
            statement_handle,
            column_number,
            data_type_ptr,
            column_size_ptr,
            decimal_digits_ptr,
            nullable_ptr,
        )?;

        unsafe {
            write_wchar_to_buffer(
                &name,
                column_name,
                buffer_length,
                name_length_ptr,
                &mut warnings,
            )
        }
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
#[cfg(not(windows))]
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
#[cfg(not(windows))]
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
    let result: api::OdbcResult<()> = (|| {
        let diag = api::diagnostic::get_diag_rec(handle_type, handle, rec_number)?;
        write_diag_rec_to_buffers_w(
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

/// # Safety
/// This function is called by the ODBC driver manager.
#[cfg(not(windows))]
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
    let mut warnings = vec![];
    let result: api::OdbcResult<()> = (|| {
        let field_value =
            api::diagnostic::get_diag_field(handle_type, handle, rec_number, diag_identifier)?;
        unsafe {
            write_field_value(
                field_value,
                diag_info_ptr,
                buffer_length,
                string_length_ptr,
                &mut warnings,
            )
        }
    })();
    result.to_sql_code_with_warnings(&warnings)
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
    let mut warnings = vec![];
    let result: api::OdbcResult<()> = (|| {
        let field_value =
            api::diagnostic::get_diag_field(handle_type, handle, rec_number, diag_identifier)?;
        unsafe {
            write_field_value_w(
                field_value,
                diag_info_ptr,
                buffer_length,
                string_length_ptr,
                &mut warnings,
            )
        }
    })();
    result.to_sql_code_with_warnings(&warnings)
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
