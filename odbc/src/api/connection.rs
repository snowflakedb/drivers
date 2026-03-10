use crate::api::InfoType;
use crate::api::bitmask::Bitmask;
use crate::api::error::Required;
use crate::api::{
    ConnectionState, FieldValue, GetDataExtensions, OdbcResult, conn_from_handle,
    error::{
        AttributeCannotBeSetNowSnafu, InvalidPortSnafu, UnknownAttributeSnafu,
        UnsupportedAttributeSnafu,
    },
    types::ConnectionAttribute,
};
use odbc_sys as sql;
use sf_core::protobuf::apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf::generated::database_driver_v1::*;
use snafu::ResultExt;
use std::collections::HashMap;
use tracing;

const SQL_AUTOCOMMIT_ON: sql::ULen = 1;

/// Default login timeout in seconds, matching the old driver's S_DEFAULT_LOGIN_TIMEOUT.
/// Used as the Okta SAML retry budget when neither the connection string nor
/// SQLSetConnectAttr provides a value.
const DEFAULT_LOGIN_TIMEOUT_SECS: &str = "300";

/// Maps ODBC connection string parameter names to their sf_core equivalents.
/// Parameters listed here are forwarded as-is via `connection_set_option_string`.
/// Parameters that need special handling (type conversion, conditional skipping,
/// side-effects) are handled separately in `driver_connect`.
const PARAM_MAPPINGS: &[(&str, &str)] = &[
    ("ACCOUNT", "account"),
    ("SERVER", "host"),
    ("PWD", "password"),
    ("UID", "user"),
    ("PROTOCOL", "protocol"),
    ("DATABASE", "database"),
    ("WAREHOUSE", "warehouse"),
    ("ROLE", "role"),
    ("SCHEMA", "schema"),
    ("AUTHENTICATOR", "authenticator"),
    ("TOKEN", "token"),
    ("TLS_CUSTOM_ROOT_STORE_PATH", "custom_root_store_path"),
    ("DISABLE_SAML_URL_CHECK", "disable_saml_url_check"),
    ("TLS_VERIFY_HOSTNAME", "verify_hostname"),
    ("TLS_VERIFY_CERTIFICATES", "verify_certificates"),
    ("CRL_ENABLED", "crl_enabled"),
];

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

