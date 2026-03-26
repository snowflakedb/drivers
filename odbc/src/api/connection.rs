use crate::api::InfoType;
use crate::api::bitmask::Bitmask;
use crate::api::encoding::{
    OdbcEncoding, read_string_from_pointer, write_string_bytes, write_string_bytes_i32,
};
use crate::api::error::Required;
use crate::api::error::{
    AttributeCannotBeSetNowSnafu, DataSourceNotFoundSnafu, InvalidBufferLengthSnafu,
    InvalidPortSnafu, OdbcRuntimeSnafu, UnknownAttributeSnafu, UnsupportedAttributeSnafu,
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

/// Maps ODBC connection string parameter names to their sf_core equivalents.
/// Parameters listed here are forwarded as-is via `connection_set_option_string`.
/// Parameters that need special handling (type conversion, conditional skipping,
/// side-effects) are handled separately in `connect_with_params`.
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
            map.insert(parts[0].trim().to_uppercase(), parts[1].trim().to_string());
        }
    }
    map
}

/// Connect using connection string (SQLDriverConnect / SQLDriverConnectW).
pub fn driver_connect<E: OdbcEncoding>(
    connection_handle: sql::Handle,
    in_connection_string: *const E::Char,
    in_string_length: sql::SmallInt,
) -> OdbcResult<()> {
    let connection_string = E::read_string(in_connection_string, in_string_length as i32)?;
    let params = parse_connection_string(&connection_string);
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

    let connection = conn_from_handle(connection_handle);

    // Check whether attribute-based key options supersede file-based connection string params.
    // Matches old driver (SFConnection.cpp): if PrivKeyContent or PrivKeyBase64 was set via
    // SQLSetConnectAttr, PRIV_KEY_FILE from the connection string is not used.
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

            for (key, value) in params {
                if key == "DRIVER" {
                    continue;
                }

                if let Some(core_key) = PARAM_MAPPINGS
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| *v)
                {
                    c.connection_set_option_string(ConnectionSetOptionStringRequest {
                        conn_handle: Some(conn_handle),
                        key: core_key.to_owned(),
                        value,
                    })
                    .await?;
                    continue;
                }

                match key.as_str() {
                    "PORT" => {
                        let port_int: i64 = value.parse().context(InvalidPortSnafu {
                            port: value.clone(),
                        })?;
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
                            value,
                        })
                        .await?;
                    }
                    "PRIV_KEY_FILE" => {
                        if attr_key_set {
                            tracing::debug!(
                                "connect_with_params: skipping PRIV_KEY_FILE — attribute-based key takes priority"
                            );
                        } else {
                            c.connection_set_option_string(ConnectionSetOptionStringRequest {
                                conn_handle: Some(conn_handle),
                                key: "private_key_file".to_owned(),
                                value,
                            })
                            .await?;
                        }
                    }
                    "PRIV_KEY_BASE64" => {
                        if attr_key_set {
                            tracing::debug!(
                                "connect_with_params: skipping PRIV_KEY_BASE64 — attribute-based key takes priority"
                            );
                        } else {
                            c.connection_set_option_string(ConnectionSetOptionStringRequest {
                                conn_handle: Some(conn_handle),
                                key: "private_key".to_owned(),
                                value,
                            })
                            .await?;
                        }
                    }
                    "PRIV_KEY_FILE_PWD" | "PRIV_KEY_PWD" => {
                        if attr_has_priv_key_password {
                            tracing::debug!(
                                "connect_with_params: skipping {key} — attribute-based password takes priority"
                            );
                        } else {
                            c.connection_set_option_string(ConnectionSetOptionStringRequest {
                                conn_handle: Some(conn_handle),
                                key: "private_key_password".to_owned(),
                                value,
                            })
                            .await?;
                        }
                    }
                    _ => {
                        tracing::warn!("connect_with_params: unknown connection string key: {key:?}");
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

    tracing::info!("connect_with_params: connection_init completed");

    connection.state = ConnectionState::Connected {
        db_handle,
        conn_handle,
    };

    Ok(())
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
#[cfg(not(windows))]
fn parse_ini_section(content: &str, section: &str) -> Option<HashMap<String, String>> {
    let mut in_section = false;
    let mut params = HashMap::new();
    let mut found = false;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            let s = &line[1..line.len() - 1];
            in_section = s.eq_ignore_ascii_case(section);
            if in_section {
                found = true;
            }
            continue;
        }
        if !in_section || line.starts_with('#') || line.starts_with(';') || line.is_empty() {
            continue;
        }
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_uppercase();
            let value = line[eq_pos + 1..].trim().to_string();
            params.insert(key, value);
        }
    }

    if found { Some(params) } else { None }
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
        ConnectionAttribute::CurrentCatalog => {
            tracing::warn!("set_connect_attr: CurrentCatalog is not yet implemented");
            crate::api::error::UnsupportedAttributeSnafu {
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
        ConnectionAttribute::CurrentCatalog => {
            if buffer_length < 0 {
                return InvalidBufferLengthSnafu {
                    length: buffer_length as i64,
                }
                .fail();
            }
            write_string_bytes_i32::<E>(
                "",
                value_ptr as *mut E::Char,
                buffer_length,
                string_length_ptr,
                Some(warnings),
            );
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

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case("UID=admin;SERVER=foo", &[("UID", "admin"), ("SERVER", "foo")] ; "basic")]
    #[test_case("UID=admin; AUTHENTICATOR=SNOWFLAKE_JWT", &[("UID", "admin"), ("AUTHENTICATOR", "SNOWFLAKE_JWT")] ; "trims keys")]
    #[test_case("UID= admin ", &[("UID", "admin")] ; "trims values")]
    #[test_case(" UID = admin ; SERVER = foo ", &[("UID", "admin"), ("SERVER", "foo")] ; "trims both")]
    #[test_case("PRIV_KEY_FILE=abc=def", &[("PRIV_KEY_FILE", "abc=def")] ; "preserves equals in value")]
    #[test_case("UID=admin;  ;SERVER=foo", &[("UID", "admin"), ("SERVER", "foo")] ; "skips blank segments")]
    #[test_case("UID=admin;", &[("UID", "admin")] ; "trailing semicolon")]
    #[test_case("uid=admin;Server=foo", &[("UID", "admin"), ("SERVER", "foo")] ; "normalizes mixed case keys")]
    fn parse_connection_string_cases(input: &str, expected: &[(&str, &str)]) {
        let map = parse_connection_string(input);
        assert_eq!(map.len(), expected.len());
        for (key, value) in expected {
            assert_eq!(map.get(*key).unwrap(), value);
        }
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
}
