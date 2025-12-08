use crate::api::{OdbcError, diagnostic::DiagnosticInfo};
use crate::cdata_types::CDataType;
use arrow::{array::RecordBatch, ffi_stream::ArrowArrayStreamReader};
use odbc_sys as sql;
use sf_core::protobuf_gen::database_driver_v1::{
    ConnectionHandle as TConnectionHandle, DatabaseHandle as TDatabaseHandle, StatementHandle,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

/// Result type for ODBC operations
pub type OdbcResult<T> = Result<T, OdbcError>;

pub trait ToSqlReturn {
    fn to_sql_return(self) -> sql::SqlReturn;
    fn to_sql_code(self) -> i16;
}

impl ToSqlReturn for OdbcResult<()> {
    fn to_sql_return(self) -> sql::SqlReturn {
        match self {
            Ok(_) => sql::SqlReturn::SUCCESS,
            Err(OdbcError::NeedData { .. }) => sql::SqlReturn::NEED_DATA,
            Err(OdbcError::NoMoreData { .. }) => sql::SqlReturn::NO_DATA,
            Err(OdbcError::InvalidHandle { .. }) => sql::SqlReturn::INVALID_HANDLE,
            Err(_) => sql::SqlReturn::ERROR,
        }
    }
    fn to_sql_code(self) -> sql::RetCode {
        self.to_sql_return().0
    }
}

pub struct Environment {
    pub odbc_version: sql::Integer,
    pub diagnostic_info: DiagnosticInfo,
}

pub enum ConnectionState {
    Disconnected,
    Connected {
        #[allow(dead_code)]
        db_handle: TDatabaseHandle,
        conn_handle: TConnectionHandle,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimestampLtzFormat {
    pub fractional: bool,
    pub include_timezone: bool,
    pub fractional_digits: Option<u8>,
    pub force_fractional: bool,
}

impl TimestampLtzFormat {
    pub const fn new(fractional: bool, include_timezone: bool) -> Self {
        Self {
            fractional,
            include_timezone,
            fractional_digits: None,
            force_fractional: false,
        }
    }

    pub const fn with_digits(mut self, digits: Option<u8>) -> Self {
        self.fractional_digits = digits;
        self
    }

    pub const fn with_force_fractional(mut self, force: bool) -> Self {
        self.force_fractional = force;
        self
    }
}

pub type TimestampNtzFormat = TimestampLtzFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampType {
    Ltz,
    Ntz,
    Tz,
}

pub struct Connection {
    pub state: ConnectionState,
    pub diagnostic_info: DiagnosticInfo,
    pub timestamp_ltz_format: TimestampLtzFormat,
    pub timestamp_ntz_format: TimestampNtzFormat,
    pub timestamp_tz_format: TimestampLtzFormat,
    pub timestamp_type_mapping: TimestampType,
    pub log_settings: Option<LogSettings>,
    pub session_timezone: Option<String>,
    pub lob_settings: LargeObjectSettings,
    pub use_custom_sql_types: bool,
    pub current_catalog: Option<String>,
    pub use_current_catalog: bool,
}

#[derive(Debug, Clone)]
pub struct LogSettings {
    pub log_path: PathBuf,
    pub log_file_count: u64,
    pub enable_pid_log_file_names: bool,
    pub curl_verbose_mode: bool,
}

impl LogSettings {
    pub fn generic_log_file(&self) -> PathBuf {
        let mut file_name = String::from("snowflake_odbc_generic");
        if self.enable_pid_log_file_names {
            file_name.push('_');
            file_name.push_str(&std::process::id().to_string());
        } else {
            file_name.push('0');
        }
        self.log_path.join(file_name).with_extension("log")
    }
}

#[derive(Debug, Clone)]
pub struct ParameterBinding {
    pub parameter_type: sql::SqlDataType,
    pub value_type: CDataType,
    pub parameter_value_ptr: sql::Pointer,
    pub buffer_length: sql::Len,
    pub str_len_or_ind_ptr: *mut sql::Len,
    pub owned_buffer: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct LargeObjectSettings {
    pub max_lob_size_in_memory: Option<i64>,
    pub enable_large_varchar_binary: Option<bool>,
    pub default_varchar_size: Option<i64>,
    pub default_binary_size: Option<i64>,
}

impl Default for LargeObjectSettings {
    fn default() -> Self {
        Self {
            max_lob_size_in_memory: None,
            enable_large_varchar_binary: None,
            default_varchar_size: None,
            default_binary_size: None,
        }
    }
}

#[derive(Debug)]
pub enum DataAtExecMode {
    ExecDirect { query_text: String },
    ExecutePrepared,
}

#[derive(Debug)]
pub struct DataAtExecState {
    pub mode: DataAtExecMode,
    pub pending_params: VecDeque<u16>,
    pub current_param: Option<u16>,
    pub awaiting_data: bool,
    pub buffers: HashMap<u16, Vec<u8>>,
    pub null_params: HashSet<u16>,
}

pub enum StatementState {
    Created,
    Executed {
        reader: ArrowArrayStreamReader,
        schema: arrow::datatypes::SchemaRef,
        rows_affected: i64,
    },
    Fetching {
        reader: ArrowArrayStreamReader,
        record_batch: RecordBatch,
        batch_idx: usize,
    },
    Done,
    Error,
}

pub struct State<T> {
    current_state: Option<T>,
}

/// # Safety
/// All public functions assume that the state is not None and leave object with current state set.
impl<T> State<T> {
    pub fn new(initial_state: T) -> Self {
        Self {
            current_state: Some(initial_state),
        }
    }

    /// # Safety
    /// This function assumes that the state is not None, make sure to call set after taking.
    fn take(&mut self) -> T {
        self.current_state.take().unwrap()
    }

    fn set(&mut self, state: T) {
        self.current_state = Some(state);
    }

    pub fn transition_or_err<R, E>(
        &mut self,
        f: impl Fn(T) -> Result<(T, R), (T, E)>,
    ) -> Result<R, E> {
        let state: T = self.take();
        match f(state) {
            Ok((next_state, result)) => {
                self.set(next_state);
                Ok(result)
            }
            Err((next_state, error)) => {
                self.set(next_state);
                Err(error)
            }
        }
    }

    pub fn as_ref(&self) -> &T {
        self.current_state.as_ref().unwrap()
    }
}

impl<T> From<T> for State<T> {
    fn from(state: T) -> Self {
        Self::new(state)
    }
}

pub trait WithState<T, R> {
    fn with_state(self, state: T) -> R;
}

impl<T, R, E> WithState<T, Result<R, (T, E)>> for Result<R, E> {
    fn with_state(self, state: T) -> Result<R, (T, E)> {
        self.map_err(|e| (state, e))
    }
}

pub struct Statement<'a> {
    pub conn: &'a mut Connection,
    pub stmt_handle: StatementHandle,
    pub state: State<StatementState>,
    pub cached_schema: Option<arrow::datatypes::SchemaRef>,
    pub is_prepared: bool,
    pub parameter_bindings: HashMap<u16, ParameterBinding>,
    pub column_bindings: HashMap<u16, ParameterBinding>, // Reuse ParameterBinding struct for columns
    pub diagnostic_info: DiagnosticInfo,
    pub query_timeout: usize,  // Query timeout in seconds (0 = no timeout)
    pub max_rows: usize,       // Maximum rows to return (0 = no limit)
    pub current_row: usize,    // Current row number (1-based)
    pub row_bind_type: usize,  // SQL_ATTR_ROW_BIND_TYPE value
    pub row_array_size: usize, // SQL_ATTR_ROW_ARRAY_SIZE value
    pub multi_statement_count: usize, // Number of statements in batch (0 = unlimited, 1 = single)
    pub paramset_size: usize,
    pub param_status_ptr: Option<*mut sql::USmallInt>,
    pub params_processed_ptr: Option<*mut sql::ULen>,
    pub param_bind_type: ParamBindType,
    pub rows_fetched_ptr: Option<*mut sql::ULen>,
    pub row_status_ptr: Option<*mut sql::USmallInt>,
    pub last_rows_affected: i64, // Preserve row count after cursor close
    pub session_timezone: Option<String>, // Session timezone for TIMESTAMP_LTZ conversion
    pub prepared_query: Option<String>, // The SQL query text after SQLPrepare
    pub child_result_ids: Vec<String>, // Child query IDs for multi-statement queries
    pub current_result_index: usize, // Current result set index (0-based)
    pub has_cursor: bool,        // True if a cursor was opened (SELECT query), false for DDL
    pub metadata_id: bool,       // SQL_ATTR_METADATA_ID flag
    pub last_query_id: Option<String>, // Most recent Snowflake query ID
    pub data_at_exec_state: Option<DataAtExecState>,
}

#[derive(Debug, Clone, Copy)]
pub enum ParamBindType {
    Column,
    Row(usize),
}

// Helper functions for handle conversion
pub fn env_from_handle<'a>(handle: sql::Handle) -> &'a mut Environment {
    let env_ptr = handle as *mut Environment;
    unsafe { env_ptr.as_mut().unwrap() }
}

pub fn conn_from_handle<'a>(handle: sql::Handle) -> &'a mut Connection {
    let conn_ptr = handle as *mut Connection;
    unsafe { conn_ptr.as_mut().unwrap() }
}

pub fn stmt_from_handle<'a>(handle: sql::Handle) -> &'a mut Statement<'a> {
    let stmt_ptr = handle as *mut Statement;
    unsafe { stmt_ptr.as_mut().unwrap() }
}
