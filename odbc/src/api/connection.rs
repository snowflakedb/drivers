use crate::api::error::Required;
use crate::api::{
    ConnectionState, OdbcResult, api_utils::cstr_to_string, conn_from_handle,
    error::{InvalidPortSnafu, UnknownAttributeSnafu},
    types::{
        SQL_SF_CONN_ATTR_APPLICATION, SQL_SF_CONN_ATTR_PRIV_KEY, SQL_SF_CONN_ATTR_PRIV_KEY_BASE64,
        SQL_SF_CONN_ATTR_PRIV_KEY_CONTENT, SQL_SF_CONN_ATTR_PRIV_KEY_PASSWORD,
    },
};
use odbc_sys as sql;
use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf_gen::database_driver_v1::*;
use snafu::ResultExt;
use std::collections::HashMap;
use tracing;

// Standard ODBC connection attribute constants (from sql.h / sqlext.h)
const SQL_ATTR_AUTOCOMMIT: i32 = 102;
const SQL_ATTR_LOGIN_TIMEOUT: i32 = 103;
const SQL_ATTR_CONNECTION_TIMEOUT: i32 = 113;
const SQL_AUTOCOMMIT_ON: sql::ULen = 1;

/// Parse connection string into key-value pairs
fn parse_connection_string(connection_string: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in connection_string.split(';') {
        let parts: Vec<&str> = pair.splitn(2, '=').collect();
        if parts.len() == 2 {
            map.insert(parts[0].to_string(), parts[1].to_string());
        }
    }
    map
}

/// Connect using connection string (SQLDriverConnect)
pub fn driver_connect(
    connection_handle: sql::Handle,
    in_connection_string: *const sql::Char,
    in_string_length: sql::SmallInt,
) -> OdbcResult<()> {
    // Parse the connection string
    let connection_string = cstr_to_string(in_connection_string, in_string_length as i32)?;
    let connection_string_map = parse_connection_string(&connection_string);
    tracing::info!(
        "driver_connect: connection_string={:?}",
        connection_string_map
    );

    let connection = conn_from_handle(connection_handle);
    let db_handle = DatabaseDriverClient::database_new(DatabaseNewRequest {})?
        .db_handle
        .required("Database handle is required")?;
    let conn_handle = DatabaseDriverClient::connection_new(ConnectionNewRequest {})?
        .conn_handle
        .required("Connection handle is required")?;

    for (key, value) in connection_string_map {
        match key.as_str() {
            // TODO: Do it more generically
            "DRIVER" => {
                // ignore
            }
            "ACCOUNT" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "account".to_owned(),
                        value,
                    },
                )?;
            }
            "SERVER" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "host".to_owned(),
                        value,
                    },
                )?;
            }
            "PWD" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "password".to_owned(),
                        value,
                    },
                )?;
            }
            "UID" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "user".to_owned(),
                        value,
                    },
                )?;
            }
            "PORT" => {
                let port_int: i64 = value.parse().context(InvalidPortSnafu {
                    port: value.clone(),
                })?;
                DatabaseDriverClient::connection_set_option_int(ConnectionSetOptionIntRequest {
                    conn_handle: Some(conn_handle),
                    key: "port".to_owned(),
                    value: port_int,
                })?;
            }
            "PROTOCOL" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "protocol".to_owned(),
                        value,
                    },
                )?;
            }
            "DATABASE" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "database".to_owned(),
                        value,
                    },
                )?;
            }
            "WAREHOUSE" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "warehouse".to_owned(),
                        value,
                    },
                )?;
            }
            "ROLE" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "role".to_owned(),
                        value,
                    },
                )?;
            }
            "SCHEMA" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "schema".to_owned(),
                        value,
                    },
                )?;
            }
            "PRIV_KEY_FILE" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "private_key_file".to_owned(),
                        value,
                    },
                )?;
            }
            "AUTHENTICATOR" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "authenticator".to_owned(),
                        value,
                    },
                )?;
            }
            "PRIV_KEY_FILE_PWD" | "PRIV_KEY_PWD" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "private_key_password".to_owned(),
                        value,
                    },
                )?;
            }
            "PRIV_KEY_BASE64" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "private_key".to_owned(),
                        value,
                    },
                )?;
            }
            "TOKEN" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "token".to_owned(),
                        value,
                    },
                )?;
            }
            "TLS_CUSTOM_ROOT_STORE_PATH" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "custom_root_store_path".to_owned(),
                        value,
                    },
                )?;
            }
            "TLS_VERIFY_HOSTNAME" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "verify_hostname".to_owned(),
                        value,
                    },
                )?;
            }
            "TLS_VERIFY_CERTIFICATES" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "verify_certificates".to_owned(),
                        value,
                    },
                )?;
            }
            // CRL settings via options
            "CRL_ENABLED" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "crl_enabled".to_owned(),
                        value,
                    },
                )?;
            }
            "CRL_MODE" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "crl_mode".to_owned(),
                        value: value.to_uppercase(),
                    },
                )?;
            }
            _ => {
                tracing::warn!("driver_connect: unknown connection string key: {:?}", key);
            }
        }
    }

    // Apply any pre-connection attributes set via SQLSetConnectAttr
    apply_pre_connection_attrs(connection, conn_handle)?;

    DatabaseDriverClient::connection_init(ConnectionInitRequest {
        conn_handle: Some(conn_handle),
        db_handle: Some(db_handle),
    })?;

    connection.state = ConnectionState::Connected {
        db_handle,
        conn_handle,
    };

    Ok(())
}

