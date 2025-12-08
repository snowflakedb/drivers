use crate::api::api_utils::{cstr_to_string, string_to_cstr};
use crate::api::error::{
    ArrowArrayStreamReaderCreationSnafu, FetchDataSnafu, InvalidBoolOptionSnafu,
    InvalidNumericOptionSnafu, InvalidPortSnafu, OdbcError, Required,
};
use crate::api::{ConnectionState, LogSettings, OdbcResult, conn_from_handle};
use arrow::array::{Array, LargeStringArray, StringArray};
use arrow::datatypes::DataType;
use arrow::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use arrow::record_batch::RecordBatch;
use odbc_sys as sql;
use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf_gen::database_driver_v1::{
    ConnectionHandle as TConnectionHandle, ConnectionInitRequest, ConnectionNewRequest,
    ConnectionSetOptionIntRequest, ConnectionSetOptionStringRequest, DatabaseNewRequest,
    StatementExecuteQueryRequest, StatementExecuteQueryResponse, StatementNewRequest,
    StatementReleaseRequest, StatementSetSqlQueryRequest,
};
use snafu::ResultExt;
use snafu::location;
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};
use tracing;

// ODBC constants that may not be in odbc_sys
const SQL_ATTR_AUTOCOMMIT: i32 = 102;
const SQL_ATTR_CURRENT_CATALOG: i32 = 109;
const SQL_AUTOCOMMIT_ON: usize = 1;
const SQL_HANDLE_DBC: u32 = 2;
const SQL_HANDLE_ENV: u32 = 1;
const SQL_COMMIT: u32 = 0;
const SQL_ROLLBACK: u32 = 1;

