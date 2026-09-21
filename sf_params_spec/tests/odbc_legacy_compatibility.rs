use std::collections::HashSet;

use sf_params_spec::{Wrapper, registry};

/// Accepted connection-string keys from snowflake-odbc
/// `SFConnection::initAcceptedConnectionKeys` and `SF_SESSION_PARAMETERS`,
/// plus `Snowflake.h` keys `SFConnection` still reads from the connection
/// map (`CLIENT_CONFIG_FILE`, `CRL_DOWNLOAD_TIMEOUT`).
///
/// `(dsn_key, canonical)` — `resolve_for(Odbc, dsn_key)` returns that name.
const SUPPORTED_LEGACY_ODBC_DSN_KEYS: &[(&str, &str)] = &[
    ("APPLICATION", "application"),
    ("SERVER", "host"),
    ("PORT", "port"),
    ("UID", "user"),
    ("PWD", "password"),
    ("TOKEN", "token"),
    ("AUTHENTICATOR", "authenticator"),
    ("PASSCODE", "passcode"),
    ("PASSCODEINPASSWORD", "passcodeInPassword"),
    ("ACCOUNT", "account"),
    ("DATABASE", "database"),
    ("SCHEMA", "schema"),
    ("WAREHOUSE", "warehouse"),
    ("ROLE", "role"),
    ("SSL", "ssl"),
    ("LOGIN_TIMEOUT", "authentication_timeout"),
    ("QUERY_TIMEOUT", "query_timeout"),
    ("PRIV_KEY_FILE", "private_key_file"),
    ("PRIV_KEY_FILE_PWD", "private_key_password"),
    ("PROXY", "proxy"),
    ("NO_PROXY", "no_proxy"),
    (
        "CLIENT_STORE_TEMPORARY_CREDENTIAL",
        "client_store_temporary_credential",
    ),
    ("PUT_FASTFAIL", "put_fastfail"),
    ("GET_FASTFAIL", "get_fastfail"),
    ("PUT_COMPRESSLV", "put_compress_level"),
    ("PUT_TEMPDIR", "put_tempdir"),
    ("PUT_MAXRETRIES", "put_get_max_attempts"),
    ("GET_MAXRETRIES", "put_get_max_attempts"),
    ("SecondaryRoles", "secondary_roles"),
    ("ProxyWithEnv", "use_proxy_env"),
    ("disableQueryContextCache", "disable_query_context_cache"),
    ("includeRetryReason", "include_retry_reason"),
    ("MaxHttpRetries", "retry_max_attempts"),
    ("AllowEmptyProxy", "allow_empty_proxy"),
    ("UseCurrentCatalog", "use_current_catalog"),
    ("enable_connection_diag", "enable_connection_diag"),
    ("connection_diag_log_path", "connection_diag_log_path"),
    (
        "connection_diag_allowlist_path",
        "connection_diag_allowlist_path",
    ),
    ("OAUTH_AUTHORIZATION_URL", "oauth_authorization_url"),
    ("OAUTH_TOKEN_REQUEST_URL", "oauth_token_request_url"),
    ("OAUTH_CLIENT_ID", "oauth_client_id"),
    ("OAUTH_CLIENT_SECRET", "oauth_client_secret"),
    ("OAUTH_REDIRECT_URI", "oauth_redirect_uri"),
    ("OAUTH_SCOPE", "oauth_scope"),
    (
        "OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS",
        "oauth_enable_single_use_refresh_tokens",
    ),
    ("WORKLOAD_IDENTITY_PROVIDER", "workload_identity_provider"),
    (
        "WORKLOAD_IDENTITY_ENTRA_RESOURCE",
        "workload_identity_entra_resource",
    ),
    (
        "WORKLOAD_IDENTITY_IMPERSONATION_PATH",
        "workload_identity_impersonation_path",
    ),
    ("PRIV_KEY_BASE64", "private_key"),
    ("PRIV_KEY_PWD", "private_key_password"),
    ("LOG_QUERY_TEXT", "log_query_text"),
    ("LOG_QUERY_PARAMETERS", "log_query_parameters"),
    ("CLIENT_SESSION_KEEP_ALIVE", "CLIENT_SESSION_KEEP_ALIVE"),
    (
        "CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY",
        "CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY",
    ),
    ("QUERY_TAG", "query_tag"),
    ("CLIENT_PREFETCH_THREADS", "CLIENT_PREFETCH_THREADS"),
    ("DSN", "dsn"),
    ("DRIVER", "driver"),
    ("FILEDSN", "filedsn"),
    ("SAVEFILE", "savefile"),
    ("DESCRIPTION", "description"),
    ("LOCALE", "locale"),
    ("SETUP", "setup"),
    ("DriverODBCVer", "driverodbcver"),
    ("APILevel", "apilevel"),
    ("SQLLevel", "sqllevel"),
    ("ConnectFunctions", "connectfunctions"),
    ("TRACING", "tracing"),
];

