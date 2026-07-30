use crate::api::InfoType;
use crate::api::bitmask::Bitmask;
use crate::api::encoding::{
    OdbcEncoding, read_pre_connection_string_attr, read_string_from_pointer, write_string_bytes,
    write_string_bytes_i32, write_string_chars, write_string_chars_i32,
};
use crate::api::error::Required;
use crate::api::error::{
    AttributeCannotBeSetNowSnafu, DataSourceNotFoundSnafu, DisconnectedSnafu,
    InvalidAttributeValueSnafu, InvalidBufferLengthSnafu, InvalidCatalogNameSnafu,
    InvalidConnectionStringSnafu, InvalidCursorStateSnafu, InvalidDuringDaeSnafu, InvalidPortSnafu,
    InvalidTransactionOperationCodeSnafu, InvalidTransactionStateSnafu, NullPointerSnafu,
    OdbcRuntimeSnafu, ReadOnlyAttributeSnafu, UnknownAttributeSnafu, UnsupportedAttributeSnafu,
};
use crate::api::get_info_bitmasks::{
    AGGREGATE_FUNCTIONS, ALTER_DOMAIN, ALTER_TABLE, BATCH_ROW_COUNT, BATCH_SUPPORT,
    BOOKMARK_PERSISTENCE, CATALOG_USAGE, CONVERT_BIGINT, CONVERT_BINARY, CONVERT_BIT, CONVERT_CHAR,
    CONVERT_DATE, CONVERT_DECIMAL, CONVERT_DOUBLE, CONVERT_FLOAT, CONVERT_FUNCTIONS, CONVERT_GUID,
    CONVERT_INTEGER, CONVERT_INTERVAL_DAY_TIME, CONVERT_INTERVAL_YEAR_MONTH, CONVERT_LONGVARBINARY,
    CONVERT_LONGVARCHAR, CONVERT_NUMERIC, CONVERT_REAL, CONVERT_SMALLINT, CONVERT_TIME,
    CONVERT_TIMESTAMP, CONVERT_TINYINT, CONVERT_VARBINARY, CONVERT_VARCHAR, CONVERT_WCHAR,
    CONVERT_WLONGVARCHAR, CONVERT_WVARCHAR, CREATE_ASSERTION, CREATE_CHARACTER_SET,
    CREATE_COLLATION, CREATE_DOMAIN, CREATE_SCHEMA, CREATE_TABLE, CREATE_TRANSLATION, CREATE_VIEW,
    DATETIME_LITERALS, DDL_INDEX, DROP_ASSERTION, DROP_CHARACTER_SET, DROP_COLLATION, DROP_DOMAIN,
    DROP_SCHEMA, DROP_TABLE, DROP_TRANSLATION, DROP_VIEW, DYNAMIC_CURSOR_ATTRIBUTES1,
    DYNAMIC_CURSOR_ATTRIBUTES2, FETCH_DIRECTION, FORWARD_ONLY_CURSOR_ATTRIBUTES1,
    FORWARD_ONLY_CURSOR_ATTRIBUTES2, INFO_SCHEMA_VIEWS, INSERT_STATEMENT,
    KEYSET_CURSOR_ATTRIBUTES1, KEYSET_CURSOR_ATTRIBUTES2, LOCK_TYPES, NUMERIC_FUNCTIONS,
    OJ_CAPABILITIES, POS_OPERATIONS, POSITIONED_STATEMENTS, SCHEMA_USAGE, SCROLL_CONCURRENCY,
    SCROLL_OPTIONS, SQL92_DATETIME_FUNCTIONS, SQL92_FOREIGN_KEY_DELETE_RULE,
    SQL92_FOREIGN_KEY_UPDATE_RULE, SQL92_GRANT, SQL92_NUMERIC_VALUE_FUNCTIONS, SQL92_PREDICATES,
    SQL92_RELATIONAL_JOIN_OPERATORS, SQL92_REVOKE, SQL92_ROW_VALUE_CONSTRUCTOR,
    SQL92_STRING_FUNCTIONS, SQL92_VALUE_EXPRESSIONS, STATIC_CURSOR_ATTRIBUTES1,
    STATIC_CURSOR_ATTRIBUTES2, STATIC_SENSITIVITY, STRING_FUNCTIONS, SUBQUERIES, SYSTEM_FUNCTIONS,
    TIMEDATE_FUNCTIONS, TIMEDATE_TSI_INTERVALS, TXN_ISOLATION_OPTION, UNION, synthesize,
};
use crate::api::handle_registry::{HandleGuard, HandleId};
use crate::api::oauth;
use crate::api::odbc_installer::resolve_driver_path;
use crate::api::runtime::global;
use crate::api::{
    ConnectionState, GetDataExtensions, OdbcError, OdbcResult, conn_from_handle, env_from_handle,
    types::{AccessMode, AutocommitValue, ConnectionAttribute, Dbc, StatementState},
};
use crate::conversion::warning::{Warning, Warnings};
use odbc_sys as sql;
use sf_core::protobuf::generated::database_driver_v1::*;
use sf_core::sensitive::SensitiveString;
use snafu::{OptionExt, ResultExt};
use std::collections::HashMap;
use tracing;

const SQL_TXN_READ_COMMITTED: sql::UInteger = 2;
const SQL_CD_FALSE: sql::UInteger = 0;
const SQL_CD_TRUE: sql::UInteger = 1;
const SQL_FALSE: sql::UInteger = 0;

const ODBC_DRIVER_NAME: &str = "ODBC";
const ODBC_DRIVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const ODBC_API_VERSION: &str = env!("SF_ODBC_API_VER");

/// Default login timeout in seconds, matching the old driver's S_DEFAULT_LOGIN_TIMEOUT.
/// Used as the Okta SAML retry budget when neither the connection string nor
/// SQLSetConnectAttr provides a value.
const DEFAULT_LOGIN_TIMEOUT_SECS: sql::UInteger = 300;

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

    // Forward known OAuth keys with their explicit `sf_core` canonical
    // (lowercase) name instead of relying on the catch-all uppercase
    // passthrough + alias resolution. Owning the mapping here keeps the
    // OAuth surface self-documenting on the wrapper side.
    if let Some(canonical) = oauth::canonical_name(&upper) {
        return Some((canonical.to_owned(), value.into()));
    }

    match upper.as_str() {
        "PORT" => Some(("port".to_owned(), value.into())),
        // APPLICATION carries the user-facing app name → CLIENT_ENVIRONMENT.APPLICATION.
        // CLIENT_APP_ID stays as the wrapper-injected driver name ("ODBC").
        "APPLICATION" => Some(("application".to_owned(), value.into())),
        "CRL_MODE" => Some(("CRL_MODE".to_owned(), value.to_uppercase().into())),
        "CRL_ENABLED" => Some((
            "CRL_ENABLED".to_owned(),
            normalize_crl_enabled_value(&value).into(),
        )),
        "CLIENT_STORE_TEMPORARY_CREDENTIAL" => {
            Some(("client_store_temporary_credential".to_owned(), value.into()))
        }
        "DISABLE_PARALLEL_USER_PROMPT" => {
            Some(("disable_parallel_user_prompt".to_owned(), value.into()))
        }
        "LOGIN_TIMEOUT" => Some(("authentication_timeout".to_owned(), value.into())),
        "PASSCODEINPASSWORD" => Some(("passcodeInPassword".to_owned(), value.into())),
        "PRIV_KEY_FILE" => Some(("private_key_file".to_owned(), value.into())),
        "PRIV_KEY_BASE64" => Some(("private_key".to_owned(), value.into())),
        "PRIV_KEY_FILE_PWD" | "PRIV_KEY_PWD" => {
            Some(("private_key_password".to_owned(), value.into()))
        }
        // Forward other keys (e.g. SERVER, UID, SSL) for `sf_core` alias resolution; do not
        // pre-canonicalize here to avoid duplicate seed keys.
        _ => Some((upper, value.into())),
    }
}

const SF_GLOBAL_SSL_VERSION_ENV: &str = "SF_GLOBAL_SSL_VERSION";

/// Resolve a `SF_GLOBAL_SSL_VERSION` value to the canonical `sf_core` TLS token,
/// accepting the legacy snowflake-odbc `SSLVersion` spellings (`TLSv1_2` /
/// `TLSv1_3`, case-insensitive, `.`/`_` separators interchangeable).
///
/// `Ok(None)` means "no override" — unset / empty / `DEFAULT` (the old driver's
/// `DEFAULT` meant "negotiate normally"). A sub-1.2 or unrecognized value is an
/// `Err`: rustls supports only TLS 1.2/1.3, so — unlike the old driver, which
/// merely warned — we fail closed rather than silently downgrade.
fn resolve_global_ssl_version(raw: &str) -> Result<Option<&'static str>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("DEFAULT") {
        return Ok(None);
    }
    // Normalize separators so TLSv1_2 / TLSv1.2 / tls12 all compare equal.
    match trimmed
        .to_ascii_lowercase()
        .replace(['.', '_'], "")
        .as_str()
    {
        "tls12" | "tlsv12" => Ok(Some("tls12")),
        "tls13" | "tlsv13" => Ok(Some("tls13")),
        "tlsv1" | "tlsv10" | "tlsv11" | "sslv2" | "sslv3" => Err(format!(
            "SF_GLOBAL_SSL_VERSION='{raw}' selects a TLS version below 1.2, which is not \
             supported (use TLSv1_2 or TLSv1_3)"
        )),
        _ => Err(format!(
            "SF_GLOBAL_SSL_VERSION='{raw}' is not a recognized TLS version (use TLSv1_2 or TLSv1_3)"
        )),
    }
}

/// Pin both `min_tls_version` and `max_tls_version` to `version`, dropping any
/// explicit MIN_TLS_VERSION / MAX_TLS_VERSION already present (resolved against
/// the `sf_core` registry so every alias and case is caught) so the global pin
/// is the single source of truth.
fn pin_tls_version(options: &mut HashMap<String, ConfigSetting>, version: &str) {
    let registry = sf_core::config::param_registry::registry();
    options.retain(|key, _| {
        !matches!(
            registry.resolve(key.as_str()).map(|def| def.canonical_name),
            Some("min_tls_version") | Some("max_tls_version")
        )
    });
    options.insert("min_tls_version".to_owned(), version.to_owned().into());
    options.insert("max_tls_version".to_owned(), version.to_owned().into());
}