/// Parse connection string into key-value pairs
fn parse_connection_string(connection_string: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut pending_dsn: Option<String> = None;
    for pair in connection_string.split(';') {
        let trimmed = pair.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = pair.splitn(2, '=').collect();
        if parts.len() == 2 {
            map.insert(parts[0].to_string(), parts[1].to_string());
        } else if pending_dsn.is_none() {
            pending_dsn = Some(trimmed.to_string());
        }
    }
    if let Some(dsn) = pending_dsn {
        map.entry("DSN".to_string()).or_insert(dsn);
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
    let resolved_options = resolve_connection_options(connection_string_map);

    connect_with_options(connection_handle, resolved_options)
}

fn connect_with_options(
    connection_handle: sql::Handle,
    mut options: HashMap<String, String>,
) -> OdbcResult<()> {
    tracing::info!("driver_connect: connection_string={:?}", options);
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/driver_connect_debug.log")
    {
        let _ = writeln!(f, "connect_with_options start options={options:?}");
    }
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/driver_connect_debug.log")
    {
        let _ = writeln!(f, "connect_with_options: starting options = {:?}", options);
    }

    let connection = conn_from_handle(connection_handle);
    let db_handle = DatabaseDriverClient::database_new(DatabaseNewRequest {})?
        .db_handle
        .required("Database handle is required")?;
    let conn_handle = DatabaseDriverClient::connection_new(ConnectionNewRequest {})?
        .conn_handle
        .required("Connection handle is required")?;

    let log_settings =
        LogSettings::from_options(&mut options).or_else(load_log_settings_from_simbaini);
    tracing::info!(
        "connect_with_options: parsed log_settings={log_settings:?} remaining options={options:?}"
    );
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/driver_connect_debug.log")
    {
        let _ = writeln!(
            f,
            "connect_with_options: log_settings={log_settings:?} remaining={options:?}"
        );
    }
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/driver_connect_debug.log")
    {
        let _ = writeln!(
            f,
            "connect_with_options: log_settings={:?}, remaining options={:?}",
            log_settings, options
        );
    }
    connection.log_settings = log_settings.clone();

    if let Some(value) = take_option_case_insensitive(&mut options, "DEFAULT_VARCHAR_SIZE") {
        let parsed = value
            .trim()
            .parse::<i64>()
            .context(InvalidNumericOptionSnafu {
                key: "DEFAULT_VARCHAR_SIZE".to_string(),
                value: value.trim().to_string(),
            })?;
        connection.lob_settings.default_varchar_size = Some(parsed);
    }

    if let Some(value) = take_option_case_insensitive(&mut options, "DEFAULT_BINARY_SIZE") {
        let parsed = value
            .trim()
            .parse::<i64>()
            .context(InvalidNumericOptionSnafu {
                key: "DEFAULT_BINARY_SIZE".to_string(),
                value: value.trim().to_string(),
            })?;
        connection.lob_settings.default_binary_size = Some(parsed);
    }

    let use_current_catalog = take_bool_option(&mut options, "USECURRENTCATALOG").unwrap_or(false);
    connection.use_current_catalog = use_current_catalog;

    let initial_catalog = options.get("DATABASE").cloned();

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/universal_driver_connect_options.log")
    {
        let _ = writeln!(file, "{:?}", options);
    }

    for (key, value) in options {
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/driver_connect_debug.log")
        {
            let _ = writeln!(f, "connect_with_options: applying option {key}={value}");
        }
        apply_connection_option(conn_handle, &key, value)?;
    }

    DatabaseDriverClient::connection_init(ConnectionInitRequest {
        conn_handle: Some(conn_handle),
        db_handle: Some(db_handle),
    })?;
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/driver_connect_debug.log")
    {
        let _ = writeln!(f, "connect_with_options: connection_init complete");
    }
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/driver_connect_debug.log")
    {
        let _ = writeln!(f, "connect_with_options: connection_init completed");
    }

    if let Some(settings) = log_settings {
        tracing::info!(
            "connect_with_options: initializing pid logs at {:?}",
            settings.log_path
        );
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/driver_connect_debug.log")
        {
            let _ = writeln!(
                f,
                "connect_with_options: initializing pid logs at {:?}",
                settings.log_path
            );
        }
        initialize_pid_logs(&settings).map_err(|err| OdbcError::ConnectionInit {
            connection: format!("Failed to initialize log files: {err}"),
            location: location!(),
        })?;
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/driver_connect_debug.log")
        {
            let _ = writeln!(f, "connect_with_options: pid logs initialized");
        }
    }

    connection.state = ConnectionState::Connected {
        db_handle,
        conn_handle,
    };
    update_connection_timezone(connection, &conn_handle);

    if let Some(handle) = current_conn_handle(&connection.state) {
        if let Err(err) = load_lob_settings(connection, &handle) {
            tracing::warn!("connect_with_options: failed to load LOB settings: {err}");
        }

        if let Some(ref value) = initial_catalog {
            connection.current_catalog = Some(value.clone());
        }

        if let Err(err) = refresh_current_catalog(connection, &handle) {
            tracing::warn!("connect_with_options: failed to refresh current catalog: {err}");
        }

        if connection.current_catalog.is_none() {
            if let Some(value) = initial_catalog.clone() {
                connection.current_catalog = Some(value);
            }
        }
    } else if let Some(value) = initial_catalog {
        connection.current_catalog = Some(value);
    }

    eprintln!(
        "connect_with_options: current_catalog={:?}",
        connection.current_catalog
    );

    Ok(())
}

fn update_connection_timezone(
    connection: &mut crate::api::Connection,
    conn_handle: &TConnectionHandle,
) {
    let core_handle = sf_core::apis::database_driver_v1::Handle {
        id: conn_handle.id as u64,
        magic: conn_handle.magic as u64,
    };
    match sf_core::apis::database_driver_v1::connection_get_timezone(core_handle) {
        Ok(tz) => {
            connection.session_timezone = tz.clone();
            if let Some(ref tz_name) = tz {
                tracing::info!("driver_connect: session timezone set to '{tz_name}'");
            } else {
                tracing::info!("driver_connect: session timezone unset (server returned None)");
            }
        }
        Err(err) => {
            tracing::warn!("driver_connect: failed to fetch session timezone: {err}");
        }
    }
}

pub(crate) fn refresh_current_catalog(
    connection: &mut crate::api::Connection,
    conn_handle: &TConnectionHandle,
) -> OdbcResult<()> {
    if let Some(value) = execute_scalar_query(conn_handle, "SELECT CURRENT_DATABASE()", 0)? {
        connection.current_catalog = Some(value);
    }
    Ok(())
}

fn extract_single_string(
    response: StatementExecuteQueryResponse,
    column_index: usize,
) -> OdbcResult<Option<String>> {
    let result = response.result.required("Execute result is required")?;
    let stream_handle: *mut FFI_ArrowArrayStream =
        result.stream.required("Stream is required")?.into();
    let stream = unsafe { FFI_ArrowArrayStream::from_raw(stream_handle) };
    let mut reader =
        ArrowArrayStreamReader::try_new(stream).context(ArrowArrayStreamReaderCreationSnafu {})?;

    if let Some(batch) = reader.next().transpose().context(FetchDataSnafu {})? {
        if batch.num_rows() == 0 || batch.num_columns() <= column_index {
            return Ok(None);
        }
        Ok(read_string_value(&batch, column_index, 0))
    } else {
        Ok(None)
    }
}

fn read_string_value(batch: &RecordBatch, column_index: usize, row: usize) -> Option<String> {
    if batch.num_columns() <= column_index {
        return None;
    }
    let column = batch.column(column_index);
    match column.data_type() {
        DataType::Utf8 => {
            let array = column.as_any().downcast_ref::<StringArray>()?;
            if array.is_null(row) {
                None
            } else {
                Some(array.value(row).to_string())
            }
        }
        DataType::LargeUtf8 => {
            let array = column.as_any().downcast_ref::<LargeStringArray>()?;
            if array.is_null(row) {
                None
            } else {
                Some(array.value(row).to_string())
            }
        }
        _ => Some(format!("{:?}", column)),
    }
}

fn run_simple_statement(conn_handle: &TConnectionHandle, sql: &str) -> OdbcResult<()> {
    let stmt_handle = DatabaseDriverClient::statement_new(StatementNewRequest {
        conn_handle: Some(*conn_handle),
    })?
    .stmt_handle
    .required("Statement handle is required")?;

    let result = (|| -> OdbcResult<()> {
        DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
            stmt_handle: Some(stmt_handle),
            query: sql.to_string(),
        })?;
        DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
            stmt_handle: Some(stmt_handle),
            describe_only: false,
        })?;
        Ok(())
    })();

    let _ = DatabaseDriverClient::statement_release(StatementReleaseRequest {
        stmt_handle: Some(stmt_handle),
    });

    result
}

