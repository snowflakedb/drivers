use crate::api::InfoType;
use crate::api::bitmask::Bitmask;
use crate::api::encoding::{
    OdbcEncoding, read_string_from_pointer, write_string_bytes, write_string_bytes_i32,
};
use crate::api::error::Required;
use crate::api::error::{
    AttributeCannotBeSetNowSnafu, DataSourceNotFoundSnafu, DisconnectedSnafu,
    InvalidAttributeValueSnafu, InvalidBufferLengthSnafu, InvalidCatalogNameSnafu,
    InvalidConnectionStringSnafu, InvalidCursorStateSnafu, InvalidPortSnafu, NullPointerSnafu,
    OdbcRuntimeSnafu, ReadOnlyAttributeSnafu, UnknownAttributeSnafu, UnsupportedAttributeSnafu,
};
use crate::api::runtime::global;
use crate::api::{
    ConnectionState, GetDataExtensions, OdbcResult, conn_from_handle,
    types::{AccessMode, AutocommitValue, ConnectionAttribute, StatementState},
};
use crate::conversion::warning::{Warning, Warnings};
use odbc_sys as sql;
use sf_core::protobuf::generated::database_driver_v1::*;
use snafu::ResultExt;
use std::collections::HashMap;
use tracing;

const SQL_TXN_READ_COMMITTED: sql::UInteger = 2;
const SQL_CD_FALSE: sql::UInteger = 0;
const SQL_CD_TRUE: sql::UInteger = 1;
const SQL_FALSE: sql::UInteger = 0;

const ODBC_DRIVER_NAME: &str = "ODBC";
const ODBC_DRIVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default login timeout in seconds, matching the old driver's S_DEFAULT_LOGIN_TIMEOUT.
/// Used as the Okta SAML retry budget when neither the connection string nor
/// SQLSetConnectAttr provides a value.
const DEFAULT_LOGIN_TIMEOUT_SECS: &str = "300";

/// Normalizes `CRL_ENABLED` values to the uppercase mode strings `sf_core` accepts for
/// `crl_check_mode` (see `build_crl_config` in `connection_config.rs`).
fn normalize_crl_enabled_value(value: &str) -> String {
    let v = value.trim();
    if v.eq_ignore_ascii_case("true") || v == "1" {
        "ENABLED".to_owned()
    } else if v.eq_ignore_ascii_case("false") || v == "0" {
        "DISABLED".to_owned()
    } else {
        v.to_ascii_uppercase()
    }
}

fn normalize_connection_string_options(
    connection_string_map: HashMap<String, String>,
) -> HashMap<String, ConfigSetting> {
    connection_string_map
        .into_iter()
        .filter_map(|(key, value)| normalize_connection_string_option(key, value))
        .collect()
}

fn normalize_connection_string_option(
    key: String,
    value: String,
) -> Option<(String, ConfigSetting)> {
    let upper = key.to_ascii_uppercase();
    if upper == "DRIVER" {
        return None;
    }

    match upper.as_str() {
        "PORT" => Some(("port".to_owned(), value.into())),
        "CRL_MODE" => Some(("CRL_MODE".to_owned(), value.to_uppercase().into())),
        "CRL_ENABLED" => Some((
            "CRL_ENABLED".to_owned(),
            normalize_crl_enabled_value(&value).into(),
        )),
        "CLIENT_STORE_TEMPORARY_CREDENTIAL" => {
            Some(("client_store_temporary_credential".to_owned(), value.into()))
        }
        "LOGIN_TIMEOUT" => Some(("authentication_timeout".to_owned(), value.into())),
        "PASSCODEINPASSWORD" => Some(("passcodeInPassword".to_owned(), value.into())),
        "PRIV_KEY_FILE" => Some(("private_key_file".to_owned(), value.into())),
        "PRIV_KEY_BASE64" => Some(("private_key".to_owned(), value.into())),
        "PRIV_KEY_FILE_PWD" | "PRIV_KEY_PWD" => {
            Some(("private_key_password".to_owned(), value.into()))
        }
        // Forward other keys (e.g. SERVER, UID) for `sf_core` alias resolution; do not
        // pre-canonicalize here to avoid duplicate seed keys.
        _ => Some((upper, value.into())),
    }
}

