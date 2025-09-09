use crate::api::{
    ConnectionState, OdbcError, OdbcResult, api_utils::cstr_to_string, conn_from_handle,
    error::InvalidPortSnafu,
};
use odbc_sys as sql;
<<<<<<< HEAD
use sf_core::thrift_apis::DatabaseDriverV1;
use sf_core::thrift_apis::client::create_client;
=======
>>>>>>> cdf415c (cleanups after API changes)
use sf_core::thrift_gen::database_driver_v1::{
    CertRevocationCheckMode as ThriftCrlMode, TlsConfig as ThriftTlsConfig,
};
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
    let mut client = create_client::<DatabaseDriverV1>();
    let db_handle = client
        .database_new()
        .map_err(OdbcError::from_thrift_error)?;
    let conn_handle = client
        .connection_new()
        .map_err(OdbcError::from_thrift_error)?;

    // Optional TLS config built from connection string
    let mut tls_config = ThriftTlsConfig::default();
    let mut tls_configured = false;

    for (key, value) in connection_string_map {
        match key.as_str() {
            // TODO: Do it more generically
            "DRIVER" => {
                // ignore
            }
            // TLS/CRL options in connection string
            "TLS_VERIFY_HOSTNAME" => {
                tls_config.verify_hostname = Some(value.to_lowercase() == "true");
                tls_configured = true;
            }
            "TLS_VERIFY_CERTS" => {
                tls_config.verify_certificates = Some(value.to_lowercase() == "true");
                tls_configured = true;
            }
            "TLS_ROOTS_PEM" => {
                tls_config.custom_root_store_path = Some(value);
                tls_configured = true;
            }
            "CRL_MODE" => {
                let m = value.to_uppercase();
                tls_config.crl_mode = Some(match m.as_str() {
                    "ENABLED" => ThriftCrlMode::ENABLED,
                    "ADVISORY" => ThriftCrlMode::ADVISORY,
                    _ => ThriftCrlMode::DISABLED,
                });
                tls_configured = true;
            }
            "CRL_HTTP_TIMEOUT" => {
                if let Ok(v) = value.parse::<i32>() {
                    tls_config.crl_http_timeout_seconds = Some(v);
                    tls_configured = true;
                }
            }
            "CRL_CONN_TIMEOUT" => {
                if let Ok(v) = value.parse::<i32>() {
                    tls_config.crl_connection_timeout_seconds = Some(v);
                    tls_configured = true;
                }
            }
            "ACCOUNT" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "account".to_owned(), value)
                    .map_err(OdbcError::from_thrift_error)?;
            }
            "SERVER" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "host".to_owned(), value)
                    .map_err(OdbcError::from_thrift_error)?;
            }
            "PWD" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "password".to_owned(), value)
                    .map_err(OdbcError::from_thrift_error)?;
            }
            "UID" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "user".to_owned(), value)
                    .map_err(OdbcError::from_thrift_error)?;
            }
            "PORT" => {
                let port_int: i64 = value.parse().context(InvalidPortSnafu {
                    port: value.clone(),
                })?;
                client
                    .connection_set_option_int(conn_handle.clone(), "port".to_owned(), port_int)
                    .map_err(OdbcError::from_thrift_error)?;
            }
            "PROTOCOL" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "protocol".to_owned(), value)
                    .map_err(OdbcError::from_thrift_error)?;
            }
            "DATABASE" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "database".to_owned(), value)
                    .map_err(OdbcError::from_thrift_error)?;
            }
            // Snowflake-style aliases for TLS
            "INSECUREMODE" => {
                let insecure = value.to_lowercase() == "true";
                tls_config.verify_hostname = Some(!insecure);
                tls_config.verify_certificates = Some(!insecure);
                tls_configured = true;
            }
            "OCSP_FAIL_OPEN" => {
                let fail_open = value.to_lowercase() == "true";
                tls_config.crl_mode = Some(if fail_open {
                    ThriftCrlMode::ADVISORY
                } else {
                    ThriftCrlMode::ENABLED
                });
                tls_configured = true;
            }
            "WAREHOUSE" => {
                client
                    .connection_set_option_string(
                        conn_handle.clone(),
                        "warehouse".to_owned(),
                        value,
                    )
                    .map_err(OdbcError::from_thrift_error)?;
            }
            "ROLE" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "role".to_owned(), value)
                    .map_err(OdbcError::from_thrift_error)?;
            }
            "SCHEMA" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "schema".to_owned(), value)
                    .map_err(OdbcError::from_thrift_error)?;
            }
            "PRIV_KEY_FILE" => {
                client
                    .connection_set_option_string(
                        conn_handle.clone(),
                        "private_key_file".to_owned(),
                        value,
                    )
                    .map_err(OdbcError::from_thrift_error)?;
            }
            "AUTHENTICATOR" => {
                client
                    .connection_set_option_string(
                        conn_handle.clone(),
                        "authenticator".to_owned(),
                        value,
                    )
                    .map_err(OdbcError::from_thrift_error)?;
            }
            "PRIV_KEY_FILE_PWD" => {
                client
                    .connection_set_option_string(
                        conn_handle.clone(),
                        "private_key_password".to_owned(),
                        value,
                    )
                    .map_err(OdbcError::from_thrift_error)?;
            }
            "TOKEN" => {
                client
                    .connection_set_option_string(conn_handle.clone(), "token".to_owned(), value)
                    .map_err(OdbcError::from_thrift_error)?;
            }
            _ => {
                tracing::warn!("driver_connect: unknown connection string key: {:?}", key);
            }
        }
    }

    // Apply TLS config if the DSN included it
    if tls_configured {
        client
            .connection_set_tls_config(conn_handle.clone(), tls_config)
            .map_err(OdbcError::from_thrift_error)?;
    }

    client
        .connection_init(conn_handle.clone(), db_handle.clone())
        .map_err(OdbcError::from_thrift_error)?;

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