fn execute_scalar_query(
    conn_handle: &TConnectionHandle,
    query: &str,
    column_index: usize,
) -> OdbcResult<Option<String>> {
    let stmt_handle = DatabaseDriverClient::statement_new(StatementNewRequest {
        conn_handle: Some(*conn_handle),
    })?
    .stmt_handle
    .required("Statement handle is required")?;

    let result = (|| -> OdbcResult<Option<String>> {
        DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
            stmt_handle: Some(stmt_handle),
            query: query.to_string(),
        })?;
        let response =
            DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
                stmt_handle: Some(stmt_handle),
                describe_only: false,
            })?;
        extract_single_string(response, column_index)
    })();

    let _ = DatabaseDriverClient::statement_release(StatementReleaseRequest {
        stmt_handle: Some(stmt_handle),
    });

    result
}

fn fetch_session_parameter_string(
    conn_handle: &TConnectionHandle,
    param_name: &str,
) -> OdbcResult<Option<String>> {
    let query = format!(
        "SHOW PARAMETERS LIKE '{}'",
        escape_single_quotes(param_name)
    );
    // SHOW PARAMETERS result: column 0=name, column 1=value
    execute_scalar_query(conn_handle, &query, 1)
}

fn load_lob_settings(
    connection: &mut crate::api::Connection,
    conn_handle: &TConnectionHandle,
) -> OdbcResult<()> {
    let needs_flag = connection
        .lob_settings
        .enable_large_varchar_binary
        .is_none();
    let needs_max = connection.lob_settings.max_lob_size_in_memory.is_none();
    if !needs_flag && !needs_max {
        return Ok(());
    }

    if needs_flag {
        if let Some(value) = fetch_session_parameter_string(
            conn_handle,
            "ENABLE_LARGE_VARCHAR_AND_BINARY_IN_RESULT",
        )? {
            connection.lob_settings.enable_large_varchar_binary =
                Some(value.eq_ignore_ascii_case("true"));
        }
    }

    if needs_max {
        if let Some(value) = fetch_session_parameter_string(conn_handle, "MAX_LOB_SIZE_IN_MEMORY")?
        {
            if let Ok(parsed) = value.trim().parse::<i64>() {
                connection.lob_settings.max_lob_size_in_memory = Some(parsed);
            }
        }
    }

    Ok(())
}

