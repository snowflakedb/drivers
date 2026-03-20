use crate::api::InfoType;
use crate::api::bitmask::Bitmask;
use crate::api::dsn::load_dsn_config;
use crate::api::encoding::{
    OdbcEncoding, read_string_from_pointer, write_string_bytes, write_string_bytes_i32,
};
use crate::api::error::Required;
use crate::api::error::{
    AttributeCannotBeSetNowSnafu, DsnNotFoundSnafu, InvalidPortSnafu, OdbcRuntimeSnafu,
    UnknownAttributeSnafu, UnsupportedAttributeSnafu,
};
use crate::api::runtime::global;
use crate::api::{
    ConnectionState, GetDataExtensions, OdbcResult, conn_from_handle, types::ConnectionAttribute,
};
use crate::conversion::warning::Warnings;
use odbc_sys as sql;
use sf_core::protobuf::generated::database_driver_v1::*;
use snafu::ResultExt;
use std::collections::HashMap;
use tracing;

const SQL_AUTOCOMMIT_ON: sql::ULen = 1;

/// Default login timeout in seconds, matching the old driver's S_DEFAULT_LOGIN_TIMEOUT.
/// Used as the Okta SAML retry budget when neither the connection string nor
/// SQLSetConnectAttr provides a value.
const DEFAULT_LOGIN_TIMEOUT_SECS: &str = "300";

/// Browse-connect template returned when required attributes are missing.
const BROWSE_CONNECT_TEMPLATE: &str = "*SERVER:Server=?;ACCOUNT:Account=?;*UID:UID=?;*PWD:PWD=?;\
     DATABASE:Database=?;WAREHOUSE:Warehouse=?;ROLE:Role=?;SCHEMA:Schema=?;";

/// Maps ODBC connection string parameter names to their sf_core equivalents.
/// Parameters listed here are forwarded as-is via `connection_set_option_string`.
/// Parameters that need special handling (type conversion, conditional skipping,
/// side-effects) are handled separately in `apply_connection_attrs_to_core`.
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

/// Parse a semicolon-separated ODBC connection string into a key/value map.
/// Keys are normalised to uppercase; empty pairs are ignored.
fn parse_connection_string(connection_string: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in connection_string.split(';') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        if let Some(eq) = pair.find('=') {
            let key = pair[..eq].trim().to_uppercase();
            let val = pair[eq + 1..].trim().to_string();
            if !key.is_empty() {
                map.insert(key, val);
            }
        }
    }
    map
}

/// Merge DSN attributes with connection-string attributes.
/// Connection-string values win on key conflicts (matching ODBC spec and old driver behaviour).
fn merge_attrs(
    dsn_attrs: HashMap<String, String>,
    conn_str_attrs: HashMap<String, String>,
) -> HashMap<String, String> {
    let mut merged = dsn_attrs;
    for (k, v) in conn_str_attrs {
        merged.insert(k, v);
    }
    merged
}

/// Read a string from an ODBC input pointer, returning an empty string if the
/// pointer is null (rather than an error).  Used for optional arguments such as
/// the UID / PWD in `SQLConnect`.
fn read_optional_string<E: OdbcEncoding>(
    ptr: *const E::Char,
    length: sql::SmallInt,
) -> OdbcResult<String> {
    if ptr.is_null() {
        return Ok(String::new());
    }
    E::read_string(ptr, length as i32)
}

// ─── Public entry points ──────────────────────────────────────────────────────

/// `SQLDriverConnect` / `SQLDriverConnectW` — connect using an inline connection string.
pub fn driver_connect<E: OdbcEncoding>(
    connection_handle: sql::Handle,
    in_connection_string: *const E::Char,
    in_string_length: sql::SmallInt,
    out_connection_string: *mut E::Char,
    out_buffer_length: sql::SmallInt,
    out_string_length: *mut sql::SmallInt,
) -> OdbcResult<()> {
    let connection_string = E::read_string(in_connection_string, in_string_length as i32)?;
    let completed = driver_connect_core(connection_handle, &connection_string)?;
    write_string_bytes::<E>(
        &completed,
        out_connection_string,
        out_buffer_length,
        out_string_length,
        None,
    );
    Ok(())
}

