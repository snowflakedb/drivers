use snafu::{OptionExt, ResultExt, Snafu};
use tokio::sync::Mutex;

use super::connection::{Connection, RefreshContext, with_valid_session};
use super::error::*;
use super::global_state::DatabaseDriverV1;
use super::multistatement;
use super::query::perform_put_get_transfer;
use super::result_set::{
    ColumnMetadata, ExecuteQueryResult, fetch_query_response_data, resolve_reader_ctx,
    response_to_descriptor,
};
use super::validation::{
    ValidationIssue, ValidationSeverity, canonicalize_setting_key, resolve_options,
    validate_statement_option_write,
};
use crate::config::ParamStore;
use crate::config::param_registry::ParamKey;
use crate::config::param_registry::param_names;
use crate::config::settings::Setting;
use crate::handle_manager::Handle;
use crate::rest::snowflake::{
    QueryExecutionMode, QueryInput, snowflake_abort_query, snowflake_query_with_client,
};

use crate::config::rest_parameters::QueryParameters;
use crate::config::retry::RetryPolicy;
use crate::rest::snowflake::async_exec::submit_statement_async;
#[cfg(test)]
use crate::rest::snowflake::query_request;
use arrow::ffi_stream::FFI_ArrowArrayStream;
use serde_json::value::RawValue;
use std::sync::atomic::Ordering;
use std::{collections::HashMap, sync::Arc};

/// Pointer to raw bytes in memory - used by query bindings
#[derive(Debug)]
pub struct DataPtr<'a> {
    /// Pointer to the data
    value: *const u8,
    /// Length of data in bytes
    length: i64,
    /// Phantom data to enforce lifetime
    _phantom: std::marker::PhantomData<&'a [u8]>,
}

// Safety: DataPtr semantically represents a &[u8] (immutable borrowed slice),
// which is Send. The raw pointer is only used for FFI interop and is always
// accessed immutably within the lifetime 'a.
//
// Callers must ensure the backing memory is not freed or mutated while
// any DataPtr (or Future holding one) is alive — including across .await
// points. All current production paths run the entire async execution
// synchronously via block_on, keeping the source data on the stack for
// the full duration, which satisfies this requirement.
unsafe impl Send for DataPtr<'_> {}

