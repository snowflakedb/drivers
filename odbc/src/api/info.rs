///! Driver and Database Information Functions
///!
///! Implements SQLGetInfo for ODBC.
use crate::api::{OdbcResult, conn_from_handle, error::UnknownAttributeSnafu};
use odbc_sys as sql;
use tracing;

// SQL_INFO constants from ODBC spec
const SQL_DRIVER_NAME: u16 = 6;
const SQL_DRIVER_VER: u16 = 7;
const SQL_DBMS_NAME: u16 = 17;
const SQL_DBMS_VER: u16 = 18;
const SQL_DATA_SOURCE_NAME: u16 = 2;
const SQL_SERVER_NAME: u16 = 13;
const SQL_DATABASE_NAME: u16 = 16;
const SQL_USER_NAME: u16 = 47;
const SQL_MAX_COLUMN_NAME_LEN: u16 = 30;
const SQL_MAX_TABLE_NAME_LEN: u16 = 35;
const SQL_MAX_SCHEMA_NAME_LEN: u16 = 32;
const SQL_MAX_CATALOG_NAME_LEN: u16 = 34;
const SQL_MAX_COLUMNS_IN_SELECT: u16 = 100;
const SQL_MAX_COLUMNS_IN_TABLE: u16 = 101;
const SQL_MAX_ROW_SIZE: u16 = 104;
const SQL_MAX_STATEMENT_LEN: u16 = 105;
const SQL_IDENTIFIER_QUOTE_CHAR: u16 = 29;
const SQL_CATALOG_NAME_SEPARATOR: u16 = 41;
const SQL_CATALOG_TERM: u16 = 42;
const SQL_SCHEMA_TERM: u16 = 39;
const SQL_TABLE_TERM: u16 = 45;
const SQL_PROCEDURE_TERM: u16 = 40;
const SQL_SEARCH_PATTERN_ESCAPE: u16 = 14;
const SQL_GETDATA_EXTENSIONS: u16 = 81;
const SQL_ODBC_VER: u16 = 10;
const SQL_DRIVER_ODBC_VER: u16 = 77;
const SQL_ODBC_INTERFACE_CONFORMANCE: u16 = 152;
const SQL_SQL_CONFORMANCE: u16 = 118;
const SQL_CATALOG_USAGE: u16 = 92;
const SQL_SCHEMA_USAGE: u16 = 89;

// SQL_GETDATA_EXTENSIONS flags
const SQL_GD_ANY_COLUMN: u32 = 0x00000001;
const SQL_GD_ANY_ORDER: u32 = 0x00000002;

// SQL_ODBC_INTERFACE_CONFORMANCE values
const SQL_OIC_CORE: u32 = 1;

// SQL_SQL_CONFORMANCE values
const SQL_SC_SQL92_ENTRY: u32 = 0x00000001;

// SQL_CATALOG_USAGE and SQL_SCHEMA_USAGE flags
const SQL_CU_DML_STATEMENTS: u32 = 0x00000001;
const SQL_CU_TABLE_DEFINITION: u32 = 0x00000004;
const SQL_SU_DML_STATEMENTS: u32 = 0x00000001;
const SQL_SU_TABLE_DEFINITION: u32 = 0x00000004;

