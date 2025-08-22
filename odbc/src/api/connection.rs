use crate::api::{ConnectionState, OdbcError, OdbcResult, conn_from_handle};
use odbc_sys as sql;
use sf_core::api_client;
use std::collections::HashMap;
use tracing;

/// Convert text pointer to String
fn text_to_string(text: *const sql::Char, length: sql::Integer) -> Result<String, OdbcError> {
    if length == sql::NTS as i32 {
        let result = unsafe { std::ffi::CStr::from_ptr(text as *const i8).to_str() };
        match result {
            Ok(s) => Ok(s.to_string()),
            Err(e) => {
                tracing::error!("text_to_string: error converting text to string: {}", e);
                Err(OdbcError::TextConversion(format!(
                    "Failed to convert text: {e}"
                )))
            }
        }
    } else {
        let text_slice = unsafe { std::slice::from_raw_parts(text, length as usize) };
        String::from_utf8(text_slice.to_vec())
            .map_err(|e| OdbcError::TextConversion(format!("Failed to convert UTF-8: {e}")))
    }
}

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
    let connection_string = text_to_string(in_connection_string, in_string_length as i32)?;
    let connection_string_map = parse_connection_string(&connection_string);
    tracing::info!(
        "driver_connect: connection_string={:?}",
        connection_string_map
    );

    let connection = conn_from_handle(connection_handle);
    let mut client = api_client::new_database_driver_v1_client();
    let db_handle = client.database_new().map_err(|e| {
        OdbcError::ConnectionInit(format!("Failed to create database handle: {e:?}"))
    })?;
    let conn_handle = client.connection_new().map_err(|e| {
        OdbcError::ConnectionInit(format!("Failed to create connection handle: {e:?}"))
    })?;

    for (key, value) in connection_string_map {
        match key.as_str() {
            // TODO: Do it more generically
            "DRIVER" => {
                // ignore
            }
            "ACCOUNT" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "account".to_owned(), value)
                    .map_err(|e| {
                        OdbcError::ConnectionInit(format!("Failed to set account: {e:?}"))
                    })?;
            }
            "SERVER" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "host".to_owned(), value)
                    .map_err(|e| {
                        OdbcError::ConnectionInit(format!("Failed to set server: {e:?}"))
                    })?;
            }
            "PWD" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "password".to_owned(), value)
                    .map_err(|e| {
                        OdbcError::ConnectionInit(format!("Failed to set password: {e:?}"))
                    })?;
            }
            "UID" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "user".to_owned(), value)
                    .map_err(|e| OdbcError::ConnectionInit(format!("Failed to set user: {e:?}")))?;
            }
            "PORT" => {
                let port_int: i64 = value.parse().map_err(|e| OdbcError::InvalidPort {
                    port: value.clone(),
                    source: e,
                })?;
                client
                    .connection_set_option_int(conn_handle.clone(), "port".to_owned(), port_int)
                    .map_err(|e| OdbcError::ConnectionInit(format!("Failed to set port: {e:?}")))?;
            }
            "PROTOCOL" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "protocol".to_owned(), value)
                    .map_err(|e| {
                        OdbcError::ConnectionInit(format!("Failed to set protocol: {e:?}"))
                    })?;
            }
            "DATABASE" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "database".to_owned(), value)
                    .map_err(|e| {
                        OdbcError::ConnectionInit(format!("Failed to set database: {e:?}"))
                    })?;
            }
            "WAREHOUSE" => {
                client
                    .connection_set_option_string(
                        conn_handle.clone(),
                        "warehouse".to_owned(),
                        value,
                    )
                    .map_err(|e| {
                        OdbcError::ConnectionInit(format!("Failed to set warehouse: {e:?}"))
                    })?;
            }
            "ROLE" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "role".to_owned(), value)
                    .map_err(|e| OdbcError::ConnectionInit(format!("Failed to set role: {e:?}")))?;
            }
            "SCHEMA" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "schema".to_owned(), value)
                    .map_err(|e| {
                        OdbcError::ConnectionInit(format!("Failed to set schema: {e:?}"))
                    })?;
            }
            "PRIV_KEY_FILE" => {
                client
                    .connection_set_option_string(
                        conn_handle.clone(),
                        "private_key_file".to_owned(),
                        value,
                    )
                    .map_err(|e| {
                        OdbcError::ConnectionInit(format!("Failed to set private key file: {e:?}"))
                    })?;
            }
            "AUTHENTICATOR" => {
                client
                    .connection_set_option_string(
                        conn_handle.clone(),
                        "authenticator".to_owned(),
                        value,
                    )
                    .map_err(|e| {
                        OdbcError::ConnectionInit(format!("Failed to set authenticator: {e:?}"))
                    })?;
            }
            "PRIV_KEY_FILE_PWD" => {
                client
                    .connection_set_option_string(
                        conn_handle.clone(),
                        "private_key_password".to_owned(),
                        value,
                    )
                    .map_err(|e| {
                        OdbcError::ConnectionInit(format!(
                            "Failed to set private key password: {e:?}"
                        ))
                    })?;
            }
            "TOKEN" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "token".to_owned(), value)
                    .map_err(|e| {
                        OdbcError::ConnectionInit(format!("Failed to set token: {e:?}"))
                    })?;
            }
            _ => {
                tracing::warn!("driver_connect: unknown connection string key: {:?}", key);
            }
        }
    }

    client
        .connection_init(conn_handle.clone(), db_handle.clone())
        .map_err(|e| {
            OdbcError::ConnectionInit(format!("Connection initialization failed: {e:?}"))
        })?;

    connection.state = ConnectionState::Connected {
        client,
        db_handle,
        conn_handle,
    };

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
