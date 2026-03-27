use snafu::{OptionExt, ResultExt, Snafu};
use tokio::sync::Mutex;

use super::connection::{Connection, RefreshContext};
use super::error::*;
use super::global_state::DatabaseDriverV1;
use super::query::process_query_response;
use super::validation::{
    ValidationIssue, ValidationSeverity, canonicalize_setting_key, resolve_options,
    validate_statement_option_write,
};
use crate::chunks::ChunkDownloadData;
use crate::config::ParamStore;
use crate::config::param_registry::param_names;
use crate::config::settings::Setting;
use crate::handle_manager::Handle;
use crate::rest::snowflake::query_response::{Data, Stats};
use crate::rest::snowflake::{
    QueryExecutionMode, QueryInput, snowflake_get_query_result, snowflake_query_with_client,
};

use arrow::ffi_stream::FFI_ArrowArrayStream;
use serde_json::value::RawValue;
use std::{collections::HashMap, sync::Arc};

#[cfg(test)]
use crate::rest::snowflake::query_request;

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

/// Column names whose values are summed to compute DML rows-affected (exact match).
const DML_AFFECTED_ROWS_COLUMNS: &[&str] = &[
    "number of rows updated",
    "number of multi-joined rows updated",
    "number of rows deleted",
];

/// Column name prefixes whose values are summed to compute DML rows-affected.
const DML_AFFECTED_ROWS_COLUMN_PREFIXES: &[&str] = &["number of rows inserted"];

// Statement type ID constants for DML detection
const STATEMENT_TYPE_ID_DML: i64 = 0x3000;
const STATEMENT_TYPE_ID_INSERT: i64 = 0x3100;
const STATEMENT_TYPE_ID_UPDATE: i64 = 0x3200;
const STATEMENT_TYPE_ID_DELETE: i64 = 0x3300;
const STATEMENT_TYPE_ID_MERGE: i64 = 0x3400;
const STATEMENT_TYPE_ID_MULTI_TABLE_INSERT: i64 = 0x3500;

/// Check if a statement type ID represents a DML operation
fn is_dml_statement(statement_type_id: Option<i64>) -> bool {
    if let Some(type_id) = statement_type_id {
        matches!(
            type_id,
            STATEMENT_TYPE_ID_DML
                | STATEMENT_TYPE_ID_INSERT
                | STATEMENT_TYPE_ID_UPDATE
                | STATEMENT_TYPE_ID_DELETE
                | STATEMENT_TYPE_ID_MERGE
                | STATEMENT_TYPE_ID_MULTI_TABLE_INSERT
        )
    } else {
        false
    }
}

/// Calculate rows affected based on statement type.
///
/// Returns `Some(count)` when rows affected is known, `None` when it is not
/// (when the statement type is unknown).
///
/// - For DML: Parse rowset columns to sum affected rows
/// - For SELECT and other queries: Use total field
/// - For unknown: Return None
pub(crate) fn calculate_rows_affected(data: &Data) -> Option<i64> {
    // Check if this is a DML statement
    if is_dml_statement(data.statement_type_id) {
        // For DML, parse the rowset to get affected rows
        if let (Some(rowset), Some(row_types)) = (&data.rowset, &data.row_type)
            && !rowset.is_empty()
            && !rowset[0].is_empty()
        {
            let mut affected_rows = 0i64;

            // Look for specific column names that indicate affected rows
            for (idx, col) in row_types.iter().enumerate() {
                let col_name = col.name.to_lowercase();

                if (DML_AFFECTED_ROWS_COLUMNS.contains(&col_name.as_str())
                    || DML_AFFECTED_ROWS_COLUMN_PREFIXES
                        .iter()
                        .any(|p| col_name.starts_with(p)))
                    && let Some(Some(value)) = rowset[0].get(idx)
                    && let Ok(count) = value.parse::<i64>()
                {
                    affected_rows += count;
                }
            }

            return Some(affected_rows);
        }
        // DML with no affected rows
        return Some(0);
    }

    // For SELECT and other queries, use total field.
    // Return None if total is not available.
    data.total
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
    pub columns: Vec<ColumnMetadata>,
}