/// `SQLConnect` / `SQLConnectW` — connect via DSN + explicit user/password.
pub fn connect<E: OdbcEncoding>(
    connection_handle: sql::Handle,
    server_name: *const E::Char,
    name_length1: sql::SmallInt,
    user_name: *const E::Char,
    name_length2: sql::SmallInt,
    authentication: *const E::Char,
    name_length3: sql::SmallInt,
) -> OdbcResult<()> {
    let dsn = read_optional_string::<E>(server_name, name_length1)?;
    let uid = read_optional_string::<E>(user_name, name_length2)?;
    let pwd = read_optional_string::<E>(authentication, name_length3)?;

    tracing::info!("connect: DSN={:?} UID={:?}", dsn, uid);

    // Build a synthetic connection string and delegate to the shared core.
    let mut conn_str = String::new();
    if !dsn.is_empty() {
        conn_str.push_str(&format!("DSN={dsn};"));
    }
    if !uid.is_empty() {
        conn_str.push_str(&format!("UID={uid};"));
    }
    if !pwd.is_empty() {
        conn_str.push_str(&format!("PWD={pwd};"));
    }

    driver_connect_core(connection_handle, &conn_str)?;
    Ok(())
}

/// Outcome of a `SQLBrowseConnect` call.
pub enum BrowseConnectOutcome {
    /// Connection established.
    Connected,
    /// More attributes needed.
    NeedData,
}

/// `SQLBrowseConnect` / `SQLBrowseConnectW`.
pub fn browse_connect<E: OdbcEncoding>(
    connection_handle: sql::Handle,
    in_connection_string: *const E::Char,
    in_string_length: sql::SmallInt,
    out_connection_string: *mut E::Char,
    out_buffer_length: sql::SmallInt,
    out_string_length: *mut sql::SmallInt,
) -> OdbcResult<BrowseConnectOutcome> {
    let input = E::read_string(in_connection_string, in_string_length as i32)?;
    let new_attrs = parse_connection_string(&input);

    // Accumulate new attributes and snapshot — then release the borrow before
    // calling driver_connect_core (which also calls conn_from_handle internally).
    let (has_server, conn_str) = {
        let connection = conn_from_handle(connection_handle);
        for (k, v) in new_attrs {
            connection.browse_connect_attrs.insert(k, v);
        }
        let accumulated = &connection.browse_connect_attrs;
        let has_server = accumulated.contains_key("SERVER")
            || accumulated.contains_key("ACCOUNT")
            || accumulated.contains_key("DSN");
        let cs: String = accumulated
            .iter()
            .map(|(k, v)| format!("{k}={v};"))
            .collect();
        (has_server, cs)
    };

    if !has_server {
        tracing::debug!("browse_connect: insufficient attrs, returning NEED_DATA");
        write_string_bytes::<E>(
            BROWSE_CONNECT_TEMPLATE,
            out_connection_string,
            out_buffer_length,
            out_string_length,
            None,
        );
        return Ok(BrowseConnectOutcome::NeedData);
    }

    // Attempt the connection with the accumulated attributes.
    match driver_connect_core(connection_handle, &conn_str) {
        Ok(completed) => {
            // Reset accumulated state on success.
            conn_from_handle(connection_handle)
                .browse_connect_attrs
                .clear();
            write_string_bytes::<E>(
                &completed,
                out_connection_string,
                out_buffer_length,
                out_string_length,
                None,
            );
            Ok(BrowseConnectOutcome::Connected)
        }
        Err(e) => Err(e),
    }
}

// ─── Internal connection pipeline ────────────────────────────────────────────

