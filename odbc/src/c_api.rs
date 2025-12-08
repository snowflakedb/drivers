//! ODBC C API functions
//!
//! This module provides the C API interface for ODBC functions.

use crate::api::{self, OdbcResult, ToSqlReturn};
use crate::cdata_types::CDataType;
use odbc_sys as sql;

const SQL_NTS_INTEGER: sql::Integer = -3;
const SQL_NTS_SMALLINT: sql::SmallInt = -3;

#[cfg(target_os = "macos")]
unsafe fn wide_ptr_to_string(ptr: *const sql::WChar, length: sql::Integer) -> String {
    if ptr.is_null() {
        return String::new();
    }

    let ptr32 = ptr as *const u32;
    let is_nts = length == SQL_NTS_INTEGER || length < 0;
    let explicit_len = if is_nts {
        usize::MAX
    } else {
        length.max(0) as usize
    };
    let mut result = String::new();
    let mut idx = 0usize;

    loop {
        if !is_nts && idx >= explicit_len {
            break;
        }
        let code_point = *ptr32.add(idx);
        if code_point == 0 {
            break;
        }
        if let Some(ch) = char::from_u32(code_point) {
            result.push(ch);
        }
        idx += 1;
        if idx > 1_000_000 {
            break;
        }
    }

    result
}

#[cfg(not(target_os = "macos"))]
unsafe fn wide_ptr_to_string(ptr: *const sql::WChar, length: sql::Integer) -> String {
    if ptr.is_null() {
        return String::new();
    }

    let ptr16 = ptr as *const u16;
    let is_nts = length == SQL_NTS_INTEGER || length < 0;
    let len_chars = if is_nts {
        let mut len = 0usize;
        loop {
            let code = *ptr16.add(len);
            if code == 0 {
                break;
            }
            len += 1;
            if len > 1_000_000 {
                break;
            }
        }
        len
    } else {
        length.max(0) as usize
    };

    let slice = std::slice::from_raw_parts(ptr16, len_chars);
    String::from_utf16_lossy(slice)
}

fn convert_wide_arg_to_cstring(
    ptr: *const sql::WChar,
    length: sql::SmallInt,
) -> Option<std::ffi::CString> {
    if ptr.is_null() {
        return None;
    }
    let string = unsafe { wide_ptr_to_string(ptr, length as sql::Integer) };
    std::ffi::CString::new(string).ok()
}