impl<'a> DataPtr<'a> {
    /// Create a new DataPtr from a raw pointer and length
    pub fn new(value: *const u8, length: i64) -> Self {
        Self {
            value,
            length,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get a slice view of the data
    pub fn slice(&self) -> &'a [u8] {
        // Safety: The caller must ensure the pointer is valid for the lifetime 'a
        unsafe { std::slice::from_raw_parts(self.value, self.length as usize) }
    }
}

#[derive(Debug)]
pub enum BindingType<'a> {
    /// JSON bindings - pointer to UTF-8 encoded JSON bytes.
    /// The bytes must represent valid UTF-8 JSON.
    Json(DataPtr<'a>),
    /// CSV bindings - pointer to raw CSV data bytes for bulk upload.
    Csv(DataPtr<'a>),
}

/// Result returned from async query submission (non-blocking).
pub struct AsyncExecuteResult {
    pub query_id: String,
}

impl DatabaseDriverV1 {
    pub fn statement_new(&self, conn_handle: Handle) -> Result<Handle, ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let stmt = Mutex::new(Statement::new(conn_ptr));
                let handle = self.statements.add_handle(stmt);
                Ok(handle)
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub fn statement_release(&self, stmt_handle: Handle) -> Result<(), ApiError> {
        match self.statements.delete_handle(stmt_handle) {
            true => Ok(()),
            false => InvalidArgumentSnafu {
                argument: "Failed to release statement handle".to_string(),
            }
            .fail(),
        }
    }

    pub async fn statement_set_option(
        &self,
        handle: Handle,
        key: String,
        value: Setting,
    ) -> Result<(), ApiError> {
        match self.statements.get_obj(handle) {
            Some(stmt_ptr) => {
                let mut stmt = stmt_ptr.lock().await;
                let (canonical, def) = canonicalize_setting_key(&key);
                validate_statement_option_write(def)?;
                stmt.settings.insert(canonical, value);
                Ok(())
            }
            None => InvalidArgumentSnafu {
                argument: "Statement handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub async fn statement_set_options(
        &self,
        handle: Handle,
        options: HashMap<String, Setting>,
    ) -> Result<Vec<ValidationIssue>, ApiError> {
        match self.statements.get_obj(handle) {
            Some(stmt_ptr) => {
                let mut stmt = stmt_ptr.lock().await;
                let (resolved, issues) = resolve_options(options);
                let error_messages: Vec<String> = issues
                    .iter()
                    .filter(|i| i.severity == ValidationSeverity::Error)
                    .map(|i| i.to_string())
                    .collect();
                if !error_messages.is_empty() {
                    return InvalidArgumentSnafu {
                        argument: error_messages.join("; "),
                    }
                    .fail();
                }
                for key in resolved.keys() {
                    let def = crate::config::param_registry::registry().resolve(key.as_str());
                    validate_statement_option_write(def)?;
                }
                for (key, value) in resolved {
                    stmt.settings.insert(key, value);
                }
                Ok(issues
                    .into_iter()
                    .filter(|i| i.severity == ValidationSeverity::Warning)
                    .collect())
            }
            None => InvalidArgumentSnafu {
                argument: "Statement handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub async fn statement_set_sql_query(
        &self,
        stmt_handle: Handle,
        query: String,
    ) -> Result<(), ApiError> {
        match self.statements.get_obj(stmt_handle) {
            Some(stmt_ptr) => {
                let mut stmt = stmt_ptr.lock().await;
                stmt.query = Some(query);
                Ok(())
            }
            None => InvalidArgumentSnafu {
                argument: "Statement handle not found".to_string(),
            }
            .fail(),
        }
    }
}

pub struct PrepareResult {
    pub stream: Box<FFI_ArrowArrayStream>,
    pub query_id: String,
    pub columns: Vec<ColumnMetadata>,
    pub number_of_binds: i32,
    pub query: String,
    pub sql_state: Option<String>,
}

impl DatabaseDriverV1 {
    pub async fn statement_prepare(&self, stmt_handle: Handle) -> Result<PrepareResult, ApiError> {
        let result = self
            .execute_query_internal(stmt_handle, None, Some(true), None)
            .await?;

        // Multi-statement query prepare is not supported.
        let ExecuteQueryResult::Single(rs_info) = result else {
            return Err(InvalidArgumentSnafu {
                argument: "Multi-statement queries cannot be prepared".to_string(),
            }
            .build());
        };
        let stream = self.result_set_get_stream(rs_info.handle).await?;
        self.result_set_release(rs_info.handle)?;

        let stmt_ptr = self.statements.get_obj(stmt_handle).ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "Statement handle not found".to_string(),
            }
            .build()
        })?;
        // TODO: re-lock the statement to just copy the query
        //       consider to carry query text in ExecuteQueryResult to avoid the re-lock
        let stmt = stmt_ptr.lock().await;
        let query = stmt.query.clone().unwrap_or_default();

        Ok(PrepareResult {
            stream,
            query_id: rs_info.descriptor.query_id,
            columns: rs_info.descriptor.columns,
            number_of_binds: rs_info.descriptor.number_of_binds,
            query,
            sql_state: rs_info.descriptor.sql_state,
        })
    }
}

impl DatabaseDriverV1 {
    pub async fn statement_execute_query<'a>(
        &self,
        stmt_handle: Handle,
        bindings: Option<BindingType<'a>>,
        timeout_seconds: Option<u32>,
    ) -> Result<ExecuteQueryResult, ApiError> {
        self.execute_query_internal(stmt_handle, bindings, None, timeout_seconds)
            .await
    }

    async fn execute_query_internal<'a>(
        &self,
        stmt_handle: Handle,
        bindings: Option<BindingType<'a>>,
        describe_only: Option<bool>,
        timeout_seconds: Option<u32>,
    ) -> Result<ExecuteQueryResult, ApiError> {
        let stmt_ptr = self.statements.get_obj(stmt_handle).ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "Statement handle not found".to_string(),
            }
            .build()
        })?;

        let stmt = stmt_ptr.lock().await;

        let query = extract_query(&stmt)?;
        let (query_parameters, http_client, retry_policy) = query_context(&stmt.conn).await?;

        let execution_mode = stmt.execution_mode(Some(&query));

        let query_bindings = resolve_query_bindings(&bindings)?;

        let query_input = QueryInput {
            sql: query.clone(),
            bindings: query_bindings,
            describe_only,
            query_parameters: build_query_parameters_with_timeout(&stmt.settings, timeout_seconds),
        };

        let conn_arc = stmt.conn.clone();
        drop(stmt);

        let response = {
            let mut ctx = RefreshContext::from_arc(&conn_arc).await?;
            let mut last_error = None;
            loop {
                let session_token = ctx.refresh_token(last_error).await?;
                match snowflake_query_with_client(
                    &http_client,
                    query_parameters.clone(),
                    session_token.reveal(),
                    query_input.clone(),
                    &retry_policy,
                    execution_mode,
                )
                .await
                {
                    Ok(result) => break Ok(result),
                    Err(e) => last_error = Some(e),
                }
            }
        }?;

        if response.success {
            let conn = conn_arc.lock().await;
            conn.update_session_params_cache(
                &query,
                response.data.parameters.as_ref(),
                &super::connection::FinalSessionNames {
                    database: response.data.final_database_name.clone(),
                    schema: response.data.final_schema_name.clone(),
                    warehouse: response.data.final_warehouse_name.clone(),
                    role: response.data.final_role_name.clone(),
                },
            )
            .await;
        }

        // Re-acquire lock to set the state
        let mut stmt = stmt_ptr.lock().await;
        stmt.state = StatementState::Executed;
        drop(stmt);

        let data = response.data;
        let descriptor = response_to_descriptor(&data, &self.wrapper_presets);

        if let Some(multi) = multistatement::try_into_multi_result(&data, descriptor.clone()) {
            return Ok(multi);
        }

        let rowset_data = match data.command.as_deref() {
            Some(command) => {
                // Build refresh context for PUT/GET so the file manager can
                // recover from STS `ExpiredToken` by re-issuing the original
                // PUT/GET SQL to obtain fresh stage credentials. The refresher
                // calls back into `RefreshContext::execute_with_refresh`, so a
                // session-token expiry mid-batch is renewed transparently.
                let stage_creds_refresh_context = super::query::StageCredsRefreshContext {
                    sql: query.clone(),
                    query_parameters: query_parameters.clone(),
                    conn: conn_arc.clone(),
                };
                let use_s3_regional_url_session_param = conn_arc
                    .lock()
                    .await
                    .use_s3_regional_url_session_param()
                    .await;
                perform_put_get_transfer(
                    command,
                    &data,
                    &self.wrapper_presets,
                    Some(stage_creds_refresh_context),
                    use_s3_regional_url_session_param,
                )
                .await
                .context(QueryResponseProcessingSnafu)?
            }
            None => data.into_rowset_data(),
        };
        let reader_ctx = resolve_reader_ctx(&conn_arc).await?;
        Ok(self.build_execute_result(rowset_data, descriptor, reader_ctx))
    }

    /// Execute query asynchronously (non-blocking) — returns immediately with query_id.
    pub async fn statement_execute_async<'a>(
        &self,
        stmt_handle: Handle,
        bindings: Option<BindingType<'a>>,
    ) -> Result<AsyncExecuteResult, ApiError> {
        let stmt_ptr = self.statements.get_obj(stmt_handle).ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "Statement handle not found".to_string(),
            }
            .build()
        })?;

        let mut stmt = stmt_ptr.lock().await;

        let query = extract_query(&stmt)?;
        let (query_parameters, http_client, retry_policy) = query_context(&stmt.conn).await?;
        let query_bindings = resolve_query_bindings(&bindings)?;
        let query_input = QueryInput {
            sql: query.clone(),
            bindings: query_bindings,
            describe_only: None,
            query_parameters: build_query_parameters(&stmt.settings),
        };
        let request_id = uuid::Uuid::new_v4();

        let result = {
            let mut ctx = RefreshContext::from_arc(&stmt.conn).await?;
            let mut last_error = None;
            loop {
                let session_token = ctx.refresh_token(last_error).await?;
                match submit_statement_async(
                    &http_client,
                    &query_parameters,
                    session_token.reveal(),
                    &query_input,
                    request_id,
                    &retry_policy,
                )
                .await
                {
                    Ok(submit_result) => break Ok(submit_result),
                    Err(e) => {
                        last_error = Some(RestError::AsyncQuery {
                            source: e,
                            request_id: Some(request_id),
                            query_id: None,
                            location: snafu::Location::new(file!(), line!(), 0),
                        });
                    }
                }
            }
        }?;

        let query_id = result.query_id.ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "No query_id returned from async submission".to_string(),
            }
            .build()
        })?;

        stmt.state = StatementState::Executed;

        Ok(AsyncExecuteResult { query_id })
    }

    pub async fn connection_get_query_result(
        &self,
        conn_handle: Handle,
        query_id: String,
    ) -> Result<ExecuteQueryResult, ApiError> {
        let conn_ptr = self.connections.get_obj(conn_handle).ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .build()
        })?;

        let data = fetch_query_response_data(&conn_ptr, &query_id).await?;
        let descriptor = response_to_descriptor(&data, &self.wrapper_presets);

        if let Some(multi) = multistatement::try_into_multi_result(&data, descriptor.clone()) {
            return Ok(multi);
        }

        let rowset_data = match data.command.as_deref() {
            Some(command) => {
                let use_s3_regional_url_session_param = conn_ptr
                    .lock()
                    .await
                    .use_s3_regional_url_session_param()
                    .await;
                perform_put_get_transfer(
                    command,
                    &data,
                    &self.wrapper_presets,
                    None,
                    use_s3_regional_url_session_param,
                )
                .await
                .context(QueryResponseProcessingSnafu)?
            }
            None => data.into_rowset_data(),
        };
        let reader_ctx = resolve_reader_ctx(&conn_ptr).await?;
        Ok(self.build_execute_result(rowset_data, descriptor, reader_ctx))
    }

    pub async fn connection_abort_query(
        &self,
        conn_handle: Handle,
        query_id: String,
    ) -> Result<(), ApiError> {
        let conn_ptr = self.connections.get_obj(conn_handle).ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .build()
        })?;

        let (query_parameters, http_client, _) = query_context(&conn_ptr).await?;

        with_valid_session(&conn_ptr, |token| {
            let http_client = &http_client;
            let query_parameters = &query_parameters;
            let query_id = &query_id;
            async move {
                snowflake_abort_query(http_client, query_parameters, token.reveal(), query_id).await
            }
        })
        .await
    }
}