fn current_conn_handle(state: &ConnectionState) -> Option<TConnectionHandle> {
    match state {
        ConnectionState::Connected { conn_handle, .. } => Some(conn_handle.clone()),
        ConnectionState::Disconnected => None,
    }
}

fn escape_single_quotes(input: &str) -> String {
    input.replace('\'', "''")
}

fn format_catalog_identifier(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        return trimmed.to_string();
    }
    if needs_identifier_quotes(trimmed) {
        let escaped = trimmed.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        trimmed.to_uppercase()
    }
}

fn needs_identifier_quotes(value: &str) -> bool {
    value.chars().any(|ch| {
        ch.is_lowercase() || ch == ' ' || ch == '-' || ch == '$' || ch == '/' || ch == '.'
    })
}

fn apply_connection_option(
    conn_handle: TConnectionHandle,
    key: &str,
    value: String,
) -> OdbcResult<()> {
    let key_upper = key.to_uppercase();
    let key_lower = key.to_lowercase();
    match key_upper.as_str() {
        "DRIVER" => Ok(()),
        "ACCOUNT" => set_option_string(conn_handle, "account", value),
        "SERVER" | "HOST" => set_option_string(conn_handle, "host", value),
        "PWD" | "PASSWORD" => set_option_string(conn_handle, "password", value),
        "UID" | "USER" => set_option_string(conn_handle, "user", value),
        "PORT" => {
            let port_int: i64 = value.parse().context(InvalidPortSnafu {
                port: value.clone(),
            })?;
            DatabaseDriverClient::connection_set_option_int(ConnectionSetOptionIntRequest {
                conn_handle: Some(conn_handle),
                key: "port".to_owned(),
                value: port_int,
            })?;
            Ok(())
        }
        "PROTOCOL" => set_option_string(conn_handle, "protocol", value),
        "DATABASE" => set_option_string(conn_handle, "database", value),
        "WAREHOUSE" => set_option_string(conn_handle, "warehouse", value),
        "ROLE" => set_option_string(conn_handle, "role", value),
        "SCHEMA" => set_option_string(conn_handle, "schema", value),
        "PRIV_KEY_FILE" | "PRIVATE_KEY_FILE" => {
            set_option_string(conn_handle, "private_key_file", value)
        }
        "PRIV_KEY_FILE_PWD" | "PRIVATE_KEY_FILE_PWD" => {
            set_option_string(conn_handle, "private_key_password", value)
        }
        "AUTHENTICATOR" => set_option_string(conn_handle, "authenticator", value),
        "TOKEN" => set_option_string(conn_handle, "token", value),
        "TLS_CUSTOM_ROOT_STORE_PATH" => {
            set_option_string(conn_handle, "custom_root_store_path", value)
        }
        "TLS_VERIFY_HOSTNAME" => set_option_string(conn_handle, "verify_hostname", value),
        "TLS_VERIFY_CERTIFICATES" => set_option_string(conn_handle, "verify_certificates", value),
        "CRL_ENABLED" => set_option_string(conn_handle, "crl_enabled", value),
        "CRL_MODE" => set_option_string(conn_handle, "crl_mode", value.to_uppercase()),
        "CLIENT_PREFETCH_THREADS" => {
            set_option_int_from_str(conn_handle, "client_prefetch_threads", &value)
        }
        "CLIENT_RESULT_PREFETCH_THREADS" => {
            set_option_int_from_str(conn_handle, "client_result_prefetch_threads", &value)
        }
        "CLIENT_RESULT_PREFETCH_SLOTS" => {
            set_option_int_from_str(conn_handle, "client_result_prefetch_slots", &value)
        }
        "CLIENT_RESULT_CHUNK_SIZE" => {
            set_option_int_from_str(conn_handle, "client_result_chunk_size", &value)
        }
        "CLIENT_SESSION_KEEP_ALIVE" => {
            set_option_bool_from_str(conn_handle, "client_session_keep_alive", &value)
        }
        "CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY" => set_option_int_from_str(
            conn_handle,
            "client_session_keep_alive_heartbeat_frequency",
            &value,
        ),
        "CLIENT_MEMORY_LIMIT" => {
            set_option_int_from_str(conn_handle, "client_memory_limit", &value)
        }
        "CLIENT_STAGE_ARRAY_BINDING_THRESHOLD" => {
            set_option_int_from_str(conn_handle, "client_stage_array_binding_threshold", &value)
        }
        "CLIENT_TIMESTAMP_TYPE_MAPPING" => {
            set_option_string(conn_handle, "client_timestamp_type_mapping", value)
        }
        "GO_QUERY_RESULT_FORMAT" => {
            set_option_string(conn_handle, "go_query_result_format", value.to_uppercase())
        }
        _ => {
            tracing::info!("driver_connect: forwarding option {} as {}", key, key_lower);
            set_option_string(conn_handle, &key_lower, value)
        }
    }
}