/// SQLGetInfo - Get driver and database information
pub fn get_info(
    connection_handle: sql::Handle,
    info_type: sql::USmallInt,
    info_value_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("get_info: info_type={}", info_type);

    let _conn = conn_from_handle(connection_handle);

    match info_type {
        SQL_DRIVER_NAME => write_string_info(
            info_value_ptr,
            buffer_length,
            string_length_ptr,
            "SnowflakeDSIIDriver",
        ),
        SQL_DRIVER_VER => {
            write_string_info(info_value_ptr, buffer_length, string_length_ptr, "3.13.0")
        }
        SQL_DBMS_NAME => write_string_info(
            info_value_ptr,
            buffer_length,
            string_length_ptr,
            "Snowflake",
        ),
        SQL_DBMS_VER => {
            write_string_info(info_value_ptr, buffer_length, string_length_ptr, "9.36.3")
        }
        SQL_DATA_SOURCE_NAME => write_string_info(
            info_value_ptr,
            buffer_length,
            string_length_ptr,
            "Snowflake",
        ),
        SQL_SERVER_NAME => write_string_info(
            info_value_ptr,
            buffer_length,
            string_length_ptr,
            "snowflake",
        ),
        SQL_DATABASE_NAME => {
            write_string_info(info_value_ptr, buffer_length, string_length_ptr, "")
        }
        SQL_USER_NAME => write_string_info(info_value_ptr, buffer_length, string_length_ptr, ""),
        SQL_MAX_COLUMN_NAME_LEN => write_u16_info(info_value_ptr, string_length_ptr, 255),
        SQL_MAX_TABLE_NAME_LEN => write_u16_info(info_value_ptr, string_length_ptr, 255),
        SQL_MAX_SCHEMA_NAME_LEN => write_u16_info(info_value_ptr, string_length_ptr, 255),
        SQL_MAX_CATALOG_NAME_LEN => write_u16_info(info_value_ptr, string_length_ptr, 255),
        SQL_MAX_COLUMNS_IN_SELECT => {
            write_u16_info(info_value_ptr, string_length_ptr, 0) // No limit
        }
        SQL_MAX_COLUMNS_IN_TABLE => {
            write_u16_info(info_value_ptr, string_length_ptr, 0) // No limit
        }
        SQL_MAX_ROW_SIZE => {
            write_u32_info(info_value_ptr, string_length_ptr, 16777216) // 16MB
        }
        SQL_MAX_STATEMENT_LEN => {
            write_u32_info(info_value_ptr, string_length_ptr, 0) // No limit
        }
        SQL_IDENTIFIER_QUOTE_CHAR => {
            write_string_info(info_value_ptr, buffer_length, string_length_ptr, "\"")
        }
        SQL_CATALOG_NAME_SEPARATOR => {
            write_string_info(info_value_ptr, buffer_length, string_length_ptr, ".")
        }
        SQL_CATALOG_TERM => {
            write_string_info(info_value_ptr, buffer_length, string_length_ptr, "database")
        }
        SQL_SCHEMA_TERM => {
            write_string_info(info_value_ptr, buffer_length, string_length_ptr, "schema")
        }
        SQL_TABLE_TERM => {
            write_string_info(info_value_ptr, buffer_length, string_length_ptr, "table")
        }
        SQL_PROCEDURE_TERM => write_string_info(
            info_value_ptr,
            buffer_length,
            string_length_ptr,
            "procedure",
        ),
        SQL_SEARCH_PATTERN_ESCAPE => {
            write_string_info(info_value_ptr, buffer_length, string_length_ptr, "\\")
        }
        SQL_GETDATA_EXTENSIONS => write_u32_info(
            info_value_ptr,
            string_length_ptr,
            SQL_GD_ANY_COLUMN | SQL_GD_ANY_ORDER,
        ),
        SQL_ODBC_VER => {
            write_string_info(info_value_ptr, buffer_length, string_length_ptr, "03.80")
        }
        SQL_DRIVER_ODBC_VER => {
            write_string_info(info_value_ptr, buffer_length, string_length_ptr, "03.80")
        }
        SQL_ODBC_INTERFACE_CONFORMANCE => {
            write_u32_info(info_value_ptr, string_length_ptr, SQL_OIC_CORE)
        }
        SQL_SQL_CONFORMANCE => {
            write_u32_info(info_value_ptr, string_length_ptr, SQL_SC_SQL92_ENTRY)
        }
        SQL_CATALOG_USAGE => write_u32_info(
            info_value_ptr,
            string_length_ptr,
            SQL_CU_DML_STATEMENTS | SQL_CU_TABLE_DEFINITION,
        ),
        SQL_SCHEMA_USAGE => write_u32_info(
            info_value_ptr,
            string_length_ptr,
            SQL_SU_DML_STATEMENTS | SQL_SU_TABLE_DEFINITION,
        ),
        _ => {
            tracing::warn!("get_info: unsupported info_type={}", info_type);
            UnknownAttributeSnafu {
                attribute: info_type as i32,
            }
            .fail()
        }
    }
}

/// Write a string value to the info buffer
fn write_string_info(
    info_value_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    value: &str,
) -> OdbcResult<()> {
    let value_bytes = value.as_bytes();
    let value_len = value_bytes.len() as sql::SmallInt;

    // Set the string length (not including null terminator)
    if !string_length_ptr.is_null() {
        unsafe {
            *string_length_ptr = value_len;
        }
    }

    // Copy the string to the buffer if provided
    if !info_value_ptr.is_null() && buffer_length > 0 {
        let copy_len = ((buffer_length - 1) as usize).min(value_bytes.len());
        unsafe {
            std::ptr::copy_nonoverlapping(
                value_bytes.as_ptr(),
                info_value_ptr as *mut u8,
                copy_len,
            );
            // Null terminate
            *((info_value_ptr as *mut u8).add(copy_len)) = 0;
        }
    }

    Ok(())
}

/// Write a u16 value to the info buffer
fn write_u16_info(
    info_value_ptr: sql::Pointer,
    string_length_ptr: *mut sql::SmallInt,
    value: u16,
) -> OdbcResult<()> {
    if !info_value_ptr.is_null() {
        unsafe {
            *(info_value_ptr as *mut u16) = value;
        }
    }

    if !string_length_ptr.is_null() {
        unsafe {
            *string_length_ptr = std::mem::size_of::<u16>() as sql::SmallInt;
        }
    }

    Ok(())
}

/// Write a u32 value to the info buffer
fn write_u32_info(
    info_value_ptr: sql::Pointer,
    string_length_ptr: *mut sql::SmallInt,
    value: u32,
) -> OdbcResult<()> {
    if !info_value_ptr.is_null() {
        unsafe {
            *(info_value_ptr as *mut u32) = value;
        }
    }

    if !string_length_ptr.is_null() {
        unsafe {
            *string_length_ptr = std::mem::size_of::<u32>() as sql::SmallInt;
        }
    }

    Ok(())
}