/// Lock the connection and extract the transport parameters, HTTP client, and retry policy
/// needed to issue a query.
///
/// Also rejects query execution if close() has already been called on the connection.
async fn query_context(
    conn: &Arc<Mutex<Connection>>,
) -> Result<(QueryParameters, reqwest::Client, RetryPolicy), ApiError> {
    let conn = conn.lock().await;
    // Reject query execution if close() has been called
    if conn.is_closed.load(Ordering::SeqCst) {
        return Err(ConnectionClosedSnafu {}.build());
    }
    Ok((
        conn.query_transport_parameters()?,
        conn.http_client
            .clone()
            .context(ConnectionNotInitializedSnafu)?,
        conn.retry_policy.clone(),
    ))
}

/// Return the SQL text attached to a statement, or an error if none has been set.
fn extract_query(stmt: &Statement) -> Result<String, ApiError> {
    stmt.query.clone().ok_or_else(|| {
        InvalidArgumentSnafu {
            argument: "Query not found".to_string(),
        }
        .build()
    })
}

pub struct Statement {
    pub state: StatementState,
    pub(crate) settings: ParamStore,
    pub query: Option<String>,
    pub conn: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone)]
pub enum StatementState {
    Initialized,
    Executed,
}

impl Statement {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Statement {
            settings: ParamStore::new(),
            state: StatementState::Initialized,
            query: None,
            conn,
        }
    }

    pub(crate) fn execution_mode(&self, query: Option<&str>) -> QueryExecutionMode {
        let async_requested = self
            .settings
            .get(param_names::ASYNC_EXECUTION)
            .and_then(parse_bool_setting)
            .unwrap_or(false);

        if async_requested && !query.is_some_and(is_file_transfer) {
            return QueryExecutionMode::Async;
        }
        QueryExecutionMode::Blocking
    }
}