/// Parse connection string into key-value pairs.
///
/// Supports brace-quoted values (e.g. `PWD={p@ss;word}`) where `}}` inside
/// braces is an escaped literal `}`. Rejects duplicate keys (case-insensitive)
/// and unterminated brace sequences.
fn parse_connection_string(connection_string: &str) -> OdbcResult<HashMap<String, String>> {
    let mut map = HashMap::new();
    let bytes = connection_string.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Skip whitespace and semicolons between pairs.
        while i < len && (bytes[i] == b';' || bytes[i].is_ascii_whitespace()) {
            i += 1;
        }
        if i >= len {
            break;
        }

        // Read key: accumulate until '='.
        let key_start = i;
        while i < len && bytes[i] != b'=' {
            i += 1;
        }
        if i >= len {
            // No '=' found — skip this trailing segment (matches old behaviour).
            break;
        }
        let key = connection_string[key_start..i].trim().to_ascii_uppercase();
        i += 1; // skip '='

        // Read value.
        let value = if i < len && bytes[i] == b'{' {
            // Brace-quoted value.
            i += 1; // skip opening '{'
            let mut val = String::new();
            let mut seg_start = i;
            loop {
                if i >= len {
                    return InvalidConnectionStringSnafu {
                        reason: format!("unterminated brace in value for key: {key}"),
                    }
                    .fail();
                }
                if bytes[i] == b'}' {
                    val.push_str(&connection_string[seg_start..i]);
                    if i + 1 < len && bytes[i + 1] == b'}' {
                        // Escaped '}}' → literal '}'.
                        val.push('}');
                        i += 2;
                        seg_start = i;
                    } else {
                        // Closing brace.
                        i += 1;
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            // After closing '}', expect ';' or end-of-string (skip whitespace).
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < len && bytes[i] != b';' {
                return InvalidConnectionStringSnafu {
                    reason: format!("unexpected character after closing brace for key: {key}"),
                }
                .fail();
            }
            val
        } else {
            // Unbraced value: accumulate until ';' or end-of-string.
            let val_start = i;
            while i < len && bytes[i] != b';' {
                i += 1;
            }
            connection_string[val_start..i].trim().to_string()
        };

        if key.is_empty() {
            continue;
        }

        if map.contains_key(&key) {
            return InvalidConnectionStringSnafu {
                reason: format!("duplicate key: {key}"),
            }
            .fail();
        }
        map.insert(key, value);
    }

    Ok(map)
}

/// Connect using connection string (SQLDriverConnect / SQLDriverConnectW).
pub fn driver_connect<E: OdbcEncoding>(
    connection_handle: sql::Handle,
    in_connection_string: *const E::Char,
    in_string_length: sql::SmallInt,
) -> OdbcResult<()> {
    let connection_string = E::read_string(in_connection_string, in_string_length as i32)?;
    let params = parse_connection_string(&connection_string)?;
    connect_with_params(connection_handle, params)
}

/// Core connection logic shared by `driver_connect` and `connect`.
///
/// Takes the already-parsed parameter map, applies it to a new sf_core connection,
/// respects pre-connection attributes set via `SQLSetConnectAttr`, and transitions
/// the handle to `Connected`.
fn connect_with_params(
    connection_handle: sql::Handle,
    params: HashMap<String, String>,
) -> OdbcResult<()> {
    {
        const REDACTED_KEYS: &[&str] = &[
            "PWD",
            "TOKEN",
            "PRIV_KEY_FILE_PWD",
            "PRIV_KEY_PWD",
            "PRIV_KEY_BASE64",
            "PASSCODE",
        ];
        let redacted_map: HashMap<&String, &str> = params
            .iter()
            .map(|(k, v)| {
                let is_sensitive = REDACTED_KEYS.iter().any(|r| k.eq_ignore_ascii_case(r));
                let v = if is_sensitive { "****" } else { v.as_str() };
                (k, v)
            })
            .collect();
        tracing::info!("connect_with_params: params={:?}", redacted_map);
    }

    let mut options = normalize_connection_string_options(params);
    if let Some(config_setting::Value::StringValue(raw_port)) = options
        .get("port")
        .and_then(|setting| setting.value.as_ref())
    {
        let port_int: i64 = raw_port.parse().context(InvalidPortSnafu {
            port: raw_port.clone(),
        })?;
        options.insert("port".to_owned(), port_int.into());
    }

    let dbc = conn_from_handle(connection_handle)?;
    // Read pre-connection data under lock, then release before the async call.
    let (pre_connection_attrs, login_timeout_in_options, login_timeout_in_attrs) = {
        let connection = dbc.connection.lock();
        apply_pre_connection_overrides(&connection.pre_connection_attrs, &mut options);
        let login_timeout_in_options = options.contains_key("authentication_timeout");
        let login_timeout_in_attrs = connection
            .pre_connection_attrs
            .contains_key(&ConnectionAttribute::LoginTimeout);
        let pre_connection_attrs = connection.pre_connection_attrs.clone();
        (
            pre_connection_attrs,
            login_timeout_in_options,
            login_timeout_in_attrs,
        )
    };

    let (db_handle, conn_handle) = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
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

        let response = c
            .connection_set_options(ConnectionSetOptionsRequest {
                conn_handle: Some(conn_handle),
                options,
            })
            .await?;

        for warning in &response.warnings {
            tracing::warn!("connection option warning: {}", warning.message);
        }

        // Optional default login timeout (Okta SAML budget).
        if !login_timeout_in_options && !login_timeout_in_attrs {
            let follow_up = HashMap::from([(
                "authentication_timeout".to_owned(),
                DEFAULT_LOGIN_TIMEOUT_SECS.to_owned().into(),
            )]);
            let response = c
                .connection_set_options(ConnectionSetOptionsRequest {
                    conn_handle: Some(conn_handle),
                    options: follow_up,
                })
                .await?;
            for warning in &response.warnings {
                tracing::warn!("connection option warning: {}", warning.message);
            }
        }

        apply_pre_connection_runtime_attrs_async(c, &pre_connection_attrs, conn_handle).await?;

        c.connection_init(ConnectionInitRequest {
            conn_handle: Some(conn_handle),
            db_handle: Some(db_handle),
            wrapper_identity: Some(WrapperIdentity {
                driver_name: Some(ODBC_DRIVER_NAME.to_string()),
                driver_version: Some(ODBC_DRIVER_VERSION.to_string()),
                language_runtime: None,
                language_version: None,
                language_compiler: None,
            }),
        })
        .await?;

        Ok::<_, crate::api::OdbcError>((db_handle, conn_handle))
    })?;

    tracing::info!("connect_with_params: connection_init completed");

    dbc.connection.lock().state = ConnectionState::Connected {
        db_handle,
        conn_handle,
    };

    // Fetch the initial catalog value. Failure here is non-fatal: the connection is
    // already established (state = Connected). Use warn-and-continue rather than `?`
    // to avoid returning an error after the state was set to Connected.
    // ConnectionHandle is Copy, so conn_handle is still accessible after the move above.
    let current_catalog = match global().context(OdbcRuntimeSnafu) {
        Ok(rt) => rt
            .block_on(async |c| {
                let info = c
                    .connection_get_info(ConnectionGetInfoRequest {
                        conn_handle: Some(conn_handle),
                        info_codes: vec![],
                        include_master_token: false,
                    })
                    .await?;
                Ok::<Option<String>, crate::api::OdbcError>(info.database)
            })
            .unwrap_or_else(|e| {
                tracing::warn!("connect_with_params: failed to fetch current catalog: {e:?}");
                None
            }),
        Err(e) => {
            tracing::warn!(
                "connect_with_params: runtime unavailable for initial catalog fetch: {e:?}"
            );
            None
        }
    };
    dbc.connection.lock().current_catalog = current_catalog;

    Ok(())
}

/// Apply SQLSetConnectAttr values as overrides into the canonical options map.
/// PrivKeyContent or PrivKeyBase64 take priority over private-key settings from
/// the connection string. PrivKeyPassword overrides private_key_password.
fn apply_pre_connection_overrides(
    attrs: &HashMap<ConnectionAttribute, String>,
    options: &mut HashMap<String, ConfigSetting>,
) {
    // PrivKeyContent or PrivKeyBase64 → canonical "private_key"
    // Suppresses connection-string private key sources.
    if let Some(content) = attrs.get(&ConnectionAttribute::PrivKeyContent) {
        use base64::{Engine as _, engine::general_purpose};
        let encoded = general_purpose::STANDARD.encode(content.as_bytes());
        options.insert("private_key".to_owned(), encoded.into());
        options.remove("private_key_file");
    } else if let Some(b64) = attrs.get(&ConnectionAttribute::PrivKeyBase64) {
        options.insert("private_key".to_owned(), b64.clone().into());
        options.remove("private_key_file");
    }

    // PrivKeyPassword overrides connection-string password keys.
    if let Some(pwd) = attrs.get(&ConnectionAttribute::PrivKeyPassword) {
        options.insert("private_key_password".to_owned(), pwd.clone().into());
    }

    // Application name
    if let Some(app) = attrs.get(&ConnectionAttribute::Application) {
        options.insert("application".to_owned(), app.clone().into());
    }

    // LoginTimeout -> authentication_timeout (matches old driver: used as Okta SAML budget)
    if let Some(timeout) = attrs.get(&ConnectionAttribute::LoginTimeout) {
        options.insert("authentication_timeout".to_owned(), timeout.clone().into());
    }
}

