///! ODBC Statement Attributes
///!
///! Implements SQLSetStmtAttr and SQLGetStmtAttr
use crate::api::{OdbcResult, ParamBindType, stmt_from_handle};
use odbc_sys as sql;
use tracing;

// Common statement attributes
const SQL_ATTR_QUERY_TIMEOUT: i32 = 0;
const SQL_ATTR_MAX_ROWS: i32 = 1;
const SQL_ATTR_NOSCAN: i32 = 2;
const SQL_ATTR_MAX_LENGTH: i32 = 3;
const SQL_ATTR_ASYNC_ENABLE: i32 = 4;
const SQL_ATTR_ROW_BIND_TYPE: i32 = 5;
const SQL_ATTR_CURSOR_TYPE: i32 = 6;
const SQL_ATTR_CONCURRENCY: i32 = 7;
const SQL_ATTR_KEYSET_SIZE: i32 = 8;
const SQL_ATTR_ROWSET_SIZE: i32 = 9;
const SQL_ATTR_SIMULATE_CURSOR: i32 = 10;
const SQL_ATTR_RETRIEVE_DATA: i32 = 11;
const SQL_ATTR_USE_BOOKMARKS: i32 = 12;
const SQL_ATTR_ROW_NUMBER: i32 = 14;
const SQL_ATTR_PARAM_BIND_TYPE: i32 = 18;
const SQL_ATTR_PARAM_STATUS_PTR: i32 = 20;
const SQL_ATTR_PARAMS_PROCESSED_PTR: i32 = 21;
const SQL_ATTR_PARAMSET_SIZE: i32 = 22;
const SQL_ATTR_ENABLE_AUTO_IPD: i32 = 15;
const SQL_ATTR_METADATA_ID: i32 = 10014;
const SQL_ATTR_ROW_STATUS_PTR: i32 = 25;
const SQL_ATTR_ROWS_FETCHED_PTR: i32 = 26;
const SQL_ATTR_ROW_ARRAY_SIZE: i32 = 27;

// Snowflake-specific statement attributes
const SQL_SF_STMT_ATTR_LAST_QUERY_ID: i32 = 16647; // SQL_DRIVER_STMT_ATTR_BASE (0x4000) + 0x106 + 1
const SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT: i32 = 16648; // SQL_DRIVER_STMT_ATTR_BASE (0x4000) + 0x106 + 2

// Cursor types
const SQL_CURSOR_FORWARD_ONLY: usize = 0;
const SQL_CURSOR_STATIC: usize = 3;

// Async enable values
const SQL_ASYNC_ENABLE_OFF: usize = 0;
const SQL_ASYNC_ENABLE_ON: usize = 1;

const SQL_PARAM_BIND_BY_COLUMN: usize = 0;
const SQL_BIND_BY_COLUMN: usize = 0;