fn setting_to_json_value(setting: &Setting) -> serde_json::Value {
    match setting {
        Setting::String(s) => serde_json::Value::String(s.clone()),
        Setting::Int(i) => serde_json::json!(i),
        Setting::Double(d) => serde_json::json!(d),
        Setting::Bool(b) => serde_json::Value::Bool(*b),
        Setting::Bytes(b) => serde_json::Value::String(String::from_utf8_lossy(b).into_owned()),
    }
}

/// Server-side parameter names forwarded in the query request.
/// Each entry maps a local `ParamKey` to the Snowflake server-side parameter name.
const QUERY_PARAMETER_NAMES: &[(ParamKey, &str)] =
    &[(param_names::MULTI_STATEMENT_COUNT, "MULTI_STATEMENT_COUNT")];

fn build_query_parameters(settings: &ParamStore) -> Option<HashMap<String, serde_json::Value>> {
    let mut params = HashMap::new();
    for (key, server_name) in QUERY_PARAMETER_NAMES {
        if let Some(setting) = settings.get(*key) {
            params.insert(server_name.to_string(), setting_to_json_value(setting));
        }
    }
    if params.is_empty() {
        None
    } else {
        Some(params)
    }
}

fn build_query_parameters_with_timeout(
    settings: &ParamStore,
    timeout_seconds: Option<u32>,
) -> Option<HashMap<String, serde_json::Value>> {
    let mut params = build_query_parameters(settings).unwrap_or_default();
    if let Some(t) = timeout_seconds.filter(|&t| t > 0) {
        params.insert(
            "STATEMENT_TIMEOUT_IN_SECONDS".to_string(),
            serde_json::Value::Number(t.into()),
        );
    }
    if params.is_empty() {
        None
    } else {
        Some(params)
    }
}

fn parse_bool_setting(setting: &Setting) -> Option<bool> {
    match setting {
        Setting::Bool(v) => Some(*v),
        Setting::String(s) => {
            let s = s.trim();
            if s.eq_ignore_ascii_case("true") || s.eq_ignore_ascii_case("yes") || s == "1" {
                Some(true)
            } else if s.eq_ignore_ascii_case("false") || s.eq_ignore_ascii_case("no") || s == "0" {
                Some(false)
            } else {
                None
            }
        }
        Setting::Int(v) => Some(*v != 0),
        _ => None,
    }
}

/// Best-effort detection of file transfer commands (PUT/GET) from SQL text.
///
/// Snowflake's async API does not support file transfers. Submitting PUT/GET with
/// asyncExec=true returns a poll URL, but polling returns error 612 "Result not found"
/// because file transfer metadata is only available synchronously.
///
/// We parse SQL to detect PUT/GET and force sync mode. If detection fails, error 612
/// triggers a retry with sync mode (see snowflake_query_with_client).
fn is_file_transfer(sql: &str) -> bool {
    let s = skip_leading_whitespace_and_comments(sql);
    if s.len() < 4 {
        return false;
    }
    let prefix = &s[..3];
    let next_char = s.as_bytes()[3];
    let is_put_or_get = prefix.eq_ignore_ascii_case("PUT") || prefix.eq_ignore_ascii_case("GET");
    // Must be followed by whitespace or comment start (-- or /*)
    let valid_separator = next_char.is_ascii_whitespace() || next_char == b'/' || next_char == b'-';
    is_put_or_get && valid_separator
}