impl DatabaseDriverV1 {
    pub async fn statement_prepare(&self, stmt_handle: Handle) -> Result<PrepareResult, ApiError> {
        let result = self
            .execute_query_internal(stmt_handle, None, Some(true))
            .await?;
        Ok(PrepareResult {
            stream: result.stream,
            columns: result.columns,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ColumnMetadata {
    pub name: String,
    pub r#type: String,
    pub precision: Option<i64>,
    pub scale: Option<i64>,
    pub length: Option<i64>,
    pub byte_length: Option<i64>,
    pub nullable: bool,
}

pub struct ExecuteResult {
    pub stream: Box<FFI_ArrowArrayStream>,
    pub rows_affected: Option<i64>,
    pub query_id: String,
    pub columns: Vec<ColumnMetadata>,
    pub statement_type_id: Option<i64>,
    pub query: String,
    pub sql_state: Option<String>,
    pub stats: Option<Stats>,
}

impl DatabaseDriverV1 {
    pub async fn statement_execute_query<'a>(
        &self,
        stmt_handle: Handle,
        bindings: Option<BindingType<'a>>,
    ) -> Result<ExecuteResult, ApiError> {
        self.execute_query_internal(stmt_handle, bindings, None)
            .await
    }

    async fn execute_query_internal<'a>(
        &self,
        stmt_handle: Handle,
        bindings: Option<BindingType<'a>>,
        describe_only: Option<bool>,
    ) -> Result<ExecuteResult, ApiError> {
        let stmt_ptr = self.statements.get_obj(stmt_handle).ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "Statement handle not found".to_string(),
            }
            .build()
        })?;

        let mut stmt = stmt_ptr.lock().await;
        let query = stmt.query.clone().ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "Query not found".to_string(),
            }
            .build()
        })?;

        let (query_parameters, http_client, retry_policy) = {
            let conn = stmt.conn.lock().await;
            (
                conn.query_transport_parameters()?,
                conn.http_client
                    .clone()
                    .context(ConnectionNotInitializedSnafu)?,
                conn.retry_policy.clone(),
            )
        };

        let execution_mode = stmt.execution_mode(Some(&query));

        // Get bindings from request.
        // JSON path: zero-copy — borrows directly from wrapper memory.
        let query_bindings: Option<&RawValue> = if let Some(binding_type) = &bindings {
            match &binding_type {
                BindingType::Json(data_ptr) => {
                    Some(parse_json_bindings(data_ptr).context(StatementSnafu)?)
                }
                BindingType::Csv(_csv_ptr) => {
                    return Err(InvalidArgumentSnafu {
                        argument: "CSV bindings are not yet implemented".to_string(),
                    }
                    .build());
                }
            }
        } else {
            None
        };

        let query_input = QueryInput {
            sql: query.clone(),
            bindings: query_bindings,
            describe_only,
        };

        let response = {
            let mut ctx = RefreshContext::from_arc(&stmt.conn).await?;
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
            let conn = stmt.conn.lock().await;
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

        stmt.chunk_info = Some(StoredChunkInfo {
            initial_chunk_base64: response.data.to_initial_base64_opt().map(String::from),
            chunks: response.data.to_chunk_download_data().unwrap_or_default(),
        });

        let result = response_to_execute_result(response.data, &http_client, query).await?;
        stmt.state = StatementState::Executed;
        Ok(result)
    }

    pub async fn statement_result_chunks(
        &self,
        stmt_handle: Handle,
    ) -> Result<StoredChunkInfo, ApiError> {
        let stmt_ptr = self.statements.get_obj(stmt_handle).ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "Statement handle not found".to_string(),
            }
            .build()
        })?;

        let stmt = stmt_ptr.lock().await;
        let chunk_info = stmt.chunk_info.as_ref().ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "No chunk info available; execute a query first".to_string(),
            }
            .build()
        })?;

        Ok(StoredChunkInfo {
            initial_chunk_base64: chunk_info.initial_chunk_base64.clone(),
            chunks: chunk_info
                .chunks
                .iter()
                .map(|c| ChunkDownloadData::new(&c.url, &c.headers))
                .collect(),
        })
    }

    pub async fn connection_get_query_result(
        &self,
        conn_handle: Handle,
        query_id: String,
    ) -> Result<ExecuteResult, ApiError> {
        let conn_ptr = self.connections.get_obj(conn_handle).ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .build()
        })?;

        let (query_parameters, http_client, retry_policy) = {
            let conn = conn_ptr.lock().await;
            (
                conn.query_transport_parameters()?,
                conn.http_client
                    .clone()
                    .context(ConnectionNotInitializedSnafu)?,
                conn.retry_policy.clone(),
            )
        };

        let response = {
            let mut ctx = RefreshContext::from_arc(&conn_ptr).await?;
            let mut last_error = None;
            loop {
                let session_token = ctx.refresh_token(last_error).await?;
                match snowflake_get_query_result(
                    &http_client,
                    &query_parameters,
                    session_token.reveal(),
                    &query_id,
                    &retry_policy,
                )
                .await
                {
                    Ok(result) => break Ok(result),
                    Err(e) => last_error = Some(e),
                }
            }
        }?;

        if response.success {
            let conn = conn_ptr.lock().await;
            conn.update_session_params_cache(
                "",
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

        response_to_execute_result(response.data, &http_client, String::new()).await
    }
}

