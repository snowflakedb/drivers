use snafu::{OptionExt, ResultExt};
use std::sync::{Mutex, MutexGuard};

use super::Handle;
use super::connection::RefreshContext;
use super::error::*;
use super::global_state::{CONN_HANDLE_MANAGER, STMT_HANDLE_MANAGER};
use crate::apis::database_driver_v1::query::process_query_response;
use crate::rest::snowflake::query_response::Data;
use crate::{
    config::{rest_parameters::QueryParameters, settings::Setting},
    rest::snowflake::{self, QueryExecutionMode, QueryInput, snowflake_query_with_client},
};

use arrow::array::{RecordBatch, StructArray};
use arrow::ffi::{FFI_ArrowArray, FFI_ArrowSchema};
use arrow::ffi_stream::FFI_ArrowArrayStream;
use arrow::{
    array::{Int32Array, StringArray},
    datatypes::DataType,
};
use serde_json::value::RawValue;
use snafu::Snafu;
use std::{collections::HashMap, sync::Arc};

use super::connection::Connection;
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
fn calculate_rows_affected(data: &Data) -> Option<i64> {
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
                    && let Some(value) = rowset[0].get(idx)
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

pub fn statement_new(conn_handle: Handle) -> Result<Handle, ApiError> {
    let handle = conn_handle;
    match CONN_HANDLE_MANAGER.get_obj(handle) {
        Some(conn_ptr) => {
            let stmt = Mutex::new(Statement::new(conn_ptr));
            let handle = STMT_HANDLE_MANAGER.add_handle(stmt);
            Ok(handle)
        }
        None => InvalidArgumentSnafu {
            argument: "Connection handle not found".to_string(),
        }
        .fail(),
    }
}

pub fn statement_release(stmt_handle: Handle) -> Result<(), ApiError> {
    match STMT_HANDLE_MANAGER.delete_handle(stmt_handle) {
        true => Ok(()),
        false => InvalidArgumentSnafu {
            argument: "Failed to release statement handle".to_string(),
        }
        .fail(),
    }
}

pub fn statement_set_option(handle: Handle, key: String, value: Setting) -> Result<(), ApiError> {
    match STMT_HANDLE_MANAGER.get_obj(handle) {
        Some(stmt_ptr) => {
            let mut stmt = stmt_ptr.lock().map_err(|_| StatementLockingSnafu.build())?;
            stmt.settings.insert(key, value);
            Ok(())
        }
        None => InvalidArgumentSnafu {
            argument: "Statement handle not found".to_string(),
        }
        .fail(),
    }
}

pub fn statement_set_sql_query(stmt_handle: Handle, query: String) -> Result<(), ApiError> {
    let handle = stmt_handle;
    match STMT_HANDLE_MANAGER.get_obj(handle) {
        Some(stmt_ptr) => {
            let mut stmt = stmt_ptr.lock().map_err(|_| StatementLockingSnafu.build())?;
            stmt.query = Some(query);
            Ok(())
        }
        None => InvalidArgumentSnafu {
            argument: "Statement handle not found".to_string(),
        }
        .fail(),
    }
}

pub struct PrepareResult {
    pub stream: Box<FFI_ArrowArrayStream>,
    pub columns: Vec<ColumnMetadata>,
}

pub fn statement_prepare(stmt_handle: Handle) -> Result<PrepareResult, ApiError> {
    let result = execute_query_internal(stmt_handle, None, Some(true))?;
    Ok(PrepareResult {
        stream: result.stream,
        columns: result.columns,
    })
}

fn with_statement<T>(
    handle: Handle,
    f: impl FnOnce(MutexGuard<Statement>) -> Result<T, ApiError>,
) -> Result<T, ApiError> {
    let stmt = STMT_HANDLE_MANAGER.get_obj(handle).ok_or_else(|| {
        InvalidArgumentSnafu {
            argument: "Statement handle not found".to_string(),
        }
        .build()
    })?;
    let guard = stmt.lock().map_err(|_| {
        InvalidArgumentSnafu {
            argument: "Statement cannot be locked".to_string(),
        }
        .build()
    })?;
    f(guard)
}

/// # Safety
///
/// This function is unsafe because it dereferences raw pointers to FFI_ArrowSchema and FFI_ArrowArray.
/// The caller must ensure that:
/// - The pointers are valid and properly aligned
/// - The pointers point to valid FFI_ArrowSchema and FFI_ArrowArray structs
/// - The structs referenced by the pointers will not be freed by the caller
/// - No other code is concurrently modifying the memory referenced by these pointers
pub unsafe fn statement_bind(
    stmt_handle: Handle,
    schema: *mut FFI_ArrowSchema,
    array: *mut FFI_ArrowArray,
) -> Result<(), ApiError> {
    let schema = unsafe { FFI_ArrowSchema::from_raw(schema) };
    let array = unsafe { FFI_ArrowArray::from_raw(array) };
    let array = unsafe { arrow::ffi::from_ffi(array, &schema) }.map_err(|_| {
        InvalidArgumentSnafu {
            argument: "Failed to convert ArrowArray".to_string(),
        }
        .build()
    })?;
    let record_batch = RecordBatch::from(StructArray::from(array));
    with_statement(stmt_handle, |mut stmt| {
        stmt.bind_parameters(record_batch);
        Ok(())
    })
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
}

pub fn statement_execute_query<'a>(
    stmt_handle: Handle,
    bindings: Option<BindingType<'a>>,
) -> Result<ExecuteResult, ApiError> {
    execute_query_internal(stmt_handle, bindings, None)
}