/// Apply pre-connection attributes that still require dedicated RPCs after
/// the canonical batch `ConnectionSetOptions` payload has been sent.
async fn apply_pre_connection_runtime_attrs_async(
    client: &sf_core::protobuf::apis::database_driver_v1::DatabaseDriverClient,
    attrs: &HashMap<ConnectionAttribute, String>,
    conn_handle: ConnectionHandle,
) -> OdbcResult<()> {
    if let Some(raw) = attrs.get(&ConnectionAttribute::Autocommit) {
        match raw
            .parse::<sql::UInteger>()
            .ok()
            .and_then(AutocommitValue::from_raw)
        {
            Some(val) => {
                client
                    .connection_set_autocommit(ConnectionSetAutocommitRequest {
                        conn_handle: Some(conn_handle),
                        autocommit: matches!(val, AutocommitValue::On),
                    })
                    .await?;
            }
            None => {
                tracing::warn!(
                    "apply_pre_connection_runtime_attrs_async: invalid cached autocommit value \
                     {raw:?}; skipping autocommit RPC to avoid silent promotion to ON"
                );
            }
        }
    }

    Ok(())
}

/// Connect using DSN (SQLConnect / SQLConnectW).
///
/// Reads DSN configuration from odbc.ini (ODBCINI env var, ~/.odbc.ini, or /etc/odbc.ini),
/// merges caller-supplied UID/PWD overrides, then delegates to `connect_with_params` to perform
/// the actual connection.
pub fn connect<E: OdbcEncoding>(
    connection_handle: sql::Handle,
    server_name: *const E::Char,
    name_length1: sql::SmallInt,
    user_name: *const E::Char,
    name_length2: sql::SmallInt,
    authentication: *const E::Char,
    name_length3: sql::SmallInt,
) -> OdbcResult<()> {
    let dsn = E::read_string(server_name, name_length1 as i32)?;

    let uid = if user_name.is_null() {
        None
    } else {
        let s = E::read_string(user_name, name_length2 as i32)?;
        if s.is_empty() { None } else { Some(s) }
    };

    let pwd = if authentication.is_null() {
        None
    } else {
        let s = E::read_string(authentication, name_length3 as i32)?;
        if s.is_empty() { None } else { Some(s) }
    };

    tracing::debug!("connect: dsn={:?}", dsn);

    let mut params = read_dsn_config(&dsn)?;

    // Caller-supplied UID/PWD override whatever is in the DSN.
    if let Some(uid) = uid {
        params.insert("UID".to_string(), uid);
    }
    if let Some(pwd) = pwd {
        params.insert("PWD".to_string(), pwd);
    }

    // Drop DSN metadata keys that have no meaning as connection parameters.
    params
        .retain(|k, _| !k.eq_ignore_ascii_case("Driver") && !k.eq_ignore_ascii_case("Description"));

    connect_with_params(connection_handle, params)
}

/// Look up DSN parameters.
///
/// On Unix: searches odbc.ini files (ODBCINI env var, ~/.odbc.ini, ODBCSYSINI/odbc.ini, /etc/odbc.ini).
/// On Windows: reads from the registry under HKCU then HKLM SOFTWARE\ODBC\ODBC.INI\<DSN>.
#[cfg(not(windows))]
fn read_dsn_config(dsn: &str) -> OdbcResult<HashMap<String, String>> {
    let mut paths = Vec::new();
    if let Ok(p) = std::env::var("ODBCINI") {
        paths.push(p);
    }
    if let Ok(home) = std::env::var("HOME") {
        paths.push(format!("{}/.odbc.ini", home));
    }
    if let Ok(p) = std::env::var("ODBCSYSINI") {
        paths.push(format!("{}/odbc.ini", p));
    }
    paths.push("/etc/odbc.ini".to_string());

    for path in &paths {
        if let Ok(content) = std::fs::read_to_string(path)
            && let Some(params) = parse_ini_section(&content, dsn)
        {
            tracing::debug!("connect: found DSN {:?} in {:?}", dsn, path);
            return Ok(params);
        }
    }
    tracing::warn!("connect: DSN {:?} not found in any odbc.ini", dsn);
    DataSourceNotFoundSnafu {
        dsn: dsn.to_string(),
    }
    .fail()
}

/// Parse an INI-format string and return the key/value pairs from `section`.
///
/// Section name matching is case-insensitive; returned keys are uppercased.
#[cfg(not(windows))]
fn parse_ini_section(content: &str, section: &str) -> Option<HashMap<String, String>> {
    let ini = ini::Ini::load_from_str_noescape(content).ok()?;
    let props = ini.iter().find_map(|(name, props)| {
        name.filter(|n| n.eq_ignore_ascii_case(section))
            .map(|_| props)
    })?;
    let params = props
        .iter()
        .map(|(k, v)| (k.to_uppercase(), v.to_string()))
        .collect();
    Some(params)
}

/// Look up DSN parameters from the Windows registry.
///
/// Checks HKEY_CURRENT_USER first (user DSNs), then HKEY_LOCAL_MACHINE (system DSNs),
/// mirroring the priority order used by the Windows ODBC Driver Manager.
#[cfg(windows)]
fn read_dsn_config(dsn: &str) -> OdbcResult<HashMap<String, String>> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::types::FromRegValue;

    const ODBC_INI: &str = "SOFTWARE\\ODBC\\ODBC.INI";

    for hive in [
        RegKey::predef(HKEY_CURRENT_USER),
        RegKey::predef(HKEY_LOCAL_MACHINE),
    ] {
        let path = format!("{}\\{}", ODBC_INI, dsn);
        if let Ok(key) = hive.open_subkey(&path) {
            let mut params = HashMap::new();
            for result in key.enum_values() {
                if let Ok((name, value)) = result {
                    if !name.is_empty() {
                        if let Ok(s) = String::from_reg_value(&value) {
                            params.insert(name.to_uppercase(), s);
                        }
                    }
                }
            }
            if !params.is_empty() {
                tracing::debug!("connect: found DSN {:?} in registry", dsn);
                return Ok(params);
            }
        }
    }
    tracing::warn!("connect: DSN {:?} not found in registry", dsn);
    DataSourceNotFoundSnafu {
        dsn: dsn.to_string(),
    }
    .fail()
}

/// Disconnect from the database, performing logout and releasing sf_core handles.
pub fn disconnect(connection_handle: sql::Handle) -> OdbcResult<()> {
    tracing::debug!("disconnect: disconnecting from database");

    let dbc = conn_from_handle(connection_handle)?;
    let mut connection = dbc.connection.lock();
    let (db_handle, conn_handle) = match &connection.state {
        ConnectionState::Connected {
            db_handle,
            conn_handle,
        } => (*db_handle, *conn_handle),
        ConnectionState::Disconnected => {
            return DisconnectedSnafu.fail();
        }
    };

    global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
        c.connection_close(ConnectionCloseRequest {
            conn_handle: Some(conn_handle),
        })
        .await?;
        c.connection_release(ConnectionReleaseRequest {
            conn_handle: Some(conn_handle),
        })
        .await?;
        c.database_release(DatabaseReleaseRequest {
            db_handle: Some(db_handle),
        })
        .await?;
        Ok::<_, crate::api::OdbcError>(())
    })?;

    connection.state = ConnectionState::Disconnected;
    Ok(())
}