fn set_option_string(conn_handle: TConnectionHandle, key: &str, value: String) -> OdbcResult<()> {
    DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
        conn_handle: Some(conn_handle),
        key: key.to_owned(),
        value,
    })?;
    Ok(())
}

fn set_option_int_from_str(
    conn_handle: TConnectionHandle,
    key: &str,
    value: &str,
) -> OdbcResult<()> {
    let parsed = value
        .trim()
        .parse::<i64>()
        .context(InvalidNumericOptionSnafu {
            key: key.to_string(),
            value: value.trim().to_string(),
        })?;
    DatabaseDriverClient::connection_set_option_int(ConnectionSetOptionIntRequest {
        conn_handle: Some(conn_handle),
        key: key.to_owned(),
        value: parsed,
    })?;
    Ok(())
}

fn set_option_bool_from_str(
    conn_handle: TConnectionHandle,
    key: &str,
    value: &str,
) -> OdbcResult<()> {
    let normalized = value.trim().to_ascii_lowercase();
    let parsed = match normalized.as_str() {
        "1" | "true" | "on" | "yes" => true,
        "0" | "false" | "off" | "no" => false,
        _ => {
            return InvalidBoolOptionSnafu {
                key: key.to_string(),
                value: value.to_string(),
            }
            .fail();
        }
    };
    DatabaseDriverClient::connection_set_option_int(ConnectionSetOptionIntRequest {
        conn_handle: Some(conn_handle),
        key: key.to_owned(),
        value: if parsed { 1 } else { 0 },
    })?;
    Ok(())
}

fn resolve_connection_options(options: HashMap<String, String>) -> HashMap<String, String> {
    let mut options = options;
    let dsn_key = options
        .keys()
        .find(|key| key.eq_ignore_ascii_case("DSN"))
        .cloned();

    if let Some(key) = dsn_key {
        if let Some(dsn_name) = options.remove(&key) {
            let mut resolved = resolve_dsn_name(&dsn_name).unwrap_or_default();
            if resolved.is_empty() {
                tracing::warn!(
                    "driver_connect: DSN '{}' not found; using connection string only",
                    dsn_name
                );
            }
            for (k, v) in options {
                resolved.insert(k, v);
            }
            return resolved;
        }
    }

    options
}

fn resolve_dsn_name(dsn: &str) -> Option<HashMap<String, String>> {
    if dsn.is_empty() {
        return None;
    }

    if let Some(mut opts) = load_dsn_from_ini(dsn) {
        ensure_driver_option(&mut opts);
        return Some(opts);
    }

    let mut opts = HashMap::new();
    let mut found = false;

    for (key, env_var) in [
        ("ACCOUNT", "SF_TEST_ACCOUNT"),
        ("SERVER", "SF_TEST_SERVER"),
        ("UID", "SF_TEST_USER"),
        ("PWD", "SF_TEST_PASSWORD"),
        ("DATABASE", "SF_TEST_DATABASE"),
        ("SCHEMA", "SF_TEST_SCHEMA"),
        ("WAREHOUSE", "SF_TEST_WAREHOUSE"),
        ("ROLE", "SF_TEST_ROLE"),
    ] {
        if let Ok(value) = std::env::var(env_var) {
            if !value.is_empty() {
                opts.insert(key.to_string(), value);
                found = true;
            }
        }
    }

    if found {
        ensure_driver_option(&mut opts);
        Some(opts)
    } else {
        None
    }
}

fn load_dsn_from_ini(dsn: &str) -> Option<HashMap<String, String>> {
    let ini_path = std::env::var("ODBCINI").ok()?;
    let contents = fs::read_to_string(PathBuf::from(ini_path)).ok()?;

    let mut current_section = None;
    let mut options = HashMap::new();

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = Some(trimmed[1..trimmed.len() - 1].trim().to_string());
            continue;
        }

        if current_section
            .as_deref()
            .map(|section| section.eq_ignore_ascii_case(dsn))
            == Some(true)
        {
            if let Some((key, value)) = trimmed.split_once('=') {
                options.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }

    if options.is_empty() {
        None
    } else {
        Some(options)
    }
}