fn execute_query_internal<'a>(
    stmt_handle: Handle,
    bindings: Option<BindingType<'a>>,
    describe_only: Option<bool>,
) -> Result<ExecuteResult, ApiError> {
    let handle = stmt_handle;
    let stmt_ptr = STMT_HANDLE_MANAGER.get_obj(handle).ok_or_else(|| {
        InvalidArgumentSnafu {
            argument: "Statement handle not found".to_string(),
        }
        .build()
    })?;

    let mut stmt = stmt_ptr.lock().map_err(|_| StatementLockingSnafu.build())?;
    let query = stmt.query.as_deref().ok_or_else(|| {
        InvalidArgumentSnafu {
            argument: "Query not found".to_string(),
        }
        .build()
    })?;

    let rt = crate::async_bridge::runtime().context(RuntimeCreationSnafu)?;

    let (query_parameters, http_client, retry_policy) = {
        let conn = stmt
            .conn
            .lock()
            .map_err(|_| ConnectionLockingSnafu.build())?;
        (
            QueryParameters::from_settings(&conn.settings).context(ConfigurationSnafu)?,
            conn.http_client
                .clone()
                .context(ConnectionNotInitializedSnafu)?,
            conn.retry_policy.clone(),
        )
    };

    let execution_mode = stmt.execution_mode(Some(query));

    // Get bindings from request or from statement's Arrow bindings.
    //
    // JSON path (from language wrappers): **ZERO COPY** - borrows directly from wrapper memory.
    // Arrow path (ODBC backwards compat): builds HashMap and serializes to JSON (allocations),
    //   stored in `owned_bindings` so `query_bindings` can borrow it.
    // Arrow path produces owned Box<RawValue>; keep it alive so query_bindings can borrow it.
    let owned_bindings = if bindings.is_none() {
        stmt.get_query_parameter_bindings()
            .context(StatementSnafu)?
    } else {
        None
    };
    let query_bindings: Option<&RawValue> = if let Some(binding_type) = &bindings {
        // Handle bindings from request
        match &binding_type {
            BindingType::Json(data_ptr) => {
                // True zero-copy: pointer → &'static RawValue (no allocation, no validation).
                // Wrapper guarantees data lives through synchronous execute call.
                Some(parse_json_bindings(data_ptr).context(StatementSnafu)?)
            }
            BindingType::Csv(_csv_ptr) => {
                // TODO: Implement CSV binding handling (stage upload)
                return Err(InvalidArgumentSnafu {
                    argument: "CSV bindings are not yet implemented".to_string(),
                }
                .build());
            }
        }
    } else {
        owned_bindings.as_deref()
    };

    let query_input = QueryInput {
        sql: query.to_string(),
        bindings: query_bindings,
        describe_only,
    };

    let response = rt.block_on(async {
        let mut ctx = RefreshContext::from_arc(&stmt.conn)?;
        let mut last_error = None;
        loop {
            let session_token = ctx.refresh_token(last_error).await?;
            match snowflake_query_with_client(
                &http_client,
                query_parameters.clone(),
                session_token,
                query_input.clone(),
                &retry_policy,
                execution_mode,
            )
            .await
            {
                Ok(result) => return Ok(result),
                Err(e) => last_error = Some(e),
            }
        }
    })?;

    if response.success {
        let conn = stmt
            .conn
            .lock()
            .map_err(|_| ConnectionLockingSnafu.build())?;
        conn.update_session_params_cache(query, response.data.parameters.as_ref());
    }

    let query_result = rt
        .block_on(process_query_response(&response.data, &http_client))
        .context(QueryResponseProcessingSnafu)?;

    let rowset_stream = Box::new(FFI_ArrowArrayStream::new(query_result.reader));

    // Extract query_id from response
    let query_id = response.data.query_id.clone().unwrap_or_default();

    // Calculate rows_affected based on statement type
    // For DML: Sum of affected rows from rowset columns
    // For SELECT: Total rows in result set
    // For DDL/Unknown: None
    let rows_affected = calculate_rows_affected(&response.data);
    let statement_type_id = response.data.statement_type_id;

    // Extract column metadata: prefer synthetic metadata from PUT/GET processing,
    // fall back to server-provided rowtype for regular queries.
    let columns = query_result.columns.unwrap_or_else(|| {
        response
            .data
            .row_type
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

    let result = ExecuteResult {
        stream: rowset_stream,
        rows_affected,
        query_id,
        columns,
        statement_type_id,
        query: query.to_string(),
    };
    stmt.state = StatementState::Executed;
    Ok(result)
}

/// Convert Arrow RecordBatch parameter bindings to `Cow::Owned(Box<RawValue>)`.
///
/// This is the backwards-compatibility path used by ODBC's `StatementBind` API.
/// Unlike the JSON path (which borrows wrapper memory with zero copy), this path
/// must allocate:
///   1. A `HashMap<String, BindParameter>` is built from Arrow column data.
///   2. `serde_json::to_string()` serializes the HashMap into a JSON `String`
///      (one heap allocation for the output buffer).
///   3. `RawValue::from_string()` validates the JSON syntax and wraps the string
///      (no additional copy -- RawValue takes ownership of the String).
///   4. `Cow::Owned` wraps the Box (no allocation, just an enum tag).
fn parameters_from_record_batch(record_batch: &RecordBatch) -> Result<String, StatementError> {
    let mut parameters = HashMap::new();
    for i in 0..record_batch.num_columns() {
        let column = record_batch.column(i);
        match column.data_type() {
            DataType::Int32 => {
                let value = column
                    .as_any()
                    .downcast_ref::<Int32Array>()
                    .unwrap()
                    .value(0);
                let json_value = serde_json::Value::String(value.to_string());
                parameters.insert(
                    format!("{}", i + 1),
                    query_request::BindParameter {
                        type_: "FIXED".to_string(),
                        value: json_value,
                        format: None,
                        schema: None,
                    },
                );
            }
            DataType::Utf8 => {
                let value = column
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .unwrap()
                    .value(0);
                let json_value = serde_json::Value::String(value.to_string());
                parameters.insert(
                    format!("{}", i + 1),
                    query_request::BindParameter {
                        type_: "TEXT".to_string(),
                        value: json_value,
                        format: None,
                        schema: None,
                    },
                );
            }
            _ => {
                UnsupportedBindParameterTypeSnafu {
                    type_: column.data_type().to_string(),
                }
                .fail()?;
            }
        }
    }
    // Serialize HashMap to a JSON string, then wrap as RawValue.
    // serde_json::to_string allocates the output buffer; RawValue::from_string
    // takes ownership of that String without copying.
    let json_string = serde_json::to_string(&parameters).map_err(|_| {
        UnsupportedBindParameterTypeSnafu {
            type_: "Failed to serialize parameters".to_string(),
        }
        .build()
    })?;
    Ok(json_string)
}

pub struct Statement {
    pub state: StatementState,
    pub settings: HashMap<String, Setting>,
    pub query: Option<String>,
    pub parameter_bindings: Option<RecordBatch>,
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
            settings: HashMap::new(),
            state: StatementState::Initialized,
            query: None,
            parameter_bindings: None,
            conn,
        }
    }

    pub fn bind_parameters(&mut self, record_batch: RecordBatch) {
        self.parameter_bindings = Some(record_batch);
    }

    pub fn get_query_parameter_bindings(&self) -> Result<Option<Box<RawValue>>, StatementError> {
        match self.parameter_bindings.as_ref() {
            Some(parameters) => {
                let json_string = parameters_from_record_batch(parameters)?;
                let raw = RawValue::from_string(json_string).map_err(|_| {
                    UnsupportedBindParameterTypeSnafu {
                        type_: "Failed to create RawValue from serialized parameters".to_string(),
                    }
                    .build()
                })?;
                Ok(Some(raw))
            }
            None => Ok(None),
        }
    }

    fn execution_mode(&self, query: Option<&str>) -> QueryExecutionMode {
        let async_requested = self
            .settings
            .get(snowflake::STATEMENT_ASYNC_EXECUTION_OPTION)
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
fn parse_json_bindings<'a>(data_ptr: &'a DataPtr<'a>) -> Result<&'a RawValue, StatementError> {
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
    // parameters_from_record_batch (Arrow backwards-compat path)
    // ---------------------------------------------------------------

    #[test]
    fn test_parameters_from_record_batch_int32() {
        use arrow::array::Int32Array;
        use arrow::datatypes::{Field, Schema};

        let schema = Schema::new(vec![Field::new("1", DataType::Int32, false)]);
        let batch =
            RecordBatch::try_new(Arc::new(schema), vec![Arc::new(Int32Array::from(vec![42]))])
                .unwrap();

        let result = parameters_from_record_batch(&batch).unwrap();
        let params: serde_json::Value = serde_json::from_str(&result).unwrap();
        let obj = params.as_object().unwrap();
        assert_eq!(obj.len(), 1);
        assert_eq!(obj["1"]["type"], "FIXED");
        assert_eq!(obj["1"]["value"], "42");
    }

    #[test]
    fn test_parameters_from_record_batch_utf8() {
        use arrow::array::StringArray;
        use arrow::datatypes::{Field, Schema};

        let schema = Schema::new(vec![Field::new("1", DataType::Utf8, false)]);
        let batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![Arc::new(StringArray::from(vec!["hello"]))],
        )
        .unwrap();

        let result = parameters_from_record_batch(&batch).unwrap();
        let params: serde_json::Value = serde_json::from_str(&result).unwrap();
        let obj = params.as_object().unwrap();
        assert_eq!(obj.len(), 1);
        assert_eq!(obj["1"]["type"], "TEXT");
        assert_eq!(obj["1"]["value"], "hello");
    }

    #[test]
    fn test_parameters_from_record_batch_mixed_columns() {
        use arrow::array::{Int32Array, StringArray};
        use arrow::datatypes::{Field, Schema};

        let schema = Schema::new(vec![
            Field::new("1", DataType::Int32, false),
            Field::new("2", DataType::Utf8, false),
        ]);
        let batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(Int32Array::from(vec![99])),
                Arc::new(StringArray::from(vec!["world"])),
            ],
        )
        .unwrap();

        let result = parameters_from_record_batch(&batch).unwrap();
        let params: serde_json::Value = serde_json::from_str(&result).unwrap();
        let obj = params.as_object().unwrap();
        assert_eq!(obj.len(), 2);

        assert_eq!(obj["1"]["type"], "FIXED");
        assert_eq!(obj["1"]["value"], "99");

        assert_eq!(obj["2"]["type"], "TEXT");
        assert_eq!(obj["2"]["value"], "world");
    }

    #[test]
    fn test_parameters_from_record_batch_unsupported_type() {
        use arrow::array::Float64Array;
        use arrow::datatypes::{Field, Schema};

        let schema = Schema::new(vec![Field::new("1", DataType::Float64, false)]);
        let batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![Arc::new(Float64Array::from(vec![1.234]))],
        )
        .unwrap();

        let result = parameters_from_record_batch(&batch);
        assert!(
            result.is_err(),
            "Float64 is not a supported bind parameter type"
        );
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
}