/// Convert a Snowflake query response into an `ExecuteResult` by processing
/// the Arrow data, extracting column metadata, and assembling all fields.
///
/// Async because `process_query_response` may download additional Arrow
/// chunks over HTTP for large result sets.
async fn response_to_execute_result(
    data: Data,
    http_client: &reqwest::Client,
    query: String,
) -> Result<ExecuteResult, ApiError> {
    let query_result = process_query_response(&data, http_client)
        .await
        .context(QueryResponseProcessingSnafu)?;

    let stream = Box::new(FFI_ArrowArrayStream::new(query_result.reader));
    let query_id = data.query_id.clone().unwrap_or_default();
    let rows_affected = calculate_rows_affected(&data);
    let statement_type_id = data.statement_type_id;

    let columns = query_result.columns.unwrap_or_else(|| {
        data.row_type
            .unwrap_or_default()
            .iter()
            .map(|rt| ColumnMetadata {
                name: rt.name.clone(),
                r#type: rt.type_.clone(),
                precision: rt.precision.map(|v| v as i64),
                scale: rt.scale.map(|v| v as i64),
                length: rt.length.map(|v| v as i64),
                byte_length: rt.byte_length.map(|v| v as i64),
                nullable: rt.nullable,
            })
            .collect()
    });

    Ok(ExecuteResult {
        stream,
        rows_affected,
        query_id,
        columns,
        statement_type_id,
        query,
        sql_state: data.sql_state,
        stats: data.stats,
    })
}

pub struct StoredChunkInfo {
    pub initial_chunk_base64: Option<String>,
    pub chunks: Vec<ChunkDownloadData>,
}

pub struct Statement {
    pub state: StatementState,
    pub(crate) settings: ParamStore,
    pub query: Option<String>,
    pub conn: Arc<Mutex<Connection>>,
    pub(crate) chunk_info: Option<StoredChunkInfo>,
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
            chunk_info: None,
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(calculate_rows_affected(&data), Some(13));
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
        assert_eq!(calculate_rows_affected(&data), Some(5));
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
        assert_eq!(calculate_rows_affected(&data), Some(0));
    }

    #[test]
    fn calculate_rows_affected_select_uses_total() {
        let data = deserialize_query_response(
            r#"{
                "total": 42
            }"#,
        );
        assert_eq!(calculate_rows_affected(&data), Some(42));
    }
}