/// Set statement attribute
pub fn set_stmt_attr(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    string_length: sql::Integer,
) -> OdbcResult<()> {
    tracing::debug!("set_stmt_attr: attribute={}, value={:?}", attribute, value);

    let stmt = stmt_from_handle(statement_handle);

    match attribute {
        SQL_ATTR_QUERY_TIMEOUT => {
            let timeout = value as usize;
            tracing::info!("set_stmt_attr: setting query timeout to {}", timeout);
            stmt.query_timeout = timeout;
            Ok(())
        }
        SQL_ATTR_MAX_ROWS => {
            let max_rows = value as usize;
            tracing::info!("set_stmt_attr: setting max rows to {}", max_rows);
            stmt.max_rows = max_rows;
            Ok(())
        }
        SQL_ATTR_CURSOR_TYPE => {
            let cursor_type = value as usize;
            match cursor_type {
                SQL_CURSOR_FORWARD_ONLY => {
                    tracing::info!("set_stmt_attr: cursor type = FORWARD_ONLY");
                }
                SQL_CURSOR_STATIC => {
                    tracing::info!("set_stmt_attr: cursor type = STATIC");
                }
                _ => {
                    tracing::warn!("set_stmt_attr: unsupported cursor type {}", cursor_type);
                }
            }
            Ok(())
        }
        SQL_ATTR_ASYNC_ENABLE => {
            let async_enable = value as usize;
            match async_enable {
                SQL_ASYNC_ENABLE_OFF => {
                    tracing::info!("set_stmt_attr: async execution OFF");
                }
                SQL_ASYNC_ENABLE_ON => {
                    tracing::info!("set_stmt_attr: async execution ON");
                    // Snowflake queries are always async, so this is a no-op
                }
                _ => {
                    tracing::warn!("set_stmt_attr: invalid async_enable value {}", async_enable);
                }
            }
            Ok(())
        }
        SQL_ATTR_ROW_BIND_TYPE => {
            let bind_type = value as usize;
            tracing::info!("set_stmt_attr: row bind type = {}", bind_type);
            stmt.row_bind_type = bind_type;
            Ok(())
        }
        SQL_ATTR_ROW_ARRAY_SIZE => {
            let mut size = interpret_row_array_size(value, string_length);
            if size == 0 {
                size = 1;
            }
            tracing::info!("set_stmt_attr: row array size = {}", size);
            stmt.row_array_size = size;
            Ok(())
        }
        SQL_ATTR_ROWS_FETCHED_PTR => {
            stmt.rows_fetched_ptr = if value.is_null() {
                None
            } else {
                Some(value as *mut sql::ULen)
            };
            Ok(())
        }
        SQL_ATTR_ROW_STATUS_PTR => {
            stmt.row_status_ptr = if value.is_null() {
                None
            } else {
                Some(value as *mut sql::USmallInt)
            };
            Ok(())
        }
        SQL_ATTR_USE_BOOKMARKS => {
            let use_bookmarks = value as usize;
            tracing::info!("set_stmt_attr: use bookmarks = {}", use_bookmarks);
            // Bookmarks not supported, but don't error
            Ok(())
        }
        SQL_ATTR_PARAM_BIND_TYPE => {
            let bind_type = value as usize;
            match bind_type {
                SQL_PARAM_BIND_BY_COLUMN => {
                    tracing::info!("set_stmt_attr: parameter bind type = COLUMN");
                    stmt.param_bind_type = ParamBindType::Column;
                }
                stride => {
                    tracing::info!(
                        "set_stmt_attr: parameter bind type = ROW stride {} (unsupported)",
                        stride
                    );
                    stmt.param_bind_type = ParamBindType::Row(stride);
                }
            }
            Ok(())
        }
        SQL_ATTR_PARAMSET_SIZE => {
            let size = value as usize;
            if size == 0 {
                tracing::warn!("set_stmt_attr: paramset size of 0 is invalid; defaulting to 1");
                stmt.paramset_size = 1;
            } else {
                tracing::info!("set_stmt_attr: paramset size = {}", size);
                stmt.paramset_size = size;
            }
            Ok(())
        }
        SQL_ATTR_PARAM_STATUS_PTR => {
            stmt.param_status_ptr = if value.is_null() {
                None
            } else {
                Some(value as *mut sql::USmallInt)
            };
            Ok(())
        }
        SQL_ATTR_PARAMS_PROCESSED_PTR => {
            stmt.params_processed_ptr = if value.is_null() {
                None
            } else {
                Some(value as *mut sql::ULen)
            };
            Ok(())
        }
        SQL_ATTR_METADATA_ID => {
            let flag = (value as isize) != 0;
            tracing::info!("set_stmt_attr: metadata id = {flag}");
            stmt.metadata_id = flag;
            Ok(())
        }
        SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT => {
            let count = value as usize;
            tracing::info!("set_stmt_attr: multi-statement count = {count}");
            stmt.multi_statement_count = count;
            // Also set it in the sf_core statement
            let handle = sf_core::handle_manager::Handle {
                id: stmt.stmt_handle.id as u64,
                magic: stmt.stmt_handle.magic as u64,
            };
            if let Err(e) = sf_core::apis::database_driver_v1::statement_set_multi_statement_count(
                handle, count,
            ) {
                tracing::error!("Failed to set multi-statement count in sf_core: {e}");
                // Continue anyway - the ODBC layer has it set
            }
            Ok(())
        }
        SQL_ATTR_ENABLE_AUTO_IPD => {
            // Enable/disable automatic population of Implementation Parameter Descriptor
            // We always support SQLDescribeParam, so just accept this attribute
            let enabled = value as usize;
            tracing::info!("set_stmt_attr: enable auto IPD = {enabled}");
            Ok(())
        }
        _ => {
            tracing::warn!("set_stmt_attr: unsupported attribute {}", attribute);
            Ok(())
        }
    }
}