/// Apply the `SF_GLOBAL_SSL_VERSION` override (if the env var is set to a usable
/// value) onto the normalized connection options, before they become the
/// connection seed.
fn apply_global_ssl_version_override(
    options: &mut HashMap<String, ConfigSetting>,
) -> OdbcResult<()> {
    let Ok(raw) = std::env::var(SF_GLOBAL_SSL_VERSION_ENV) else {
        return Ok(());
    };
    let version = resolve_global_ssl_version(&raw)
        .map_err(|reason| InvalidConnectionStringSnafu { reason }.build())?;
    if let Some(version) = version {
        tracing::info!("SF_GLOBAL_SSL_VERSION={raw} pins TLS to {version} (overrides min/max)");
        pin_tls_version(options, version);
    }
    Ok(())
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

/// Serialise a key=value parameter map back to a semicolon-separated
/// connection string. Values that contain `;` are brace-quoted so the
/// result round-trips through `parse_connection_string`. Keys are sorted
/// for deterministic output.
fn serialize_connection_string(params: &HashMap<String, String>) -> String {
    let mut parts: Vec<String> = params
        .iter()
        .map(|(k, v)| {
            if v.contains(';') {
                format!("{k}={{{v}}}")
            } else {
                format!("{k}={v}")
            }
        })
        .collect();
    parts.sort();
    parts.join(";")
}

/// Read, parse, and connect from a raw connection string — the pipeline shared
/// by `SQLDriverConnect` and `SQLBrowseConnect`. On success the caller-supplied
/// connection string is returned so it can be echoed into the output buffer.
fn resolve_and_connect<E: OdbcEncoding>(
    connection_handle: sql::Handle,
    in_connection_string: *const E::Char,
    in_string_length: sql::SmallInt,
) -> OdbcResult<String> {
    let connection_string = E::read_string(in_connection_string, in_string_length as i32)?;
    let params = parse_connection_string(&connection_string)?;
    // Capture the original `DRIVER=` / `DSN=` keywords (if any) before
    // they get normalised away — they are needed later to resolve the
    // driver's installed file path for `SQLGetInfo(SQL_DRIVER_NAME)`.
    let driver_section = params.get("DRIVER").cloned();
    let dsn_name = params.get("DSN").cloned();
    // Expand any DSN-stored attributes (account, host, user, credentials)
    // underneath the caller-supplied connection-string params so that a bare
    // "DSN=<name>" string picks up everything stored in odbc.ini / registry.
    let params = merge_dsn_config(params, dsn_name.as_deref())?;
    connect_with_params(connection_handle, params, driver_section, dsn_name)?;
    Ok(connection_string)
}

/// Connect using connection string (SQLDriverConnect / SQLDriverConnectW).
///
/// On success the completed connection string is written back to
/// `out_connection_string` (character units, per the ODBC spec for
/// `SQLDriverConnect`); a too-small buffer is truncated and reported as a
/// `01004` warning via `warnings`.
pub fn driver_connect<E: OdbcEncoding>(
    connection_handle: sql::Handle,
    in_connection_string: *const E::Char,
    in_string_length: sql::SmallInt,
    out_connection_string: *mut E::Char,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    let connection_string = E::read_string(in_connection_string, in_string_length as i32)?;
    let params = parse_connection_string(&connection_string)?;
    // Capture the original `DRIVER=` / `DSN=` keywords (if any) before
    // they get normalised away — they are needed later to resolve the
    // driver's installed file path for `SQLGetInfo(SQL_DRIVER_NAME)`.
    let driver_section = params.get("DRIVER").cloned();
    let dsn_name = params.get("DSN").cloned();
    // Expand any DSN-stored attributes (account, host, user, credentials)
    // underneath the caller-supplied connection-string params so that a bare
    // "DSN=<name>" string picks up everything stored in odbc.ini / registry.
    let params = merge_dsn_config(params, dsn_name.as_deref())?;
    // Build the completed connection string before params are consumed.
    // The ODBC spec requires OutConnectionString to contain all attributes
    // including those expanded from the DSN, and must include DSN= or DRIVER=.
    let completed_conn_str = {
        let mut out = params.clone();
        if let Some(ref dsn) = dsn_name {
            out.insert("DSN".to_owned(), dsn.clone());
        }
        if let Some(ref driver) = driver_section {
            out.insert("DRIVER".to_owned(), driver.clone());
        }
        SensitiveString::from(serialize_connection_string(&out))
    };
    connect_with_params(connection_handle, params, driver_section, dsn_name)?;

    // Per the ODBC spec, `SQLDriverConnect` returns the completed connection
    // string — all attributes including those expanded from the DSN — so the
    // application can persist it and reconnect without re-reading odbc.ini.
    // Buffer/length units are characters; a short buffer truncates and posts
    // a `01004` warning.
    write_string_chars::<E>(
        completed_conn_str.reveal(),
        out_connection_string,
        buffer_length,
        string_length_ptr,
        Some(warnings),
    );
    Ok(())
}

/// Outcome of a successful [`browse_connect`] call.
///
/// Snowflake does not implement iterative attribute discovery, so a completed
/// connection is either fully established (`Complete`) or the output buffer was
/// too small to receive the connection string (`NeedData`). The FFI shim maps
/// `NeedData` to `SQL_NEED_DATA`.
pub enum BrowseOutcome {
    Complete,
    NeedData,
}

/// Connect using connection string (SQLBrowseConnect / SQLBrowseConnectW).
///
/// Behaves like [`driver_connect`] — Snowflake requires a complete connection
/// string up front and does not iteratively prompt for attributes — with one
/// spec-mandated difference: when the output buffer is too small to hold the
/// connection string (including `buffer_length == 0`), it returns
/// `SQL_NEED_DATA` (via [`BrowseOutcome::NeedData`]) rather than the `01004`
/// truncation warning `SQLDriverConnect` uses. `*string_length_ptr` always
/// receives the full required length. An incomplete string, bad credentials, or
/// an unknown DSN surface as the usual connect errors (`SQL_ERROR` / `28000` /
/// `IM002`).
///
/// Note: on `NeedData` the connection has already been established (Snowflake
/// connects in a single round-trip); a subsequent call on the same handle would
/// therefore be rejected with `08002`.
pub fn browse_connect<E: OdbcEncoding>(
    connection_handle: sql::Handle,
    in_connection_string: *const E::Char,
    in_string_length: sql::SmallInt,
    out_connection_string: *mut E::Char,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
) -> OdbcResult<BrowseOutcome> {
    let connection_string =
        resolve_and_connect::<E>(connection_handle, in_connection_string, in_string_length)?;

    // `None` warnings: SQLBrowseConnect signals a short buffer via SQL_NEED_DATA,
    // not a `01004` warning. `*string_length_ptr` still receives the full length.
    let truncated = write_string_chars::<E>(
        &connection_string,
        out_connection_string,
        buffer_length,
        string_length_ptr,
        None,
    );
    if truncated {
        Ok(BrowseOutcome::NeedData)
    } else {
        Ok(BrowseOutcome::Complete)
    }
}

/// Core connection logic shared by `driver_connect` and `connect`.
///
/// Takes the already-parsed parameter map, applies it to a new sf_core connection,
/// respects pre-connection attributes set via `SQLSetConnectAttr`, and transitions
/// the handle to `Connected`.
fn connect_with_params(
    connection_handle: sql::Handle,
    params: HashMap<String, String>,
    driver_section: Option<String>,
    dsn_name: Option<String>,
) -> OdbcResult<()> {
    tracing::info!(
        "connect_with_params: params={:?}",
        oauth::redacted_param_map(&params)
    );

    // Stash the ini-identity hints on the DBC up front so they are
    // available to `SQLGetInfo(SQL_DRIVER_NAME)` even if the connection
    // itself fails partway through. Connection-string parsing has
    // already validated the strings; we just retain them verbatim.
    {
        let dbc = conn_from_handle(connection_handle)?;
        let mut conn = dbc.connection.lock();
        conn.driver_section = driver_section;
        conn.dsn_name = dsn_name;
    }

    let mut options = normalize_connection_string_options(params);
    apply_global_ssl_version_override(&mut options)?;
    if let Some(config_setting::Value::StringValue(raw_port)) = options
        .get("port")
        .and_then(|setting| setting.value.as_ref())
    {
        let port_int: i64 = raw_port.parse().context(InvalidPortSnafu {
            port: raw_port.clone(),
        })?;
        options.insert("port".to_owned(), port_int.into());
    }

    // Legacy ODBC silently swallows all logout errors (destructor catch-all).
    options
        .entry("LOGOUT_ERROR_STRATEGY".to_owned())
        .or_insert_with(|| "best_effort".to_owned().into());

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
                // ODBC always connects via a connection string / DSN, so there
                // is no bare-connect default-profile fallback to trigger.
                no_connection_details: false,
            })
            .await?;

        for warning in &response.warnings {
            tracing::warn!("connection option warning: {}", warning.message);
        }

        // Optional default login timeout (Okta SAML budget).
        if !login_timeout_in_options && !login_timeout_in_attrs {
            let follow_up = HashMap::from([(
                "authentication_timeout".to_owned(),
                DEFAULT_LOGIN_TIMEOUT_SECS.to_string().into(),
            )]);
            let response = c
                .connection_set_options(ConnectionSetOptionsRequest {
                    conn_handle: Some(conn_handle),
                    options: follow_up,
                    no_connection_details: false,
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
                // Set at compile time in `build.rs` (`SF_ODBC_*`) from Cargo / rustc.
                language_runtime: Some(env!("SF_ODBC_WRAPPER_LANGUAGE_RUNTIME").to_string()),
                language_version: Some(env!("SF_ODBC_BUILD_RUST_SEMVER").to_string()),
                language_compiler: None,
            }),
        })
        .await?;

        Ok::<_, crate::api::OdbcError>((db_handle, conn_handle))
    })?;

    tracing::info!("connect_with_params: connection_init completed");

    // Set `state` and the lock-free telemetry cache together within one
    // critical section so they stay ordered with each other. The cache lets
    // telemetry resolve the session without ever locking `connection` (which a
    // synchronous query holds for its full duration). `mark_connected` is the
    // single writer that keeps the two in sync.
    {
        let mut c = dbc.connection.lock();
        dbc.mark_connected(&mut c, db_handle, conn_handle);
        // A freshly-established session has no open transaction, regardless of
        // any stale flag left on a reused handle from a prior connection.
        c.open_transaction = false;
    }

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

    // SQL_SF_CONN_ATTR_APPLICATION → CLIENT_ENVIRONMENT.APPLICATION via the
    // canonical ``application`` setting. CLIENT_APP_ID stays as the
    // wrapper-injected driver name (matches the old ODBC driver).
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
/// merges caller-supplied UID/PWD overrides via `merge_dsn_config`, then delegates to
/// `connect_with_params` to perform the actual connection.
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

    // UID/PWD supplied by the caller override whatever the DSN entry holds.
    let mut explicit = HashMap::new();
    if let Some(uid) = uid {
        explicit.insert("UID".to_string(), uid);
    }
    if let Some(pwd) = pwd {
        explicit.insert("PWD".to_string(), pwd);
    }
    let params = merge_dsn_config(explicit, Some(&dsn))?;

    // The DSN name is what reaches `SQLGetInfo(SQL_DRIVER_NAME)` for
    // resolving the driver's installed file path via `odbc.ini` →
    // `odbcinst.ini`. SQLConnect never carries a `DRIVER=` keyword, so
    // there is no direct driver section to capture here.
    connect_with_params(connection_handle, params, None, Some(dsn))
}

/// Merge DSN-stored attributes underneath caller-supplied params.
///
/// Explicit params (connection string / UID+PWD) win over DSN-stored values.
/// Strips DSN metadata keys (`Driver`, `Description`, `DSN`) from the result.
/// No-op when `dsn` is `None`.
fn merge_dsn_config(
    explicit: HashMap<String, String>,
    dsn: Option<&str>,
) -> OdbcResult<HashMap<String, String>> {
    merge_dsn_config_impl(explicit, dsn, read_dsn_config)
}