/// Accepted by snowflake-odbc but not a registry parameter. Grouped by why
/// the key stays out of the registry.
const UNSUPPORTED_LEGACY_ODBC_DSN_KEYS: &[&str] = &[
    // Wrapper-owned logging and Simba diagnostics. These configure the
    // driver's logger, not `sf_core`. `CPTIMEOUT` is the installer
    // connection-pool idle timeout; the legacy driver accepted the key and
    // never applied it. `CLIENT_CONFIG_FILE` is the `sf_client_config.json`
    // path: `Snowflake.h` defines it and `SFConnection` reads it, but it is
    // not in `initAcceptedConnectionKeys`.
    "LogLevel",
    "LogPath",
    "LogFileSize",
    "LogFileCount",
    "CURLVerboseMode",
    "EnablePidLogFileNames",
    "CLIENT_CONFIG_FILE",
    "CPTIMEOUT",
    // Test-only injectors from `Snowflake.h`.
    "INJECT_CURL_TIMEOUT",
    "INJECT_INCIDENT1",
    "CURL_NO_IDLE_CHECK",
    // Deprecated Simba translation-DLL key.
    "TRANSLATE",
    // OCSP knobs. UD has no OCSP check (BD#124).
    "DisableOCSPCheck",
    "OCSP_FAIL_OPEN",
    // `CRL_ADVISORY` combines with `CRL_CHECK` in the legacy driver to
    // choose ENABLED vs ADVISORY. Two keys cannot 1:1-alias one HashMap
    // entry (last write wins), so this key is not a registry alias.
    "CRL_ADVISORY",
    // Documented GA keys with no matching registry parameter yet.
    // `CRL_DOWNLOAD_TIMEOUT` is defined and applied in snowflake-odbc
    // (`GetOptionalSettingAsUnsigned`) but is not in
    // `initAcceptedConnectionKeys`; `crl_http_timeout` is the registry
    // param and has no ODBC alias for this spelling.
    "CRL_DOWNLOAD_TIMEOUT",
    "ValidateSessionParam",
    "GET_SIZE_THRESHOLD",
    "DEFAULT_VARCHAR_SIZE",
    "DEFAULT_BINARY_SIZE",
    "MapToLongVarchar",
    "ODBC_SCHEMA_CACHING",
    "OUT_OF_RANGE_TIMESTAMP_EXCEPTION",
    "EnableDescribeDirectExec",
    // Boolean 403-retry switch. The new driver retries extra statuses via
    // `retry_extra_status_codes` (BD#146).
    "RetryOn403",
    // Client-only TIMESTAMP COLUMN_SIZE toggle. The new driver always
    // reports 29 (BD#151).
    "ODBC_USE_STANDARD_TIMESTAMP_COLUMNSIZE",
    // Legacy DSN spellings that do not resolve as ODBC aliases.
    "RetryTimeout",
    // Login-only retry cap. `retry_max_attempts` is global (BD#145).
    "MAX_CON_RETRY_ATTEMPTS",
    "BROWSER_RESPONSE_TIMEOUT",
    "disableSamlUrlCheck",
    "singleAuthenticationPrompt",
    "CLIENT_REQUEST_MFA_TOKEN",
    "CRL_CHECK",
    "CRL_ALLOW_NO_CRL",
    "CRL_DISK_CACHING",
    "CRL_MEMORY_CACHING",
    "CABundleFile",
    // Documented `NETWORK_TIMEOUT`. The legacy driver accepted the key;
    // its `getNetworkTimeout()` reader is unused today.
    "NETWORK_TIMEOUT",
    // Hidden JWT, WIF, console-login, and platform-detection knobs.
    "JWT_TIMEOUT",
    "JWT_CNXN_WAIT_TIME",
    "WORKLOAD_IDENTITY_HOST_KEY",
    "disableConsoleLogin",
    "disablePlatformDetection",
    "platformDetectionTimeoutMs",
    // Hidden stage-bind safeguards.
    "DisableStageBindFallback",
    "StageBindMaxFileSize",
    "StageBindThreshold",
    "StageBindCompressLevel",
    // Session parameters. The connection-string parser forwards unknown
    // keys as session parameters, so they do not need registry aliases.
    "TIMEZONE",
    "AUTOCOMMIT",
    "CLIENT_TIMESTAMP_TYPE_MAPPING",
    "CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX",
    "SERVICE_NAME",
    "GCS_USE_DOWNSCOPED_CREDENTIAL",
    "CLIENT_OUT_OF_BAND_TELEMETRY_ENABLED",
];

#[test]
fn test_odbc_legacy_compatibility() {
    let r = registry();
    let mut seen = HashSet::new();
    let mut missing = Vec::new();
    let mut wrong_canonical = Vec::new();
    let mut unexpected = Vec::new();

    for (key, canonical) in SUPPORTED_LEGACY_ODBC_DSN_KEYS {
        assert!(
            seen.insert(key.to_ascii_lowercase()),
            "duplicate legacy DSN key {key:?}"
        );
        match r.resolve_for(Wrapper::Odbc, key) {
            Some(def) if def.canonical_name == *canonical => {}
            Some(def) => wrong_canonical.push((*key, def.canonical_name, *canonical)),
            None => missing.push(*key),
        }
    }

    for key in UNSUPPORTED_LEGACY_ODBC_DSN_KEYS {
        assert!(
            seen.insert(key.to_ascii_lowercase()),
            "duplicate legacy DSN key {key:?}"
        );
        if let Some(def) = r.resolve_for(Wrapper::Odbc, key) {
            unexpected.push((*key, def.canonical_name));
        }
    }

    assert!(
        missing.is_empty() && wrong_canonical.is_empty() && unexpected.is_empty(),
        "legacy ODBC DSN keys not respected by the param registry:\n\
         unresolved: {missing:?}\n\
         wrong canonical (key, actual, expected): {wrong_canonical:?}\n\
         unexpectedly resolved (key, canonical): {unexpected:?}"
    );
}