/// Connect using connection string (SQLDriverConnect).
///
/// The caller (c_api.rs) is responsible for decoding the raw C string into
/// a Rust `&str` using the encoding module before calling this function.
pub fn driver_connect(connection_handle: sql::Handle, connection_string: &str) -> OdbcResult<()> {
    let connection_string_map = parse_connection_string(connection_string);
    {
        const REDACTED_KEYS: &[&str] = &[
            "PWD",
            "TOKEN",
            "PRIV_KEY_FILE_PWD",
            "PRIV_KEY_PWD",
            "PRIV_KEY_BASE64",
        ];
        let redacted_map: HashMap<&String, &str> = connection_string_map
            .iter()
            .map(|(k, v)| {
                let is_sensitive = REDACTED_KEYS.iter().any(|r| k.eq_ignore_ascii_case(r));
                let v = if is_sensitive { "****" } else { v.as_str() };
                (k, v)
            })
            .collect();
        tracing::info!("driver_connect: connection_string={:?}", redacted_map);
    }

    let connection = conn_from_handle(connection_handle);
    let db_handle = DatabaseDriverClient::database_new(DatabaseNewRequest {})?
        .db_handle
        .required("Database handle is required")?;
    let conn_handle = DatabaseDriverClient::connection_new(ConnectionNewRequest {})?
        .conn_handle
        .required("Connection handle is required")?;

    // Check whether attribute-based key options supersede file-based connection string params.
    // Matches old driver (SFConnection.cpp): if PrivKeyContent or PrivKeyBase64 was set via
    // SQLSetConnectAttr, PRIV_KEY_FILE from the connection string is not used.
    let attr_key_set = connection
        .pre_connection_attrs
        .contains_key(&ConnectionAttribute::PrivKeyContent)
        || connection
            .pre_connection_attrs
            .contains_key(&ConnectionAttribute::PrivKeyBase64);

    let mut login_timeout_set = false;

    for (key, value) in connection_string_map {
        if key == "DRIVER" {
            continue;
        }

        if let Some(core_key) = PARAM_MAPPINGS
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
        {
            DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
                conn_handle: Some(conn_handle),
                key: core_key.to_owned(),
                value,
            })?;
            continue;
        }

        match key.as_str() {
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
            "CRL_MODE" => {
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "crl_mode".to_owned(),
                        value: value.to_uppercase(),
                    },
                )?;
            }
            "LOGIN_TIMEOUT" => {
                login_timeout_set = true;
                DatabaseDriverClient::connection_set_option_string(
                    ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: "authentication_timeout".to_owned(),
                        value,
                    },
                )?;
            }
            "PRIV_KEY_FILE" => {
                if attr_key_set {
                    tracing::debug!(
                        "driver_connect: skipping PRIV_KEY_FILE — attribute-based key takes priority"
                    );
                } else {
                    DatabaseDriverClient::connection_set_option_string(
                        ConnectionSetOptionStringRequest {
                            conn_handle: Some(conn_handle),
                            key: "private_key_file".to_owned(),
                            value,
                        },
                    )?;
                }
            }
            "PRIV_KEY_BASE64" => {
                if attr_key_set {
                    tracing::debug!(
                        "driver_connect: skipping PRIV_KEY_BASE64 — attribute-based key takes priority"
                    );
                } else {
                    DatabaseDriverClient::connection_set_option_string(
                        ConnectionSetOptionStringRequest {
                            conn_handle: Some(conn_handle),
                            key: "private_key".to_owned(),
                            value,
                        },
                    )?;
                }
            }
            "PRIV_KEY_FILE_PWD" | "PRIV_KEY_PWD" => {
                if connection
                    .pre_connection_attrs
                    .contains_key(&ConnectionAttribute::PrivKeyPassword)
                {
                    tracing::debug!(
                        "driver_connect: skipping {} — attribute-based password takes priority",
                        key
                    );
                } else {
                    DatabaseDriverClient::connection_set_option_string(
                        ConnectionSetOptionStringRequest {
                            conn_handle: Some(conn_handle),
                            key: "private_key_password".to_owned(),
                            value,
                        },
                    )?;
                }
            }
            _ => {
                tracing::warn!("driver_connect: unknown connection string key: {:?}", key);
            }
        }
    }

    // Apply SQLSetConnectAttr values (override connection string parameters).
    let login_timeout_from_attr = apply_pre_connection_attrs(connection, conn_handle)?;

    // Old driver defaults LOGIN_TIMEOUT to 300 s (S_DEFAULT_LOGIN_TIMEOUT).
    // If neither the connection string nor SQLSetConnectAttr provided a value,
    // apply the same default so sf_core's Okta SAML retry budget matches.
    if !login_timeout_set && !login_timeout_from_attr {
        DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
            conn_handle: Some(conn_handle),
            key: "authentication_timeout".to_owned(),
            value: DEFAULT_LOGIN_TIMEOUT_SECS.to_owned(),
        })?;
    }

    DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
        conn_handle: Some(conn_handle),
        key: "client_app_id".to_owned(),
        value: "ODBC".to_owned(),
    })?;

    DatabaseDriverClient::connection_init(ConnectionInitRequest {
        conn_handle: Some(conn_handle),
        db_handle: Some(db_handle),
    })?;

    tracing::info!("driver_connect: connection_init completed");

    connection.state = ConnectionState::Connected {
        db_handle,
        conn_handle,
    };

    Ok(())
}