/// Translate SQL text to its native form (SQLNativeSql / SQLNativeSqlW).
///
/// Snowflake does not perform ODBC escape sequence translation, so this is
/// a simple pass-through that copies the input SQL to the output buffer.
pub fn native_sql<E: OdbcEncoding>(
    connection_handle: sql::Handle,
    in_statement_text: *const E::Char,
    text_length1: sql::Integer,
    out_statement_text: *mut E::Char,
    buffer_length: sql::Integer,
    text_length2_ptr: *mut sql::Integer,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    tracing::debug!("native_sql: connection_handle={connection_handle:?}");

    if in_statement_text.is_null() {
        return NullPointerSnafu.fail();
    }
    if text_length1 != sql::NTS as sql::Integer && text_length1 < 0 {
        return InvalidBufferLengthSnafu {
            length: text_length1 as i64,
        }
        .fail();
    }
    if !out_statement_text.is_null() && buffer_length < 0 {
        return InvalidBufferLengthSnafu {
            length: buffer_length as i64,
        }
        .fail();
    }

    let dbc = conn_from_handle(connection_handle)?;
    if matches!(dbc.connection.lock().state, ConnectionState::Disconnected) {
        return crate::api::error::DisconnectedSnafu.fail();
    }

    let sql_text = if text_length1 == 0 {
        String::new()
    } else {
        E::read_string(in_statement_text, text_length1)?
    };

    write_string_bytes_i32::<E>(
        &sql_text,
        out_statement_text,
        buffer_length,
        text_length2_ptr,
        Some(warnings),
    );

    Ok(())
}

/// Query a session parameter from sf_core's cached session state.
fn get_session_parameter(conn_handle: &ConnectionHandle, key: &str) -> OdbcResult<Option<String>> {
    global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
        let resp = c
            .connection_get_parameter(ConnectionGetParameterRequest {
                conn_handle: Some(*conn_handle),
                key: key.to_string(),
            })
            .await?;
        Ok(resp.value)
    })
}