fn copy_str_to_sqlwchar(
    dest: *mut sql::WChar,
    capacity_chars: usize,
    value: &str,
) -> sql::SmallInt {
    if dest.is_null() || capacity_chars == 0 {
        return 0;
    }

    #[cfg(target_os = "macos")]
    {
        let dest32 = dest as *mut u32;
        let chars: Vec<u32> = value.chars().map(|c| c as u32).collect();
        let copy_len = chars.len().min(capacity_chars.saturating_sub(1));
        unsafe {
            if copy_len > 0 {
                std::ptr::copy_nonoverlapping(chars.as_ptr(), dest32, copy_len);
            }
            std::ptr::write(dest32.add(copy_len), 0);
        }
        copy_len as sql::SmallInt
    }

    #[cfg(not(target_os = "macos"))]
    {
        let dest16 = dest as *mut u16;
        let encoded: Vec<u16> = value.encode_utf16().collect();
        let copy_len = encoded.len().min(capacity_chars.saturating_sub(1));
        unsafe {
            if copy_len > 0 {
                std::ptr::copy_nonoverlapping(encoded.as_ptr(), dest16, copy_len);
            }
            std::ptr::write(dest16.add(copy_len), 0);
        }
        copy_len as sql::SmallInt
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
    eprintln!("SQLAllocHandle(handle_type={handle_type:?}, input_handle={input_handle:p})");
    let result = api::handle_allocation::sql_alloc_handle(handle_type, input_handle, output_handle);

    // Determine which handle should receive diagnostics.
    // On success diagnostics are associated with the newly created handle.
    // On failure diagnostics are associated with the parent handle (input_handle).
    let (diag_type, diag_handle) = match (handle_type, &result) {
        (sql::HandleType::Env, Ok(_)) => (sql::HandleType::Env, unsafe { *output_handle }),
        (sql::HandleType::Dbc, Ok(_)) => (sql::HandleType::Dbc, unsafe { *output_handle }),
        (sql::HandleType::Stmt, Ok(_)) => (sql::HandleType::Stmt, unsafe { *output_handle }),
        (sql::HandleType::Desc, Ok(_)) => (sql::HandleType::Desc, unsafe { *output_handle }),
        (sql::HandleType::Env, Err(_)) => (sql::HandleType::Env, input_handle),
        (sql::HandleType::Dbc, Err(_)) => (sql::HandleType::Env, input_handle),
        (sql::HandleType::Stmt, Err(_)) => (sql::HandleType::Dbc, input_handle),
        (sql::HandleType::Desc, Err(_)) => (sql::HandleType::Dbc, input_handle),
        (other, _) => (other, input_handle),
    };

    api::diagnostic::clear_diag_info(diag_type, diag_handle);
    api::diagnostic::set_diag_info_from_result(diag_type, diag_handle, &result);

    result.to_sql_code()
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
    let result = api::statement::exec_direct(statement_handle, statement_text, text_length);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager (wide-character version).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLExecDirectW(
    statement_handle: sql::Handle,
    statement_text: *const sql::WChar,
    text_length: sql::Integer,
) -> sql::RetCode {
    let sql_str = wide_ptr_to_string(statement_text, text_length);
    let narrow_str = std::ffi::CString::new(sql_str).unwrap_or_default();
    SQLExecDirect(
        statement_handle,
        narrow_str.as_ptr() as *const sql::Char,
        SQL_NTS_INTEGER,
    )
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
/// This function is called by the ODBC driver manager (wide-character version).
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
    fn wide_arg_to_cstring(
        ptr: *const sql::WChar,
        length: sql::SmallInt,
    ) -> Option<std::ffi::CString> {
        if ptr.is_null() {
            None
        } else {
            let string = unsafe { wide_ptr_to_string(ptr, length as sql::Integer) };
            Some(std::ffi::CString::new(string).unwrap_or_default())
        }
    }

    let server_cstr = wide_arg_to_cstring(server_name, name_length1);
    let user_cstr = wide_arg_to_cstring(user_name, name_length2);
    let auth_cstr = wide_arg_to_cstring(authentication, name_length3);

    let server_ptr = server_cstr
        .as_ref()
        .map(|s| s.as_ptr() as *const sql::Char)
        .unwrap_or(std::ptr::null());
    let user_ptr = user_cstr
        .as_ref()
        .map(|s| s.as_ptr() as *const sql::Char)
        .unwrap_or(std::ptr::null());
    let auth_ptr = auth_cstr
        .as_ref()
        .map(|s| s.as_ptr() as *const sql::Char)
        .unwrap_or(std::ptr::null());

    let server_len = if server_ptr.is_null() {
        0
    } else {
        SQL_NTS_SMALLINT
    };
    let user_len = if user_ptr.is_null() {
        0
    } else {
        SQL_NTS_SMALLINT
    };
    let auth_len = if auth_ptr.is_null() {
        0
    } else {
        SQL_NTS_SMALLINT
    };

    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result = api::connection::connect(
        connection_handle,
        server_ptr,
        server_len,
        user_ptr,
        user_len,
        auth_ptr,
        auth_len,
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
    if let Err(ref err) = result {
        eprintln!("SQLDriverConnect error: {err:?}");
    }
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager (wide-character version).
/// On macOS with unixODBC, SQLWCHAR is u16 (UTF-16), but the driver manager
/// may pass UTF-32 (wchar_t) data. We handle both cases.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLDriverConnectW(
    connection_handle: sql::Handle,
    _window_handle: sql::Handle,
    in_connection_string: *const sql::WChar,
    in_string_length: sql::SmallInt,
    _out_connection_string: *mut sql::WChar,
    _buffer_length: sql::SmallInt,
    _out_string_length: *mut sql::SmallInt,
    _driver_completion: sql::SmallInt,
) -> sql::RetCode {
    const SQL_NTS: sql::SmallInt = -3;

    // Try to detect if the data is UTF-16 or UTF-32
    // UTF-16 ASCII characters have the form 0x00XX where XX is the ASCII value
    // If we see 0x0000 after a non-null character, it's likely UTF-32

    // Read the raw bytes to determine the encoding
    let raw_ptr = in_connection_string as *const u8;
    let first_byte = *raw_ptr;
    let second_byte = *raw_ptr.add(1);
    let third_byte = *raw_ptr.add(2);
    let fourth_byte = *raw_ptr.add(3);

    // If the pattern looks like ASCII char followed by 3 null bytes, it's UTF-32
    let is_utf32 = first_byte != 0 && second_byte == 0 && third_byte == 0 && fourth_byte == 0;

    let conn_str = if is_utf32 {
        // Read as UTF-32 (wchar_t on macOS)
        let ptr32 = in_connection_string as *const u32;
        let mut chars = Vec::new();
        let mut i = 0;
        loop {
            let c = *ptr32.add(i);
            if c == 0 {
                break;
            }
            if c <= 0x10FFFF {
                if let Some(ch) = char::from_u32(c) {
                    chars.push(ch);
                }
            }
            i += 1;
            if i > 10000 {
                break;
            } // Safety limit
        }
        chars.into_iter().collect::<String>()
    } else {
        // Read as UTF-16
        if in_string_length == SQL_NTS {
            let mut len = 0;
            while *in_connection_string.add(len) != 0 {
                len += 1;
                if len > 10000 {
                    break;
                }
            }
            let slice = std::slice::from_raw_parts(in_connection_string, len);
            String::from_utf16_lossy(slice)
        } else if in_string_length > 0 {
            let slice = std::slice::from_raw_parts(in_connection_string, in_string_length as usize);
            String::from_utf16_lossy(slice)
        } else {
            String::new()
        }
    };

    // Call the connection function directly
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let narrow_str = std::ffi::CString::new(conn_str).unwrap_or_default();
    let result = api::connection::driver_connect(
        connection_handle,
        narrow_str.as_ptr() as *const sql::Char,
        SQL_NTS,
    );
    if let Err(ref err) = result {
        eprintln!("SQLDriverConnectW error: {err:?}");
    }
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLBrowseConnect(
    connection_handle: sql::Handle,
    in_connection_string: *const sql::Char,
    string_length1: sql::SmallInt,
    out_connection_string: *mut sql::Char,
    buffer_length: sql::SmallInt,
    string_length2_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result =
        api::connection::driver_connect(connection_handle, in_connection_string, string_length1);

    if result.is_ok() {
        if !out_connection_string.is_null() && buffer_length > 0 {
            unsafe {
                *out_connection_string = 0;
            }
        }
        if !string_length2_ptr.is_null() {
            unsafe {
                *string_length2_ptr = 0;
            }
        }
    }

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
pub unsafe extern "C" fn SQLSetConnectAttr(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    string_length: sql::Integer,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result =
        api::connection::set_connect_attr(connection_handle, attribute, value, string_length);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager (wide-character version).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLSetConnectAttrW(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    string_length: sql::Integer,
) -> sql::RetCode {
    const SQL_ATTR_CURRENT_CATALOG: sql::Integer = 109;

    if attribute == SQL_ATTR_CURRENT_CATALOG && !value.is_null() {
        let narrow =
            convert_wide_arg_to_cstring(value as *const sql::WChar, string_length as sql::SmallInt);
        if let Some(narrow_value) = narrow {
            eprintln!(
                "SQLSetConnectAttrW(SQL_ATTR_CURRENT_CATALOG) -> {}",
                narrow_value.to_string_lossy()
            );
            return SQLSetConnectAttr(
                connection_handle,
                attribute,
                narrow_value.as_ptr() as sql::Pointer,
                SQL_NTS_INTEGER,
            );
        }
    }

    SQLSetConnectAttr(connection_handle, attribute, value, string_length)
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetConnectAttr(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);
    let result = api::connection::get_connect_attr(
        connection_handle,
        attribute,
        value_ptr,
        buffer_length,
        string_length_ptr,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);
    result.to_sql_code()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetConnectAttrW(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    const SQL_ATTR_CURRENT_CATALOG: sql::Integer = 109;

    if attribute != SQL_ATTR_CURRENT_CATALOG {
        return SQLGetConnectAttr(
            connection_handle,
            attribute,
            value_ptr,
            buffer_length,
            string_length_ptr,
        );
    }

    api::diagnostic::clear_diag_info(sql::HandleType::Dbc, connection_handle);

    let mut ansi_len: sql::Integer = 0;
    let probe_result = api::connection::get_connect_attr(
        connection_handle,
        attribute,
        std::ptr::null_mut(),
        0,
        &mut ansi_len,
    );

    if probe_result.is_err() {
        api::diagnostic::set_diag_info_from_result(
            sql::HandleType::Dbc,
            connection_handle,
            &probe_result,
        );
        return probe_result.to_sql_code();
    }

    let ansi_capacity = ansi_len.saturating_add(1).max(1) as usize;
    let mut ansi_buffer = vec![0u8; ansi_capacity];
    let mut written_len: sql::Integer = 0;

    let result = api::connection::get_connect_attr(
        connection_handle,
        attribute,
        ansi_buffer.as_mut_ptr() as sql::Pointer,
        ansi_capacity as sql::Integer,
        &mut written_len,
    );

    api::diagnostic::set_diag_info_from_result(sql::HandleType::Dbc, connection_handle, &result);

    if let Err(err) = result {
        return Err(err).to_sql_code();
    }

    let actual_len = written_len.max(0) as usize;
    let ansi_value = std::str::from_utf8(&ansi_buffer[..actual_len]).unwrap_or_default();
    let char_count = ansi_value.chars().count() as sql::Integer;

    if !value_ptr.is_null() && buffer_length > 0 {
        let capacity_chars = (buffer_length as usize) / std::mem::size_of::<sql::WChar>();
        copy_str_to_sqlwchar(value_ptr as *mut sql::WChar, capacity_chars, ansi_value);
    }

    if !string_length_ptr.is_null() {
        unsafe {
            *string_length_ptr = char_count;
        }
    }

    sql::SqlReturn::SUCCESS.0
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
    let result = api::info::get_info(
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
/// This function is called by the ODBC driver manager (wide-character version).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetInfoW(
    connection_handle: sql::Handle,
    info_type: sql::USmallInt,
    info_value_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
) -> sql::RetCode {
    // For now, just call the narrow version
    // TODO: Handle string info types properly by converting to wide
    SQLGetInfo(
        connection_handle,
        info_type,
        info_value_ptr,
        buffer_length,
        string_length_ptr,
    )
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLEndTran(
    handle_type: sql::SmallInt,
    handle: sql::Handle,
    completion_type: sql::SmallInt,
) -> sql::RetCode {
    // ODBC constants
    const SQL_HANDLE_ENV: u32 = 1;
    const SQL_HANDLE_DBC: u32 = 2;

    // Determine the handle type for diagnostics
    let diag_handle_type = match handle_type as u32 {
        SQL_HANDLE_ENV => sql::HandleType::Env,
        SQL_HANDLE_DBC => sql::HandleType::Dbc,
        _ => sql::HandleType::Dbc, // Default to connection
    };

    api::diagnostic::clear_diag_info(diag_handle_type, handle);
    let result = api::connection::end_tran(handle_type, handle, completion_type);
    api::diagnostic::set_diag_info_from_result(diag_handle_type, handle, &result);
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
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::data::bind_col(
        statement_handle,
        column_number,
        target_type,
        target_value_ptr,
        buffer_length,
        str_len_or_ind_ptr,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLFetch(statement_handle: sql::Handle) -> sql::RetCode {
    let result = api::data::fetch(statement_handle);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
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
    let result = api::data::get_data(
        statement_handle,
        col_or_param_num,
        target_type,
        target_value_ptr,
        buffer_length,
        str_len_or_ind_ptr,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
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
    let result = api::statement::prepare(statement_handle, statement_text, text_length);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager (wide-character version).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLPrepareW(
    statement_handle: sql::Handle,
    statement_text: *const sql::WChar,
    text_length: sql::Integer,
) -> sql::RetCode {
    const SQL_NTS: sql::Integer = -3;

    // Detect UTF-32 vs UTF-16
    let raw_ptr = statement_text as *const u8;
    let first_byte = *raw_ptr;
    let second_byte = *raw_ptr.add(1);
    let third_byte = *raw_ptr.add(2);
    let fourth_byte = *raw_ptr.add(3);
    let is_utf32 = first_byte != 0 && second_byte == 0 && third_byte == 0 && fourth_byte == 0;

    let sql_str = if is_utf32 {
        let ptr32 = statement_text as *const u32;
        let mut chars = Vec::new();
        let mut i = 0;
        loop {
            let c = *ptr32.add(i);
            if c == 0 {
                break;
            }
            if c <= 0x10FFFF {
                if let Some(ch) = char::from_u32(c) {
                    chars.push(ch);
                }
            }
            i += 1;
            if i > 100000 {
                break;
            }
        }
        chars.into_iter().collect::<String>()
    } else if text_length == SQL_NTS {
        let mut len = 0;
        while *statement_text.add(len) != 0 {
            len += 1;
            if len > 100000 {
                break;
            }
        }
        let slice = std::slice::from_raw_parts(statement_text, len);
        String::from_utf16_lossy(slice)
    } else if text_length > 0 {
        let slice = std::slice::from_raw_parts(statement_text, text_length as usize);
        String::from_utf16_lossy(slice)
    } else {
        String::new()
    };

    let narrow_str = std::ffi::CString::new(sql_str).unwrap_or_default();
    SQLPrepare(
        statement_handle,
        narrow_str.as_ptr() as *const sql::Char,
        SQL_NTS,
    )
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
pub unsafe extern "C" fn SQLParamData(
    statement_handle: sql::Handle,
    value_ptr_ptr: *mut sql::Pointer,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    match api::statement::param_data(statement_handle) {
        Ok(api::statement::ParamDataStatus::NeedData(ptr)) => {
            if !value_ptr_ptr.is_null() {
                unsafe {
                    *value_ptr_ptr = ptr;
                }
            }
            sql::SqlReturn::NEED_DATA.0
        }
        Ok(api::statement::ParamDataStatus::Success) => sql::SqlReturn::SUCCESS.0,
        Err(err) => {
            let result: OdbcResult<()> = Err(err);
            api::diagnostic::set_diag_info_from_result(
                sql::HandleType::Stmt,
                statement_handle,
                &result,
            );
            result.to_sql_code()
        }
    }
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLPutData(
    statement_handle: sql::Handle,
    data_ptr: sql::Pointer,
    str_len_or_ind_ptr: sql::Len,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement::put_data(statement_handle, data_ptr, str_len_or_ind_ptr);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLTables(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::Char,
    table_name_length: sql::SmallInt,
    table_type: *const sql::Char,
    table_type_length: sql::SmallInt,
) -> sql::RetCode {
    api::catalog::tables(
        statement_handle,
        catalog_name,
        catalog_name_length,
        schema_name,
        schema_name_length,
        table_name,
        table_name_length,
        table_type,
        table_type_length,
    )
    .to_sql_code()
}

/// # Safety
/// Wide-character version of SQLTables
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLTablesW(
    statement_handle: sql::Handle,
    catalog_name: *const sql::WChar,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::WChar,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::WChar,
    table_name_length: sql::SmallInt,
    table_type: *const sql::WChar,
    table_type_length: sql::SmallInt,
) -> sql::RetCode {
    let catalog = convert_wide_arg_to_cstring(catalog_name, catalog_name_length);
    let schema = convert_wide_arg_to_cstring(schema_name, schema_name_length);
    let table = convert_wide_arg_to_cstring(table_name, table_name_length);
    let table_type_str = convert_wide_arg_to_cstring(table_type, table_type_length);

    let (catalog_ptr, catalog_len) = match &catalog {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if catalog_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };
    let (schema_ptr, schema_len) = match &schema {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if schema_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };
    let (table_ptr, table_len) = match &table {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if table_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };
    let (type_ptr, type_len) = match &table_type_str {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if table_type_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };

    SQLTables(
        statement_handle,
        catalog_ptr,
        catalog_len,
        schema_ptr,
        schema_len,
        table_ptr,
        table_len,
        type_ptr,
        type_len,
    )
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLColumns(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::Char,
    table_name_length: sql::SmallInt,
    column_name: *const sql::Char,
    column_name_length: sql::SmallInt,
) -> sql::RetCode {
    let result = api::catalog::columns(
        statement_handle,
        catalog_name,
        catalog_name_length,
        schema_name,
        schema_name_length,
        table_name,
        table_name_length,
        column_name,
        column_name_length,
    );

    match &result {
        Ok(_) => {
            tracing::debug!("SQLColumns returning SQL_SUCCESS");
        }
        Err(err) => {
            tracing::error!("SQLColumns returning error: {}", err);
            eprintln!("SQLColumns error: {err}");
        }
    }

    let code = result.to_sql_code();
    eprintln!("SQLColumns retcode={code}");
    code
}

/// # Safety
/// Wide-character version of SQLColumns
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLColumnsW(
    statement_handle: sql::Handle,
    catalog_name: *const sql::WChar,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::WChar,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::WChar,
    table_name_length: sql::SmallInt,
    column_name: *const sql::WChar,
    column_name_length: sql::SmallInt,
) -> sql::RetCode {
    let catalog = convert_wide_arg_to_cstring(catalog_name, catalog_name_length);
    let schema = convert_wide_arg_to_cstring(schema_name, schema_name_length);
    let table = convert_wide_arg_to_cstring(table_name, table_name_length);
    let column = convert_wide_arg_to_cstring(column_name, column_name_length);

    let (catalog_ptr, catalog_len) = match &catalog {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if catalog_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };
    let (schema_ptr, schema_len) = match &schema {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if schema_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };
    let (table_ptr, table_len) = match &table {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if table_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };
    let (column_ptr, column_len) = match &column {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if column_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };

    SQLColumns(
        statement_handle,
        catalog_ptr,
        catalog_len,
        schema_ptr,
        schema_len,
        table_ptr,
        table_len,
        column_ptr,
        column_len,
    )
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLColumnPrivileges(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::Char,
    table_name_length: sql::SmallInt,
    column_name: *const sql::Char,
    column_name_length: sql::SmallInt,
) -> sql::RetCode {
    api::catalog::column_privileges(
        statement_handle,
        catalog_name,
        catalog_name_length,
        schema_name,
        schema_name_length,
        table_name,
        table_name_length,
        column_name,
        column_name_length,
    )
    .to_sql_code()
}

/// # Safety
/// Wide-character version of SQLColumnPrivileges
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLColumnPrivilegesW(
    statement_handle: sql::Handle,
    catalog_name: *const sql::WChar,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::WChar,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::WChar,
    table_name_length: sql::SmallInt,
    column_name: *const sql::WChar,
    column_name_length: sql::SmallInt,
) -> sql::RetCode {
    let catalog = convert_wide_arg_to_cstring(catalog_name, catalog_name_length);
    let schema = convert_wide_arg_to_cstring(schema_name, schema_name_length);
    let table = convert_wide_arg_to_cstring(table_name, table_name_length);
    let column = convert_wide_arg_to_cstring(column_name, column_name_length);

    let (catalog_ptr, catalog_len) = match &catalog {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if catalog_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };
    let (schema_ptr, schema_len) = match &schema {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if schema_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };
    let (table_ptr, table_len) = match &table {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if table_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };
    let (column_ptr, column_len) = match &column {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if column_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };

    SQLColumnPrivileges(
        statement_handle,
        catalog_ptr,
        catalog_len,
        schema_ptr,
        schema_len,
        table_ptr,
        table_len,
        column_ptr,
        column_len,
    )
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLTablePrivileges(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::Char,
    table_name_length: sql::SmallInt,
) -> sql::RetCode {
    api::catalog::table_privileges(
        statement_handle,
        catalog_name,
        catalog_name_length,
        schema_name,
        schema_name_length,
        table_name,
        table_name_length,
    )
    .to_sql_code()
}

/// # Safety
/// Wide-character version of SQLTablePrivileges
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLTablePrivilegesW(
    statement_handle: sql::Handle,
    catalog_name: *const sql::WChar,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::WChar,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::WChar,
    table_name_length: sql::SmallInt,
) -> sql::RetCode {
    let catalog = convert_wide_arg_to_cstring(catalog_name, catalog_name_length);
    let schema = convert_wide_arg_to_cstring(schema_name, schema_name_length);
    let table = convert_wide_arg_to_cstring(table_name, table_name_length);

    let (catalog_ptr, catalog_len) = match &catalog {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if catalog_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };
    let (schema_ptr, schema_len) = match &schema {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if schema_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };
    let (table_ptr, table_len) = match &table {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if table_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };

    SQLTablePrivileges(
        statement_handle,
        catalog_ptr,
        catalog_len,
        schema_ptr,
        schema_len,
        table_ptr,
        table_len,
    )
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetTypeInfo(
    statement_handle: sql::Handle,
    data_type: sql::SmallInt,
) -> sql::RetCode {
    api::catalog::get_type_info(statement_handle, data_type).to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLPrimaryKeys(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::Char,
    table_name_length: sql::SmallInt,
) -> sql::RetCode {
    api::catalog::primary_keys(
        statement_handle,
        catalog_name,
        catalog_name_length,
        schema_name,
        schema_name_length,
        table_name,
        table_name_length,
    )
    .to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLStatistics(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::Char,
    table_name_length: sql::SmallInt,
    unique: sql::USmallInt,
    reserved: sql::USmallInt,
) -> sql::RetCode {
    api::catalog::statistics(
        statement_handle,
        catalog_name,
        catalog_name_length,
        schema_name,
        schema_name_length,
        table_name,
        table_name_length,
        unique,
        reserved,
    )
    .to_sql_code()
}

/// # Safety
/// Wide-character version of SQLStatistics
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLStatisticsW(
    statement_handle: sql::Handle,
    catalog_name: *const sql::WChar,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::WChar,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::WChar,
    table_name_length: sql::SmallInt,
    unique: sql::USmallInt,
    reserved: sql::USmallInt,
) -> sql::RetCode {
    let catalog = convert_wide_arg_to_cstring(catalog_name, catalog_name_length);
    let schema = convert_wide_arg_to_cstring(schema_name, schema_name_length);
    let table = convert_wide_arg_to_cstring(table_name, table_name_length);

    let (catalog_ptr, catalog_len) = match &catalog {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if catalog_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };
    let (schema_ptr, schema_len) = match &schema {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if schema_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };
    let (table_ptr, table_len) = match &table {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if table_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };

    SQLStatistics(
        statement_handle,
        catalog_ptr,
        catalog_len,
        schema_ptr,
        schema_len,
        table_ptr,
        table_len,
        unique,
        reserved,
    )
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLSpecialColumns(
    statement_handle: sql::Handle,
    identifier_type: sql::USmallInt,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::Char,
    table_name_length: sql::SmallInt,
    scope: sql::USmallInt,
    nullable: sql::USmallInt,
) -> sql::RetCode {
    api::catalog::special_columns(
        statement_handle,
        identifier_type,
        catalog_name,
        catalog_name_length,
        schema_name,
        schema_name_length,
        table_name,
        table_name_length,
        scope,
        nullable,
    )
    .to_sql_code()
}

/// # Safety
/// Wide-character version of SQLSpecialColumns
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLSpecialColumnsW(
    statement_handle: sql::Handle,
    identifier_type: sql::USmallInt,
    catalog_name: *const sql::WChar,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::WChar,
    schema_name_length: sql::SmallInt,
    table_name: *const sql::WChar,
    table_name_length: sql::SmallInt,
    scope: sql::USmallInt,
    nullable: sql::USmallInt,
) -> sql::RetCode {
    let catalog = convert_wide_arg_to_cstring(catalog_name, catalog_name_length);
    let schema = convert_wide_arg_to_cstring(schema_name, schema_name_length);
    let table = convert_wide_arg_to_cstring(table_name, table_name_length);

    let (catalog_ptr, catalog_len) = match &catalog {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if catalog_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };
    let (schema_ptr, schema_len) = match &schema {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if schema_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };
    let (table_ptr, table_len) = match &table {
        Some(cstr) => (
            cstr.as_ptr() as *const sql::Char,
            if table_name_length == SQL_NTS_SMALLINT {
                SQL_NTS_SMALLINT
            } else {
                cstr.to_bytes().len().min(i16::MAX as usize) as sql::SmallInt
            },
        ),
        None => (std::ptr::null(), 0),
    };

    SQLSpecialColumns(
        statement_handle,
        identifier_type,
        catalog_ptr,
        catalog_len,
        schema_ptr,
        schema_len,
        table_ptr,
        table_len,
        scope,
        nullable,
    )
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLProcedures(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    proc_name: *const sql::Char,
    proc_name_length: sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::procedures(
        statement_handle,
        catalog_name,
        catalog_name_length,
        schema_name,
        schema_name_length,
        proc_name,
        proc_name_length,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// Wide-character version of SQLProcedures.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLProceduresW(
    statement_handle: sql::Handle,
    catalog_name: *const sql::WChar,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::WChar,
    schema_name_length: sql::SmallInt,
    proc_name: *const sql::WChar,
    proc_name_length: sql::SmallInt,
) -> sql::RetCode {
    let catalog_utf8 = convert_wide_arg_to_cstring(catalog_name, catalog_name_length);
    let schema_utf8 = convert_wide_arg_to_cstring(schema_name, schema_name_length);
    let proc_utf8 = convert_wide_arg_to_cstring(proc_name, proc_name_length);

    let catalog_ptr = catalog_utf8
        .as_ref()
        .map_or(std::ptr::null(), |s| s.as_ptr() as *const sql::Char);
    let schema_ptr = schema_utf8
        .as_ref()
        .map_or(std::ptr::null(), |s| s.as_ptr() as *const sql::Char);
    let proc_ptr = proc_utf8
        .as_ref()
        .map_or(std::ptr::null(), |s| s.as_ptr() as *const sql::Char);

    SQLProcedures(
        statement_handle,
        catalog_ptr,
        if catalog_utf8.is_some() {
            SQL_NTS_SMALLINT
        } else {
            0
        },
        schema_ptr,
        if schema_utf8.is_some() {
            SQL_NTS_SMALLINT
        } else {
            0
        },
        proc_ptr,
        if proc_utf8.is_some() {
            SQL_NTS_SMALLINT
        } else {
            0
        },
    )
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLProcedureColumns(
    statement_handle: sql::Handle,
    catalog_name: *const sql::Char,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::Char,
    schema_name_length: sql::SmallInt,
    proc_name: *const sql::Char,
    proc_name_length: sql::SmallInt,
    column_name: *const sql::Char,
    column_name_length: sql::SmallInt,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::catalog::procedure_columns(
        statement_handle,
        catalog_name,
        catalog_name_length,
        schema_name,
        schema_name_length,
        proc_name,
        proc_name_length,
        column_name,
        column_name_length,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// Wide-character version of SQLProcedureColumns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLProcedureColumnsW(
    statement_handle: sql::Handle,
    catalog_name: *const sql::WChar,
    catalog_name_length: sql::SmallInt,
    schema_name: *const sql::WChar,
    schema_name_length: sql::SmallInt,
    proc_name: *const sql::WChar,
    proc_name_length: sql::SmallInt,
    column_name: *const sql::WChar,
    column_name_length: sql::SmallInt,
) -> sql::RetCode {
    let catalog_utf8 = convert_wide_arg_to_cstring(catalog_name, catalog_name_length);
    let schema_utf8 = convert_wide_arg_to_cstring(schema_name, schema_name_length);
    let proc_utf8 = convert_wide_arg_to_cstring(proc_name, proc_name_length);
    let column_utf8 = convert_wide_arg_to_cstring(column_name, column_name_length);

    SQLProcedureColumns(
        statement_handle,
        catalog_utf8
            .as_ref()
            .map_or(std::ptr::null(), |s| s.as_ptr() as *const sql::Char),
        if catalog_utf8.is_some() {
            SQL_NTS_SMALLINT
        } else {
            0
        },
        schema_utf8
            .as_ref()
            .map_or(std::ptr::null(), |s| s.as_ptr() as *const sql::Char),
        if schema_utf8.is_some() {
            SQL_NTS_SMALLINT
        } else {
            0
        },
        proc_utf8
            .as_ref()
            .map_or(std::ptr::null(), |s| s.as_ptr() as *const sql::Char),
        if proc_utf8.is_some() {
            SQL_NTS_SMALLINT
        } else {
            0
        },
        column_utf8
            .as_ref()
            .map_or(std::ptr::null(), |s| s.as_ptr() as *const sql::Char),
        if column_utf8.is_some() {
            SQL_NTS_SMALLINT
        } else {
            0
        },
    )
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLCloseCursor(statement_handle: sql::Handle) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::cursor::close_cursor(statement_handle);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
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
    // Don't clear/set diag info for SQL_DROP as the handle is being freed
    if option == 1 {
        // SQL_DROP
        return api::cursor::free_stmt(statement_handle, option).to_sql_code();
    }
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::cursor::free_stmt(statement_handle, option);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLMoreResults(statement_handle: sql::Handle) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::cursor::more_results(statement_handle);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLCancel(statement_handle: sql::Handle) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::cursor::cancel(statement_handle);
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
    eprintln!(
        "SQLGetDiagRec(handle_type={handle_type:?}, handle={handle:p}, rec_number={rec_number})"
    );
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
    eprintln!(
        "SQLGetDiagRecW(handle_type={handle_type:?}, handle={handle:p}, rec_number={rec_number})"
    );
    unsafe {
        api::diagnostic::get_diag_rec_w(
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

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLSetStmtAttr(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    string_length: sql::Integer,
) -> sql::RetCode {
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result =
        api::statement_attr::set_stmt_attr(statement_handle, attribute, value, string_length);
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager (wide character version).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLSetStmtAttrW(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    string_length: sql::Integer,
) -> sql::RetCode {
    // For non-string attributes, SQLSetStmtAttrW is identical to SQLSetStmtAttr
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result =
        api::statement_attr::set_stmt_attr(statement_handle, attribute, value, string_length);
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
    let result = api::column_metadata::describe_col(
        statement_handle,
        column_number,
        column_name,
        buffer_length,
        name_length_ptr,
        data_type_ptr,
        column_size_ptr,
        decimal_digits_ptr,
        nullable_ptr,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager (wide-character version).
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
    // Call the narrow version first to get the data
    let mut narrow_name = vec![0u8; buffer_length.max(256) as usize];
    let result = SQLDescribeCol(
        statement_handle,
        column_number,
        narrow_name.as_mut_ptr() as *mut sql::Char,
        narrow_name.len() as sql::SmallInt,
        name_length_ptr,
        data_type_ptr,
        column_size_ptr,
        decimal_digits_ptr,
        nullable_ptr,
    );

    // Convert the column name to wide string
    if result == sql::SqlReturn::SUCCESS.0 || result == sql::SqlReturn::SUCCESS_WITH_INFO.0 {
        let name_str =
            std::ffi::CStr::from_ptr(narrow_name.as_ptr() as *const i8).to_string_lossy();
        let wide: Vec<u16> = name_str.encode_utf16().chain(std::iter::once(0)).collect();
        let copy_len = wide.len().min(buffer_length as usize);
        if !column_name.is_null() && copy_len > 0 {
            std::ptr::copy_nonoverlapping(wide.as_ptr(), column_name, copy_len);
        }
    }

    result
}

/// # Safety
/// This function is called by the ODBC driver manager.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLColAttribute(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    field_identifier: sql::USmallInt,
    character_attribute_ptr: *mut sql::Char,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    numeric_attribute_ptr: *mut sql::Len,
) -> sql::RetCode {
    tracing::debug!(
        "SQLColAttribute called: col={}, field={}, buffer_len={}",
        column_number,
        field_identifier,
        buffer_length
    );
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::column_metadata::col_attribute(
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLColAttributeW(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    field_identifier: sql::USmallInt,
    character_attribute_ptr: *mut sql::WChar,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    numeric_attribute_ptr: *mut sql::Len,
) -> sql::RetCode {
    tracing::debug!(
        "SQLColAttributeW called: col={}, field={}, buffer_len={}",
        column_number,
        field_identifier,
        buffer_length
    );
    let needs_char_result = !character_attribute_ptr.is_null() && buffer_length > 0;
    let wide_char_capacity = if buffer_length <= 0 {
        0
    } else {
        (buffer_length as usize) / std::mem::size_of::<sql::WChar>()
    };

    let mut ansi_storage: Vec<sql::Char> = Vec::new();
    let (ansi_ptr, ansi_buffer_len) = if needs_char_result {
        let len = buffer_length.max(1) as usize;
        ansi_storage.resize(len, 0);
        (ansi_storage.as_mut_ptr(), buffer_length)
    } else {
        (character_attribute_ptr as *mut sql::Char, buffer_length)
    };

    let mut ansi_length_written: sql::SmallInt = 0;
    let ansi_length_ptr = if needs_char_result && !string_length_ptr.is_null() {
        &mut ansi_length_written as *mut sql::SmallInt
    } else {
        string_length_ptr
    };

    let rc = SQLColAttribute(
        statement_handle,
        column_number,
        field_identifier,
        ansi_ptr,
        ansi_buffer_len,
        ansi_length_ptr,
        numeric_attribute_ptr,
    );

    if needs_char_result
        && matches!(
            sql::SqlReturn(rc),
            sql::SqlReturn::SUCCESS | sql::SqlReturn::SUCCESS_WITH_INFO
        )
    {
        let ansi_len = if !string_length_ptr.is_null() {
            ansi_length_written.max(0) as usize
        } else {
            // ANSI buffer is null-terminated
            let mut len = 0usize;
            while len < ansi_storage.len() {
                let byte = *ansi_storage.as_ptr().add(len);
                if byte == 0 {
                    break;
                }
                len += 1;
            }
            len
        };

        let ansi_slice = std::slice::from_raw_parts(ansi_storage.as_ptr() as *const u8, ansi_len);
        let value = String::from_utf8_lossy(ansi_slice);

        let written_chars =
            copy_str_to_sqlwchar(character_attribute_ptr, wide_char_capacity, &value);
        if !string_length_ptr.is_null() {
            *string_length_ptr = written_chars;
        }
    }

    tracing::debug!("SQLColAttributeW returning {:?}", sql::SqlReturn(rc));
    rc
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
    api::diagnostic::clear_diag_info(sql::HandleType::Stmt, statement_handle);
    let result = api::statement_attr::get_stmt_attr(
        statement_handle,
        attribute,
        value_ptr,
        buffer_length,
        string_length_ptr,
    );
    api::diagnostic::set_diag_info_from_result(sql::HandleType::Stmt, statement_handle, &result);
    result.to_sql_code()
}

/// # Safety
/// This function is called by the ODBC driver manager (wide-character version).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetStmtAttrW(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> sql::RetCode {
    // For now, just call the narrow version
    SQLGetStmtAttr(
        statement_handle,
        attribute,
        value_ptr,
        buffer_length,
        string_length_ptr,
    )
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
pub unsafe extern "C" fn SQLGetFunctions(
    _connection_handle: sql::Handle,
    function_id: sql::USmallInt,
    supported_ptr: *mut sql::USmallInt,
) -> sql::RetCode {
    // If function_id is SQL_API_ALL_FUNCTIONS (0) or SQL_API_ODBC3_ALL_FUNCTIONS (999),
    // we need to fill an array. Otherwise, we just set a single value.
    const SQL_API_ALL_FUNCTIONS: u16 = 0;
    const SQL_API_ODBC3_ALL_FUNCTIONS: u16 = 999;

    // List of functions we support
    const SUPPORTED_FUNCTIONS: &[u16] = &[
        1,    // SQLAllocConnect
        2,    // SQLAllocEnv
        3,    // SQLAllocStmt
        4,    // SQLBindCol
        5,    // SQLCancel
        6,    // SQLColAttributes (deprecated, use SQLColAttribute)
        7,    // SQLConnect
        8,    // SQLDescribeCol
        9,    // SQLDisconnect
        10,   // SQLError (deprecated, use SQLGetDiagRec)
        11,   // SQLExecDirect
        12,   // SQLExecute
        13,   // SQLFetch
        14,   // SQLFreeConnect (deprecated, use SQLFreeHandle)
        15,   // SQLFreeEnv (deprecated, use SQLFreeHandle)
        16,   // SQLFreeStmt
        17,   // SQLGetCursorName
        18,   // SQLNumResultCols
        19,   // SQLPrepare
        20,   // SQLRowCount
        21,   // SQLSetCursorName
        23,   // SQLTransact (deprecated, use SQLEndTran)
        40,   // SQLColumns
        41,   // SQLDriverConnect
        42,   // SQLGetConnectOption (deprecated)
        43,   // SQLGetData
        45,   // SQLGetInfo
        46,   // SQLGetStmtOption (deprecated)
        47,   // SQLGetTypeInfo
        48,   // SQLParamData
        49,   // SQLPutData
        50,   // SQLSetConnectOption (deprecated)
        51,   // SQLSetStmtOption (deprecated)
        52,   // SQLSpecialColumns
        53,   // SQLStatistics
        54,   // SQLTables
        55,   // SQLBrowseConnect
        56,   // SQLColumnPrivileges
        58,   // SQLDescribeParam
        59,   // SQLExtendedFetch
        60,   // SQLForeignKeys
        61,   // SQLMoreResults
        62,   // SQLNativeSql
        63,   // SQLNumParams
        64,   // SQLParamOptions (deprecated)
        65,   // SQLPrimaryKeys
        66,   // SQLProcedureColumns
        67,   // SQLProcedures
        68,   // SQLSetPos
        69,   // SQLSetScrollOptions (deprecated)
        70,   // SQLTablePrivileges
        71,   // SQLDrivers
        72,   // SQLBindParameter
        1001, // SQLAllocHandle
        1002, // SQLBindParam
        1003, // SQLCloseCursor
        1004, // SQLColAttribute
        1005, // SQLCopyDesc
        1006, // SQLEndTran
        1007, // SQLFetchScroll
        1008, // SQLFreeHandle
        1009, // SQLGetConnectAttr
        1010, // SQLGetDescField
        1011, // SQLGetDescRec
        1012, // SQLGetDiagField
        1013, // SQLGetDiagRec
        1014, // SQLGetEnvAttr
        1015, // SQLGetStmtAttr
        1016, // SQLSetConnectAttr
        1017, // SQLSetDescField
        1018, // SQLSetDescRec
        1019, // SQLSetEnvAttr
        1020, // SQLSetStmtAttr
        1021, // SQLBulkOperations
    ];

    if supported_ptr.is_null() {
        return sql::SqlReturn::ERROR.0;
    }

    match function_id {
        SQL_API_ALL_FUNCTIONS => {
            // Fill array of 100 elements (functions 0-99)
            let array = std::slice::from_raw_parts_mut(supported_ptr, 100);
            for i in 0..100 {
                array[i] = if SUPPORTED_FUNCTIONS.contains(&(i as u16)) {
                    1
                } else {
                    0
                };
            }
        }
        SQL_API_ODBC3_ALL_FUNCTIONS => {
            // Fill array of 250 elements for ODBC 3.x functions
            // This is a bit array where bit N indicates function N is supported
            let array = std::slice::from_raw_parts_mut(supported_ptr, 250);
            for elem in array.iter_mut() {
                *elem = 0;
            }
            for &func_id in SUPPORTED_FUNCTIONS {
                let word_idx = (func_id / 16) as usize;
                let bit_idx = func_id % 16;
                if word_idx < 250 {
                    array[word_idx] |= 1 << bit_idx;
                }
            }
        }
        _ => {
            // Single function query
            *supported_ptr = if SUPPORTED_FUNCTIONS.contains(&function_id) {
                1
            } else {
                0
            };
        }
    }

    sql::SqlReturn::SUCCESS.0
}
