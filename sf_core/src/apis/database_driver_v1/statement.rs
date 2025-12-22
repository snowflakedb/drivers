use snafu::{OptionExt, ResultExt};
use std::sync::{Mutex, MutexGuard};

use super::Handle;
use super::connection::with_valid_session;
use super::error::*;
use super::global_state::{CONN_HANDLE_MANAGER, STMT_HANDLE_MANAGER};
use crate::apis::database_driver_v1::query::process_query_response;
use crate::{
    config::{rest_parameters::QueryParameters, settings::Setting},
    rest::snowflake::{self, QueryExecutionMode, snowflake_query_with_client},
};

use arrow::array::{RecordBatch, StructArray};
use arrow::ffi::{FFI_ArrowArray, FFI_ArrowSchema};
use arrow::ffi_stream::FFI_ArrowArrayStream;
use arrow::{
    array::{Int32Array, StringArray},
    datatypes::DataType,
};
use snafu::Snafu;
use std::{collections::HashMap, sync::Arc};

use super::connection::Connection;
use crate::rest::snowflake::query_request;

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

pub fn statement_prepare(_stmt_handle: Handle) -> Result<(), ApiError> {
    // TODO: Implement statement preparation logic if required.
    Ok(())
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
        stmt.bind_parameters(record_batch).map_err(|_| {
            InvalidArgumentSnafu {
                argument: "Failed to bind parameters".to_string(),
            }
            .build()
        })
    })
}

pub struct ExecuteResult {
    pub stream: Box<FFI_ArrowArrayStream>,
    pub rows_affected: i64,
}

pub fn statement_execute_query(stmt_handle: Handle) -> Result<ExecuteResult, ApiError> {
    let handle = stmt_handle;
    let stmt_ptr = STMT_HANDLE_MANAGER.get_obj(handle).ok_or_else(|| {
        InvalidArgumentSnafu {
            argument: "Statement handle not found".to_string(),
        }
        .build()
    })?;

    let mut stmt = stmt_ptr.lock().map_err(|_| StatementLockingSnafu.build())?;
    let query_str = stmt.query.as_deref().ok_or_else(|| {
        InvalidArgumentSnafu {
            argument: "Query not found".to_string(),
        }
        .build()
    })?;

    // Create a blocking runtime for the async operations
    let rt = tokio::runtime::Runtime::new().context(RuntimeCreationSnafu)?;

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

    let execution_mode = stmt.execution_mode(Some(query_str));
    let query = stmt.query.take().expect("query must be present");
    let bindings = stmt
        .get_query_parameter_bindings()
        .context(StatementSnafu)?;

    // Execute query with automatic session refresh on 401
    let conn = stmt.conn.clone();
    let response = rt.block_on(with_valid_session(&conn, |session_token| {
        let http_client = http_client.clone();
        let query_parameters = query_parameters.clone();
        let query = query.clone();
        let bindings = bindings.clone();
        let retry_policy = retry_policy.clone();
        async move {
            snowflake_query_with_client(
                &http_client,
                query_parameters,
                session_token,
                query,
                bindings,
                &retry_policy,
                execution_mode,
            )
            .await
        }
    }))?;

    let response_reader = rt
        .block_on(process_query_response(&response.data, &http_client))
        .context(QueryResponseProcessingSnafu)?;

    let rowset_stream = Box::new(FFI_ArrowArrayStream::new(response_reader));

    // Serialize pointer into integer
    stmt.state = StatementState::Executed;
    Ok(ExecuteResult {
        stream: rowset_stream,
        rows_affected: 0,
    })
}

fn parameters_from_record_batch(
    record_batch: &RecordBatch,
) -> Result<HashMap<String, query_request::BindParameter>, StatementError> {
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
    Ok(parameters)
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

    pub fn bind_parameters(&mut self, record_batch: RecordBatch) -> Result<(), StatementError> {
        match self.state {
            StatementState::Initialized => {
                self.parameter_bindings = Some(record_batch);
            }
            _ => {
                InvalidStateTransitionSnafu {
                    msg: format!("Cannot bind parameters in state={:?}", self.state),
                }
                .fail()?;
            }
        }
        Ok(())
    }

    pub fn get_query_parameter_bindings(
        &self,
    ) -> Result<Option<HashMap<String, query_request::BindParameter>>, StatementError> {
        match self.parameter_bindings.as_ref() {
            Some(parameters) => Ok(Some(parameters_from_record_batch(parameters)?)),
            None => Ok(None),
        }
    }

    fn execution_mode(&self, query: Option<&str>) -> QueryExecutionMode {
        match self
            .settings
            .get(snowflake::STATEMENT_ASYNC_EXECUTION_OPTION)
        {
            Some(setting) => match parse_bool_setting(setting) {
                Some(true) => QueryExecutionMode::Async,
                Some(false) => QueryExecutionMode::Blocking,
                None => QueryExecutionMode::Async,
            },
            None => match query {
                Some(sql) if is_file_transfer(sql) => QueryExecutionMode::Blocking,
                _ => QueryExecutionMode::Async,
            },
        }
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

#[derive(Snafu, Debug)]
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
}