fn ensure_driver_option(options: &mut HashMap<String, String>) {
    if options.keys().any(|key| key.eq_ignore_ascii_case("DRIVER")) {
        return;
    }

    if let Ok(driver_path) = std::env::var("UNIVERSAL_DRIVER_PATH") {
        if !driver_path.is_empty() {
            options.insert("DRIVER".to_string(), driver_path);
        }
    }
}

impl LogSettings {
    fn from_options(options: &mut HashMap<String, String>) -> Option<Self> {
        let log_path = match take_option_case_insensitive(options, "LOGPATH") {
            Some(value) => value,
            None => {
                tracing::debug!("LogSettings::from_options: LogPath not set");
                return None;
            }
        };
        tracing::debug!("LogSettings::from_options: LogPath set to {}", log_path);
        let _ = take_option_case_insensitive(options, "LOGFILESIZE");
        let log_file_count = take_option_case_insensitive(options, "LOGFILECOUNT")
            .and_then(|value| value.trim().parse::<u64>().ok())
            .unwrap_or(3);
        let enable_pid_log_file_names =
            take_bool_option(options, "ENABLEPIDLOGFILENAMES").unwrap_or(false);
        let curl_verbose_mode = take_bool_option(options, "CURLVERBOSEMODE").unwrap_or(false);
        let _ = take_bool_option(options, "TRACING");
        // Consume LOGLEVEL even though we don't use it so the server doesn't reject it.
        let _ = take_option_case_insensitive(options, "LOGLEVEL");

        Some(LogSettings {
            log_path: PathBuf::from(log_path),
            log_file_count,
            enable_pid_log_file_names,
            curl_verbose_mode,
        })
    }
}

fn load_log_settings_from_simbaini() -> Option<LogSettings> {
    let simba_ini_path = std::env::var("SIMBAINI").ok()?;
    let contents = fs::read_to_string(PathBuf::from(simba_ini_path)).ok()?;

    let mut current_section = None;
    let mut log_path: Option<String> = None;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = Some(trimmed[1..trimmed.len() - 1].trim().to_string());
            continue;
        }

        if current_section
            .as_deref()
            .map(|section| section.eq_ignore_ascii_case("DriverManager"))
            != Some(true)
        {
            continue;
        }

        if let Some((key, value)) = trimmed.split_once('=') {
            if key.trim().eq_ignore_ascii_case("LogPath") {
                log_path = Some(value.trim().to_string());
                break;
            }
        }
    }

    let path = log_path?;
    Some(LogSettings {
        log_path: PathBuf::from(path),
        log_file_count: 3,
        enable_pid_log_file_names: false,
        curl_verbose_mode: false,
    })
}

fn take_option_case_insensitive(
    options: &mut HashMap<String, String>,
    key: &str,
) -> Option<String> {
    let matching_key = options
        .keys()
        .find(|existing| existing.eq_ignore_ascii_case(key))
        .cloned()?;
    options.remove(&matching_key)
}

