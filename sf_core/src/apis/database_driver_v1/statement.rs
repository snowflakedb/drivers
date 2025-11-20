use snafu::ResultExt;
use std::sync::{Mutex, MutexGuard};

use super::Handle;
use super::error::*;
use super::global_state::{CONN_HANDLE_MANAGER, STMT_HANDLE_MANAGER};
use crate::apis::database_driver_v1::query::process_query_response;
use crate::{
    config::{rest_parameters::QueryParameters, settings::Setting},
    rest::snowflake::{self, QueryExecutionMode, snowflake_query},
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
            let mut stmt = stmt_ptr
                .lock()
                .map_err(|_| StatementLockingSnafu {}.build())?;
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
            let mut stmt = stmt_ptr
                .lock()
                .map_err(|_| StatementLockingSnafu {}.build())?;
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

    let mut stmt = stmt_ptr
        .lock()
        .map_err(|_| StatementLockingSnafu {}.build())?;
    let query = stmt.query.take().ok_or_else(|| {
        InvalidArgumentSnafu {
            argument: "Query not found".to_string(),
        }
        .build()
    })?;

    // Create a blocking runtime for the async operations
    let rt = tokio::runtime::Runtime::new().context(RuntimeCreationSnafu)?;

    let (query_parameters, session_token) = {
        let conn = stmt
            .conn
            .lock()
            .map_err(|_| ConnectionLockingSnafu {}.build())?;
        (
            QueryParameters::from_settings(&conn.settings).context(ConfigurationSnafu)?,
            conn.session_token.clone().ok_or_else(|| {
                InvalidArgumentSnafu {
                    argument: "Session token not found".to_string(),
                }
                .build()
            })?,
        )
    };

    let response = rt
        .block_on(snowflake_query(
            query_parameters,
            session_token,
            query,
            stmt.get_query_parameter_bindings().map_err(|_| {
                InvalidArgumentSnafu {
                    argument: "Failed to get query parameter bindings".to_string(),
                }
                .build()
            })?,
            stmt.execution_mode(),
        ))
        .context(LoginSnafu)?;

    let response_reader = rt
        .block_on(process_query_response(&response.data))
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

    fn execution_mode(&self) -> QueryExecutionMode {
        match self
            .settings
            .get(snowflake::STATEMENT_ASYNC_EXECUTION_OPTION)
        {
            Some(Setting::String(value)) => {
                if value.eq_ignore_ascii_case("true")
                    || value.eq_ignore_ascii_case("yes")
                    || value == "1"
                {
                    QueryExecutionMode::Async
                } else {
                    QueryExecutionMode::Blocking
                }
            }
            Some(Setting::Int(value)) => {
                if *value != 0 {
                    QueryExecutionMode::Async
                } else {
                    QueryExecutionMode::Blocking
                }
            }
            Some(Setting::Double(value)) => {
                if *value != 0.0 {
                    QueryExecutionMode::Async
                } else {
                    QueryExecutionMode::Blocking
                }
            }
            Some(Setting::Bytes(_)) | None => QueryExecutionMode::Blocking,
        }
    }
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