/// Strips leading whitespace, line comments (--), and block comments (/* */)
fn skip_leading_whitespace_and_comments(s: &str) -> &str {
    let mut s = s;
    loop {
        s = s.trim_start();

        // Skip line comments: -- ... \n
        if s.starts_with("--") {
            match s.find('\n') {
                Some(pos) => s = &s[pos + 1..],
                None => return "", // Comment extends to end
            }
            continue;
        }

        // Skip block comments: /* ... */
        if s.starts_with("/*") {
            match s.find("*/") {
                Some(pos) => s = &s[pos + 2..],
                None => return "", // Unterminated comment
            }
            continue;
        }

        break;
    }
    s
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
pub enum StatementError {
    #[snafu(display("Unsupported bind parameter type: {type_}"))]
    UnsupportedBindParameterType {
        type_: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Invalid state transition: {msg}"))]
    InvalidStateTransition {
        msg: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

/// Extract JSON bindings from a `DataPtr` -- **zero-copy with validation**.
///
/// Returns a reference directly into the language wrapper's memory with **no allocation**.
/// The lifetime is tied to the DataPtr, ensuring the slice doesn't outlive the pointed data.
///
/// ## Memory / allocation details
///
/// Total allocations: **ZERO**
/// 1. **DataPtr.slice()**: creates a `&[u8]` slice over wrapper memory (no allocation)
/// 2. **UTF-8 validation**: validates bytes are valid UTF-8 (no allocation)
/// 3. **JSON syntax validation**: RawValue::from_string checks JSON syntax (no allocation)
///
/// ## Validation
///
/// This function performs validation to catch errors early:
/// - **UTF-8 validation**: Ensures the bytes are valid UTF-8
/// - **JSON syntax validation**: RawValue validates basic JSON structure
///
/// The Snowflake server still validates the full JSON structure, types, and formats.
///
/// ## Safety contract
///
/// The caller (language wrapper) MUST guarantee:
/// 1. The pointer points to memory that remains valid for the entire `statement_execute_query` call
/// 2. `statement_execute_query` is called synchronously (blocks until HTTP completes)
pub(crate) fn parse_json_bindings<'a>(
    data_ptr: &'a DataPtr<'a>,
) -> Result<&'a RawValue, StatementError> {
    // Get the byte slice from the pointer - zero allocation.
    // The slice lifetime is tied to DataPtr, ensuring safety.
    let json_bytes: &'a [u8] = data_ptr.slice();

    // Validate UTF-8 encoding - zero allocation.
    let json_str: &'a str = std::str::from_utf8(json_bytes).map_err(|_| {
        UnsupportedBindParameterTypeSnafu {
            type_: "Bindings data is not valid UTF-8".to_string(),
        }
        .build()
    })?;

    // Validate JSON syntax - zero allocation.
    // RawValue::from_string checks that the string is valid JSON without parsing it fully.
    let raw: &'a RawValue = serde_json::from_str(json_str).map_err(|e| {
        UnsupportedBindParameterTypeSnafu {
            type_: format!("Bindings data is not valid JSON: {}", e),
        }
        .build()
    })?;

    Ok(raw)
}

