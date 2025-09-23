use crate::api::error::Required;
use crate::api::{
    ConnectionState, OdbcResult, api_utils::cstr_to_string, conn_from_handle,
    error::InvalidPortSnafu,
};
use odbc_sys as sql;
use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf_gen::database_driver_v1::*;
use snafu::ResultExt;
use std::collections::HashMap;
use tracing;

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
            "PRIV_KEY_FILE_PWD" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "private_key_password".to_owned(),
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