/// Apply pre-connection attributes (set via SQLSetConnectAttr) to the sf_core connection.
///
/// Attributes set via SQLSetConnectAttr take priority over connection string parameters,
/// matching the behavior of the old Snowflake ODBC driver (snowflake-odbc). Connection
/// string parameters are applied first, then pre-connection attributes override them.
///
/// Private key priority (matching old driver SFConnection.cpp):
///   1. SQL_SF_CONN_ATTR_PRIV_KEY_CONTENT (PEM string)
///   2. SQL_SF_CONN_ATTR_PRIV_KEY_BASE64 (base64-encoded key)
///   3. Connection string PRIV_KEY_BASE64
///   4. Connection string PRIV_KEY_FILE / PRIV_KEY_FILE_PWD (lowest priority)
fn apply_pre_connection_attrs(
    connection: &mut crate::api::Connection,
    conn_handle: ConnectionHandle,
) -> OdbcResult<()> {
    let attrs = &connection.pre_connection_attrs;

    // Private key: PRIV_KEY_CONTENT takes priority over PRIV_KEY_BASE64 (matching old driver).
    // Only one of these should be forwarded to core as "private_key".
    if let Some(ref content) = attrs.private_key_content {
        // SQL_SF_CONN_ATTR_PRIV_KEY_CONTENT -> private_key (PEM string sent as base64 to core)
        use base64::{Engine as _, engine::general_purpose};
        let encoded = general_purpose::STANDARD.encode(content.as_bytes());
        DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
            conn_handle: Some(conn_handle),
            key: "private_key".to_owned(),
            value: encoded,
        })?;
    } else if let Some(ref base64_key) = attrs.private_key_base64 {
        // SQL_SF_CONN_ATTR_PRIV_KEY_BASE64 -> private_key (already base64-encoded)
        DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
            conn_handle: Some(conn_handle),
            key: "private_key".to_owned(),
            value: base64_key.clone(),
        })?;
    }

    // SQL_SF_CONN_ATTR_PRIV_KEY_PASSWORD -> private_key_password
    if let Some(ref password) = attrs.private_key_password {
        DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
            conn_handle: Some(conn_handle),
            key: "private_key_password".to_owned(),
            value: password.clone(),
        })?;
    }

    // SQL_SF_CONN_ATTR_APPLICATION -> application
    if let Some(ref app) = attrs.application {
        DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
            conn_handle: Some(conn_handle),
            key: "application".to_owned(),
            value: app.clone(),
        })?;
    }

    Ok(())
}