/// Resolve `BindingType` into a borrowed `&RawValue` suitable for query submission.
///
/// Zero-copy for JSON bindings; returns `Err` for unsupported CSV bindings.
fn resolve_query_bindings<'a>(
    bindings: &'a Option<BindingType<'a>>,
) -> Result<Option<&'a RawValue>, ApiError> {
    match bindings {
        Some(BindingType::Json(data_ptr)) => {
            Ok(Some(parse_json_bindings(data_ptr).context(StatementSnafu)?))
        }
        Some(BindingType::Csv(_)) => Err(InvalidArgumentSnafu {
            argument: "CSV bindings are not yet implemented".to_string(),
        }
        .build()),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::super::result_set;
    use super::*;
    use crate::rest::snowflake::query_response::Data;

    #[test]
    fn parse_bool_setting_accepts_native_bool_values() {
        assert_eq!(parse_bool_setting(&Setting::Bool(true)), Some(true));
        assert_eq!(parse_bool_setting(&Setting::Bool(false)), Some(false));
    }

    #[test]
    fn execution_mode_uses_native_bool_async_setting() {
        let conn = Arc::new(Mutex::new(Connection::new()));
        let mut stmt = Statement::new(conn);
        stmt.settings
            .insert("async_execution".to_string(), Setting::Bool(true));

        assert_eq!(
            stmt.execution_mode(Some("SELECT 1")),
            QueryExecutionMode::Async
        );
    }

    #[tokio::test]
    async fn statement_rejects_connection_scoped_param() {
        let ds = DatabaseDriverV1::new();
        let ch = ds.connection_new();
        let sh = ds.statement_new(ch).unwrap();
        let err = ds
            .statement_set_option(sh, "host".into(), Setting::String("h".into()))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("not statement-scoped"),
            "unexpected: {err}"
        );
        ds.statement_release(sh).unwrap();
        ds.connection_release(ch).unwrap();
    }

    #[test]
    fn is_file_transfer_detects_put_statements() {
        assert!(is_file_transfer("PUT file://local @stage"));
        assert!(is_file_transfer("put file://local @stage"));
        assert!(is_file_transfer("Put file://local @stage"));
    }

    #[test]
    fn is_file_transfer_detects_get_statements() {
        assert!(is_file_transfer("GET @stage file://local"));
        assert!(is_file_transfer("get @stage file://local"));
        assert!(is_file_transfer("Get @stage file://local"));
    }

    #[test]
    fn is_file_transfer_handles_whitespace_after_command() {
        // Space
        assert!(is_file_transfer("PUT file://local"));
        // Tab
        assert!(is_file_transfer("PUT\tfile://local"));
        // Newline
        assert!(is_file_transfer("PUT\nfile://local"));
        assert!(is_file_transfer("GET\n@stage"));
    }

    #[test]
    fn is_file_transfer_handles_comment_after_command() {
        // Block comment immediately after PUT/GET
        assert!(is_file_transfer("PUT/* comment */file://local"));
        assert!(is_file_transfer("GET/**/file://local"));
        // Line comment immediately after PUT/GET
        assert!(is_file_transfer("PUT-- comment\nfile://local"));
        assert!(is_file_transfer("GET--\n@stage"));
    }

    #[test]
    fn is_file_transfer_handles_leading_whitespace() {
        assert!(is_file_transfer("  PUT file://local @stage"));
        assert!(is_file_transfer("\t\nGET @stage file://local"));
    }

    #[test]
    fn is_file_transfer_handles_line_comments() {
        assert!(is_file_transfer("-- comment\nPUT file://local @stage"));
        assert!(is_file_transfer(
            "-- line1\n-- line2\nGET @stage file://local"
        ));
        assert!(is_file_transfer("  -- indented comment\nPUT file://local"));
    }

    #[test]
    fn is_file_transfer_handles_block_comments() {
        assert!(is_file_transfer("/* comment */PUT file://local @stage"));
        assert!(is_file_transfer("/* comment */ PUT file://local @stage"));
        assert!(is_file_transfer(
            "/* c1 */ /* c2 */ GET @stage file://local"
        ));
        assert!(is_file_transfer("/*\nmultiline\n*/PUT file://local"));
    }

    #[test]
    fn is_file_transfer_handles_mixed_comments() {
        assert!(is_file_transfer("-- line\n/* block */PUT file://local"));
        assert!(is_file_transfer("/* block */-- line\nGET @stage"));
        assert!(is_file_transfer(
            "  /* block */ -- line\n  PUT file://local"
        ));
    }

    #[test]
    fn is_file_transfer_rejects_comment_only() {
        assert!(!is_file_transfer("-- just a comment"));
        assert!(!is_file_transfer("/* unterminated comment"));
        assert!(!is_file_transfer("-- comment\n-- another"));
    }

    #[test]
    fn is_file_transfer_rejects_bare_commands() {
        // PUT or GET alone is not a valid command
        assert!(!is_file_transfer("PUT"));
        assert!(!is_file_transfer("GET"));
        assert!(!is_file_transfer("put"));
        assert!(!is_file_transfer("get"));
    }

    #[test]
    fn is_file_transfer_rejects_non_blocking_statements() {
        assert!(!is_file_transfer("SELECT * FROM table"));
        assert!(!is_file_transfer("INSERT INTO table VALUES (1)"));
        assert!(!is_file_transfer("UPDATE table SET x = 1"));
        assert!(!is_file_transfer("DELETE FROM table"));
        assert!(!is_file_transfer("CREATE TABLE t (id INT)"));
    }

    #[test]
    fn is_file_transfer_rejects_similar_prefixes() {
        // Should not match words that start with PUT/GET but aren't commands
        assert!(!is_file_transfer("PUTTING"));
        assert!(!is_file_transfer("GETTING"));
        assert!(!is_file_transfer("PUTTER"));
        assert!(!is_file_transfer("GETAWAY"));
    }

    #[test]
    fn is_file_transfer_handles_edge_cases() {
        assert!(!is_file_transfer(""));
        assert!(!is_file_transfer("   "));
        assert!(!is_file_transfer("PU"));
        assert!(!is_file_transfer("GE"));
        assert!(!is_file_transfer("P"));
    }

    #[test]
    fn test_parse_json_bindings() {
        // Test simple bindings
        let json =
            r#"{"1": {"type": "FIXED", "value": "123"}, "2": {"type": "TEXT", "value": "hello"}}"#;

        // Create a pointer to the JSON bytes (simulating Python's no-copy scheme)
        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let raw = parse_json_bindings(&data_ptr).unwrap();
        let params: serde_json::Value = serde_json::from_str(raw.get()).unwrap();

        // Verify it's a JSON object with 2 keys
        assert!(params.is_object());
        let obj = params.as_object().unwrap();
        assert_eq!(obj.len(), 2);

        // Verify parameter 1
        let param1 = obj.get("1").unwrap();
        assert_eq!(param1["type"], "FIXED");
        assert_eq!(param1["value"], "123");

        // Verify parameter 2
        let param2 = obj.get("2").unwrap();
        assert_eq!(param2["type"], "TEXT");
        assert_eq!(param2["value"], "hello");
    }

    #[test]
    fn test_parse_json_bindings_with_array() {
        // Test array bindings (multi-row)
        let json = r#"{"1": {"type": "FIXED", "value": ["1", "2", "3"]}, "2": {"type": "TEXT", "value": ["a", "b", "c"]}}"#;

        // Create a pointer to the JSON bytes (simulating Python's no-copy scheme)
        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let raw = parse_json_bindings(&data_ptr).unwrap();
        let params: serde_json::Value = serde_json::from_str(raw.get()).unwrap();

        // Verify it's a JSON object with 2 keys
        assert!(params.is_object());
        let obj = params.as_object().unwrap();
        assert_eq!(obj.len(), 2);

        // Verify parameter 1
        let param1 = obj.get("1").unwrap();
        assert_eq!(param1["type"], "FIXED");
        assert!(param1["value"].is_array());

        // Verify parameter 2
        let param2 = obj.get("2").unwrap();
        assert_eq!(param2["type"], "TEXT");
        assert!(param2["value"].is_array());
    }

    // ---------------------------------------------------------------
    // parse_json_bindings: error cases
    // ---------------------------------------------------------------

    // Note: These tests are removed as pointer validation is now handled at construction time
    // by the caller (language wrapper), not in parse_json_bindings

    #[test]
    fn test_parse_json_bindings_rejects_invalid_utf8() {
        // Create a byte buffer with invalid UTF-8 (0xFF is never valid in UTF-8).
        // With validation, this should be rejected early.
        let bad_bytes: Vec<u8> = vec![0xFF, 0xFE, 0x7B, 0x7D]; // invalid followed by "{}"
        let ptr = bad_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, bad_bytes.len() as i64);

        let result = parse_json_bindings(&data_ptr);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("not valid UTF-8"),
            "Expected UTF-8 validation error, got: {err_msg}"
        );
    }

    #[test]
    fn test_parse_json_bindings_rejects_invalid_json() {
        // Valid UTF-8 but not valid JSON.
        // With validation, this should be rejected early.
        let bad_json = "{ this is not json }";
        let json_bytes = bad_json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, bad_json.len() as i64);

        let result = parse_json_bindings(&data_ptr);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("not valid JSON"),
            "Expected JSON validation error, got: {err_msg}"
        );
    }

    #[test]
    fn test_parse_json_bindings_rejects_truncated_json() {
        // JSON that starts valid but is cut short.
        // With validation, this should be rejected early.
        let truncated = r#"{"1": {"type": "FIXED""#;
        let json_bytes = truncated.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, truncated.len() as i64);

        let result = parse_json_bindings(&data_ptr);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("not valid JSON"),
            "Expected JSON validation error, got: {err_msg}"
        );
    }

    // ---------------------------------------------------------------
    // parse_json_bindings: zero-copy verification
    // ---------------------------------------------------------------

    #[test]
    fn test_parse_json_bindings_zero_copy() {
        let json = r#"{"1": {"type": "TEXT", "value": "abc"}}"#;
        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let result = parse_json_bindings(&data_ptr).unwrap();

        // Verify the returned reference points into the original buffer (zero-copy)
        let raw_ptr = result.get().as_ptr() as usize;
        let original_start = json_bytes.as_ptr() as usize;
        let original_end = original_start + json_bytes.len();
        assert!(
            raw_ptr >= original_start && raw_ptr < original_end,
            "Zero-copy: RawValue should point into original buffer"
        );

        // Verify the content is correct
        let params: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(params["1"]["type"], "TEXT");
        assert_eq!(params["1"]["value"], "abc");
    }

    // ---------------------------------------------------------------
    // parse_json_bindings: additional happy-path cases
    // ---------------------------------------------------------------

    #[test]
    fn test_parse_json_bindings_single_parameter() {
        let json = r#"{"1": {"type": "FIXED", "value": "42"}}"#;
        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let result = parse_json_bindings(&data_ptr).unwrap();
        let params: serde_json::Value = serde_json::from_str(result.get()).unwrap();

        let obj = params.as_object().unwrap();
        assert_eq!(obj.len(), 1);
        assert_eq!(obj["1"]["type"], "FIXED");
        assert_eq!(obj["1"]["value"], "42");
    }

    #[test]
    fn test_parse_json_bindings_with_null_values() {
        let json = r#"{"1": {"type": "TEXT", "value": null}}"#;
        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let result = parse_json_bindings(&data_ptr).unwrap();
        let params: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert!(params["1"]["value"].is_null());
    }

    #[test]
    fn test_parse_json_bindings_with_unicode_values() {
        let json = r#"{"1": {"type": "TEXT", "value": "日本語テスト 🎉"}}"#;
        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let result = parse_json_bindings(&data_ptr).unwrap();
        let params: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(params["1"]["value"], "日本語テスト 🎉");
    }

    #[test]
    fn test_parse_json_bindings_with_special_characters() {
        let json = r#"{"1": {"type": "TEXT", "value": "line1\nline2\ttab\"quote"}}"#;
        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let result = parse_json_bindings(&data_ptr).unwrap();
        let params: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert!(params["1"]["value"].is_string());
    }

    #[test]
    fn test_parse_json_bindings_empty_object() {
        // An empty JSON object is valid -- zero bindings
        let json = r#"{}"#;
        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let result = parse_json_bindings(&data_ptr).unwrap();
        let params: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert!(params.is_object());
        assert_eq!(params.as_object().unwrap().len(), 0);
    }

    #[test]
    fn test_parse_json_bindings_many_parameters() {
        // Build a JSON object with 20 parameters
        let mut entries: Vec<String> = Vec::new();
        for i in 1..=20 {
            entries.push(format!(r#""{i}": {{"type": "FIXED", "value": "{i}"}}"#));
        }
        let json = format!("{{{}}}", entries.join(", "));

        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let result = parse_json_bindings(&data_ptr).unwrap();
        let params: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(params.as_object().unwrap().len(), 20);
    }

    // ---------------------------------------------------------------
    // Request serialization round-trip
    // ---------------------------------------------------------------

    #[test]
    fn test_request_serialization_with_bindings() {
        let json = r#"{"1":{"type":"FIXED","value":"7"}}"#;
        let raw: &RawValue = serde_json::from_str(json).unwrap();

        let request = query_request::Request {
            sql_text: "SELECT ?".to_string(),
            async_exec: false,
            sequence_id: 1,
            query_submission_time: 0,
            is_internal: false,
            describe_only: None,
            parameters: None,
            bindings: Some(raw),
            bind_stage: None,
            query_context: query_request::QueryContext { entries: None },
        };

        let serialized = serde_json::to_string(&request).unwrap();

        // The bindings JSON should appear verbatim in the serialized output
        assert!(
            serialized.contains(json),
            "Serialized request must contain the raw JSON verbatim.\nSerialized: {serialized}"
        );
    }

    #[test]
    fn test_request_serialization_with_owned_bindings() {
        // Simulate the Arrow path: Box<RawValue> (owned, passed by reference)
        let json = r#"{"1":{"type":"TEXT","value":"test"}}"#;
        let raw = RawValue::from_string(json.to_string()).unwrap();

        let request = query_request::Request {
            sql_text: "SELECT ?".to_string(),
            async_exec: false,
            sequence_id: 1,
            query_submission_time: 0,
            is_internal: false,
            describe_only: None,
            parameters: None,
            bindings: Some(&*raw),
            bind_stage: None,
            query_context: query_request::QueryContext { entries: None },
        };

        let serialized = serde_json::to_string(&request).unwrap();
        assert!(
            serialized.contains(json),
            "Serialized request must contain the raw JSON verbatim.\nSerialized: {serialized}"
        );
    }

    #[test]
    fn test_request_serialization_without_bindings() {
        // When bindings is None, the "bindings" key should be omitted entirely
        let request = query_request::Request {
            sql_text: "SELECT 1".to_string(),
            async_exec: false,
            sequence_id: 1,
            query_submission_time: 0,
            is_internal: false,
            describe_only: None,
            parameters: None,
            bindings: None,
            bind_stage: None,
            query_context: query_request::QueryContext { entries: None },
        };

        let serialized = serde_json::to_string(&request).unwrap();
        assert!(
            !serialized.contains("bindings"),
            "None bindings should be omitted from serialized output.\nSerialized: {serialized}"
        );
    }

    fn deserialize_query_response(json: &str) -> Data {
        serde_json::from_str(json).expect("test JSON must be valid query response Data")
    }

    #[test]
    fn calculate_rows_affected_sums_dml_columns() {
        let data = deserialize_query_response(
            r#"{
                "statementTypeId": 12544,
                "rowset": [["10", "3"]],
                "rowtype": [
                    {"name": "number of rows inserted", "type": "FIXED", "nullable": false, "scale": 0, "precision": 10},
                    {"name": "number of rows updated", "type": "FIXED", "nullable": false, "scale": 0, "precision": 10}
                ]
            }"#,
        );
        assert_eq!(result_set::calculate_rows_affected(&data), Some(13));
    }

    #[test]
    fn calculate_rows_affected_skips_null_cells() {
        let data = deserialize_query_response(
            r#"{
                "statementTypeId": 12544,
                "rowset": [["5", null]],
                "rowtype": [
                    {"name": "number of rows inserted", "type": "FIXED", "nullable": false, "scale": 0, "precision": 10},
                    {"name": "number of rows deleted", "type": "FIXED", "nullable": true, "scale": 0, "precision": 10}
                ]
            }"#,
        );
        assert_eq!(result_set::calculate_rows_affected(&data), Some(5));
    }

    #[test]
    fn calculate_rows_affected_all_null_cells() {
        let data = deserialize_query_response(
            r#"{
                "statementTypeId": 12544,
                "rowset": [[null]],
                "rowtype": [
                    {"name": "number of rows inserted", "type": "FIXED", "nullable": true, "scale": 0, "precision": 10}
                ]
            }"#,
        );
        assert_eq!(result_set::calculate_rows_affected(&data), Some(0));
    }

    #[test]
    fn calculate_rows_affected_select_uses_total() {
        let data = deserialize_query_response(
            r#"{
                "total": 42
            }"#,
        );
        assert_eq!(result_set::calculate_rows_affected(&data), Some(42));
    }

    #[test]
    fn extract_query_returns_sql_when_set() {
        let conn = Arc::new(Mutex::new(Connection::new()));
        let mut stmt = Statement::new(conn);
        stmt.query = Some("SELECT 1".to_string());

        let result = extract_query(&stmt).unwrap();
        assert_eq!(result, "SELECT 1");
    }

    #[test]
    fn extract_query_errors_when_no_query() {
        let conn = Arc::new(Mutex::new(Connection::new()));
        let stmt = Statement::new(conn);

        let err = extract_query(&stmt).unwrap_err();
        assert!(
            err.to_string().contains("Query not found"),
            "unexpected: {err}"
        );
    }

    #[tokio::test]
    async fn query_context_returns_transport_fields() {
        let mut conn = Connection::new();
        conn.server_url = Some("https://account.snowflakecomputing.com".to_string());
        conn.client_info = Some(crate::config::rest_parameters::test_fixtures::test_client_info());
        conn.http_client = Some(reqwest::Client::new());
        conn.retry_policy = RetryPolicy::default();
        let conn = Arc::new(Mutex::new(conn));

        let (params, _client, _retry) = query_context(&conn).await.unwrap();
        assert_eq!(params.server_url, "https://account.snowflakecomputing.com");
    }

    #[tokio::test]
    async fn query_context_errors_when_not_initialized() {
        let conn = Arc::new(Mutex::new(Connection::new()));

        let err = query_context(&conn).await.err().unwrap();
        assert!(
            err.to_string().contains("not initialized"),
            "unexpected: {err}"
        );
    }
}