fn take_bool_option(options: &mut HashMap<String, String>, key: &str) -> Option<bool> {
    take_option_case_insensitive(options, key).and_then(|value| parse_bool(&value))
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn initialize_pid_logs(settings: &LogSettings) -> io::Result<()> {
    fs::create_dir_all(&settings.log_path)?;
    let pid = if settings.enable_pid_log_file_names {
        Some(std::process::id())
    } else {
        None
    };

    create_series(
        build_log_base(&settings.log_path, "snowflake_odbc_connection", pid, "_"),
        ".log",
        settings.log_file_count,
    )?;
    create_series(
        build_log_base(&settings.log_path, "snowflake_odbc_driver", pid, "_"),
        ".log",
        settings.log_file_count,
    )?;
    create_series(
        build_log_base(&settings.log_path, "snowflake_odbc_generic", pid, ""),
        ".log",
        settings.log_file_count,
    )?;

    if settings.curl_verbose_mode {
        create_series(
            build_log_base(&settings.log_path, "snowflake_odbc_curl", pid, "_"),
            ".dmp",
            settings.log_file_count,
        )?;
    }

    Ok(())
}

fn build_log_base(path: &Path, prefix: &str, pid: Option<u32>, separator: &str) -> String {
    let mut file_name = prefix.to_string();
    if let Some(pid_value) = pid {
        file_name.push_str(separator);
        file_name.push_str(&pid_value.to_string());
    } else {
        file_name.push('0');
    }
    path.join(file_name).to_string_lossy().into_owned()
}

fn create_series(base: String, ext: &str, count: u64) -> io::Result<()> {
    let total = count.max(1);
    for index in 0..total {
        let file_path = if index == 0 {
            format!("{base}{ext}")
        } else {
            format!("{base}.{index}{ext}")
        };
        let mut file = File::create(&file_path)?;
        file.write_all(b"")?;
    }
    Ok(())
}

/// Simple connect function (SQLConnect)
pub fn connect(
    connection_handle: sql::Handle,
    server_name: *const sql::Char,
    name_length1: sql::SmallInt,
    user_name: *const sql::Char,
    name_length2: sql::SmallInt,
    authentication: *const sql::Char,
    name_length3: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("connect: connecting via SQLConnect");

    let dsn_or_server = cstr_to_string(server_name, name_length1 as i32)?;
    let user = cstr_to_string(user_name, name_length2 as i32)?;
    let password = cstr_to_string(authentication, name_length3 as i32)?;

    let mut options = if !dsn_or_server.is_empty() {
        resolve_dsn_name(&dsn_or_server).unwrap_or_else(HashMap::new)
    } else {
        HashMap::new()
    };

    if !user.is_empty() {
        options.insert("UID".to_string(), user);
    }
    if !password.is_empty() {
        options.insert("PWD".to_string(), password);
    }

    if options.is_empty() && !dsn_or_server.is_empty() {
        options.insert("SERVER".to_string(), dsn_or_server);
    }

    connect_with_options(connection_handle, options)
}

/// Disconnect from the database
pub fn disconnect(connection_handle: sql::Handle) -> OdbcResult<()> {
    tracing::debug!("disconnect: disconnecting from database");

    let connection = conn_from_handle(connection_handle);

    // Reset connection state to Disconnected
    connection.state = ConnectionState::Disconnected;

    tracing::info!("disconnect: successfully disconnected");
    Ok(())
}

/// Set connection attribute
pub fn set_connect_attr(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    _string_length: sql::Integer,
) -> OdbcResult<()> {
    tracing::debug!("set_connect_attr: attribute={}", attribute);

    let connection = conn_from_handle(connection_handle);

    match connection.state {
        ConnectionState::Connected { conn_handle, .. } => {
            match attribute {
                SQL_ATTR_AUTOCOMMIT => {
                    let autocommit_value = value as usize;
                    let autocommit_enabled = autocommit_value == SQL_AUTOCOMMIT_ON;

                    tracing::info!(
                        "set_connect_attr: setting autocommit to {}",
                        autocommit_enabled
                    );

                    // Execute SET AUTOCOMMIT statement
                    let sql_stmt = if autocommit_enabled {
                        "ALTER SESSION SET AUTOCOMMIT = TRUE"
                    } else {
                        "ALTER SESSION SET AUTOCOMMIT = FALSE"
                    };

                    let stmt_handle = DatabaseDriverClient::statement_new(StatementNewRequest {
                        conn_handle: Some(conn_handle),
                    })?
                    .stmt_handle
                    .required("Statement handle is required")?;

                    DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
                        stmt_handle: Some(stmt_handle),
                        query: sql_stmt.to_string(),
                    })?;

                    DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
                        stmt_handle: Some(stmt_handle),
                        describe_only: false,
                    })?;

                    Ok(())
                }
                SQL_ATTR_CURRENT_CATALOG => {
                    if value.is_null() {
                        tracing::warn!("set_connect_attr: null value for SQL_ATTR_CURRENT_CATALOG");
                        return Ok(());
                    }
                    let catalog = cstr_to_string(value as *const sql::Char, _string_length as i32)?;
                    if catalog.trim().is_empty() {
                        return Ok(());
                    }
                    eprintln!("SQLSetConnectAttr(SQL_ATTR_CURRENT_CATALOG) -> {}", catalog);
                    let query = format!("USE DATABASE {}", format_catalog_identifier(&catalog));
                    run_simple_statement(&conn_handle, &query)?;
                    connection.current_catalog = Some(catalog);
                    refresh_current_catalog(connection, &conn_handle)?;
                    Ok(())
                }
                _ => {
                    tracing::warn!("set_connect_attr: unsupported attribute {}", attribute);
                    Ok(())
                }
            }
        }
        ConnectionState::Disconnected => {
            tracing::warn!("set_connect_attr: connection not established");
            Ok(())
        }
    }
}