/// Simple connect function (SQLConnect) - currently a placeholder
pub fn connect(
    _connection_handle: sql::Handle,
    _server_name: *const sql::Char,
    _name_length1: sql::SmallInt,
    _user_name: *const sql::Char,
    _name_length2: sql::SmallInt,
    _authentication: *const sql::Char,
    _name_length3: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("connect: currently a placeholder implementation");
    // TODO: Implement proper SQLConnect functionality
    Ok(())
}

/// Disconnect from the database
pub fn disconnect(_connection_handle: sql::Handle) -> OdbcResult<()> {
    tracing::debug!("disconnect: disconnecting from database");
    // TODO: Implement proper disconnect functionality
    Ok(())
}

/// Set a connection attribute (SQLSetConnectAttr).
/// Handles both standard ODBC attributes and custom Snowflake attributes.
pub fn set_connect_attr(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    string_length: sql::Integer,
) -> OdbcResult<()> {
    let connection = conn_from_handle(connection_handle);
    tracing::debug!("set_connect_attr: attribute={}", attribute);

    match attribute {
        // Standard ODBC attributes
        SQL_ATTR_LOGIN_TIMEOUT => {
            tracing::debug!("set_connect_attr: SQL_ATTR_LOGIN_TIMEOUT (ignored)");
            Ok(())
        }
        SQL_ATTR_CONNECTION_TIMEOUT => {
            tracing::debug!("set_connect_attr: SQL_ATTR_CONNECTION_TIMEOUT (ignored)");
            Ok(())
        }
        SQL_ATTR_AUTOCOMMIT => {
            tracing::debug!("set_connect_attr: SQL_ATTR_AUTOCOMMIT (ignored)");
            Ok(())
        }

        // Custom Snowflake attributes for private key authentication
        SQL_SF_CONN_ATTR_PRIV_KEY => {
            // The old driver accepted an EVP_PKEY pointer here. We cannot support raw
            // OpenSSL pointers in the Rust driver, so this attribute is not supported.
            tracing::warn!(
                "set_connect_attr: SQL_SF_CONN_ATTR_PRIV_KEY (EVP_PKEY pointer) is not supported. \
                 Use SQL_SF_CONN_ATTR_PRIV_KEY_CONTENT or SQL_SF_CONN_ATTR_PRIV_KEY_BASE64 instead."
            );
            UnknownAttributeSnafu {
                attribute: SQL_SF_CONN_ATTR_PRIV_KEY,
            }
            .fail()
        }
        SQL_SF_CONN_ATTR_PRIV_KEY_CONTENT => {
            let value = read_string_attr(value_ptr, string_length)?;
            tracing::debug!("set_connect_attr: SQL_SF_CONN_ATTR_PRIV_KEY_CONTENT (set)");
            connection.pre_connection_attrs.private_key_content = Some(value);
            Ok(())
        }
        SQL_SF_CONN_ATTR_PRIV_KEY_PASSWORD => {
            let value = read_string_attr(value_ptr, string_length)?;
            tracing::debug!("set_connect_attr: SQL_SF_CONN_ATTR_PRIV_KEY_PASSWORD (set)");
            connection.pre_connection_attrs.private_key_password = Some(value);
            Ok(())
        }
        SQL_SF_CONN_ATTR_PRIV_KEY_BASE64 => {
            let value = read_string_attr(value_ptr, string_length)?;
            tracing::debug!("set_connect_attr: SQL_SF_CONN_ATTR_PRIV_KEY_BASE64 (set)");
            connection.pre_connection_attrs.private_key_base64 = Some(value);
            Ok(())
        }
        SQL_SF_CONN_ATTR_APPLICATION => {
            let value = read_string_attr(value_ptr, string_length)?;
            tracing::debug!("set_connect_attr: SQL_SF_CONN_ATTR_APPLICATION = {}", value);
            connection.pre_connection_attrs.application = Some(value);
            Ok(())
        }

        _ => {
            tracing::warn!("set_connect_attr: unknown attribute {}", attribute);
            // Return Ok for unrecognized standard attributes to avoid breaking
            // driver manager attribute propagation
            Ok(())
        }
    }
}