fn merge_dsn_config_impl(
    mut explicit: HashMap<String, String>,
    dsn: Option<&str>,
    lookup: impl Fn(&str) -> OdbcResult<HashMap<String, String>>,
) -> OdbcResult<HashMap<String, String>> {
    if let Some(dsn) = dsn {
        let stored = lookup(dsn)?;
        for (k, v) in stored {
            explicit.entry(k).or_insert(v);
        }
    }
    explicit.retain(|k, _| {
        !k.eq_ignore_ascii_case("Driver")
            && !k.eq_ignore_ascii_case("Description")
            && !k.eq_ignore_ascii_case("DSN")
    });
    Ok(explicit)
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

    // Per the ODBC spec, SQLDisconnect must return 25000 (invalid transaction
    // state) and leave the connection open when an ODBC-managed transaction is
    // still in process (autocommit OFF with pending work). The application must
    // SQLEndTran (commit/rollback) before disconnecting. Checked under the
    // `connection` lock, which is also held across statement execution, so there
    // is no race with a concurrent execute that opens a transaction.
    if connection.open_transaction {
        return InvalidTransactionStateSnafu.fail();
    }

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

    // Clear the telemetry cache post-teardown (after the `block_on` succeeds).
    // This runs under the `connection` guard acquired at the top of this
    // function, so `state` and the cache flip together under the lock — the same
    // lock-consistency the connect path gets from its scoped critical section.
    // The clear is deliberately deferred to here rather than done at disconnect
    // entry: `state` stays `Connected` and the cache stays `Some` across the
    // teardown, so the `?`-returns above leave both set, upholding "cache is
    // `Some` iff `Connected`". A telemetry read racing the teardown may still
    // resolve the handle being released — acceptable, since disconnect is
    // single-threaded under normal ODBC use and telemetry is best-effort (core
    // drops events for released handles). `mark_disconnected` clears `state` and
    // the cache together, upholding "cache is `Some` iff `Connected`".
    dbc.mark_disconnected(&mut connection);
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

    write_string_chars_i32::<E>(
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

// SQLEndTran completion-type codes (odbc_sys::CompletionType: Commit = 0, Rollback = 1).
const SQL_COMMIT: sql::SmallInt = 0;
const SQL_ROLLBACK: sql::SmallInt = 1;

/// Operation effected by `SQLEndTran`.
#[derive(Copy, Clone, Debug)]
enum TxnOp {
    Commit,
    Rollback,
}

/// Parse a `SQLEndTran` completion type into a `TxnOp`. Returns HY012
/// (invalid transaction operation code) for anything other than
/// `SQL_COMMIT` / `SQL_ROLLBACK`.
fn parse_completion_type(completion_type: sql::SmallInt) -> OdbcResult<TxnOp> {
    match completion_type {
        SQL_COMMIT => Ok(TxnOp::Commit),
        SQL_ROLLBACK => Ok(TxnOp::Rollback),
        _ => InvalidTransactionOperationCodeSnafu { completion_type }.fail(),
    }
}

/// End the current transaction on a connection (`SQLEndTran` with `SQL_HANDLE_DBC`).
pub fn end_tran(connection_handle: sql::Handle, completion_type: sql::SmallInt) -> OdbcResult<()> {
    let op = parse_completion_type(completion_type)?;
    let dbc = conn_from_handle(connection_handle)?;
    commit_or_rollback(&dbc, op)
}

/// End the current transaction on every connection owned by an environment
/// (`SQLEndTran` with `SQL_HANDLE_ENV`). Per the ODBC spec this attempts to
/// commit/rollback every connection owned by the environment, so we keep going
/// on failure and surface the first error after the loop rather than leaving
/// later connections with their transactions still open. Connections that are
/// disconnected by the time `commit_or_rollback` runs are silently skipped; we
/// detect that from its `Disconnected` error rather than pre-checking the
/// state, which would race with a concurrent disconnect.
pub fn end_tran_env(env_handle: sql::Handle, completion_type: sql::SmallInt) -> OdbcResult<()> {
    let op = parse_completion_type(completion_type)?;
    let env = env_from_handle(env_handle)?;
    let conn_ids: Vec<HandleId> = env.environment.lock().connections.clone();
    let mut result: OdbcResult<()> = Ok(());
    for conn_id in conn_ids {
        let Ok(dbc) = conn_from_handle(conn_id.into()) else {
            continue;
        };
        // Skip connections already disconnected by the time we reach them.
        // Treating Err(Disconnected) from commit_or_rollback as a skip (rather
        // than pre-checking the state) avoids a TOCTOU race with a concurrent
        // disconnect. Other errors are aggregated: keep going and surface the
        // first one after the loop.
        match commit_or_rollback(&dbc, op) {
            Ok(()) | Err(OdbcError::Disconnected { .. }) => {}
            Err(e) if result.is_ok() => result = Err(e),
            Err(_) => {}
        }
    }
    result
}

/// Commit or rollback the transaction on one connection, then close any open
/// cursors on its statements per `SQL_CB_CLOSE`.
///
/// Returns 08003 if the connection is closed and HY010 if any statement on the
/// connection is awaiting data-at-execution.
fn commit_or_rollback(dbc: &Dbc, op: TxnOp) -> OdbcResult<()> {
    let mut connection = dbc.connection.lock();
    let conn_handle = match &connection.state {
        ConnectionState::Connected { conn_handle, .. } => *conn_handle,
        ConnectionState::Disconnected => return DisconnectedSnafu.fail(),
    };

    let g = global().context(OdbcRuntimeSnafu)?;
    let child_ids: Vec<HandleId> = connection.child_statements.clone();
    // HY010 if any statement on the connection is mid data-at-execution.
    for &child_id in &child_ids {
        if let Ok(stmt_guard) = g.stmt_registry.get(child_id)
            && stmt_guard.inner.lock().state.as_ref().is_need_data()
        {
            return InvalidDuringDaeSnafu.fail();
        }
    }

    // Hold `connection` across the RPC and the cursor cleanup below. SQLExecute
    // (`statement::execute`) also holds `dbc.connection` for the whole duration
    // of its query, so keeping it locked here serializes the transaction
    // boundary against statement execution on the same connection: a running
    // statement blocks this call until it finishes, and no new statement can
    // start while the commit/rollback RPC is in flight. Same lock order
    // (`connection` -> `stmt.inner`) as `execute`, so no deadlock; the RPC only
    // touches the sf_core connection, not `dbc.connection`.
    g.block_on(async |c| -> OdbcResult<()> {
        match op {
            TxnOp::Commit => {
                c.connection_commit(ConnectionCommitRequest {
                    conn_handle: Some(conn_handle),
                })
                .await?;
            }
            TxnOp::Rollback => {
                c.connection_rollback(ConnectionRollbackRequest {
                    conn_handle: Some(conn_handle),
                })
                .await?;
            }
        }
        Ok(())
    })?;

    // The manual-commit transaction (if any) is now closed. Clearing under the
    // still-held `connection` lock means a subsequent SQLDisconnect on this
    // connection will not spuriously return 25000.
    connection.open_transaction = false;

    close_open_cursors(&child_ids);
    Ok(())
}

/// Close any open cursors on each statement in `child_ids`, mirroring
/// `SQLFreeStmt(SQL_CLOSE)`. Prepared statements return to `Prepared` so they
/// stay re-executable; directly-executed ones return to `Created`. This is the
/// `SQL_CB_CLOSE` cursor behavior `SQLEndTran` advertises via `SQLGetInfo`.
fn close_open_cursors(child_ids: &[HandleId]) {
    let Ok(g) = global() else {
        return;
    };
    for &child_id in child_ids {
        let Ok(stmt_guard) = g.stmt_registry.get(child_id) else {
            continue;
        };
        let mut inner = stmt_guard.inner.lock();
        let next = match inner.state.as_ref() {
            StatementState::QueryExecuted { origin, .. }
            | StatementState::Fetching { origin, .. }
            | StatementState::DdlExecuted { origin, .. }
            | StatementState::DmlExecuted { origin, .. }
            | StatementState::Done { origin, .. } => origin.restore_state(),
            _ => continue,
        };
        let desc_count = match &next {
            StatementState::Prepared { schema } => schema.fields().len() as sql::SmallInt,
            _ => 0,
        };
        inner.state.set(next);
        inner.ird.desc_count = desc_count;
        inner.get_data_state = None;
        inner.used_extended_fetch = false;
    }
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

    const SQL_ATTR_ASYNC_DBC_FUNCTIONS_ENABLE: sql::Integer = 117;
    if attribute == SQL_ATTR_ASYNC_DBC_FUNCTIONS_ENABLE {
        return UnknownAttributeSnafu { attribute }.fail();
    }

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
            let mode = AccessMode::from_raw(value_ptr as sql::UInteger).with_context(|| {
                InvalidAttributeValueSnafu {
                    attribute: attr.as_raw(),
                    value: value_ptr as i64,
                }
            })?;
            connection.access_mode = mode;
            Ok(())
        }
        ConnectionAttribute::Autocommit => {
            let val = AutocommitValue::from_raw(value_ptr as sql::UInteger).with_context(|| {
                InvalidAttributeValueSnafu {
                    attribute: attr.as_raw(),
                    value: value_ptr as i64,
                }
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
                    // Hold `connection` across the RPC (same as `commit_or_rollback`
                    // and `execute`) so a concurrent execute on another thread cannot
                    // set `open_transaction = true` between the RPC and the clear
                    // below and have it silently clobbered back to false, which would
                    // let SQLDisconnect proceed while a real transaction is still
                    // open. The RPC only touches the sf_core connection, not
                    // `dbc.connection`; same lock order as `execute`, so no deadlock.
                    global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
                        c.connection_set_autocommit(ConnectionSetAutocommitRequest {
                            conn_handle: Some(conn_handle),
                            autocommit: autocommit_on,
                        })
                        .await
                    })?;
                    connection.cached_autocommit = val;
                    // Switching autocommit ON commits any in-flight manual-commit
                    // transaction, so the open-transaction flag no longer applies.
                    // Cleared under the still-held lock that guards the disconnect gate.
                    if matches!(val, AutocommitValue::On) {
                        connection.open_transaction = false;
                    }
                    // Keep pre_connection_attrs in sync so a reconnect on the same handle
                    // re-applies the value set while connected rather than the stale pre-connect value.
                    connection
                        .pre_connection_attrs
                        .insert(attr, val.as_raw().to_string());
                    Ok(())
                }
                // Per the ODBC spec, SQL_ATTR_AUTOCOMMIT may be set before
                // connecting; cache it and let `apply_pre_connection_runtime_attrs_async`
                // toggle the server on connect (SNOW-3235550 / SNOW-87908). Only the
                // live server-toggle path requires an open connection, so we do NOT
                // return 08003 here.
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
            // Snowflake always runs at READ COMMITTED. Full isolation-level support
            // (HY011 when a transaction is open) is deferred to SNOW-3240589.
            // Per ODBC spec §SQLSetConnectAttr: emit 01S02 whenever the driver
            // substitutes the requested value.  READ_COMMITTED is accepted as-is;
            // every other level is silently substituted so pools / ORMs that
            // read-then-restore the isolation level see the expected warning.
            let requested = value_ptr as sql::UInteger;
            if requested != SQL_TXN_READ_COMMITTED {
                tracing::debug!(
                    "set_connect_attr: TxnIsolation={requested} substituted with READ_COMMITTED"
                );
                warnings.push(Warning::OptionValueChanged);
            } else {
                tracing::debug!("set_connect_attr: TxnIsolation=READ_COMMITTED accepted");
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
            // These are Snowflake-custom attribute IDs; iODBC's narrow→wide
            // bridge does not transcode them, so the W variant of the
            // driver may receive a narrow buffer. `read_pre_connection_string_attr`
            // sniffs the leading bytes and reads narrow-or-wide as needed.
            let value = read_pre_connection_string_attr::<E>(value_ptr, string_length)?;
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
        // Per ODBC, a get of a valid-but-unsupported attribute returns HYC00,
        // while an identifier outside the ODBC-defined range returns HY092
        // (SNOW-3235557).
        None if ConnectionAttribute::is_known_odbc(attribute) => {
            tracing::warn!("get_connect_attr: unsupported ODBC attribute {attribute}");
            return UnsupportedAttributeSnafu { attribute }.fail();
        }
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
                    DEFAULT_LOGIN_TIMEOUT_SECS
                }),
                None => DEFAULT_LOGIN_TIMEOUT_SECS,
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
            // The current catalog is a server-side session property, so it is
            // indeterminate without an open connection: return 08003 when
            // disconnected (SNOW-3235557) rather than a stale/empty cached value.
            if matches!(connection.state, ConnectionState::Disconnected) {
                return DisconnectedSnafu.fail();
            }
            drop(connection);
            let database = current_database(&dbc)?;
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

/// Write an ODBC string value into the `SQLGetInfo` output buffers.
/// Both pointers are individually null-guarded — the Driver Manager passes
/// null for either side when it only wants the other.
///
/// `warnings` is only consulted when a truncation actually occurs (i.e. when
/// `info_value_ptr` is non-null — see the `write_str` closure in
/// [`get_info`], which omits it entirely for a NULL-buffer length probe so
/// that case stays plain `SQL_SUCCESS` rather than `SQL_SUCCESS_WITH_INFO`).
fn write_get_info_string<E: OdbcEncoding>(
    value: &str,
    info_value_ptr: sql::Pointer,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    warnings: Option<&mut Warnings>,
) {
    write_string_bytes::<E>(
        value,
        info_value_ptr as *mut E::Char,
        buffer_length,
        string_length_ptr,
        warnings,
    );
}

/// Write a `SQLUSMALLINT` info value (used by InfoTypes whose return type is
/// `SQLUSMALLINT`, e.g. `SQL_CONCAT_NULL_BEHAVIOR`).
fn write_get_info_u16(
    value: u16,
    info_value_ptr: sql::Pointer,
    string_length_ptr: *mut sql::SmallInt,
) {
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
}

/// Write a `SQLUINTEGER` info value (used by all bitmask InfoTypes and the
/// `SQLUINTEGER`-typed numeric InfoTypes).
fn write_get_info_u32(
    value: u32,
    info_value_ptr: sql::Pointer,
    string_length_ptr: *mut sql::SmallInt,
) {
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
}

/// Current database (catalog) for this connection.
///
/// Reads sf_core's authoritative, continuously-updated session state via
/// `connection_get_info` — an in-process lock read, not a Snowflake round
/// trip — so it reflects server-side `USE DATABASE` issued as queries, which
/// the odbc-layer `current_catalog` cache does not track. The cache is
/// refreshed as a side-effect on success.
///
/// Failure policy (matches `SQL_DBMS_VER` in `get_info`):
///
/// * Disconnected: returns `Ok(cached value)`. Catalog is indeterminate
///   pre-connect; erroring would break apps that probe before `SQLConnect`.
/// * Connected, RPC ok: returns `Ok(database)`, cache refreshed.
/// * Connected, RPC err: propagates (missing handle / poisoned lock only).
fn current_database(dbc: &HandleGuard<Dbc>) -> OdbcResult<Option<String>> {
    let (conn_handle, cached) = {
        let conn = dbc.connection.lock();
        let ch = match conn.state {
            ConnectionState::Connected { conn_handle, .. } => Some(conn_handle),
            ConnectionState::Disconnected => None,
        };
        (ch, conn.current_catalog.clone())
    };
    match conn_handle {
        Some(handle) => {
            let db = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
                let info = c
                    .connection_get_info(ConnectionGetInfoRequest {
                        conn_handle: Some(handle),
                        info_codes: vec![],
                        include_master_token: false,
                    })
                    .await?;
                Ok::<Option<String>, crate::api::OdbcError>(info.database)
            })?;
            dbc.connection.lock().current_catalog = db.clone();
            Ok(db)
        }
        None => Ok(cached),
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
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    tracing::debug!("get_info: connection_handle={connection_handle:?}, info_type={info_type}");

    let dbc = conn_from_handle(connection_handle)?;

    let info_type = InfoType::try_from(info_type)?;
    tracing::debug!("get_info: info_type={info_type:?}");

    // Local aliases to keep each match arm a single readable line.
    //
    // `write_str` only forwards `warnings` when `info_value_ptr` is non-null:
    // a NULL-buffer call is a pure length probe with nothing to truncate, so
    // it must stay plain `SQL_SUCCESS` rather than surfacing a spurious
    // `SQL_SUCCESS_WITH_INFO` / 01004.
    let mut write_str = |s: &str| {
        let w = if info_value_ptr.is_null() {
            None
        } else {
            Some(&mut *warnings)
        };
        write_get_info_string::<E>(s, info_value_ptr, buffer_length, string_length_ptr, w);
    };
    let write_u16 = |v: u16| write_get_info_u16(v, info_value_ptr, string_length_ptr);
    let write_u32 = |v: u32| write_get_info_u32(v, info_value_ptr, string_length_ptr);

    match info_type {
        // ----- Strings -----------------------------------------------------
        InfoType::DataSourceName => {
            // Per ODBC spec, "the data source name used during connection".
            // We captured this at connect time from `SQLConnect`'s server-name
            // argument or `SQLDriverConnect`'s `DSN=...` (see `dsn_name`,
            // also read by `SQL_DRIVER_NAME` above). On most platforms the
            // Driver Manager intercepts this InfoType and answers it itself
            // without ever calling into the driver, but Windows does not, so
            // the driver must answer with the DSN it was handed.
            let dsn_name = dbc.connection.lock().dsn_name.clone();
            write_str(dsn_name.as_deref().unwrap_or(""));
        }
        InfoType::DriverName => {
            // Per ODBC spec, `SQL_DRIVER_NAME` returns "a character string
            // with the file name of the driver used to access the data
            // source" — i.e. the on-disk path of the shared library the
            // Driver Manager loaded. We resolve it via the DM's installer
            // API (`SQLGetPrivateProfileString`) using whichever lookup
            // hints we captured at connect time; see
            // [`odbc_installer::resolve_driver_path`] for the layering.
            let (driver_section, dsn_name) = {
                let conn = dbc.connection.lock();
                (conn.driver_section.clone(), conn.dsn_name.clone())
            };
            let path = resolve_driver_path(driver_section.as_deref(), dsn_name.as_deref());
            write_str(&path);
        }
        InfoType::DriverVer => write_str(ODBC_DRIVER_VERSION),
        InfoType::DbmsName => write_str("Snowflake"),
        InfoType::DatabaseName => {
            let db = current_database(&dbc)?;
            write_str(db.as_deref().unwrap_or(""))
        }
        InfoType::DbmsVer => {
            // Sourced from `serverVersion` in the login response (parsed in
            // [`sf_core::rest::snowflake::auth::AuthResponseMain`]). Matches
            // the legacy driver and avoids the extra `SELECT CURRENT_VERSION()`
            // round trip that JDBC currently performs.
            //
            // Uses the dedicated `connection_get_server_version` getter
            // rather than `connection_get_info` — Excel polls this attribute
            // during `SQLDriverConnect` and the full info aggregation is
            // wasteful when only the version is needed.
            //
            // Before the connection is established, sf_core has no
            // `server_version` yet — return an empty string instead of
            // surfacing an error so callers that probe this attribute during
            // `SQLDriverConnect` (Excel does) still succeed.
            let conn_handle = match dbc.connection.lock().state {
                ConnectionState::Connected { conn_handle, .. } => Some(conn_handle),
                ConnectionState::Disconnected => None,
            };
            let version = match conn_handle {
                Some(handle) => global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
                    let resp = c
                        .connection_get_server_version(ConnectionGetServerVersionRequest {
                            conn_handle: Some(handle),
                        })
                        .await?;
                    Ok::<Option<String>, crate::api::OdbcError>(resp.server_version)
                })?,
                None => None,
            };
            write_str(version.as_deref().unwrap_or(""));
        }
        InfoType::UserName => {
            // Login user for this connection. Sourced from the same
            // connection-seed `USER` setting the legacy driver uses for
            // `DSI_CONN_USER_NAME` — not a server round trip — via
            // `connection_get_info`'s in-process session-state read (see
            // `current_database` above for the same aggregation call).
            //
            // Before the connection is established there is no session to
            // read; return an empty string rather than erroring so callers
            // that probe this attribute pre-`SQLConnect` still succeed.
            let conn_handle = match dbc.connection.lock().state {
                ConnectionState::Connected { conn_handle, .. } => Some(conn_handle),
                ConnectionState::Disconnected => None,
            };
            let user = match conn_handle {
                Some(handle) => global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
                    let info = c
                        .connection_get_info(ConnectionGetInfoRequest {
                            conn_handle: Some(handle),
                            info_codes: vec![],
                            include_master_token: false,
                        })
                        .await?;
                    Ok::<Option<String>, crate::api::OdbcError>(info.user)
                })?,
                None => None,
            };
            write_str(user.as_deref().unwrap_or(""));
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
            write_str(ODBC_API_VERSION);
        }
        InfoType::SearchPatternEscape => write_str("\\"),
        InfoType::IdentifierQuoteChar => write_str("\""),
        InfoType::SchemaTerm => write_str("schema"),
        InfoType::CatalogNameSeparator => write_str("."),
        InfoType::CatalogTerm => write_str("database"),
        InfoType::DataSourceReadOnly => write_str("N"),
        InfoType::MultResultSets => write_str("N"),
        InfoType::TableTerm => write_str("table"),
        InfoType::ColumnAlias => write_str("Y"),
        InfoType::OrderByColumnsInSelect => write_str("N"),
        InfoType::SpecialCharacters => write_str(""),
        InfoType::NeedLongDataLen => write_str("N"),
        InfoType::CatalogName => write_str("Y"),
        // New string InfoTypes
        InfoType::ServerName => write_str("Snowflake"),
        InfoType::RowUpdates => write_str("N"),
        InfoType::AccessibleTables => write_str("Y"),
        InfoType::AccessibleProcedures => write_str("Y"),
        InfoType::Procedures => write_str("N"),
        InfoType::ExpressionsInOrderby => write_str("Y"),
        InfoType::MultipleActiveTxn => write_str("Y"),
        InfoType::ProcedureTerm => write_str("procedure"),
        InfoType::Integrity => write_str("N"),
        InfoType::Keywords => write_str(""),
        InfoType::MaxRowSizeIncludesLong => write_str("N"),
        InfoType::LikeEscapeClause => write_str("Y"),
        InfoType::XopenCliYear => write_str("1995"),
        InfoType::DescribeParameter => write_str("Y"),
        InfoType::CollationSeq => {
            if cfg!(windows) {
                write_str("UTF-16LE_BINARY")
            } else {
                write_str("UTF-32LE_BINARY")
            }
        }

        // ----- Scalar `SQLUSMALLINT` --------------------------------------
        InfoType::ActiveStatements => write_u16(0),
        InfoType::CursorCommitBehavior | InfoType::CursorRollbackBehavior => write_u16(1), // SQL_CB_CLOSE
        InfoType::ConcatNullBehavior => write_u16(0), // SQL_CB_NULL
        InfoType::GroupBy => write_u16(2),            // SQL_GB_GROUP_BY_CONTAINS_SELECT
        InfoType::MaxSchemaNameLen => write_u16(255),
        InfoType::MaxColumnsInGroupBy => write_u16(65535),
        InfoType::MaxColumnsInOrderBy => write_u16(65535),
        InfoType::MaxColumnsInSelect => write_u16(65535),
        InfoType::CatalogLocation => write_u16(1), // SQL_CL_START
        InfoType::MaxIdentifierLen => write_u16(255),
        InfoType::TxnCapable => write_u16(3), // SQL_TC_DDL_COMMIT
        InfoType::CorrelationName => write_u16(2), // SQL_CN_ANY
        InfoType::NonNullableColumns => write_u16(0), // SQL_NNC_NULL
        InfoType::FileUsage => write_u16(0),  // SQL_FILE_NOT_SUPPORTED
        // New SQLUSMALLINT InfoTypes
        InfoType::MaxDriverConnections => write_u16(0),
        InfoType::OdbcApiConformance => write_u16(2), // SQL_OAC_LEVEL2
        InfoType::OdbcSqlConformance => write_u16(1), // SQL_OSC_CORE
        InfoType::IdentifierCase => write_u16(1),     // SQL_IC_UPPER
        InfoType::MaxColumnNameLen => write_u16(255),
        InfoType::MaxCursorNameLen => write_u16(0),
        InfoType::MaxProcedureNameLen => write_u16(0),
        InfoType::MaxCatalogNameLen => write_u16(255),
        InfoType::MaxTableNameLen => write_u16(255),
        InfoType::NullCollation => write_u16(0), // SQL_NC_HIGH
        InfoType::QuotedIdentifierCase => write_u16(3), // SQL_IC_SENSITIVE
        InfoType::MaxColumnsInIndex => write_u16(0),
        InfoType::MaxColumnsInTable => write_u16(65535),
        InfoType::MaxTablesInSelect => write_u16(0),
        InfoType::MaxUserNameLen => write_u16(0),
        InfoType::ActiveEnvironments => write_u16(0),

        // ----- Scalar `SQLUINTEGER` ---------------------------------------
        InfoType::DefaultTxnIsolation => write_u32(SQL_TXN_READ_COMMITTED),
        InfoType::SqlConformance => write_u32(1), // SQL_SC_SQL92_ENTRY
        InfoType::OdbcInterfaceConformance => write_u32(1), // SQL_OIC_CORE
        InfoType::AsyncMode => write_u32(2),      // SQL_AM_STATEMENT
        InfoType::MaxAsyncConcurrentStatements => write_u32(0),
        InfoType::AsyncDbcFunctions => write_u32(1), // SQL_ASYNC_DBC_CAPABLE
        InfoType::AsyncNotification => write_u32(0), // SQL_ASYNC_NOTIFICATION_NOT_CAPABLE
        // New scalar SQLUINTEGER InfoTypes
        InfoType::MaxBinaryLiteralLen => write_u32(0),
        InfoType::MaxCharLiteralLen => write_u32(16_777_216),
        InfoType::MaxIndexSize => write_u32(0),
        InfoType::MaxRowSize => write_u32(16_777_216),
        InfoType::MaxStatementLen => write_u32(0),
        InfoType::CursorSensitivity => write_u32(0), // SQL_UNSPECIFIED
        InfoType::DriverAwarePoolingSupported => write_u32(0),
        InfoType::IndexKeywords => write_u32(0), // SQL_IK_NONE
        InfoType::ParamArrayRowCounts => write_u32(2), // SQL_PARC_NO_BATCH
        InfoType::ParamArraySelects => write_u32(2), // SQL_PAS_NO_BATCH
        InfoType::StandardCliConformance => write_u32(2), // SQL_SCC_ISO92_CLI

        // ----- Bitmask `SQLUINTEGER` (with-slice families) -----------------
        InfoType::GetDataExtensions => write_u32(
            [
                GetDataExtensions::AnyColumn,
                GetDataExtensions::AnyOrder,
                GetDataExtensions::Bound,
            ]
            .bitmask(),
        ),
        InfoType::AggregateFunctions => write_u32(synthesize(AGGREGATE_FUNCTIONS)),
        InfoType::CatalogUsage => write_u32(synthesize(CATALOG_USAGE)),
        InfoType::SchemaUsage => write_u32(synthesize(SCHEMA_USAGE)),
        InfoType::ConvertFunctions => write_u32(synthesize(CONVERT_FUNCTIONS)),
        InfoType::NumericFunctions => write_u32(synthesize(NUMERIC_FUNCTIONS)),
        InfoType::StringFunctions => write_u32(synthesize(STRING_FUNCTIONS)),
        InfoType::SystemFunctions => write_u32(synthesize(SYSTEM_FUNCTIONS)),
        InfoType::TimedateFunctions => write_u32(synthesize(TIMEDATE_FUNCTIONS)),
        InfoType::TimedateAddIntervals => write_u32(synthesize(TIMEDATE_TSI_INTERVALS)),
        InfoType::TimedateDiffIntervals => write_u32(synthesize(TIMEDATE_TSI_INTERVALS)),
        InfoType::Sql92Predicates => write_u32(synthesize(SQL92_PREDICATES)),
        InfoType::Sql92RelationalJoinOperators => {
            write_u32(synthesize(SQL92_RELATIONAL_JOIN_OPERATORS))
        }
        InfoType::Sql92ValueExpressions => write_u32(synthesize(SQL92_VALUE_EXPRESSIONS)),
        InfoType::ScrollConcurrency => write_u32(synthesize(SCROLL_CONCURRENCY)),
        InfoType::ScrollOptions => write_u32(synthesize(SCROLL_OPTIONS)),
        InfoType::TxnIsolationOption => write_u32(synthesize(TXN_ISOLATION_OPTION)),
        InfoType::LockTypes => write_u32(synthesize(LOCK_TYPES)),
        InfoType::PosOperations => write_u32(synthesize(POS_OPERATIONS)),
        InfoType::BookmarkPersistence => write_u32(synthesize(BOOKMARK_PERSISTENCE)),
        InfoType::StaticSensitivity => write_u32(synthesize(STATIC_SENSITIVITY)),
        InfoType::ForwardOnlyCursorAttributes1 => {
            write_u32(synthesize(FORWARD_ONLY_CURSOR_ATTRIBUTES1))
        }
        InfoType::ForwardOnlyCursorAttributes2 => {
            write_u32(synthesize(FORWARD_ONLY_CURSOR_ATTRIBUTES2))
        }
        InfoType::KeysetCursorAttributes1 => write_u32(synthesize(KEYSET_CURSOR_ATTRIBUTES1)),
        InfoType::KeysetCursorAttributes2 => write_u32(synthesize(KEYSET_CURSOR_ATTRIBUTES2)),
        InfoType::StaticCursorAttributes1 => write_u32(synthesize(STATIC_CURSOR_ATTRIBUTES1)),
        InfoType::StaticCursorAttributes2 => write_u32(synthesize(STATIC_CURSOR_ATTRIBUTES2)),
        InfoType::DynamicCursorAttributes1 => write_u32(synthesize(DYNAMIC_CURSOR_ATTRIBUTES1)),
        // New bitmask InfoTypes
        InfoType::FetchDirection => write_u32(synthesize(FETCH_DIRECTION)),
        InfoType::AlterTable => write_u32(synthesize(ALTER_TABLE)),
        InfoType::AlterDomain => write_u32(synthesize(ALTER_DOMAIN)),
        InfoType::OjCapabilities => write_u32(synthesize(OJ_CAPABILITIES)),
        InfoType::DatetimeLiterals => write_u32(synthesize(DATETIME_LITERALS)),
        InfoType::BatchRowCount => write_u32(synthesize(BATCH_ROW_COUNT)),
        InfoType::BatchSupport => write_u32(synthesize(BATCH_SUPPORT)),
        InfoType::CreateAssertion => write_u32(synthesize(CREATE_ASSERTION)),
        InfoType::CreateCharacterSet => write_u32(synthesize(CREATE_CHARACTER_SET)),
        InfoType::CreateCollation => write_u32(synthesize(CREATE_COLLATION)),
        InfoType::CreateDomain => write_u32(synthesize(CREATE_DOMAIN)),
        InfoType::CreateSchema => write_u32(synthesize(CREATE_SCHEMA)),
        InfoType::CreateTable => write_u32(synthesize(CREATE_TABLE)),
        InfoType::CreateTranslation => write_u32(synthesize(CREATE_TRANSLATION)),
        InfoType::CreateView => write_u32(synthesize(CREATE_VIEW)),
        InfoType::DropAssertion => write_u32(synthesize(DROP_ASSERTION)),
        InfoType::DropCharacterSet => write_u32(synthesize(DROP_CHARACTER_SET)),
        InfoType::DropCollation => write_u32(synthesize(DROP_COLLATION)),
        InfoType::DropDomain => write_u32(synthesize(DROP_DOMAIN)),
        InfoType::DropSchema => write_u32(synthesize(DROP_SCHEMA)),
        InfoType::DropTable => write_u32(synthesize(DROP_TABLE)),
        InfoType::DropTranslation => write_u32(synthesize(DROP_TRANSLATION)),
        InfoType::DropView => write_u32(synthesize(DROP_VIEW)),
        InfoType::DynamicCursorAttributes2 => write_u32(synthesize(DYNAMIC_CURSOR_ATTRIBUTES2)),
        InfoType::InfoSchemaViews => write_u32(synthesize(INFO_SCHEMA_VIEWS)),
        InfoType::PositionedStatements => write_u32(synthesize(POSITIONED_STATEMENTS)),
        InfoType::Subqueries => write_u32(synthesize(SUBQUERIES)),
        InfoType::Union => write_u32(synthesize(UNION)),
        InfoType::Sql92DatetimeFunctions => write_u32(synthesize(SQL92_DATETIME_FUNCTIONS)),
        InfoType::Sql92ForeignKeyDeleteRule => write_u32(synthesize(SQL92_FOREIGN_KEY_DELETE_RULE)),
        InfoType::Sql92ForeignKeyUpdateRule => write_u32(synthesize(SQL92_FOREIGN_KEY_UPDATE_RULE)),
        InfoType::Sql92Grant => write_u32(synthesize(SQL92_GRANT)),
        InfoType::Sql92NumericValueFunctions => {
            write_u32(synthesize(SQL92_NUMERIC_VALUE_FUNCTIONS))
        }
        InfoType::Sql92Revoke => write_u32(synthesize(SQL92_REVOKE)),
        InfoType::Sql92RowValueConstructor => write_u32(synthesize(SQL92_ROW_VALUE_CONSTRUCTOR)),
        InfoType::Sql92StringFunctions => write_u32(synthesize(SQL92_STRING_FUNCTIONS)),
        InfoType::DdlIndex => write_u32(synthesize(DDL_INDEX)),
        InfoType::InsertStatement => write_u32(synthesize(INSERT_STATEMENT)),

        // ----- `SQL_CONVERT_<source>` bitmasks (per-source-type) ----------
        InfoType::ConvertBigint => write_u32(synthesize(CONVERT_BIGINT)),
        InfoType::ConvertBinary => write_u32(synthesize(CONVERT_BINARY)),
        InfoType::ConvertBit => write_u32(synthesize(CONVERT_BIT)),
        InfoType::ConvertChar => write_u32(synthesize(CONVERT_CHAR)),
        InfoType::ConvertDate => write_u32(synthesize(CONVERT_DATE)),
        InfoType::ConvertDecimal => write_u32(synthesize(CONVERT_DECIMAL)),
        InfoType::ConvertDouble => write_u32(synthesize(CONVERT_DOUBLE)),
        InfoType::ConvertFloat => write_u32(synthesize(CONVERT_FLOAT)),
        InfoType::ConvertGuid => write_u32(synthesize(CONVERT_GUID)),
        InfoType::ConvertInteger => write_u32(synthesize(CONVERT_INTEGER)),
        InfoType::ConvertLongVarbinary => write_u32(synthesize(CONVERT_LONGVARBINARY)),
        InfoType::ConvertLongVarchar => write_u32(synthesize(CONVERT_LONGVARCHAR)),
        InfoType::ConvertNumeric => write_u32(synthesize(CONVERT_NUMERIC)),
        InfoType::ConvertReal => write_u32(synthesize(CONVERT_REAL)),
        InfoType::ConvertSmallint => write_u32(synthesize(CONVERT_SMALLINT)),
        InfoType::ConvertTime => write_u32(synthesize(CONVERT_TIME)),
        InfoType::ConvertTimestamp => write_u32(synthesize(CONVERT_TIMESTAMP)),
        InfoType::ConvertTinyint => write_u32(synthesize(CONVERT_TINYINT)),
        InfoType::ConvertVarbinary => write_u32(synthesize(CONVERT_VARBINARY)),
        InfoType::ConvertVarchar => write_u32(synthesize(CONVERT_VARCHAR)),
        InfoType::ConvertWchar => write_u32(synthesize(CONVERT_WCHAR)),
        InfoType::ConvertWlongVarchar => write_u32(synthesize(CONVERT_WLONGVARCHAR)),
        InfoType::ConvertWvarchar => write_u32(synthesize(CONVERT_WVARCHAR)),
        InfoType::ConvertIntervalDayTime => write_u32(synthesize(CONVERT_INTERVAL_DAY_TIME)),
        InfoType::ConvertIntervalYearMonth => write_u32(synthesize(CONVERT_INTERVAL_YEAR_MONTH)),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// SQLGetFunctions