/// Get connection attribute
pub fn get_connect_attr(
    connection_handle: sql::Handle,
    attribute: sql::Integer,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> OdbcResult<()> {
    tracing::debug!("get_connect_attr: attribute={}", attribute);

    let connection = conn_from_handle(connection_handle);

    match attribute {
        SQL_ATTR_AUTOCOMMIT => {
            // Return current autocommit setting (default is ON)
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut usize) = SQL_AUTOCOMMIT_ON;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = std::mem::size_of::<usize>() as sql::Integer;
                }
            }
            Ok(())
        }
        SQL_ATTR_CURRENT_CATALOG => {
            if let Some(handle) = current_conn_handle(&connection.state) {
                if connection.current_catalog.is_none() {
                    refresh_current_catalog(connection, &handle)?;
                }
            }

            eprintln!(
                "SQLGetConnectAttr: catalog state {:?}",
                connection.current_catalog
            );

            if let Some(catalog) = connection.current_catalog.as_deref() {
                eprintln!("SQLGetConnectAttr(SQL_ATTR_CURRENT_CATALOG) -> {}", catalog);
                if !value_ptr.is_null() {
                    string_to_cstr(
                        catalog,
                        value_ptr as *mut sql::Char,
                        buffer_length as sql::Len,
                    )?;
                }
                if !string_length_ptr.is_null() {
                    unsafe {
                        *string_length_ptr = catalog.len() as sql::Integer;
                    }
                }
            } else {
                if !value_ptr.is_null() && buffer_length > 0 {
                    unsafe {
                        *(value_ptr as *mut sql::Char) = 0;
                    }
                }
                if !string_length_ptr.is_null() {
                    unsafe {
                        *string_length_ptr = 0;
                    }
                }
            }
            Ok(())
        }
        _ => {
            tracing::warn!("get_connect_attr: unsupported attribute {}", attribute);
            // Return 0 for unknown attributes
            if !value_ptr.is_null() {
                unsafe {
                    *(value_ptr as *mut usize) = 0;
                }
            }
            if !string_length_ptr.is_null() {
                unsafe {
                    *string_length_ptr = std::mem::size_of::<usize>() as sql::Integer;
                }
            }
            Ok(())
        }
    }
}

/// End transaction (commit or rollback)
pub fn end_tran(
    handle_type: sql::SmallInt,
    handle: sql::Handle,
    completion_type: sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!(
        "end_tran: handle_type={}, completion_type={}",
        handle_type,
        completion_type
    );

    // Get the connection handle
    let conn_handle = match handle_type as u32 {
        SQL_HANDLE_DBC => {
            let connection = conn_from_handle(handle);
            match connection.state {
                ConnectionState::Connected { conn_handle, .. } => conn_handle,
                ConnectionState::Disconnected => {
                    tracing::warn!("end_tran: connection not established");
                    return Ok(());
                }
            }
        }
        SQL_HANDLE_ENV => {
            tracing::warn!("end_tran: environment handle not yet supported");
            return Ok(());
        }
        _ => {
            tracing::warn!("end_tran: invalid handle type {}", handle_type);
            return Ok(());
        }
    };

    // Determine the SQL statement to execute
    let sql_stmt = match completion_type as u32 {
        SQL_COMMIT => "COMMIT",
        SQL_ROLLBACK => "ROLLBACK",
        _ => {
            tracing::warn!("end_tran: invalid completion type {}", completion_type);
            return Ok(());
        }
    };

    tracing::info!("end_tran: executing {}", sql_stmt);

    // Create a temporary statement and execute the transaction command
    let stmt_handle = DatabaseDriverClient::statement_new(StatementNewRequest {
        conn_handle: Some(conn_handle),
    })?
    .stmt_handle
    .required("Statement handle is required")?;

    DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
        stmt_handle: Some(stmt_handle),
        query: sql_stmt.to_string(),
    })?;

    DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
        stmt_handle: Some(stmt_handle),
        describe_only: false,
    })?;

    Ok(())
}