/// Get a connection attribute (SQLGetConnectAttr).
/// Handles both standard ODBC attributes and custom Snowflake attributes.
/// Returns true if string data was truncated (caller should return SQL_SUCCESS_WITH_INFO).
pub fn get_connect_attr(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> OdbcResult<bool> {
    let connection = conn_from_handle(connection_handle);
    tracing::debug!("get_connect_attr: attribute={}", attribute);

    match attribute {
        SQL_SF_CONN_ATTR_PRIV_KEY_CONTENT => {
            let truncated = write_string_attr(
                connection.pre_connection_attrs.private_key_content.as_deref().unwrap_or(""),
                value_ptr,
                buffer_length,
                string_length_ptr,
            );
            Ok(truncated)
        }
        SQL_SF_CONN_ATTR_PRIV_KEY_PASSWORD => {
            let truncated = write_string_attr(
                connection.pre_connection_attrs.private_key_password.as_deref().unwrap_or(""),
                value_ptr,
                buffer_length,
                string_length_ptr,
            );
            Ok(truncated)
        }
        SQL_SF_CONN_ATTR_PRIV_KEY_BASE64 => {
            let truncated = write_string_attr(
                connection.pre_connection_attrs.private_key_base64.as_deref().unwrap_or(""),
                value_ptr,
                buffer_length,
                string_length_ptr,
            );
            Ok(truncated)
        }
        SQL_SF_CONN_ATTR_APPLICATION => {
            let truncated = write_string_attr(
                connection.pre_connection_attrs.application.as_deref().unwrap_or(""),
                value_ptr,
                buffer_length,
                string_length_ptr,
            );
            Ok(truncated)
        }
        SQL_ATTR_AUTOCOMMIT => {
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::ULen) = SQL_AUTOCOMMIT_ON as sql::ULen;
                }
            }
            Ok(false)
        }
        _ => {
            tracing::warn!("get_connect_attr: unknown attribute {}", attribute);
            Ok(false)
        }
    }
}

/// Read a string value from a SQLSetConnectAttr value pointer
fn read_string_attr(value_ptr: sql::Pointer, string_length: sql::Integer) -> OdbcResult<String> {
    if value_ptr.is_null() {
        return Ok(String::new());
    }
    let c_str_ptr = value_ptr as *const sql::Char;
    cstr_to_string(c_str_ptr, string_length)
}

/// Write a string value to a SQLGetConnectAttr output buffer.
/// Returns true if the value was truncated (caller should report SQL_SUCCESS_WITH_INFO / 01004).
fn write_string_attr(
    value: &str,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> bool {
    // Always report the full length, even if truncated (per ODBC spec)
    if !string_length_ptr.is_null() {
        unsafe {
            *string_length_ptr = value.len() as sql::Integer;
        }
    }
    if !value_ptr.is_null() && buffer_length > 0 {
        let buf = value_ptr as *mut sql::Char;
        let max_len = std::cmp::min(value.len(), (buffer_length - 1) as usize);
        unsafe {
            std::ptr::copy_nonoverlapping(value.as_ptr() as *const sql::Char, buf, max_len);
            *buf.add(max_len) = 0; // NUL terminate
        }
        // Truncation occurred if the value is longer than the available buffer
        value.len() > (buffer_length - 1) as usize
    } else {
        false
    }
}
