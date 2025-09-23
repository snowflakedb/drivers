use crate::api::{OdbcError, diagnostic::DiagnosticInfo};
use crate::cdata_types::CDataType;
use arrow::{array::RecordBatch, ffi_stream::ArrowArrayStreamReader};
use odbc_sys as sql;
use sf_core::protobuf_gen::database_driver_v1::{
    ConnectionHandle as TConnectionHandle, DatabaseHandle as TDatabaseHandle, StatementHandle,
};
use std::collections::HashMap;

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

pub struct Connection {
    pub state: ConnectionState,
    pub diagnostic_info: DiagnosticInfo,
}

#[derive(Debug, Clone)]
pub struct ParameterBinding {
    pub parameter_type: sql::SqlDataType,
    pub value_type: CDataType,
    pub parameter_value_ptr: sql::Pointer,
    pub buffer_length: sql::Len,
    pub str_len_or_ind_ptr: *mut sql::Len,
}

pub enum StatementState {
    Created,
    Executed {
        reader: ArrowArrayStreamReader,
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
    pub parameter_bindings: HashMap<u16, ParameterBinding>,
    pub diagnostic_info: DiagnosticInfo,
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