// ---------------------------------------------------------------------------

const SQL_API_ALL_FUNCTIONS: sql::USmallInt = 0;
const SQL_API_ODBC3_ALL_FUNCTIONS: sql::USmallInt = 999;
const SQL_API_ODBC3_ALL_FUNCTIONS_SIZE: usize = 250;
const SQL_TRUE_U16: sql::USmallInt = 1;
const SQL_FALSE_U16: sql::USmallInt = 0;

/// All known ODBC function IDs that can be queried via `SQLGetFunctions`.
///
/// Every standard ODBC 2.x/3.x function has a variant here.  The
/// [`OdbcFunction::is_supported`] method returns `true` only for the
/// entry-points this driver actually implements.
///
/// When a new entry-point is added to `c_api.rs`, flip its
/// `is_supported` return value to `true`.
#[repr(u16)]
#[derive(Clone, Copy)]
enum OdbcFunction {
    // ---- Handle management ------------------------------------------------
    AllocHandle = 1001,
    FreeHandle = 1006,
    FreeStmt = 16,

    // ---- Connection -------------------------------------------------------
    BrowseConnect = 55,
    Connect = 7,
    DriverConnect = 41,
    Disconnect = 9,

    // ---- Driver information -----------------------------------------------
    DataSources = 57,
    Drivers = 71,
    GetFunctions = 44,
    GetInfo = 45,
    GetTypeInfo = 47,