/// Set a connection attribute (SQLSetConnectAttr / SQLSetConnectAttrW).
// TODO: Clear sensitive pre_connection_attrs after apply_pre_connection_attrs.
pub fn set_connect_attr<E: OdbcEncoding>(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    string_length: sql::Integer,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    let dbc = conn_from_handle(connection_handle)?;
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

    let mut connection = dbc.connection.lock();
    match attr {
        ConnectionAttribute::AccessMode => {
            let mode = AccessMode::from_raw(value_ptr as sql::UInteger).ok_or_else(|| {
                InvalidAttributeValueSnafu {
                    attribute: attr.as_raw(),
                    value: value_ptr as i64,
                }
                .build()
            })?;
            connection.access_mode = mode;
            Ok(())
        }
        ConnectionAttribute::Autocommit => {
            let val = AutocommitValue::from_raw(value_ptr as sql::UInteger).ok_or_else(|| {
                InvalidAttributeValueSnafu {
                    attribute: attr.as_raw(),
                    value: value_ptr as i64,
                }
                .build()
            })?;
            // NOTE: Per ODBC spec, HY011 must be returned if a transaction is currently open.
            // Transaction state tracking requires server-side awareness — deferred to SNOW-3240589.
            let maybe_conn_handle = match &connection.state {
                ConnectionState::Connected { conn_handle, .. } => Some(*conn_handle),
                ConnectionState::Disconnected => None,
            };
            match maybe_conn_handle {
                Some(conn_handle) => {
                    let autocommit_on = matches!(val, AutocommitValue::On);
                    drop(connection);
                    global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
                        c.connection_set_autocommit(ConnectionSetAutocommitRequest {
                            conn_handle: Some(conn_handle),
                            autocommit: autocommit_on,
                        })
                        .await
                    })?;
                    let mut connection = dbc.connection.lock();
                    connection.cached_autocommit = val;
                    // Keep pre_connection_attrs in sync so a reconnect on the same handle
                    // re-applies the value set while connected rather than the stale pre-connect value.
                    connection
                        .pre_connection_attrs
                        .insert(attr, val.as_raw().to_string());
                    Ok(())
                }
                None => {
                    connection.cached_autocommit = val;
                    connection
                        .pre_connection_attrs
                        .insert(attr, val.as_raw().to_string());
                    Ok(())
                }
            }
        }
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
        ConnectionAttribute::TxnIsolation => {
            // Snowflake supports only READ_COMMITTED. Accept it silently; substitute any
            // other requested level with READ_COMMITTED and return 01S02 per ODBC spec.
            // NOTE: HY011 when a transaction is open is deferred to SNOW-3240589.
            if value_ptr as sql::UInteger != SQL_TXN_READ_COMMITTED {
                warnings.push(Warning::OptionValueChanged);
            }
            Ok(())
        }
        ConnectionAttribute::CurrentCatalog => {
            let conn_handle = match &connection.state {
                ConnectionState::Connected { conn_handle, .. } => *conn_handle,
                ConnectionState::Disconnected => return DisconnectedSnafu.fail(),
            };
            let g = global().context(OdbcRuntimeSnafu)?;
            // Return 24000 if any statement has an open cursor.
            for &child_id in &connection.child_statements {
                if let Ok(stmt_guard) = g.stmt_registry.get(child_id) {
                    let inner = stmt_guard.inner.lock();
                    let is_cursor_open = matches!(
                        inner.state.as_ref(),
                        StatementState::QueryExecuted { .. } | StatementState::Fetching { .. }
                    );
                    if is_cursor_open {
                        return InvalidCursorStateSnafu.fail();
                    }
                }
            }
            let catalog = read_string_from_pointer::<E>(value_ptr, string_length)?;
            let catalog = catalog.trim().to_string();
            drop(connection);
            global()
                .context(OdbcRuntimeSnafu)?
                .block_on(async |c| {
                    c.connection_use_database(ConnectionUseDatabaseRequest {
                        conn_handle: Some(conn_handle),
                        database: catalog.clone(),
                    })
                    .await
                })
                .map_err(|e| -> crate::api::OdbcError {
                    // Map any application-level USE DATABASE error to 3D000 (invalid catalog
                    // name). Snowflake returns 42000 for a non-existent database, which is not
                    // a meaningful ODBC state for this context. Transport/protocol errors are
                    // always propagated as-is.
                    match &e {
                        proto_utils::ProtoError::Application(_) => InvalidCatalogNameSnafu {
                            name: catalog.clone(),
                        }
                        .build(),
                        _ => e.into(),
                    }
                })?;
            dbc.connection.lock().current_catalog = Some(catalog);
            Ok(())
        }
        ConnectionAttribute::QuietMode => {
            connection.quiet_mode = value_ptr;
            Ok(())
        }
        ConnectionAttribute::PacketSize => {
            if matches!(connection.state, ConnectionState::Connected { .. }) {
                return AttributeCannotBeSetNowSnafu {
                    attribute: attr.as_raw(),
                }
                .fail();
            }
            connection.packet_size = value_ptr as sql::UInteger;
            Ok(())
        }
        ConnectionAttribute::ConnectionTimeout => {
            tracing::debug!("set_connect_attr: ConnectionTimeout (ignored)");
            Ok(())
        }
        ConnectionAttribute::MetadataId => {
            connection.metadata_id = value_ptr as sql::ULen != 0;
            Ok(())
        }
        ConnectionAttribute::ConnectionDead | ConnectionAttribute::AutoIpd => {
            // Read-only attributes — cannot be set
            ReadOnlyAttributeSnafu {
                attribute: attr.as_raw(),
            }
            .fail()
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
    let dbc = conn_from_handle(connection_handle)?;
    tracing::debug!("get_connect_attr: attribute={attribute}");

    let attr = match ConnectionAttribute::from_raw(attribute) {
        Some(a) => a,
        None => {
            tracing::warn!("get_connect_attr: unknown attribute {attribute}");
            return UnknownAttributeSnafu { attribute }.fail();
        }
    };

    let connection = dbc.connection.lock();
    match attr {
        ConnectionAttribute::AccessMode => {
            let access_mode = connection.access_mode;
            drop(connection);
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::UInteger) = access_mode.as_raw();
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = std::mem::size_of::<sql::UInteger>() as sql::Integer;
                }
            }
            Ok(())
        }
        ConnectionAttribute::Autocommit => {
            // Per spec: query the server for the actual autocommit state when connected;
            // fall back to the cached value if the RPC fails or the parameter is absent.
            // The cache is the authoritative source when disconnected.
            let maybe_conn_handle = match &connection.state {
                ConnectionState::Connected { conn_handle, .. } => Some(*conn_handle),
                ConnectionState::Disconnected => None,
            };
            let cached = connection.cached_autocommit;
            drop(connection);
            let val: sql::UInteger = match maybe_conn_handle {
                Some(conn_handle) => match get_session_parameter(&conn_handle, "AUTOCOMMIT") {
                    Ok(Some(v)) if v.eq_ignore_ascii_case("true") => {
                        dbc.connection.lock().cached_autocommit = AutocommitValue::On;
                        AutocommitValue::On.as_raw()
                    }
                    Ok(Some(_)) => {
                        dbc.connection.lock().cached_autocommit = AutocommitValue::Off;
                        AutocommitValue::Off.as_raw()
                    }
                    Ok(None) => {
                        tracing::warn!(
                            "get_connect_attr: AUTOCOMMIT session parameter missing, \
                                 falling back to cached value"
                        );
                        cached.as_raw()
                    }
                    Err(e) => {
                        tracing::warn!(
                            "get_connect_attr: failed to read AUTOCOMMIT session parameter \
                                 ({e}), falling back to cached value"
                        );
                        cached.as_raw()
                    }
                },
                None => cached.as_raw(),
            };
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::UInteger) = val;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = std::mem::size_of::<sql::UInteger>() as sql::Integer;
                }
            }
            Ok(())
        }
        ConnectionAttribute::LoginTimeout => {
            let timeout: sql::UInteger = match connection.pre_connection_attrs.get(&attr) {
                Some(s) => s.parse().unwrap_or_else(|_| {
                    tracing::warn!(
                        "get_connect_attr: LoginTimeout value {s:?} is not a valid integer, \
                         returning default {DEFAULT_LOGIN_TIMEOUT_SECS}",
                    );
                    DEFAULT_LOGIN_TIMEOUT_SECS.parse().unwrap()
                }),
                None => DEFAULT_LOGIN_TIMEOUT_SECS.parse().unwrap(),
            };
            drop(connection);
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::UInteger) = timeout;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = std::mem::size_of::<sql::UInteger>() as sql::Integer;
                }
            }
            Ok(())
        }
        ConnectionAttribute::TxnIsolation => {
            drop(connection);
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::UInteger) = SQL_TXN_READ_COMMITTED;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = std::mem::size_of::<sql::UInteger>() as sql::Integer;
                }
            }
            Ok(())
        }
        ConnectionAttribute::CurrentCatalog => {
            if buffer_length < 0 {
                return InvalidBufferLengthSnafu {
                    length: buffer_length as i64,
                }
                .fail();
            }
            let maybe_conn_handle = match &connection.state {
                ConnectionState::Connected { conn_handle, .. } => Some(*conn_handle),
                ConnectionState::Disconnected => None,
            };
            let cached_catalog = connection.current_catalog.clone();
            drop(connection);
            let database = match maybe_conn_handle {
                Some(conn_handle) => {
                    match global().context(OdbcRuntimeSnafu).and_then(|rt| {
                        rt.block_on(async |c| {
                            let info = c
                                .connection_get_info(ConnectionGetInfoRequest {
                                    conn_handle: Some(conn_handle),
                                    info_codes: vec![],
                                    include_master_token: false,
                                })
                                .await?;
                            Ok::<Option<String>, crate::api::OdbcError>(info.database)
                        })
                    }) {
                        Ok(db) => {
                            dbc.connection.lock().current_catalog = db.clone();
                            db
                        }
                        Err(e) => {
                            tracing::warn!(
                                "get_connect_attr: failed to fetch current catalog from server: \
                                 {e:?}; falling back to cached value"
                            );
                            cached_catalog
                        }
                    }
                }
                // When disconnected, return the cached catalog (or empty string).
                // Per ODBC spec the catalog is indeterminate before connecting;
                // returning an error would break applications that probe this attribute
                // before calling SQLConnect.
                None => cached_catalog,
            };
            let database_str = database.as_deref().unwrap_or("");
            write_string_bytes_i32::<E>(
                database_str,
                value_ptr as *mut E::Char,
                buffer_length,
                string_length_ptr,
                Some(warnings),
            );
            Ok(())
        }
        ConnectionAttribute::QuietMode => {
            let quiet_mode = connection.quiet_mode;
            drop(connection);
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::Pointer) = quiet_mode;
                }
            }
            Ok(())
        }
        ConnectionAttribute::PacketSize => {
            let packet_size = connection.packet_size;
            drop(connection);
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::UInteger) = packet_size;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = std::mem::size_of::<sql::UInteger>() as sql::Integer;
                }
            }
            Ok(())
        }
        ConnectionAttribute::ConnectionTimeout => {
            drop(connection);
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::UInteger) = 0;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = std::mem::size_of::<sql::UInteger>() as sql::Integer;
                }
            }
            Ok(())
        }
        ConnectionAttribute::ConnectionDead => {
            let dead = match connection.state {
                ConnectionState::Connected { .. } => SQL_CD_FALSE,
                ConnectionState::Disconnected => SQL_CD_TRUE,
            };
            drop(connection);
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::UInteger) = dead;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = std::mem::size_of::<sql::UInteger>() as sql::Integer;
                }
            }
            Ok(())
        }
        ConnectionAttribute::AutoIpd => {
            drop(connection);
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::UInteger) = SQL_FALSE;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = std::mem::size_of::<sql::UInteger>() as sql::Integer;
                }
            }
            Ok(())
        }
        ConnectionAttribute::MetadataId => {
            let metadata_id = connection.metadata_id;
            drop(connection);
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut sql::ULen) = metadata_id as sql::ULen;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = std::mem::size_of::<sql::ULen>() as sql::Integer;
                }
            }
            Ok(())
        }
        ConnectionAttribute::PrivKeyContent
        | ConnectionAttribute::PrivKeyPassword
        | ConnectionAttribute::PrivKeyBase64
        | ConnectionAttribute::Application => {
            let value = connection
                .pre_connection_attrs
                .get(&attr)
                .map(|s| s.as_str())
                .unwrap_or("")
                .to_owned();
            drop(connection);
            write_string_bytes_i32::<E>(
                &value,
                value_ptr as *mut E::Char,
                buffer_length,
                string_length_ptr,
                Some(warnings),
            );
            Ok(())
        }
        ConnectionAttribute::PrivKey => {
            drop(connection);
            UnsupportedAttributeSnafu {
                attribute: attr.as_raw(),
            }
            .fail()
        }
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

    let _conn = conn_from_handle(connection_handle)?;

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
        InfoType::DbmsName => {
            write_string_bytes::<E>(
                "Snowflake",
                info_value_ptr as *mut E::Char,
                buffer_length,
                string_length_ptr,
                None,
            );
            Ok(())
        }
        InfoType::DriverOdbcVer => {
            // ODBC 3.80 — matches the level the legacy Snowflake ODBC
            // driver advertises (`DriverODBCVer=03.52` in the .ini and
            // `03.80` in the SQLGetInfoValues fixture). Critically, the
            // Microsoft Windows ODBC Driver Manager refuses to forward
            // `SQLBindParameter(SQL_C_GUID, …)` with `HYC00` when the
            // driver advertises `<03.50`, because `SQL_C_GUID` is an
            // ODBC 3.5+ C type. Returning `03.80` is also a superset
            // claim: every API the driver currently implements is
            // available at that level.
            write_string_bytes::<E>(
                "03.80",
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
        InfoType::ConvertGuid => {
            write_u32_bitmask(convert_guid_mask(), info_value_ptr, string_length_ptr);
            Ok(())
        }
        InfoType::ConvertChar
        | InfoType::ConvertVarchar
        | InfoType::ConvertLongVarchar
        | InfoType::ConvertWChar
        | InfoType::ConvertWVarchar
        | InfoType::ConvertWLongVarchar => {
            write_u32_bitmask(
                convert_to_character_mask(),
                info_value_ptr,
                string_length_ptr,
            );
            Ok(())
        }
    }
}