/// Shared connection implementation used by all three connection entry points.
/// Returns the completed connection string (for output buffer writes).
fn driver_connect_core(
    connection_handle: sql::Handle,
    connection_string: &str,
) -> OdbcResult<String> {
    let mut conn_str_attrs = parse_connection_string(connection_string);

    // Log connection string with sensitive values redacted.
    {
        const REDACTED_KEYS: &[&str] = &[
            "PWD",
            "TOKEN",
            "PRIV_KEY_FILE_PWD",
            "PRIV_KEY_PWD",
            "PRIV_KEY_BASE64",
        ];
        let redacted: HashMap<&String, &str> = conn_str_attrs
            .iter()
            .map(|(k, v)| {
                let sensitive = REDACTED_KEYS.iter().any(|r| k.eq_ignore_ascii_case(r));
                (k, if sensitive { "****" } else { v.as_str() })
            })
            .collect();
        tracing::info!("driver_connect_core: connection_string={:?}", redacted);
    }

    // If a DSN is referenced, load its attributes and merge (conn string wins).
    let merged_attrs = if let Some(dsn_name) = conn_str_attrs.remove("DSN") {
        tracing::debug!("driver_connect_core: resolving DSN {:?}", dsn_name);
        match load_dsn_config(&dsn_name) {
            Some(dsn_attrs) => {
                let mut m = merge_attrs(dsn_attrs, conn_str_attrs);
                // Preserve the DSN name in the merged map for reference.
                m.insert("DSN".to_string(), dsn_name);
                m
            }
            None => {
                tracing::warn!("driver_connect_core: DSN {:?} not found", dsn_name);
                return DsnNotFoundSnafu { dsn: dsn_name }.fail();
            }
        }
    } else {
        conn_str_attrs
    };

    // Build a completed connection string from the merged attributes for the
    // output buffer (sensitive values excluded).
    let completed_conn_str = build_completed_connection_string(&merged_attrs);

    let connection = conn_from_handle(connection_handle);

    let attr_key_set = connection
        .pre_connection_attrs
        .contains_key(&ConnectionAttribute::PrivKeyContent)
        || connection
            .pre_connection_attrs
            .contains_key(&ConnectionAttribute::PrivKeyBase64);

    let attr_has_priv_key_password = connection
        .pre_connection_attrs
        .contains_key(&ConnectionAttribute::PrivKeyPassword);

    let pre_attrs = connection.pre_connection_attrs.clone();

    let (db_handle, conn_handle) =
        global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
            let db_handle = c
                .database_new(DatabaseNewRequest {})
                .await?
                .db_handle
                .required("Database handle is required")?;
            let conn_handle = c
                .connection_new(ConnectionNewRequest {})
                .await?
                .conn_handle
                .required("Connection handle is required")?;

            let mut login_timeout_set = false;

            for (key, value) in &merged_attrs {
                if key == "DRIVER" || key == "DSN" || key == "DESCRIPTION" || key == "SSL" || key == "LOCALE" || key == "TRACING" {
                    continue;
                }

                if let Some(core_key) = PARAM_MAPPINGS
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(key))
                    .map(|(_, v)| *v)
                {
                    c.connection_set_option_string(ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: core_key.to_owned(),
                        value: value.clone(),
                    })
                    .await?;
                    continue;
                }

                match key.as_str() {
                    "PORT" => {
                        let port_int: i64 =
                            value.parse().context(InvalidPortSnafu { port: value.clone() })?;
                        c.connection_set_option_int(ConnectionSetOptionIntRequest {
                            conn_handle: Some(conn_handle),
                            key: "port".to_owned(),
                            value: port_int,
                        })
                        .await?;
                    }
                    "CRL_MODE" => {
                        c.connection_set_option_string(ConnectionSetOptionStringRequest {
                            conn_handle: Some(conn_handle),
                            key: "crl_mode".to_owned(),
                            value: value.to_uppercase(),
                        })
                        .await?;
                    }
                    "LOGIN_TIMEOUT" => {
                        login_timeout_set = true;
                        c.connection_set_option_string(ConnectionSetOptionStringRequest {
                            conn_handle: Some(conn_handle),
                            key: "authentication_timeout".to_owned(),
                            value: value.clone(),
                        })
                        .await?;
                    }
                    "PRIV_KEY_FILE" => {
                        if attr_key_set {
                            tracing::debug!(
                                "driver_connect_core: skipping PRIV_KEY_FILE — attribute-based key takes priority"
                            );
                        } else {
                            c.connection_set_option_string(ConnectionSetOptionStringRequest {
                                conn_handle: Some(conn_handle),
                                key: "private_key_file".to_owned(),
                                value: value.clone(),
                            })
                            .await?;
                        }
                    }
                    "PRIV_KEY_BASE64" => {
                        if attr_key_set {
                            tracing::debug!(
                                "driver_connect_core: skipping PRIV_KEY_BASE64 — attribute-based key takes priority"
                            );
                        } else {
                            c.connection_set_option_string(ConnectionSetOptionStringRequest {
                                conn_handle: Some(conn_handle),
                                key: "private_key".to_owned(),
                                value: value.clone(),
                            })
                            .await?;
                        }
                    }
                    "PRIV_KEY_FILE_PWD" | "PRIV_KEY_PWD" => {
                        if attr_has_priv_key_password {
                            tracing::debug!(
                                "driver_connect_core: skipping {key} — attribute-based password takes priority"
                            );
                        } else {
                            c.connection_set_option_string(ConnectionSetOptionStringRequest {
                                conn_handle: Some(conn_handle),
                                key: "private_key_password".to_owned(),
                                value: value.clone(),
                            })
                            .await?;
                        }
                    }
                    _ => {
                        tracing::warn!(
                            "driver_connect_core: unknown connection string key: {key:?}"
                        );
                    }
                }
            }

            let login_timeout_from_attr =
                apply_pre_connection_attrs_async(c, &pre_attrs, conn_handle).await?;

            if !login_timeout_set && !login_timeout_from_attr {
                c.connection_set_option_string(ConnectionSetOptionStringRequest {
                    conn_handle: Some(conn_handle),
                    key: "authentication_timeout".to_owned(),
                    value: DEFAULT_LOGIN_TIMEOUT_SECS.to_owned(),
                })
                .await?;
            }

            c.connection_set_option_string(ConnectionSetOptionStringRequest {
                conn_handle: Some(conn_handle),
                key: "client_app_id".to_owned(),
                value: "ODBC".to_owned(),
            })
            .await?;

            c.connection_init(ConnectionInitRequest {
                conn_handle: Some(conn_handle),
                db_handle: Some(db_handle),
            })
            .await?;

            Ok::<_, crate::api::error::OdbcError>((db_handle, conn_handle))
        })?;

    tracing::info!("driver_connect_core: connection_init completed");

    conn_from_handle(connection_handle).state = ConnectionState::Connected {
        db_handle,
        conn_handle,
    };

    Ok(completed_conn_str)
}