    // ---- Catalog ----------------------------------------------------------
    ColumnPrivileges = 56,
    Columns = 40,
    ForeignKeys = 60,
    PrimaryKeys = 65,
    ProcedureColumns = 66,
    Procedures = 67,
    SpecialColumns = 52,
    Statistics = 53,
    TablePrivileges = 70,
    Tables = 54,

    // ---- Statement preparation --------------------------------------------
    BindParameter = 72,
    GetCursorName = 17,
    Prepare = 19,
    SetCursorName = 21,
    SetScrollOptions = 69,

    // ---- Result retrieval -------------------------------------------------
    BindCol = 4,
    BulkOperations = 24,
    ColAttribute = 6, // also SQLColAttributes (ODBC 2.x), same ID
    DescribeCol = 8,
    ExtendedFetch = 59,
    Fetch = 13,
    FetchScroll = 1021,
    GetData = 43,
    GetDiagField = 1010,
    GetDiagRec = 1011,
    MoreResults = 61,
    NumResultCols = 18,
    RowCount = 20,
    SetPos = 68,

    // ---- Descriptor -------------------------------------------------------
    CopyDesc = 1004,
    GetDescField = 1008,
    GetDescRec = 1009,
    SetDescField = 1017,
    SetDescRec = 1018,

    // ---- Attributes -------------------------------------------------------
    GetConnectAttr = 1007,
    GetEnvAttr = 1012,
    GetStmtAttr = 1014,
    ParamOptions = 64,
    SetConnectAttr = 1016,
    SetEnvAttr = 1019,
    SetStmtAttr = 1020,

    // ---- Execution --------------------------------------------------------
    DescribeParam = 58,
    ExecDirect = 11,
    Execute = 12,
    NativeSql = 62,
    NumParams = 63,
    ParamData = 48,
    PutData = 49,

    // ---- Statement / transaction termination ------------------------------
    Cancel = 5,
    CancelHandle = 1022,
    CloseCursor = 1003,
    Transact = 23,
    EndTran = 1005,
}

impl TryFrom<u16> for OdbcFunction {
    type Error = ();