/// Apply pre-connection attributes to sf_core. SQLSetConnectAttr values override
/// connection string parameters. PrivKeyContent takes priority over PrivKeyBase64.
/// Returns `true` if LoginTimeout was set via attributes.
fn apply_pre_connection_attrs(
    connection: &mut crate::api::Connection,
    conn_handle: ConnectionHandle,
) -> OdbcResult<bool> {
    let attrs = &connection.pre_connection_attrs;

    if let Some(content) = attrs.get(&ConnectionAttribute::PrivKeyContent) {
        // PrivKeyContent -> private_key (PEM string sent as base64 to core)
        use base64::{Engine as _, engine::general_purpose};
        let encoded = general_purpose::STANDARD.encode(content.as_bytes());
        DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
            conn_handle: Some(conn_handle),
            key: "private_key".to_owned(),
            value: encoded,
        })?;
    } else if let Some(base64_key) = attrs.get(&ConnectionAttribute::PrivKeyBase64) {
        // PrivKeyBase64 -> private_key (already base64-encoded)
        DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
            conn_handle: Some(conn_handle),
            key: "private_key".to_owned(),
            value: base64_key.clone(),
        })?;
    }

    // PrivKeyPassword -> private_key_password
    if let Some(password) = attrs.get(&ConnectionAttribute::PrivKeyPassword) {
        DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
            conn_handle: Some(conn_handle),
            key: "private_key_password".to_owned(),
            value: password.clone(),
        })?;
    }

    // Application -> application
    if let Some(app) = attrs.get(&ConnectionAttribute::Application) {
        DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
            conn_handle: Some(conn_handle),
            key: "application".to_owned(),
            value: app.clone(),
        })?;
    }

    // LoginTimeout -> authentication_timeout (matches old driver: used as Okta SAML budget)
    if let Some(timeout) = attrs.get(&ConnectionAttribute::LoginTimeout) {
        DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
            conn_handle: Some(conn_handle),
            key: "authentication_timeout".to_owned(),
            value: timeout.clone(),
        })?;
        return Ok(true);
    }

    Ok(false)
}

/// Simple connect function (SQLConnect) - currently a placeholder.
///
/// The caller (c_api.rs) is responsible for decoding the raw C strings
/// using the encoding module before calling this function.
pub fn connect(
    _connection_handle: sql::Handle,
    _server_name: &str,
    _user_name: Option<&str>,
    _authentication: Option<&str>,
) -> OdbcResult<()> {
    tracing::debug!("connect: currently a placeholder implementation");
    // TODO: Implement proper SQLConnect functionality
    Ok(())
}

/// Disconnect from the database
pub fn disconnect(connection_handle: sql::Handle) -> OdbcResult<()> {
    tracing::debug!("disconnect: disconnecting from database");

    let connection = conn_from_handle(connection_handle);
    if let ConnectionState::Connected {
        db_handle,
        conn_handle,
    } = std::mem::replace(&mut connection.state, ConnectionState::Disconnected)
    {
        if let Err(e) = DatabaseDriverClient::connection_release(ConnectionReleaseRequest {
            conn_handle: Some(conn_handle),
        }) {
            tracing::warn!("Failed to release core connection handle: {e:?}");
        }
        if let Err(e) = DatabaseDriverClient::database_release(DatabaseReleaseRequest {
            db_handle: Some(db_handle),
        }) {
            tracing::warn!("Failed to release core database handle: {e:?}");
        }
    }

    Ok(())
}

/// Set a connection attribute (SQLSetConnectAttr).
///
/// `string_value` is the pre-decoded string for string-typed attributes;
/// the caller (c_api.rs) decodes it from the raw C pointer using the
/// encoding module. For integer-typed attributes it is `None` and
/// `value_ptr` is used directly.
// TODO: Clear sensitive pre_connection_attrs after apply_pre_connection_attrs.
pub fn set_connect_attr(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    string_value: Option<&str>,
) -> OdbcResult<()> {
    let connection = conn_from_handle(connection_handle);
    tracing::debug!("set_connect_attr: attribute={attribute}");

    let attr = match ConnectionAttribute::from_raw(attribute) {
        Some(a) => a,
        None if ConnectionAttribute::is_snowflake_custom(attribute) => {
            return UnknownAttributeSnafu { attribute }.fail();
        }
        None => {
            tracing::debug!("set_connect_attr: ignoring standard attribute {attribute}");
            return Ok(());
        }
    };

    match attr {
        // Standard ODBC attributes
        ConnectionAttribute::LoginTimeout => {
            // Matches old driver: LOGIN_TIMEOUT is reused as the Okta SAML retry budget.
            // SQL_ATTR_LOGIN_TIMEOUT is an integer attribute: the value is passed as
            // (SQLPOINTER)(uintptr_t)seconds, not as a string pointer.
            if matches!(connection.state, ConnectionState::Connected { .. }) {
                return AttributeCannotBeSetNowSnafu {
                    attribute: attr.as_raw(),
                }
                .fail();
            }
            let seconds = value_ptr as usize;
            tracing::debug!("set_connect_attr: LoginTimeout={seconds}");
            connection
                .pre_connection_attrs
                .insert(attr, seconds.to_string());
            Ok(())
        }
        ConnectionAttribute::ConnectionTimeout => {
            tracing::debug!("set_connect_attr: ConnectionTimeout (ignored)");
            Ok(())
        }
        ConnectionAttribute::Autocommit => {
            tracing::debug!("set_connect_attr: Autocommit (ignored)");
            Ok(())
        }

        ConnectionAttribute::PrivKey => {
            tracing::warn!(
                "set_connect_attr: PrivKey (EVP_PKEY pointer) is not supported. \
                 Use PrivKeyContent or PrivKeyBase64 instead."
            );
            UnsupportedAttributeSnafu {
                attribute: attr.as_raw(),
            }
            .fail()
        }
        ConnectionAttribute::PrivKeyContent
        | ConnectionAttribute::PrivKeyPassword
        | ConnectionAttribute::PrivKeyBase64
        | ConnectionAttribute::Application => {
            if matches!(connection.state, ConnectionState::Connected { .. }) {
                return AttributeCannotBeSetNowSnafu {
                    attribute: attr.as_raw(),
                }
                .fail();
            }
            let value = string_value.unwrap_or("").to_string();
            tracing::debug!("set_connect_attr: {attr:?} (set)");
            connection.pre_connection_attrs.insert(attr, value);
            Ok(())
        }
    }
}