/// Write a 32-bit bitmask into the `SQLGetInfo` output buffer, following the
/// ODBC contract that `*StringLengthPtr` reports the number of bytes written.
fn write_u32_bitmask(
    mask: u32,
    info_value_ptr: sql::Pointer,
    string_length_ptr: *mut sql::SmallInt,
) {
    if !info_value_ptr.is_null() {
        unsafe {
            *(info_value_ptr as *mut u32) = mask;
        }
    }
    if !string_length_ptr.is_null() {
        unsafe {
            *string_length_ptr = std::mem::size_of::<u32>() as sql::SmallInt;
        }
    }
}

/// `SQL_CVT_*` bitmask values from `sqlext.h`. Per the ODBC spec these are
/// the only legal bits in any `SQL_CONVERT_*` `SQLGetInfo` response — setting
/// other bits is undefined.
mod sql_cvt {
    pub const CHAR: u32 = 0x0000_0001;
    pub const NUMERIC: u32 = 0x0000_0002;
    pub const DECIMAL: u32 = 0x0000_0004;
    pub const INTEGER: u32 = 0x0000_0008;
    pub const SMALLINT: u32 = 0x0000_0010;
    pub const FLOAT: u32 = 0x0000_0020;
    pub const REAL: u32 = 0x0000_0040;
    pub const DOUBLE: u32 = 0x0000_0080;
    pub const VARCHAR: u32 = 0x0000_0100;
    pub const LONGVARCHAR: u32 = 0x0000_0200;
    pub const BINARY: u32 = 0x0000_0400;
    pub const VARBINARY: u32 = 0x0000_0800;
    pub const BIT: u32 = 0x0000_1000;
    pub const TINYINT: u32 = 0x0000_2000;
    pub const BIGINT: u32 = 0x0000_4000;
    pub const DATE: u32 = 0x0000_8000;
    pub const TIME: u32 = 0x0001_0000;
    pub const TIMESTAMP: u32 = 0x0002_0000;
    pub const LONGVARBINARY: u32 = 0x0004_0000;
    pub const INTERVAL_YEAR_MONTH: u32 = 0x0008_0000;
    pub const INTERVAL_DAY_TIME: u32 = 0x0010_0000;
    pub const WCHAR: u32 = 0x0020_0000;
    pub const WLONGVARCHAR: u32 = 0x0040_0000;
    pub const WVARCHAR: u32 = 0x0080_0000;
    pub const GUID: u32 = 0x0100_0000;
}

/// Bitmask returned for `SQL_CONVERT_GUID`. Enumerates the SQL targets the
/// driver can convert *from* `SQL_GUID`: every character SQL type (the driver
/// formats `SQL_C_GUID` as the canonical 8-4-4-4-12 upper-case hex literal —
/// see `varchar.rs::SnowflakeVarchar::read_odbc`), plus the identity
/// conversion. Binary / varbinary routes are not implemented yet, so those
/// bits stay off.
fn convert_guid_mask() -> u32 {
    sql_cvt::CHAR
        | sql_cvt::VARCHAR
        | sql_cvt::LONGVARCHAR
        | sql_cvt::WCHAR
        | sql_cvt::WVARCHAR
        | sql_cvt::WLONGVARCHAR
        | sql_cvt::GUID
}

/// Bitmask returned for `SQL_CONVERT_<character>` (CHAR / VARCHAR /
/// LONGVARCHAR / WCHAR / WVARCHAR / WLONGVARCHAR). Enumerates the SQL source
/// types the driver can convert *into* a character target. Mirrors every C
/// type accepted by `varchar.rs::SnowflakeVarchar::read_odbc`, mapped through
/// the standard C-type ↔ SQL-type correspondence in ODBC Appendix D.
///
/// This is also the bitmask the Microsoft Windows ODBC DM consults at
/// `SQLBindParameter` time when the parameter SQL type is one of the
/// character targets — without these bits set, the DM rejects binds like
/// `SQLBindParameter(SQL_C_GUID, SQL_VARCHAR, …)` with `HYC00` before the
/// call reaches the driver.
fn convert_to_character_mask() -> u32 {
    sql_cvt::CHAR
        | sql_cvt::NUMERIC
        | sql_cvt::DECIMAL
        | sql_cvt::INTEGER
        | sql_cvt::SMALLINT
        | sql_cvt::FLOAT
        | sql_cvt::REAL
        | sql_cvt::DOUBLE
        | sql_cvt::VARCHAR
        | sql_cvt::LONGVARCHAR
        | sql_cvt::BINARY
        | sql_cvt::VARBINARY
        | sql_cvt::BIT
        | sql_cvt::TINYINT
        | sql_cvt::BIGINT
        | sql_cvt::DATE
        | sql_cvt::TIME
        | sql_cvt::TIMESTAMP
        | sql_cvt::LONGVARBINARY
        | sql_cvt::INTERVAL_YEAR_MONTH
        | sql_cvt::INTERVAL_DAY_TIME
        | sql_cvt::WCHAR
        | sql_cvt::WLONGVARCHAR
        | sql_cvt::WVARCHAR
        | sql_cvt::GUID
}