/// Get statement attribute
pub fn get_stmt_attr(
    statement_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    _buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> OdbcResult<()> {
    tracing::debug!("get_stmt_attr: attribute={}", attribute);

    let stmt = stmt_from_handle(statement_handle);

    match attribute {
        SQL_ATTR_QUERY_TIMEOUT => {
            // Return current timeout
            unsafe {
                *(value_ptr as *mut usize) = stmt.query_timeout;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<usize>() as sql::Integer;
                }
            }
            Ok(())
        }
        SQL_ATTR_MAX_ROWS => {
            // Return current max rows
            unsafe {
                *(value_ptr as *mut usize) = stmt.max_rows;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<usize>() as sql::Integer;
                }
            }
            Ok(())
        }
        SQL_ATTR_CURSOR_TYPE => {
            // Return forward-only cursor
            unsafe {
                *(value_ptr as *mut usize) = SQL_CURSOR_FORWARD_ONLY;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<usize>() as sql::Integer;
                }
            }
            Ok(())
        }
        SQL_ATTR_ASYNC_ENABLE => {
            // Return async enabled (Snowflake is always async)
            unsafe {
                *(value_ptr as *mut usize) = SQL_ASYNC_ENABLE_ON;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<usize>() as sql::Integer;
                }
            }
            Ok(())
        }
        SQL_ATTR_ROW_BIND_TYPE => {
            unsafe {
                *(value_ptr as *mut usize) = stmt.row_bind_type;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<usize>() as sql::Integer;
                }
            }
            Ok(())
        }
        SQL_ATTR_PARAM_BIND_TYPE => {
            unsafe {
                *(value_ptr as *mut usize) = match stmt.param_bind_type {
                    ParamBindType::Column => SQL_PARAM_BIND_BY_COLUMN,
                    ParamBindType::Row(stride) => stride,
                };
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<usize>() as sql::Integer;
                }
            }
            Ok(())
        }
        SQL_ATTR_ROW_ARRAY_SIZE => {
            unsafe {
                *(value_ptr as *mut usize) = stmt.row_array_size;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<usize>() as sql::Integer;
                }
            }
            Ok(())
        }
        SQL_ATTR_ROWS_FETCHED_PTR => {
            unsafe {
                *(value_ptr as *mut sql::Pointer) = stmt
                    .rows_fetched_ptr
                    .map(|ptr| ptr as sql::Pointer)
                    .unwrap_or(std::ptr::null_mut());
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<sql::Pointer>() as sql::Integer;
                }
            }
            Ok(())
        }
        SQL_ATTR_ROW_STATUS_PTR => {
            unsafe {
                *(value_ptr as *mut sql::Pointer) = stmt
                    .row_status_ptr
                    .map(|ptr| ptr as sql::Pointer)
                    .unwrap_or(std::ptr::null_mut());
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<sql::Pointer>() as sql::Integer;
                }
            }
            Ok(())
        }
        SQL_ATTR_PARAMSET_SIZE => {
            unsafe {
                *(value_ptr as *mut usize) = stmt.paramset_size;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<usize>() as sql::Integer;
                }
            }
            Ok(())
        }
        SQL_ATTR_PARAM_STATUS_PTR => {
            unsafe {
                *(value_ptr as *mut sql::Pointer) = stmt
                    .param_status_ptr
                    .map(|ptr| ptr as sql::Pointer)
                    .unwrap_or(std::ptr::null_mut());
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<sql::Pointer>() as sql::Integer;
                }
            }
            Ok(())
        }
        SQL_ATTR_METADATA_ID => {
            unsafe {
                *(value_ptr as *mut usize) = if stmt.metadata_id { 1 } else { 0 };
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<usize>() as sql::Integer;
                }
            }
            Ok(())
        }
        SQL_ATTR_PARAMS_PROCESSED_PTR => {
            unsafe {
                *(value_ptr as *mut sql::Pointer) = stmt
                    .params_processed_ptr
                    .map(|ptr| ptr as sql::Pointer)
                    .unwrap_or(std::ptr::null_mut());
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<sql::Pointer>() as sql::Integer;
                }
            }
            Ok(())
        }
        SQL_ATTR_ROW_NUMBER => {
            // Return current row number (1-based)
            unsafe {
                *(value_ptr as *mut usize) = stmt.current_row;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<usize>() as sql::Integer;
                }
            }
            Ok(())
        }
        SQL_SF_STMT_ATTR_LAST_QUERY_ID => {
            let query_id = stmt.last_query_id.as_deref().unwrap_or("");
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = query_id.len() as sql::Integer;
                }
            }
            if !value_ptr.is_null() && _buffer_length > 0 {
                let buffer_len = _buffer_length as usize;
                if buffer_len > 0 {
                    let bytes = query_id.as_bytes();
                    let copy_len = std::cmp::min(bytes.len(), buffer_len.saturating_sub(1));
                    unsafe {
                        let buffer =
                            std::slice::from_raw_parts_mut(value_ptr as *mut u8, buffer_len);
                        if copy_len > 0 {
                            buffer[..copy_len].copy_from_slice(&bytes[..copy_len]);
                        }
                        buffer[copy_len] = 0;
                    }
                }
            }
            Ok(())
        }
        _ => {
            tracing::warn!("get_stmt_attr: unsupported attribute {}", attribute);
            // Return 0 for unknown attributes
            unsafe {
                *(value_ptr as *mut usize) = 0;
                if !string_length_ptr.is_null() {
                    *string_length_ptr = std::mem::size_of::<usize>() as sql::Integer;
                }
            }
            Ok(())
        }
    }
}

const ROW_ARRAY_SIZE_PTR_THRESHOLD: usize = 1 << 32;

fn interpret_row_array_size(value: sql::Pointer, string_length: sql::Integer) -> usize {
    if string_length == sql::IS_POINTER {
        return read_usize_from_pointer(value);
    }
    let raw = value as usize;
    if raw > ROW_ARRAY_SIZE_PTR_THRESHOLD {
        read_usize_from_pointer(value)
    } else {
        raw
    }
}

fn read_usize_from_pointer(ptr: sql::Pointer) -> usize {
    if ptr.is_null() {
        0
    } else {
        unsafe { *(ptr as *const sql::ULen) as usize }
    }
}