    #[rustfmt::skip]
    fn try_from(v: u16) -> Result<Self, ()> {
        match v {
            1001 => Ok(Self::AllocHandle),
            1006 => Ok(Self::FreeHandle),
            16   => Ok(Self::FreeStmt),
            55   => Ok(Self::BrowseConnect),
            7    => Ok(Self::Connect),
            41   => Ok(Self::DriverConnect),
            9    => Ok(Self::Disconnect),
            57   => Ok(Self::DataSources),
            71   => Ok(Self::Drivers),
            44   => Ok(Self::GetFunctions),
            45   => Ok(Self::GetInfo),
            47   => Ok(Self::GetTypeInfo),
            56   => Ok(Self::ColumnPrivileges),
            40   => Ok(Self::Columns),
            60   => Ok(Self::ForeignKeys),
            65   => Ok(Self::PrimaryKeys),
            66   => Ok(Self::ProcedureColumns),
            67   => Ok(Self::Procedures),
            52   => Ok(Self::SpecialColumns),
            53   => Ok(Self::Statistics),
            70   => Ok(Self::TablePrivileges),
            54   => Ok(Self::Tables),
            72   => Ok(Self::BindParameter),
            17   => Ok(Self::GetCursorName),
            19   => Ok(Self::Prepare),
            21   => Ok(Self::SetCursorName),
            69   => Ok(Self::SetScrollOptions),
            4    => Ok(Self::BindCol),
            24   => Ok(Self::BulkOperations),
            6    => Ok(Self::ColAttribute),
            8    => Ok(Self::DescribeCol),
            59   => Ok(Self::ExtendedFetch),
            13   => Ok(Self::Fetch),
            1021 => Ok(Self::FetchScroll),
            43   => Ok(Self::GetData),
            1010 => Ok(Self::GetDiagField),
            1011 => Ok(Self::GetDiagRec),
            61   => Ok(Self::MoreResults),
            18   => Ok(Self::NumResultCols),
            20   => Ok(Self::RowCount),
            68   => Ok(Self::SetPos),
            1004 => Ok(Self::CopyDesc),
            1008 => Ok(Self::GetDescField),
            1009 => Ok(Self::GetDescRec),
            1017 => Ok(Self::SetDescField),
            1018 => Ok(Self::SetDescRec),
            1007 => Ok(Self::GetConnectAttr),
            1012 => Ok(Self::GetEnvAttr),
            1014 => Ok(Self::GetStmtAttr),
            64   => Ok(Self::ParamOptions),
            1016 => Ok(Self::SetConnectAttr),
            1019 => Ok(Self::SetEnvAttr),
            1020 => Ok(Self::SetStmtAttr),
            58   => Ok(Self::DescribeParam),
            11   => Ok(Self::ExecDirect),
            12   => Ok(Self::Execute),
            62   => Ok(Self::NativeSql),
            63   => Ok(Self::NumParams),
            48   => Ok(Self::ParamData),
            49   => Ok(Self::PutData),
            5    => Ok(Self::Cancel),
            1022 => Ok(Self::CancelHandle),
            1003 => Ok(Self::CloseCursor),
            23   => Ok(Self::Transact),
            1005 => Ok(Self::EndTran),
            _    => Err(()),
        }
    }
}

impl OdbcFunction {
    /// Whether this driver exports the function in `c_api.rs`.
    fn is_supported(self) -> bool {
        !matches!(
            self,
            Self::BrowseConnect
                | Self::BulkOperations
                | Self::DataSources
                | Self::Drivers
                | Self::ParamOptions
                | Self::SetPos
                | Self::SetScrollOptions
        )
    }

    const ALL: &[Self] = &[
        Self::AllocHandle,
        Self::FreeHandle,
        Self::FreeStmt,
        Self::BrowseConnect,
        Self::Connect,
        Self::DriverConnect,
        Self::Disconnect,
        Self::DataSources,
        Self::Drivers,
        Self::GetFunctions,
        Self::GetInfo,
        Self::GetTypeInfo,
        Self::ColumnPrivileges,
        Self::Columns,
        Self::ForeignKeys,
        Self::PrimaryKeys,
        Self::ProcedureColumns,
        Self::Procedures,
        Self::SpecialColumns,
        Self::Statistics,
        Self::TablePrivileges,
        Self::Tables,
        Self::BindParameter,
        Self::GetCursorName,
        Self::Prepare,
        Self::SetCursorName,
        Self::SetScrollOptions,
        Self::BindCol,
        Self::BulkOperations,
        Self::ColAttribute,
        Self::DescribeCol,
        Self::ExtendedFetch,
        Self::Fetch,
        Self::FetchScroll,
        Self::GetData,
        Self::GetDiagField,
        Self::GetDiagRec,
        Self::MoreResults,
        Self::NumResultCols,
        Self::RowCount,
        Self::SetPos,
        Self::CopyDesc,
        Self::GetDescField,
        Self::GetDescRec,
        Self::SetDescField,
        Self::SetDescRec,
        Self::GetConnectAttr,
        Self::GetEnvAttr,
        Self::GetStmtAttr,
        Self::ParamOptions,
        Self::SetConnectAttr,
        Self::SetEnvAttr,
        Self::SetStmtAttr,
        Self::DescribeParam,
        Self::ExecDirect,
        Self::Execute,
        Self::NativeSql,
        Self::NumParams,
        Self::ParamData,
        Self::PutData,
        Self::Cancel,
        Self::CancelHandle,
        Self::CloseCursor,
        Self::Transact,
        Self::EndTran,
    ];
}

/// Fill the ODBC 3.x bitmap (4 000 bits, `SQL_API_ODBC3_ALL_FUNCTIONS_SIZE`
/// words) with supported function IDs.
fn fill_odbc3_bitmap(supported_ptr: *mut sql::USmallInt) {
    let bitmap =
        unsafe { std::slice::from_raw_parts_mut(supported_ptr, SQL_API_ODBC3_ALL_FUNCTIONS_SIZE) };
    bitmap.fill(0);
    for &f in OdbcFunction::ALL {
        if !f.is_supported() {
            continue;
        }
        let fid = f as u16;
        let word = (fid >> 4) as usize;
        let bit = fid & 0x000F;
        bitmap[word] |= 1 << bit;
    }
}

/// Fill the ODBC 2.x 100-element array with `SQL_TRUE` / `SQL_FALSE` per index.
fn fill_odbc2_array(supported_ptr: *mut sql::USmallInt) {
    let array = unsafe { std::slice::from_raw_parts_mut(supported_ptr, 100) };
    array.fill(SQL_FALSE_U16);
    for &f in OdbcFunction::ALL {
        if !f.is_supported() {
            continue;
        }
        let idx = f as usize;
        if idx < 100 {
            array[idx] = SQL_TRUE_U16;
        }
    }
}