/// Build a sanitised completed connection string from the merged attribute map.
/// Sensitive keys (password, tokens, private keys) are excluded.
fn build_completed_connection_string(attrs: &HashMap<String, String>) -> String {
    const EXCLUDE: &[&str] = &[
        "PWD",
        "TOKEN",
        "PRIV_KEY_FILE_PWD",
        "PRIV_KEY_PWD",
        "PRIV_KEY_BASE64",
    ];
    let mut parts: Vec<String> = attrs
        .iter()
        .filter(|(k, _)| !EXCLUDE.iter().any(|e| k.eq_ignore_ascii_case(e)))
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    parts.sort(); // deterministic order
    parts.join(";")
}

/// Apply pre-connection attributes to sf_core. SQLSetConnectAttr values override
/// connection string parameters. PrivKeyContent takes priority over PrivKeyBase64.
/// Returns `true` if LoginTimeout was set via attributes.
async fn apply_pre_connection_attrs_async(
    client: &sf_core::protobuf::apis::database_driver_v1::DatabaseDriverClient,
    attrs: &HashMap<ConnectionAttribute, String>,
    conn_handle: ConnectionHandle,
) -> OdbcResult<bool> {
    if let Some(content) = attrs.get(&ConnectionAttribute::PrivKeyContent) {
        use base64::{Engine as _, engine::general_purpose};
        let encoded = general_purpose::STANDARD.encode(content.as_bytes());
        client
            .connection_set_option_string(ConnectionSetOptionStringRequest {
                conn_handle: Some(conn_handle),
                key: "private_key".to_owned(),
                value: encoded,
            })
            .await?;
    } else if let Some(base64_key) = attrs.get(&ConnectionAttribute::PrivKeyBase64) {
        client
            .connection_set_option_string(ConnectionSetOptionStringRequest {
                conn_handle: Some(conn_handle),
                key: "private_key".to_owned(),
                value: base64_key.clone(),
            })
            .await?;
    }

    if let Some(password) = attrs.get(&ConnectionAttribute::PrivKeyPassword) {
        client
            .connection_set_option_string(ConnectionSetOptionStringRequest {
                conn_handle: Some(conn_handle),
                key: "private_key_password".to_owned(),
                value: password.clone(),
            })
            .await?;
    }

    if let Some(app) = attrs.get(&ConnectionAttribute::Application) {
        client
            .connection_set_option_string(ConnectionSetOptionStringRequest {
                conn_handle: Some(conn_handle),
                key: "application".to_owned(),
                value: app.clone(),
            })
            .await?;
    }

    if let Some(timeout) = attrs.get(&ConnectionAttribute::LoginTimeout) {
        client
            .connection_set_option_string(ConnectionSetOptionStringRequest {
                conn_handle: Some(conn_handle),
                key: "authentication_timeout".to_owned(),
                value: timeout.clone(),
            })
            .await?;
        return Ok(true);
    }

    Ok(false)
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
        global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
            if let Err(e) = c
                .connection_release(ConnectionReleaseRequest {
                    conn_handle: Some(conn_handle),
                })
                .await
            {
                tracing::warn!("Failed to release core connection handle: {e:?}");
            }
            if let Err(e) = c
                .database_release(DatabaseReleaseRequest {
                    db_handle: Some(db_handle),
                })
                .await
            {
                tracing::warn!("Failed to release core database handle: {e:?}");
            }
        });
    }

    Ok(())
}

