use crate::api::{OdbcError, diagnostic::DiagnosticInfo};
use crate::cdata_types::CDataType;
use arrow::{array::RecordBatch, ffi_stream::ArrowArrayStreamReader};
use odbc_sys as sql;
use sf_core::thrift_gen::database_driver_v1::{
    ConnectionHandle as TConnectionHandle, DatabaseHandle as TDatabaseHandle, ExecuteResult,
    StatementHandle, TDatabaseDriverSyncClient,
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
        client: Box<dyn TDatabaseDriverSyncClient + Send>,
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
        result: ExecuteResult,
    },
    Fetching {
        reader: ArrowArrayStreamReader,
        record_batch: RecordBatch,
        batch_idx: usize,
    },
    Done,
}

pub struct Statement<'a> {
    pub conn: &'a mut Connection,
    pub stmt_handle: StatementHandle,
    pub state: StatementState,
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