/// Retrieve supported-function information (SQLGetFunctions).
pub fn get_functions(
    connection_handle: sql::Handle,
    function_id: sql::USmallInt,
    supported_ptr: *mut sql::USmallInt,
) -> OdbcResult<()> {
    tracing::debug!(
        "get_functions: connection_handle={connection_handle:?}, function_id={function_id}"
    );

    let dbc = conn_from_handle(connection_handle)?;

    if matches!(dbc.connection.lock().state, ConnectionState::Disconnected) {
        return DisconnectedSnafu.fail();
    }

    if supported_ptr.is_null() {
        return Ok(());
    }

    if function_id == SQL_API_ODBC3_ALL_FUNCTIONS {
        fill_odbc3_bitmap(supported_ptr);
        return Ok(());
    }
    if function_id == SQL_API_ALL_FUNCTIONS {
        fill_odbc2_array(supported_ptr);
        return Ok(());
    }

    if function_id >= 4000 {
        return crate::api::error::FunctionTypeOutOfRangeSnafu { function_id }.fail();
    }

    let supported = OdbcFunction::try_from(function_id)
        .map(|f| f.is_supported())
        .unwrap_or(false);
    unsafe {
        *supported_ptr = if supported {
            SQL_TRUE_U16
        } else {
            SQL_FALSE_U16
        };
    }
    Ok(())
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
    fn normalize_connection_string_options_forwards_tls_version_keys_for_core_resolution() {
        // MIN_TLS_VERSION and MAX_TLS_VERSION flow through as UPPERCASE so
        // sf_core's registry can resolve them (case-insensitive) to the
        // canonical min_tls_version / max_tls_version names.  Values are
        // preserved as-is; TlsVersion::parse lowercases before matching.
        let options = normalize_connection_string_options(HashMap::from([
            ("MIN_TLS_VERSION".to_owned(), "tls12".to_owned()),
            ("MAX_TLS_VERSION".to_owned(), "tls13".to_owned()),
        ]));

        assert_eq!(config_string(&options, "MIN_TLS_VERSION"), Some("tls12"));
        assert_eq!(config_string(&options, "MAX_TLS_VERSION"), Some("tls13"));
        assert!(!options.contains_key("min_tls_version"));
        assert!(!options.contains_key("max_tls_version"));
    }

    #[test]
    fn resolve_global_ssl_version_maps_legacy_odbc_spellings() {
        // The values old snowflake-odbc accepted for `SSLVersion`, mapped to the
        // canonical core tokens.
        assert_eq!(
            resolve_global_ssl_version("TLSv1_2").unwrap(),
            Some("tls12")
        );
        assert_eq!(
            resolve_global_ssl_version("TLSv1_3").unwrap(),
            Some("tls13")
        );
        assert_eq!(
            resolve_global_ssl_version("tlsv1_3").unwrap(),
            Some("tls13")
        );
        assert_eq!(
            resolve_global_ssl_version("TLSv1.2").unwrap(),
            Some("tls12")
        );
        assert_eq!(resolve_global_ssl_version("tls13").unwrap(), Some("tls13"));
    }

    #[test]
    fn resolve_global_ssl_version_default_or_empty_means_no_override() {
        assert_eq!(resolve_global_ssl_version("DEFAULT").unwrap(), None);
        assert_eq!(resolve_global_ssl_version("default").unwrap(), None);
        assert_eq!(resolve_global_ssl_version("   ").unwrap(), None);
    }

    #[test]
    fn resolve_global_ssl_version_rejects_sub_tls12_and_unknown() {
        // rustls cannot honour anything below TLS 1.2, so these fail closed
        // rather than silently downgrade (the old driver merely warned).
        for v in ["TLSv1", "TLSv1_0", "TLSv1_1", "SSLv2", "SSLv3", "bogus"] {
            assert!(
                resolve_global_ssl_version(v).is_err(),
                "value {v} should be rejected"
            );
        }
    }

    #[test]
    fn pin_tls_version_overrides_explicit_min_and_max() {
        // Explicit user keys (uppercase, as forwarded) are dropped and replaced
        // by the pinned canonical version.
        let mut options = normalize_connection_string_options(HashMap::from([
            ("MIN_TLS_VERSION".to_owned(), "tls12".to_owned()),
            ("MAX_TLS_VERSION".to_owned(), "tls12".to_owned()),
        ]));
        pin_tls_version(&mut options, "tls13");
        assert_eq!(config_string(&options, "min_tls_version"), Some("tls13"));
        assert_eq!(config_string(&options, "max_tls_version"), Some("tls13"));
        assert!(!options.contains_key("MIN_TLS_VERSION"));
        assert!(!options.contains_key("MAX_TLS_VERSION"));
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
    fn normalize_connection_string_options_forwards_proxy_keys_for_core_aliases() {
        // Proxy keys are forwarded UPPERCASE; sf_core's param registry resolves
        // them to canonical lowercase names via the registered aliases.
        let options = normalize_connection_string_options(HashMap::from([
            ("PROXY_HOST".to_owned(), "p.example.com".to_owned()),
            ("PROXY_PORT".to_owned(), "8080".to_owned()),
            ("PROXY_USER".to_owned(), "puser".to_owned()),
            ("PROXY_PASSWORD".to_owned(), "ppass".to_owned()),
            ("NO_PROXY".to_owned(), "internal,*.local".to_owned()),
        ]));

        assert_eq!(config_string(&options, "PROXY_HOST"), Some("p.example.com"));
        assert_eq!(config_string(&options, "PROXY_PORT"), Some("8080"));
        assert_eq!(config_string(&options, "PROXY_USER"), Some("puser"));
        assert_eq!(config_string(&options, "PROXY_PASSWORD"), Some("ppass"));
        assert_eq!(
            config_string(&options, "NO_PROXY"),
            Some("internal,*.local")
        );
        // Pre-canonicalisation is the registry's job; ODBC layer does not
        // emit lowercase canonical keys.
        assert!(!options.contains_key("proxy_host"));
        assert!(!options.contains_key("no_proxy"));
    }

    #[test]
    fn normalize_connection_string_options_passes_through_legacy_proxy_url_form() {
        // Legacy ODBC DSNs use `PROXY=[scheme://][user:pass@]host[:port]`.
        // sf_core's `ProxyConfig::from_settings` parses the URL.  The ODBC
        // layer just forwards the value unchanged.
        let options = normalize_connection_string_options(HashMap::from([(
            "PROXY".to_owned(),
            "http://user:pass@p.example.com:8080".to_owned(),
        )]));

        assert_eq!(
            config_string(&options, "PROXY"),
            Some("http://user:pass@p.example.com:8080")
        );
    }

    #[test]
    fn normalize_connection_string_options_passes_through_legacy_odbc_proxy_aliases() {
        // Legacy ODBC also accepts NOPROXY / PROXYWITHENV / ALLOWEMPTYPROXY.
        // These flow through as UPPERCASE and sf_core's registry resolves
        // them to canonical names.
        let options = normalize_connection_string_options(HashMap::from([
            ("NOPROXY".to_owned(), "*.corp".to_owned()),
            ("PROXYWITHENV".to_owned(), "true".to_owned()),
            ("ALLOWEMPTYPROXY".to_owned(), "false".to_owned()),
        ]));
        assert_eq!(config_string(&options, "NOPROXY"), Some("*.corp"));
        assert_eq!(config_string(&options, "PROXYWITHENV"), Some("true"));
        assert_eq!(config_string(&options, "ALLOWEMPTYPROXY"), Some("false"));
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
    fn normalize_connection_string_options_maps_disable_parallel_user_prompt() {
        for input_value in ["true", "false", "1", "0"] {
            let options = normalize_connection_string_options(HashMap::from([(
                "DISABLE_PARALLEL_USER_PROMPT".to_owned(),
                input_value.to_owned(),
            )]));

            assert_eq!(
                config_string(&options, "disable_parallel_user_prompt"),
                Some(input_value),
                "value {input_value:?} should pass through unchanged"
            );
            // The original upper-case key must not survive normalization.
            assert!(
                !options.contains_key("DISABLE_PARALLEL_USER_PROMPT"),
                "upper-case key must be consumed by normalize"
            );
        }
    }

    #[test]
    fn normalize_connection_string_options_forwards_oauth_keys_with_canonical_names() {
        let options = normalize_connection_string_options(HashMap::from([
            ("OAUTH_CLIENT_ID".to_owned(), "client-123".to_owned()),
            ("OAUTH_CLIENT_SECRET".to_owned(), "shhh".to_owned()),
            (
                "OAUTH_REDIRECT_URI".to_owned(),
                "http://127.0.0.1:0".to_owned(),
            ),
            ("oauth_scope".to_owned(), "session:role:R".to_owned()),
        ]));

        // OAuth keys must be forwarded with their lowercase `sf_core`
        // canonical name (not the original SCREAMING_SNAKE form).
        assert_eq!(
            config_string(&options, "oauth_client_id"),
            Some("client-123")
        );
        assert_eq!(config_string(&options, "oauth_client_secret"), Some("shhh"));
        assert_eq!(
            config_string(&options, "oauth_redirect_uri"),
            Some("http://127.0.0.1:0")
        );
        assert_eq!(
            config_string(&options, "oauth_scope"),
            Some("session:role:R")
        );
        for upper in [
            "OAUTH_CLIENT_ID",
            "OAUTH_CLIENT_SECRET",
            "OAUTH_REDIRECT_URI",
            "OAUTH_SCOPE",
        ] {
            assert!(
                !options.contains_key(upper),
                "{upper} must not be forwarded as the uppercase passthrough form"
            );
        }
    }

    #[test]
    fn redacted_param_map_hides_oauth_client_secret_in_logs() {
        // Wiring guard: the connection-string log line must never
        // expose the OAUTH_CLIENT_SECRET value, regardless of the
        // case the caller used.
        let params = HashMap::from([
            ("UID".to_owned(), "joe".to_owned()),
            ("oauth_client_secret".to_owned(), "do-not-log".to_owned()),
        ]);
        let redacted = oauth::redacted_param_map(&params);
        let rendered = format!("{redacted:?}");
        assert!(
            !rendered.contains("do-not-log"),
            "redacted param map leaked the OAuth client secret: {rendered}"
        );
    }

    /// Belt-and-braces: every OAuth key declared in `oauth::ALL_OAUTH_KEYS`
    /// must round-trip through `normalize_connection_string_options` to its
    /// `sf_core` canonical lowercase name. Picks a plausible
    /// non-secret string value for every key so the assertion is uniform.
    #[test]
    fn normalize_connection_string_options_canonicalizes_every_oauth_key() {
        let mut input: HashMap<String, String> = HashMap::new();
        for &key in oauth::ALL_OAUTH_KEYS {
            // Use the key name itself as the value: makes any leak
            // immediately greppable, and keeps every key distinct in
            // the resulting options map.
            input.insert(key.to_owned(), format!("v-for-{key}"));
        }
        let options = normalize_connection_string_options(input);

        for &key in oauth::ALL_OAUTH_KEYS {
            let canonical = oauth::canonical_name(key)
                .unwrap_or_else(|| panic!("missing canonical name for {key}"));
            assert_eq!(
                config_string(&options, canonical),
                Some(format!("v-for-{key}").as_str()),
                "{key} did not round-trip to {canonical}"
            );
            assert!(
                !options.contains_key(key),
                "{key} should not survive as the SCREAMING_SNAKE form"
            );
        }
    }

    /// Mixed-case variants of OAuth keys (e.g. as a user would type
    /// them in a DSN file) must canonicalize to the same lowercase
    /// `sf_core` parameter name as the SCREAMING_SNAKE form.
    #[test]
    fn normalize_connection_string_options_oauth_keys_are_case_insensitive() {
        for &key in oauth::ALL_OAUTH_KEYS {
            let canonical = oauth::canonical_name(key).unwrap();
            for variant in [
                key.to_owned(),
                key.to_lowercase(),
                key.chars()
                    .enumerate()
                    .map(|(i, c)| {
                        if i.is_multiple_of(2) {
                            c.to_ascii_lowercase()
                        } else {
                            c.to_ascii_uppercase()
                        }
                    })
                    .collect::<String>(),
            ] {
                let options = normalize_connection_string_options(HashMap::from([(
                    variant.clone(),
                    "v".to_owned(),
                )]));
                assert_eq!(
                    config_string(&options, canonical),
                    Some("v"),
                    "variant {variant:?} of {key} did not canonicalize to {canonical}"
                );
            }
        }
    }

    /// Wiring guard: the OAuth canonical-name forwarding must not
    /// shadow the existing explicit special-key arms (PORT,
    /// CRL_ENABLED, CLIENT_STORE_TEMPORARY_CREDENTIAL, etc.). Mixing
    /// OAuth keys with these in one map must canonicalize each key
    /// according to its own arm, with no cross-contamination.
    #[test]
    fn normalize_connection_string_options_does_not_shadow_existing_special_keys() {
        let options = normalize_connection_string_options(HashMap::from([
            ("PORT".to_owned(), "9000".to_owned()),
            ("CRL_ENABLED".to_owned(), "1".to_owned()),
            ("OAUTH_CLIENT_ID".to_owned(), "abc".to_owned()),
            (
                "CLIENT_STORE_TEMPORARY_CREDENTIAL".to_owned(),
                "true".to_owned(),
            ),
            ("OAUTH_DISABLE_PKCE".to_owned(), "true".to_owned()),
        ]));

        assert_eq!(config_string(&options, "port"), Some("9000"));
        assert_eq!(config_string(&options, "CRL_ENABLED"), Some("ENABLED"));
        assert_eq!(config_string(&options, "oauth_client_id"), Some("abc"));
        assert_eq!(
            config_string(&options, "client_store_temporary_credential"),
            Some("true")
        );
        assert_eq!(config_string(&options, "oauth_disable_pkce"), Some("true"));
    }

    /// Wiring guard: every OAuth key forwarded by the wrapper must be
    /// resolvable by `sf_core::config::param_registry` to its
    /// canonical lowercase name. Catches accidental drift between the
    /// ODBC-side `oauth::canonical_name` map and the sf_core
    /// `param_registry` aliases.
    #[test]
    fn every_oauth_canonical_name_is_known_to_sf_core_param_registry() {
        let registry = sf_core::config::param_registry::registry();
        for &key in oauth::ALL_OAUTH_KEYS {
            let canonical = oauth::canonical_name(key).unwrap();
            assert!(
                registry.is_known(canonical),
                "sf_core param_registry does not know {canonical} (from ODBC key {key}); \
                 ODBC and sf_core OAuth canonicals are out of sync"
            );
        }
    }

    /// Wiring guard for `connect_with_params` redaction: building the
    /// redacted map for params that contain every sensitive key the
    /// wrapper recognises (legacy + OAuth) must produce `"****"` for
    /// each sensitive value AND must NOT contain any of the original
    /// values verbatim in its `Debug` rendering. This is the
    /// single-source-of-truth check the connection-string log relies
    /// on.
    #[test]
    fn redacted_param_map_redacts_all_sensitive_keys() {
        let unique_marker = "DO_NOT_LEAK_THIS_TOKEN_42";
        let params = HashMap::from([
            ("UID".to_owned(), "joe".to_owned()),
            ("PWD".to_owned(), unique_marker.to_owned()),
            ("PRIV_KEY_FILE_PWD".to_owned(), unique_marker.to_owned()),
            ("PRIV_KEY_PWD".to_owned(), unique_marker.to_owned()),
            ("PRIV_KEY_BASE64".to_owned(), unique_marker.to_owned()),
            ("PASSCODE".to_owned(), unique_marker.to_owned()),
            ("OAUTH_CLIENT_SECRET".to_owned(), unique_marker.to_owned()),
            ("TOKEN".to_owned(), unique_marker.to_owned()),
        ]);
        let redacted = oauth::redacted_param_map(&params);
        let rendered = format!("{redacted:?}");
        assert!(
            !rendered.contains(unique_marker),
            "redacted map leaked sensitive value: {rendered}"
        );
        // Spot-check a few keys still produce the redaction marker.
        for sensitive in ["PWD", "OAUTH_CLIENT_SECRET", "TOKEN"] {
            assert_eq!(
                redacted.get(&sensitive.to_owned()).map(|v| v.as_ref()),
                Some("****"),
                "{sensitive} should render as ****"
            );
        }
        // Non-sensitive UID is preserved verbatim.
        assert_eq!(
            redacted.get(&"UID".to_owned()).map(|v| v.as_ref()),
            Some("joe")
        );
    }

    /// End-to-end parse → normalize for the canonical OAuth
    /// authorization-code connection string. The
    /// resulting options map must contain every OAuth field as its
    /// `sf_core` lowercase canonical name AND must not have any
    /// SCREAMING_SNAKE residue.
    #[test]
    fn parse_connection_string_oauth_authorization_code_then_normalize() {
        let conn_str = "DRIVER={SnowflakeUD};SERVER=acct.snowflakecomputing.com;UID=joe;\
                        AUTHENTICATOR=OAUTH_AUTHORIZATION_CODE;OAUTH_CLIENT_ID=cid-1;\
                        OAUTH_CLIENT_SECRET=secret-shhh;\
                        OAUTH_AUTHORIZATION_URL=https://idp.example.com/oauth/authorize;\
                        OAUTH_TOKEN_REQUEST_URL=https://idp.example.com/oauth/token;\
                        OAUTH_REDIRECT_URI=http://127.0.0.1:0/cb;\
                        OAUTH_SCOPE=session:role:R;OAUTH_DISABLE_PKCE=false;\
                        OAUTH_ENABLE_DPOP=false;OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS=true";
        let parsed = parse_connection_string(conn_str).expect("parse OK");
        let options = normalize_connection_string_options(parsed);

        assert_eq!(
            config_string(&options, "AUTHENTICATOR"),
            Some("OAUTH_AUTHORIZATION_CODE")
        );
        assert_eq!(config_string(&options, "oauth_client_id"), Some("cid-1"));
        assert_eq!(
            config_string(&options, "oauth_client_secret"),
            Some("secret-shhh")
        );
        assert_eq!(
            config_string(&options, "oauth_authorization_url"),
            Some("https://idp.example.com/oauth/authorize")
        );
        assert_eq!(
            config_string(&options, "oauth_token_request_url"),
            Some("https://idp.example.com/oauth/token")
        );
        assert_eq!(
            config_string(&options, "oauth_redirect_uri"),
            Some("http://127.0.0.1:0/cb")
        );
        assert_eq!(
            config_string(&options, "oauth_scope"),
            Some("session:role:R")
        );
        assert_eq!(
            config_string(&options, "oauth_enable_single_use_refresh_tokens"),
            Some("true")
        );
        for upper in [
            "OAUTH_CLIENT_ID",
            "OAUTH_CLIENT_SECRET",
            "OAUTH_AUTHORIZATION_URL",
            "OAUTH_TOKEN_REQUEST_URL",
            "OAUTH_REDIRECT_URI",
            "OAUTH_SCOPE",
            "OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS",
            "OAUTH_DISABLE_PKCE",
            "OAUTH_ENABLE_DPOP",
        ] {
            assert!(
                !options.contains_key(upper),
                "{upper} leaked through as the SCREAMING_SNAKE form"
            );
        }
    }

    /// Brace-quoted OAuth values (e.g. token URLs containing `;`)
    /// must round-trip safely through `parse_connection_string` and
    /// land as the canonical lowercase key without losing the
    /// embedded delimiter — important because IdP token URLs in the
    /// wild often carry `?api-version=...;client=...` query strings.
    #[test]
    fn parse_connection_string_oauth_brace_quoted_token_url() {
        let conn_str = "DRIVER={SF};AUTHENTICATOR=OAUTH_CLIENT_CREDENTIALS;\
                        OAUTH_CLIENT_ID=cid;OAUTH_CLIENT_SECRET=cs;\
                        OAUTH_TOKEN_REQUEST_URL={https://idp/token?a=1;b=2}";
        let parsed = parse_connection_string(conn_str).expect("parse OK");
        let options = normalize_connection_string_options(parsed);
        assert_eq!(
            config_string(&options, "oauth_token_request_url"),
            Some("https://idp/token?a=1;b=2")
        );
    }

    /// Wiring guard: connection strings that omit OAuth params still
    /// round-trip cleanly — the OAuth canonical-name forwarding must
    /// not interfere with non-OAuth parameter handling.
    #[test]
    fn parse_connection_string_without_oauth_keys_is_unaffected() {
        let conn_str = "DRIVER={SF};SERVER=h;UID=joe;PWD=p;AUTHENTICATOR=SNOWFLAKE_JWT";
        let parsed = parse_connection_string(conn_str).expect("parse OK");
        let options = normalize_connection_string_options(parsed);

        assert_eq!(config_string(&options, "SERVER"), Some("h"));
        assert_eq!(config_string(&options, "UID"), Some("joe"));
        assert_eq!(config_string(&options, "PWD"), Some("p"));
        assert_eq!(
            config_string(&options, "AUTHENTICATOR"),
            Some("SNOWFLAKE_JWT")
        );
    }

    /// Wiring guard: the legacy `AUTHENTICATOR=OAUTH` (pre-acquired
    /// access token) flow forwards the `TOKEN` parameter unchanged
    /// to `sf_core`. The token value is sensitive and MUST be
    /// redacted in `redacted_param_map`, but it must NOT be dropped
    /// from the options map (otherwise the login request would have
    /// no token to send).
    #[test]
    fn legacy_oauth_token_passthrough_redacts_in_logs_but_preserves_value() {
        let raw_params = HashMap::from([
            ("UID".to_owned(), "joe".to_owned()),
            ("AUTHENTICATOR".to_owned(), "OAUTH".to_owned()),
            ("TOKEN".to_owned(), "header.payload.sig".to_owned()),
        ]);
        let redacted = oauth::redacted_param_map(&raw_params);
        assert_eq!(
            redacted.get(&"TOKEN".to_owned()).map(|v| v.as_ref()),
            Some("****")
        );
        let options = normalize_connection_string_options(raw_params);
        assert_eq!(config_string(&options, "TOKEN"), Some("header.payload.sig"));
        assert_eq!(config_string(&options, "AUTHENTICATOR"), Some("OAUTH"));
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
    fn normalize_connection_string_options_forwards_session_keep_alive_params() {
        let options = normalize_connection_string_options(HashMap::from([
            ("CLIENT_SESSION_KEEP_ALIVE".to_owned(), "true".to_owned()),
            (
                "CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY".to_owned(),
                "1800".to_owned(),
            ),
        ]));

        assert_eq!(
            config_string(&options, "CLIENT_SESSION_KEEP_ALIVE"),
            Some("true")
        );
        assert_eq!(
            config_string(&options, "CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY"),
            Some("1800")
        );
    }

    #[test]
    fn normalize_connection_string_options_maps_application_key() {
        // APPLICATION on the connection string is the user-facing app name —
        // it must land in the canonical ``application`` setting
        // (CLIENT_ENVIRONMENT.APPLICATION on the wire), never in client_app_id
        // (CLIENT_APP_ID stays as the wrapper-injected driver name "ODBC").
        // Mirrors the old ODBC driver's behaviour.
        let options = normalize_connection_string_options(HashMap::from([(
            "APPLICATION".to_owned(),
            "Tableau".to_owned(),
        )]));

        assert_eq!(config_string(&options, "application"), Some("Tableau"));
        assert!(!options.contains_key("APPLICATION"));
        assert!(!options.contains_key("client_app_id"));
    }

    #[test]
    fn apply_pre_connection_overrides_routes_application_attr() {
        // SQL_SF_CONN_ATTR_APPLICATION (programmatic) follows the same routing
        // as the connection-string APPLICATION key.
        let mut options = HashMap::new();
        let attrs = HashMap::from([(ConnectionAttribute::Application, "PowerBI".to_owned())]);

        apply_pre_connection_overrides(&attrs, &mut options);

        assert_eq!(config_string(&options, "application"), Some("PowerBI"));
        assert!(!options.contains_key("client_app_id"));
    }

    #[test]
    fn apply_pre_connection_overrides_application_attr_overrides_connection_string() {
        // The override layer wins, matching the established pattern for
        // private-key attributes.
        let mut options = normalize_connection_string_options(HashMap::from([(
            "APPLICATION".to_owned(),
            "FromDsn".to_owned(),
        )]));
        let attrs = HashMap::from([(ConnectionAttribute::Application, "FromAttr".to_owned())]);

        apply_pre_connection_overrides(&attrs, &mut options);

        assert_eq!(config_string(&options, "application"), Some("FromAttr"));
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

    // ---- SQLGetFunctions supported-function bitmap -------------------------

    /// Read a function-support bit out of the ODBC 3.x bitmap the same way the
    /// `SQL_FUNC_EXISTS` driver-manager macro does: `word = id >> 4`,
    /// `bit = id & 0x000F`.
    fn odbc3_bit_set(bitmap: &[sql::USmallInt], function_id: u16) -> bool {
        let word = (function_id >> 4) as usize;
        let bit = function_id & 0x000F;
        bitmap[word] & (1 << bit) != 0
    }

    /// `SQLEndTran` is implemented and exported in `c_api.rs`, so
    /// `SQLGetFunctions` must report it as supported. If it is left in the
    /// `is_supported` exclusion list, strict driver managers (unixODBC) consult
    /// the bitmap and refuse to dispatch `SQLEndTran`, returning `SQL_ERROR`
    /// before the driver's entry point ever runs (iODBC / Windows dispatch
    /// regardless, which is why the regression only showed up under unixODBC).
    #[test]
    fn transact_is_reported_supported() {
        assert!(
            OdbcFunction::Transact.is_supported(),
            "SQLTransact is exported in c_api.rs; SQLGetFunctions must report it supported"
        );
    }

    #[test]
    fn end_tran_is_reported_supported() {
        assert!(
            OdbcFunction::EndTran.is_supported(),
            "SQLEndTran is exported in c_api.rs; SQLGetFunctions must report it supported"
        );
    }

    /// `SQLForeignKeys` is implemented and exported in `c_api.rs`, so
    /// `SQLGetFunctions` must report it as supported. If it is left in the
    /// `is_supported` exclusion list, strict driver managers (unixODBC) consult
    /// the bitmap and refuse to dispatch `SQLForeignKeys`, returning `SQL_ERROR`
    /// before the driver's entry point ever runs (iODBC / Windows dispatch
    /// regardless, which is why the regression only showed up under unixODBC).
    #[test]
    fn foreign_keys_is_reported_supported() {
        assert!(
            OdbcFunction::ForeignKeys.is_supported(),
            "SQLForeignKeys is exported in c_api.rs; SQLGetFunctions must report it supported"
        );
    }

    #[test]
    fn odbc3_bitmap_marks_end_tran_supported() {
        let mut bitmap = [0 as sql::USmallInt; SQL_API_ODBC3_ALL_FUNCTIONS_SIZE];
        fill_odbc3_bitmap(bitmap.as_mut_ptr());
        // SQL_API_SQLENDTRAN == 1005
        assert!(
            odbc3_bit_set(&bitmap, OdbcFunction::EndTran as u16),
            "SQL_API_SQLENDTRAN bit must be set in the ODBC3 all-functions bitmap"
        );
    }

    #[test]
    fn odbc2_array_marks_transact_supported() {
        let mut array = [SQL_FALSE_U16; 100];
        fill_odbc2_array(array.as_mut_ptr());
        assert_eq!(array[OdbcFunction::Transact as usize], SQL_TRUE_U16);
    }

    #[test]
    fn odbc2_array_marks_supported_functions_below_100() {
        let mut array = [SQL_TRUE_U16; 100];
        fill_odbc2_array(array.as_mut_ptr());
        // ExecDirect (11) is supported and < 100, so it must be SQL_TRUE; an
        // unsupported low id like SetPos (68) must be SQL_FALSE.
        assert_eq!(array[OdbcFunction::ExecDirect as usize], SQL_TRUE_U16);
        assert_eq!(array[OdbcFunction::SetPos as usize], SQL_FALSE_U16);
    }

    mod merge_dsn_config_tests {
        use super::*;

        fn ok_lookup(
            map: HashMap<String, String>,
        ) -> impl Fn(&str) -> OdbcResult<HashMap<String, String>> {
            move |_dsn| Ok(map.clone())
        }

        fn err_lookup() -> impl Fn(&str) -> OdbcResult<HashMap<String, String>> {
            |dsn| {
                DataSourceNotFoundSnafu {
                    dsn: dsn.to_string(),
                }
                .fail()
            }
        }

        #[test]
        fn should_be_no_op_when_dsn_is_none() {
            let explicit = HashMap::from([("SERVER".to_owned(), "myhost".to_owned())]);
            let result = merge_dsn_config_impl(explicit.clone(), None, err_lookup()).unwrap();
            assert_eq!(result, explicit);
        }

        #[test]
        fn should_fill_missing_keys_from_stored_dsn() {
            let stored = HashMap::from([
                ("ACCOUNT".to_owned(), "myaccount".to_owned()),
                ("SERVER".to_owned(), "stored-host".to_owned()),
            ]);
            let explicit = HashMap::new();
            let result =
                merge_dsn_config_impl(explicit, Some("TestDSN"), ok_lookup(stored)).unwrap();
            assert_eq!(result.get("ACCOUNT").unwrap(), "myaccount");
            assert_eq!(result.get("SERVER").unwrap(), "stored-host");
        }

        #[test]
        fn should_prefer_explicit_over_stored_dsn_value() {
            let stored = HashMap::from([("SERVER".to_owned(), "stored-host".to_owned())]);
            let explicit = HashMap::from([("SERVER".to_owned(), "explicit-host".to_owned())]);
            let result =
                merge_dsn_config_impl(explicit, Some("TestDSN"), ok_lookup(stored)).unwrap();
            assert_eq!(result.get("SERVER").unwrap(), "explicit-host");
        }

        #[test]
        fn should_strip_driver_description_dsn_metadata_keys() {
            let stored = HashMap::from([
                ("DRIVER".to_owned(), "SnowflakeDSIIDriver".to_owned()),
                ("Description".to_owned(), "My Snowflake DSN".to_owned()),
                ("DSN".to_owned(), "TestDSN".to_owned()),
                ("SERVER".to_owned(), "myhost".to_owned()),
            ]);
            let result =
                merge_dsn_config_impl(HashMap::new(), Some("TestDSN"), ok_lookup(stored)).unwrap();
            assert!(!result.contains_key("DRIVER"));
            assert!(!result.contains_key("Description"));
            assert!(!result.contains_key("DSN"));
            assert_eq!(result.get("SERVER").unwrap(), "myhost");
        }

        #[test]
        fn should_propagate_data_source_not_found_error() {
            let result = merge_dsn_config_impl(HashMap::new(), Some("Missing"), err_lookup());
            assert!(result.is_err());
            let err = result.unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains("Missing"), "error should name the DSN: {msg}");
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