/// Set a connection attribute (SQLSetConnectAttr / SQLSetConnectAttrW).
// TODO: Clear sensitive pre_connection_attrs after apply_pre_connection_attrs.
pub fn set_connect_attr<E: OdbcEncoding>(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    string_length: sql::Integer,
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
        ConnectionAttribute::LoginTimeout => {
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
            let value = read_string_from_pointer::<E>(value_ptr, string_length)?;
            tracing::debug!("set_connect_attr: {attr:?} (set)");
            connection.pre_connection_attrs.insert(attr, value);
            Ok(())
        }
    }
}

/// Get a connection attribute (SQLGetConnectAttr / SQLGetConnectAttrW).
pub fn get_connect_attr<E: OdbcEncoding>(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
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
                .map(|s| s.as_str())
                .unwrap_or("");
            write_string_bytes_i32::<E>(
                value,
                value_ptr as *mut E::Char,
                buffer_length,
                string_length_ptr,
                Some(warnings),
            );
            Ok(())
        }
        ConnectionAttribute::Autocommit => {
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::ULen) = SQL_AUTOCOMMIT_ON;
                }
            }
            Ok(())
        }
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
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::ULen) = timeout;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = std::mem::size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        ConnectionAttribute::ConnectionTimeout => {
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::ULen) = 0;
                }
            }
            Ok(())
        }
        ConnectionAttribute::PrivKey => UnsupportedAttributeSnafu {
            attribute: attr.as_raw(),
        }
        .fail(),
    }
}

/// Retrieve general information about the driver and data source
/// (SQLGetInfo / SQLGetInfoW).
pub fn get_info<E: OdbcEncoding>(
    connection_handle: sql::Handle,
    info_type: sql::USmallInt,
    info_value_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("get_info: connection_handle={connection_handle:?}, info_type={info_type}");

    let _conn = conn_from_handle(connection_handle);

    let info_type = InfoType::try_from(info_type)?;
    tracing::debug!("get_info: info_type={info_type:?}");

    match info_type {
        InfoType::CursorCommitBehavior | InfoType::CursorRollbackBehavior => {
            let cb_close: u16 = 1;
            if !info_value_ptr.is_null() {
                unsafe {
                    *(info_value_ptr as *mut u16) = cb_close;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = std::mem::size_of::<u16>() as sql::SmallInt;
                }
            }
            Ok(())
        }
        InfoType::DriverOdbcVer => {
            write_string_bytes::<E>(
                "03.00",
                info_value_ptr as *mut E::Char,
                buffer_length,
                string_length_ptr,
                None,
            );
            Ok(())
        }
        InfoType::GetDataExtensions => {
            let extensions = [
                GetDataExtensions::AnyColumn,
                GetDataExtensions::AnyOrder,
                GetDataExtensions::Bound,
            ];
            if !info_value_ptr.is_null() {
                unsafe {
                    *(info_value_ptr as *mut u32) = extensions.bitmask();
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = std::mem::size_of::<u32>() as sql::SmallInt;
                }
            }
            Ok(())
        }
    }
}