/// Get a connection attribute (SQLGetConnectAttr).
///
/// Returns the attribute value; the caller (c_api.rs) is responsible for
/// writing it to the output buffer and handling truncation.
pub fn get_connect_attr(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
) -> OdbcResult<FieldValue> {
    let connection = conn_from_handle(connection_handle);
    tracing::debug!("get_connect_attr: attribute={attribute}");

    let attr = match ConnectionAttribute::from_raw(attribute) {
        Some(a) => a,
        None => {
            tracing::warn!("get_connect_attr: unknown attribute {attribute}");
            return UnknownAttributeSnafu { attribute }.fail();
        }
    };

    match attr {
        ConnectionAttribute::PrivKeyContent
        | ConnectionAttribute::PrivKeyPassword
        | ConnectionAttribute::PrivKeyBase64
        | ConnectionAttribute::Application => {
            let value = connection
                .pre_connection_attrs
                .get(&attr)
                .cloned()
                .unwrap_or_default();
            Ok(FieldValue::String(value))
        }
        ConnectionAttribute::Autocommit => Ok(FieldValue::ULen(SQL_AUTOCOMMIT_ON)),
        ConnectionAttribute::LoginTimeout => {
            let timeout: sql::ULen = match connection.pre_connection_attrs.get(&attr) {
                Some(s) => s.parse().unwrap_or_else(|_| {
                    tracing::warn!(
                        "get_connect_attr: LoginTimeout value {s:?} is not a valid integer, \
                         returning default {DEFAULT_LOGIN_TIMEOUT_SECS}",
                    );
                    DEFAULT_LOGIN_TIMEOUT_SECS.parse().unwrap()
                }),
                None => DEFAULT_LOGIN_TIMEOUT_SECS.parse().unwrap(),
            };
            Ok(FieldValue::ULen(timeout))
        }
        ConnectionAttribute::ConnectionTimeout => Ok(FieldValue::ULen(0)),
        ConnectionAttribute::PrivKey => UnsupportedAttributeSnafu {
            attribute: attr.as_raw(),
        }
        .fail(),
    }
}

/// Retrieve general information about the driver and data source (SQLGetInfo).
///
/// Returns the info value; the caller (c_api.rs) is responsible for writing
/// it to the output buffer.
pub fn get_info(
    connection_handle: sql::Handle,
    info_type: sql::USmallInt,
) -> OdbcResult<FieldValue> {
    tracing::debug!("get_info: connection_handle={connection_handle:?}, info_type={info_type}");

    let _conn = conn_from_handle(connection_handle);

    let info_type = InfoType::try_from(info_type)?;

    match info_type {
        InfoType::CursorCommitBehavior | InfoType::CursorRollbackBehavior => {
            Ok(FieldValue::USmallInt(1)) // SQL_CB_CLOSE
        }
        InfoType::DriverOdbcVer => Ok(FieldValue::String("03.00".to_string())),
        InfoType::GetDataExtensions => {
            let extensions = [
                GetDataExtensions::AnyColumn,
                GetDataExtensions::AnyOrder,
                GetDataExtensions::Bound,
            ];
            Ok(FieldValue::UInteger(extensions.bitmask()))
        }
    }
}