#[cfg(test)]
mod tests {
    use super::*;
    use sf_core::protobuf::generated::database_driver_v1::config_setting;
    use test_case::test_case;

    fn config_string<'a>(
        options: &'a HashMap<String, ConfigSetting>,
        key: &str,
    ) -> Option<&'a str> {
        match options.get(key)?.value.as_ref()? {
            config_setting::Value::StringValue(value) => Some(value.as_str()),
            _ => None,
        }
    }

    #[test]
    fn normalize_connection_string_options_maps_login_timeout() {
        let options = normalize_connection_string_options(HashMap::from([(
            "LOGIN_TIMEOUT".to_owned(),
            "42".to_owned(),
        )]));

        assert_eq!(
            config_string(&options, "authentication_timeout"),
            Some("42")
        );
        assert!(!options.contains_key("LOGIN_TIMEOUT"));
    }

    #[test]
    fn normalize_connection_string_options_is_case_insensitive_for_special_keys() {
        let options = normalize_connection_string_options(HashMap::from([
            ("login_timeout".to_owned(), "99".to_owned()),
            ("priv_key_base64".to_owned(), "dsn-key".to_owned()),
        ]));

        assert_eq!(
            config_string(&options, "authentication_timeout"),
            Some("99")
        );
        assert_eq!(config_string(&options, "private_key"), Some("dsn-key"));
    }

    #[test]
    fn normalize_connection_string_options_normalizes_crl_enabled_for_core() {
        let options = normalize_connection_string_options(HashMap::from([(
            "CRL_ENABLED".to_owned(),
            "true".to_owned(),
        )]));

        assert_eq!(config_string(&options, "CRL_ENABLED"), Some("ENABLED"));
        assert!(!options.contains_key("crl_check_mode"));
    }

    #[test]
    fn normalize_connection_string_options_crl_enabled_zero_maps_to_disabled() {
        let options = normalize_connection_string_options(HashMap::from([(
            "CRL_ENABLED".to_owned(),
            "0".to_owned(),
        )]));

        assert_eq!(config_string(&options, "CRL_ENABLED"), Some("DISABLED"));
    }

    #[test]
    fn normalize_connection_string_options_uppercases_crl_mode() {
        let options = normalize_connection_string_options(HashMap::from([(
            "CRL_MODE".to_owned(),
            "enabled".to_owned(),
        )]));

        assert_eq!(config_string(&options, "CRL_MODE"), Some("ENABLED"));
    }

    #[test]
    fn normalize_connection_string_options_forwards_standard_keys_for_core_aliases() {
        let options = normalize_connection_string_options(HashMap::from([
            ("SERVER".to_owned(), "example.com".to_owned()),
            ("UID".to_owned(), "u".to_owned()),
        ]));

        assert_eq!(config_string(&options, "SERVER"), Some("example.com"));
        assert_eq!(config_string(&options, "UID"), Some("u"));
        assert!(!options.contains_key("host"));
        assert!(!options.contains_key("user"));
    }

    #[test]
    fn normalize_connection_string_options_maps_passcodeinpassword() {
        let options = normalize_connection_string_options(HashMap::from([(
            "PASSCODEINPASSWORD".to_owned(),
            "true".to_owned(),
        )]));

        assert_eq!(config_string(&options, "passcodeInPassword"), Some("true"));
        assert!(!options.contains_key("PASSCODEINPASSWORD"));
    }

    #[test]
    fn normalize_connection_string_options_maps_client_store_temporary_credential() {
        let options = normalize_connection_string_options(HashMap::from([(
            "CLIENT_STORE_TEMPORARY_CREDENTIAL".to_owned(),
            "true".to_owned(),
        )]));

        assert_eq!(
            config_string(&options, "client_store_temporary_credential"),
            Some("true")
        );
        assert!(!options.contains_key("CLIENT_STORE_TEMPORARY_CREDENTIAL"));
    }

    #[test]
    fn normalize_connection_string_options_preserves_unrecognized_keys() {
        let options = normalize_connection_string_options(HashMap::from([(
            "QUERY_TAG".to_owned(),
            "from-odbc".to_owned(),
        )]));

        assert_eq!(config_string(&options, "QUERY_TAG"), Some("from-odbc"));
    }

    #[test]
    fn apply_pre_connection_overrides_makes_priv_key_base64_authoritative() {
        let mut options = normalize_connection_string_options(HashMap::from([
            ("PRIV_KEY_BASE64".to_owned(), "dsn-key".to_owned()),
            ("PRIV_KEY_FILE".to_owned(), "/tmp/key.p8".to_owned()),
        ]));
        let attrs = HashMap::from([(ConnectionAttribute::PrivKeyBase64, "attr-key".to_owned())]);

        apply_pre_connection_overrides(&attrs, &mut options);

        assert_eq!(config_string(&options, "private_key"), Some("attr-key"));
        assert!(!options.contains_key("private_key_file"));
    }

    #[test_case("UID=admin;SERVER=foo", &[("UID", "admin"), ("SERVER", "foo")] ; "basic")]
    #[test_case("UID=admin; AUTHENTICATOR=SNOWFLAKE_JWT", &[("UID", "admin"), ("AUTHENTICATOR", "SNOWFLAKE_JWT")] ; "trims keys")]
    #[test_case("UID= admin ", &[("UID", "admin")] ; "trims values")]
    #[test_case(" UID = admin ; SERVER = foo ", &[("UID", "admin"), ("SERVER", "foo")] ; "trims both")]
    #[test_case("PRIV_KEY_FILE=abc=def", &[("PRIV_KEY_FILE", "abc=def")] ; "preserves equals in value")]
    #[test_case("UID=admin;  ;SERVER=foo", &[("UID", "admin"), ("SERVER", "foo")] ; "skips blank segments")]
    #[test_case("UID=admin;", &[("UID", "admin")] ; "trailing semicolon")]
    #[test_case("uid=admin;Server=foo", &[("UID", "admin"), ("SERVER", "foo")] ; "normalizes mixed case keys")]
    #[test_case("PWD={p@ss;word};SERVER=foo", &[("PWD", "p@ss;word"), ("SERVER", "foo")] ; "brace quoted semicolon in value")]
    #[test_case("PWD={val=ue};UID=admin", &[("PWD", "val=ue"), ("UID", "admin")] ; "brace quoted equals in value")]
    #[test_case("PWD={};UID=admin", &[("PWD", ""), ("UID", "admin")] ; "empty braced value")]
    #[test_case("PWD={a}}b};UID=admin", &[("PWD", "a}b"), ("UID", "admin")] ; "escaped brace in value")]
    #[test_case("DRIVER={/usr/lib/driver.so};UID=admin", &[("DRIVER", "/usr/lib/driver.so"), ("UID", "admin")] ; "typical driver path")]
    #[test_case("UID=admin;PWD=p\u{00E4}ss", &[("UID", "admin"), ("PWD", "p\u{00E4}ss")] ; "unbraced value with multibyte utf8")]
    #[test_case("PWD={p\u{00E4}ss;w\u{00F6}rd};UID=admin", &[("PWD", "p\u{00E4}ss;w\u{00F6}rd"), ("UID", "admin")] ; "braced value with multibyte utf8")]
    #[test_case("k\u{00E9}y=val", &[("K\u{00E9}Y", "val")] ; "multibyte utf8 in key")]
    #[test_case("PWD= {val};UID=admin", &[("PWD", "{val}"), ("UID", "admin")] ; "whitespace before opening brace falls back to unbraced")]
    #[test_case("", &[] ; "empty string")]
    #[test_case("   ", &[] ; "whitespace only")]
    #[test_case("UID=;SERVER=foo", &[("UID", ""), ("SERVER", "foo")] ; "key with empty value before semicolon")]
    #[test_case("UID=", &[("UID", "")] ; "key with empty value at end")]
    #[test_case("PWD={a}}b}}c};UID=admin", &[("PWD", "a}b}c"), ("UID", "admin")] ; "multiple escaped braces")]
    fn parse_connection_string_cases(input: &str, expected: &[(&str, &str)]) {
        let map = parse_connection_string(input).unwrap();
        assert_eq!(map.len(), expected.len());
        for (key, value) in expected {
            assert_eq!(map.get(*key).unwrap(), value);
        }
    }

    #[test]
    fn parse_connection_string_rejects_duplicate_key() {
        let result = parse_connection_string("UID=admin;UID=other");
        assert!(result.is_err());
    }

    #[test]
    fn parse_connection_string_rejects_duplicate_key_case_insensitive() {
        let result = parse_connection_string("UID=admin;uid=other");
        assert!(result.is_err());
    }

    #[test]
    fn parse_connection_string_rejects_unterminated_brace() {
        let result = parse_connection_string("PWD={unterminated");
        assert!(result.is_err());
    }

    #[test]
    fn parse_connection_string_rejects_chars_after_closing_brace() {
        let result = parse_connection_string("PWD={val}extra;UID=admin");
        assert!(result.is_err());
    }

    #[cfg(not(windows))]
    mod ini_tests {
        use super::*;

        #[test]
        fn parse_ini_section_normalizes_keys_to_uppercase() {
            let ini = "\
[MyDSN]
Server = myserver.snowflakecomputing.com
Uid = myuser
pwd = mypass
Account = myaccount
";
            let params = parse_ini_section(ini, "MyDSN").unwrap();
            assert_eq!(
                params.get("SERVER").unwrap(),
                "myserver.snowflakecomputing.com"
            );
            assert_eq!(params.get("UID").unwrap(), "myuser");
            assert_eq!(params.get("PWD").unwrap(), "mypass");
            assert_eq!(params.get("ACCOUNT").unwrap(), "myaccount");
            assert!(!params.contains_key("Server"));
        }

        #[test]
        fn parse_ini_section_not_found() {
            let ini = "[OtherDSN]\nServer = foo\n";
            assert!(parse_ini_section(ini, "MyDSN").is_none());
        }

        #[test]
        fn parse_ini_section_skips_comments_and_empty_lines() {
            let ini = "\
[MyDSN]
# this is a comment
; this is also a comment

Server = myserver
";
            let params = parse_ini_section(ini, "MyDSN").unwrap();
            assert_eq!(params.len(), 1);
            assert_eq!(params.get("SERVER").unwrap(), "myserver");
        }

        #[test]
        fn parse_ini_section_case_insensitive_section_name() {
            let ini = "[mydsn]\nServer = foo\n";
            let params = parse_ini_section(ini, "MyDSN").unwrap();
            assert_eq!(params.get("SERVER").unwrap(), "foo");
        }
    }

    /// `SQL_CONVERT_GUID` must contain `SQL_CVT_VARCHAR` so that the
    /// Microsoft Windows DM accepts `SQLBindParameter(SQL_C_GUID,
    /// SQL_VARCHAR, …)` — the rejection mechanism is observed empirically
    /// in `e2e_types_c_guid_to_sql_string` on Windows runners (HYC00 from
    /// the DM, before the call reaches the driver).
    #[test]
    fn convert_guid_mask_includes_every_character_target_and_identity() {
        let mask = convert_guid_mask();
        assert_ne!(mask & sql_cvt::CHAR, 0);
        assert_ne!(mask & sql_cvt::VARCHAR, 0);
        assert_ne!(mask & sql_cvt::LONGVARCHAR, 0);
        assert_ne!(mask & sql_cvt::WCHAR, 0);
        assert_ne!(mask & sql_cvt::WVARCHAR, 0);
        assert_ne!(mask & sql_cvt::WLONGVARCHAR, 0);
        assert_ne!(mask & sql_cvt::GUID, 0);
        assert_eq!(
            mask & sql_cvt::BINARY,
            0,
            "binary route is not implemented yet"
        );
        assert_eq!(
            mask & sql_cvt::VARBINARY,
            0,
            "varbinary route is not implemented yet"
        );
    }

    /// `SQL_CONVERT_<character>` must contain `SQL_CVT_GUID` so the DM
    /// accepts the bind from the *target* side too — observed Windows DMs
    /// probe both directions.
    #[test]
    fn convert_to_character_mask_includes_guid() {
        assert_ne!(convert_to_character_mask() & sql_cvt::GUID, 0);
    }

    /// `convert_to_character_mask` should mirror every C type accepted by
    /// `varchar.rs::SnowflakeVarchar::read_odbc`. Spot-check the breadth so
    /// a future regression that drops a bit fails loudly.
    #[test]
    fn convert_to_character_mask_includes_every_implemented_source() {
        let mask = convert_to_character_mask();
        for bit in [
            sql_cvt::CHAR,
            sql_cvt::VARCHAR,
            sql_cvt::LONGVARCHAR,
            sql_cvt::WCHAR,
            sql_cvt::WVARCHAR,
            sql_cvt::WLONGVARCHAR,
            sql_cvt::NUMERIC,
            sql_cvt::DECIMAL,
            sql_cvt::INTEGER,
            sql_cvt::SMALLINT,
            sql_cvt::TINYINT,
            sql_cvt::BIGINT,
            sql_cvt::FLOAT,
            sql_cvt::REAL,
            sql_cvt::DOUBLE,
            sql_cvt::BIT,
            sql_cvt::BINARY,
            sql_cvt::VARBINARY,
            sql_cvt::LONGVARBINARY,
            sql_cvt::DATE,
            sql_cvt::TIME,
            sql_cvt::TIMESTAMP,
            sql_cvt::INTERVAL_YEAR_MONTH,
            sql_cvt::INTERVAL_DAY_TIME,
            sql_cvt::GUID,
        ] {
            assert_ne!(mask & bit, 0, "missing SQL_CVT bit 0x{bit:08X}");
        }
    }
}
