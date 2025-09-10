use crate::apis::database_driver_v1::query::process_query_response;
use crate::config::ConfigError;
use crate::config::rest_parameters::{LoginParameters, QueryParameters};
use crate::config::settings::Setting;
use crate::driver::{Connection, Database, Statement};
use crate::handle_manager::{Handle, HandleManager};
use crate::rest::snowflake::{RestError, snowflake_query};
use crate::thrift_apis::ThriftApi;
use snafu;
use snafu::{Location, Report, ResultExt, location};
extern crate lazy_static;
use lazy_static::lazy_static;
use thrift::protocol::{TInputProtocol, TOutputProtocol};

use crate::thrift_gen::database_driver_v1::{
    ArrowArrayPtr, ArrowSchemaPtr, AuthenticationError, ConnectionHandle, DatabaseDriverSyncClient,
    DatabaseDriverSyncHandler, DatabaseDriverSyncProcessor, DatabaseHandle, DriverError,
    DriverException, ExecuteResult, GenericError, InfoCode, InternalError, InvalidParameterValue,
    LoginError, MissingParameter, PartitionedResult, StatementHandle, StatusCode,
    TDatabaseDriverSyncClient,
};

use arrow::array::{RecordBatch, StructArray};
use arrow::ffi::{FFI_ArrowArray, FFI_ArrowSchema};
use arrow::ffi_stream::FFI_ArrowArrayStream;
use std::mem::size_of;
use std::sync::{Mutex, MutexGuard};
use thrift::server::TProcessor;
use thrift::{Error, OrderedFloat};
use tracing::instrument;

use crate::driver::StatementState;
use crate::thrift_gen::database_driver_v1::ArrowArrayStreamPtr;

impl From<Box<ArrowArrayStreamPtr>> for *mut FFI_ArrowArrayStream {
    fn from(ptr: Box<ArrowArrayStreamPtr>) -> Self {
        unsafe { std::ptr::read(ptr.value.as_ptr() as *const *mut FFI_ArrowArrayStream) }
    }
}
#[allow(clippy::from_over_into)]
impl Into<*mut FFI_ArrowSchema> for ArrowSchemaPtr {
    fn into(self) -> *mut FFI_ArrowSchema {
        unsafe { std::ptr::read(self.value.as_ptr() as *const *mut FFI_ArrowSchema) }
    }
}

#[allow(clippy::from_over_into)]
impl Into<*mut FFI_ArrowArray> for ArrowArrayPtr {
    fn into(self) -> *mut FFI_ArrowArray {
        unsafe { std::ptr::read(self.value.as_ptr() as *const *mut FFI_ArrowArray) }
    }
}

impl From<Handle> for DatabaseHandle {
    fn from(handle: Handle) -> Self {
        DatabaseHandle {
            id: handle.id as i64,
            magic: handle.magic as i64,
        }
    }
}

impl From<*mut FFI_ArrowArrayStream> for ArrowArrayStreamPtr {
    fn from(raw: *mut FFI_ArrowArrayStream) -> Self {
        let len = size_of::<*mut FFI_ArrowArrayStream>();
        let buf_ptr = std::ptr::addr_of!(raw) as *const u8;
        let slice = unsafe { std::slice::from_raw_parts(buf_ptr, len) };
        let vec = slice.to_vec();
        ArrowArrayStreamPtr { value: vec }
    }
}

impl From<DatabaseHandle> for Handle {
    fn from(val: DatabaseHandle) -> Self {
        Handle {
            id: val.id as u64,
            magic: val.magic as u64,
        }
    }
}

impl From<Handle> for ConnectionHandle {
    fn from(handle: Handle) -> Self {
        ConnectionHandle {
            id: handle.id as i64,
            magic: handle.magic as i64,
        }
    }
}

impl From<ConnectionHandle> for Handle {
    fn from(val: ConnectionHandle) -> Self {
        Handle {
            id: val.id as u64,
            magic: val.magic as u64,
        }
    }
}

impl From<Handle> for StatementHandle {
    fn from(handle: Handle) -> Self {
        StatementHandle {
            id: handle.id as i64,
            magic: handle.magic as i64,
        }
    }
}

impl From<StatementHandle> for Handle {
    fn from(val: StatementHandle) -> Self {
        Handle {
            id: val.id as u64,
            magic: val.magic as u64,
        }
    }
}

lazy_static! {
    static ref DB_HANDLE_MANAGER: HandleManager<Mutex<Database>> = HandleManager::new();
    static ref CONN_HANDLE_MANAGER: HandleManager<Mutex<Connection>> = HandleManager::new();
    static ref STMT_HANDLE_MANAGER: HandleManager<Mutex<Statement>> = HandleManager::new();
}

pub struct DatabaseDriverV1 {}
pub struct DatabaseDriverV1Server {}

impl ThriftApi for DatabaseDriverV1 {
    type ClientInterface = Box<dyn TDatabaseDriverSyncClient + Send>;
    fn client(
        input_protocol: impl TInputProtocol + Send + 'static,
        output_protocol: impl TOutputProtocol + Send + 'static,
    ) -> Self::ClientInterface {
        Box::new(DatabaseDriverSyncClient::new(
            input_protocol,
            output_protocol,
        ))
    }
    fn server() -> Box<dyn TProcessor + Send + Sync> {
        Box::new(DatabaseDriverSyncProcessor::new(
            DatabaseDriverV1Server::new(),
        ))
    }
}

impl Default for DatabaseDriverV1Server {
    fn default() -> Self {
        Self::new()
    }
}

use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum ApiError {
    #[snafu(display("Generic error"))]
    GenericError {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to create runtime"))]
    FailedToCreateRuntime {
        #[snafu(implicit)]
        location: Location,
        source: std::io::Error,
    },
    #[snafu(display("Configuration error: {source}"))]
    ConfigurationError {
        #[snafu(implicit)]
        location: Location,
        source: ConfigError,
    },
    #[snafu(display("Invalid argument: {argument}"))]
    InvalidArgument {
        argument: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to login: {source}"))]
    FailedToLogin {
        #[snafu(implicit)]
        location: Location,
        source: RestError,
    },
    #[snafu(display("Failed to lock connection"))]
    FailedToLockConnection {
        #[snafu(implicit)]
        location: Location,
    },
}

impl ApiError {
    fn to_driver_error(&self) -> DriverError {
        match self {
            ApiError::GenericError { .. } => DriverError::GenericError(GenericError::new()),
            ApiError::FailedToCreateRuntime { .. } => {
                DriverError::InternalError(InternalError::new())
            }
            ApiError::ConfigurationError {
                source:
                    ConfigError::InvalidParameterValue {
                        parameter,
                        value,
                        explanation,
                        ..
                    },
                ..
            } => DriverError::InvalidParameterValue(InvalidParameterValue::new(
                parameter.clone(),
                value.clone(),
                explanation.clone(),
            )),
            ApiError::ConfigurationError {
                source: ConfigError::MissingParameter { parameter, .. },
                ..
            } => DriverError::MissingParameter(MissingParameter::new(parameter.clone())),
            ApiError::InvalidArgument { .. } => DriverError::InternalError(InternalError::new()),
            ApiError::FailedToLogin {
                source: RestError::LoginError { message, code, .. },
                ..
            } => DriverError::LoginError(LoginError {
                message: message.clone(),
                code: *code,
            }),
            ApiError::FailedToLogin { source, .. } => {
                DriverError::AuthError(AuthenticationError::new(source.to_string()))
            }
            ApiError::FailedToLockConnection { .. } => {
                DriverError::InternalError(InternalError::new())
            }
        }
    }
}

fn status_code_of_driver_error(error: &DriverError) -> StatusCode {
    match error {
        DriverError::GenericError(_) => StatusCode::GENERIC_ERROR,
        DriverError::InternalError(_) => StatusCode::INTERNAL_ERROR,
        DriverError::AuthError(_) => StatusCode::AUTHENTICATION_ERROR,
        DriverError::MissingParameter(_) => StatusCode::MISSING_PARAMETER,
        DriverError::InvalidParameterValue(_) => StatusCode::INVALID_PARAMETER_VALUE,
        DriverError::LoginError(_) => StatusCode::LOGIN_ERROR,
    }
}

impl From<ApiError> for thrift::Error {
    fn from(error: ApiError) -> Self {
        let driver_error = error.to_driver_error();
        let message = error.to_string();
        let report = Report::from_error(error).to_string();
        Error::from(DriverException::new(
            message,
            status_code_of_driver_error(&driver_error),
            driver_error,
            report,
        ))
    }
}

trait ToThriftResult {
    fn to_thrift(self) -> thrift::Result<()>;
}

impl<E> ToThriftResult for Result<(), E>
where
    E: Into<thrift::Error>,
{
    fn to_thrift(self) -> thrift::Result<()> {
        self.map_err(|e| e.into())
    }
}

fn connection_init(
    conn_handle: ConnectionHandle,
    _db_handle: DatabaseHandle,
) -> Result<(), ApiError> {
    let handle = conn_handle.into();
    match CONN_HANDLE_MANAGER.get_obj(handle) {
        Some(conn_ptr) => {
            // Create a blocking runtime for the login process
            let rt = tokio::runtime::Runtime::new().context(FailedToCreateRuntimeSnafu)?;
            // .map_err(|e| internal_error(format!("Failed to create runtime: {e}")))?;

            let login_parameters =
                LoginParameters::from_settings(&conn_ptr.lock().unwrap().settings)
                    .context(ConfigurationSnafu)?;

            let login_result = rt
                .block_on(async {
                    crate::rest::snowflake::snowflake_login(&login_parameters).await
                })
                .context(FailedToLoginSnafu)?;

            conn_ptr
                .lock()
                .map_err(|_| ApiError::FailedToLockConnection {
                    location: location!(),
                })?
                .session_token = Some(login_result);
            Ok(())
        }
        None => Err(ApiError::InvalidArgument {
            argument: "Connection handle not found".to_string(),
            location: location!(),
        }),
    }
}

impl DatabaseDriverV1Server {
    /// Helper to create a standard DriverException with commonly used defaults
    fn driver_error(message: impl Into<String>, status: StatusCode) -> Error {
        Error::from(DriverException::new(
            message.into(),
            status,
            DriverError::GenericError(GenericError::new()),
            "".to_string(),
        ))
    }

    /// Helper to create an invalid argument error
    fn invalid_argument(message: impl Into<String>) -> Error {
        Self::driver_error(message, StatusCode::INVALID_ARGUMENT)
    }

    /// Helper to create an invalid state error
    fn invalid_state(message: impl Into<String>) -> Error {
        Self::driver_error(message, StatusCode::INVALID_STATE)
    }

    /// Helper to create an unknown error
    fn internal_error(message: impl Into<String>) -> Error {
        Self::driver_error(message, StatusCode::GENERIC_ERROR)
    }

    fn with_statement<T>(
        &self,
        handle: StatementHandle,
        f: impl FnOnce(MutexGuard<Statement>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let handle = handle.into();
        let stmt = STMT_HANDLE_MANAGER
            .get_obj(handle)
            .ok_or_else(|| Self::invalid_argument("Statement handle not found"))?;
        let guard = stmt
            .lock()
            .map_err(|_| Self::invalid_state("Statement cannot be locked"))?;
        f(guard)
    }

    pub fn new() -> DatabaseDriverV1Server {
        DatabaseDriverV1Server {}
    }

    pub fn database_set_option(
        &self,
        db_handle: DatabaseHandle,
        key: String,
        value: Setting,
    ) -> thrift::Result<()> {
        let handle = db_handle.into();
        match DB_HANDLE_MANAGER.get_obj(handle) {
            Some(db_ptr) => {
                let mut db = db_ptr.lock().unwrap();
                db.settings.insert(key, value);
                Ok(())
            }
            None => Err(Self::invalid_argument("Database handle not found")),
        }
    }

    fn connection_set_option(
        &self,
        handle: ConnectionHandle,
        key: String,
        value: Setting,
    ) -> thrift::Result<()> {
        let handle = handle.into();
        match CONN_HANDLE_MANAGER.get_obj(handle) {
            Some(conn_ptr) => {
                let mut conn = conn_ptr.lock().unwrap();
                conn.settings.insert(key, value);
                Ok(())
            }
            None => Err(Self::invalid_argument("Connection handle not found")),
        }
    }

    fn statement_set_option(
        &self,
        handle: StatementHandle,
        key: String,
        value: Setting,
    ) -> thrift::Result<()> {
        let handle = handle.into();
        match STMT_HANDLE_MANAGER.get_obj(handle) {
            Some(stmt_ptr) => {
                let mut stmt = stmt_ptr.lock().unwrap();
                stmt.settings.insert(key, value);
                Ok(())
            }
            None => Err(Self::invalid_argument("Statement handle not found")),
        }
    }
}

impl DatabaseDriverSyncHandler for DatabaseDriverV1Server {
    #[instrument(name = "DatabaseDriverV1::database_new", skip(self))]
    fn handle_database_new(&self) -> thrift::Result<DatabaseHandle> {
        let handle = DB_HANDLE_MANAGER.add_handle(Mutex::new(Database::new()));
        Ok(DatabaseHandle::from(handle))
    }

    #[instrument(name = "DatabaseDriverV1::database_set_option_string", skip(self))]
    fn handle_database_set_option_string(
        &self,
        db_handle: DatabaseHandle,
        key: String,
        value: String,
    ) -> thrift::Result<()> {
        self.database_set_option(db_handle, key, Setting::String(value))
    }

    #[instrument(name = "DatabaseDriverV1::database_set_option_bytes", skip(self))]
    fn handle_database_set_option_bytes(
        &self,
        db_handle: DatabaseHandle,
        key: String,
        value: Vec<u8>,
    ) -> thrift::Result<()> {
        self.database_set_option(db_handle, key, Setting::Bytes(value))
    }

    #[instrument(name = "DatabaseDriverV1::database_set_option_int", skip(self))]
    fn handle_database_set_option_int(
        &self,
        db_handle: DatabaseHandle,
        key: String,
        value: i64,
    ) -> thrift::Result<()> {
        self.database_set_option(db_handle, key, Setting::Int(value))
    }

    #[instrument(name = "DatabaseDriverV1::database_set_option_double", skip(self))]
    fn handle_database_set_option_double(
        &self,
        db_handle: DatabaseHandle,
        key: String,
        value: OrderedFloat<f64>,
    ) -> thrift::Result<()> {
        self.database_set_option(db_handle, key, Setting::Double(value.into_inner()))
    }

    #[instrument(name = "DatabaseDriverV1::database_init", skip(self))]
    fn handle_database_init(&self, db_handle: DatabaseHandle) -> thrift::Result<()> {
        let handle = db_handle.into();
        match DB_HANDLE_MANAGER.get_obj(handle) {
            Some(_db_ptr) => Ok(()),
            None => Err(Self::invalid_argument("Database handle not found")),
        }
    }

    #[instrument(name = "DatabaseDriverV1::database_release", skip(self))]
    fn handle_database_release(&self, db_handle: DatabaseHandle) -> thrift::Result<()> {
        match DB_HANDLE_MANAGER.delete_handle(db_handle.into()) {
            true => Ok(()),
            false => Err(Self::invalid_argument("Failed to release database handle")),
        }
    }

    #[instrument(name = "DatabaseDriverV1::connection_new", skip(self))]
    fn handle_connection_new(&self) -> thrift::Result<ConnectionHandle> {
        let handle = CONN_HANDLE_MANAGER.add_handle(Mutex::new(Connection::new()));
        Ok(ConnectionHandle::from(handle))
    }

    #[instrument(name = "DatabaseDriverV1::connection_set_option_string", skip(self))]
    fn handle_connection_set_option_string(
        &self,
        conn_handle: ConnectionHandle,
        key: String,
        value: String,
    ) -> thrift::Result<()> {
        self.connection_set_option(conn_handle, key, Setting::String(value))
    }

    #[instrument(name = "DatabaseDriverV1::connection_set_option_bytes", skip(self))]
    fn handle_connection_set_option_bytes(
        &self,
        conn_handle: ConnectionHandle,
        key: String,
        value: Vec<u8>,
    ) -> thrift::Result<()> {
        self.connection_set_option(conn_handle, key, Setting::Bytes(value))
    }

    #[instrument(name = "DatabaseDriverV1::connection_set_option_int", skip(self))]
    fn handle_connection_set_option_int(
        &self,
        conn_handle: ConnectionHandle,
        key: String,
        value: i64,
    ) -> thrift::Result<()> {
        self.connection_set_option(conn_handle, key, Setting::Int(value))
    }

    #[instrument(name = "DatabaseDriverV1::connection_set_option_double", skip(self))]
    fn handle_connection_set_option_double(
        &self,
        conn_handle: ConnectionHandle,
        key: String,
        value: OrderedFloat<f64>,
    ) -> thrift::Result<()> {
        self.connection_set_option(conn_handle, key, Setting::Double(value.into_inner()))
    }

    #[instrument(name = "DatabaseDriverV1::connection_init", skip(self))]
    fn handle_connection_init(
        &self,
        conn_handle: ConnectionHandle,
        db_handle: DatabaseHandle,
    ) -> thrift::Result<()> {
        connection_init(conn_handle, db_handle).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::connection_release", skip(self))]
    fn handle_connection_release(&self, conn_handle: ConnectionHandle) -> thrift::Result<()> {
        match CONN_HANDLE_MANAGER.delete_handle(conn_handle.into()) {
            true => Ok(()),
            false => Err(Self::invalid_argument(
                "Failed to release connection handle",
            )),
        }
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_info", skip(self))]
    fn handle_connection_get_info(
        &self,
        _conn_handle: ConnectionHandle,
        _info_codes: Vec<InfoCode>,
    ) -> thrift::Result<Vec<u8>> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_objects", skip(self))]
    fn handle_connection_get_objects(
        &self,
        _conn_handle: ConnectionHandle,
        _depth: i32,
        _catalog: String,
        _db_schema: String,
        _table_name: String,
        _table_type: Vec<String>,
        _column_name: String,
    ) -> thrift::Result<Vec<u8>> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_table_schema", skip(self))]
    fn handle_connection_get_table_schema(
        &self,
        _conn_handle: ConnectionHandle,
        _catalog: String,
        _db_schema: String,
        _table_name: String,
    ) -> thrift::Result<Vec<u8>> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_table_types", skip(self))]
    fn handle_connection_get_table_types(
        &self,
        _conn_handle: ConnectionHandle,
    ) -> thrift::Result<Vec<u8>> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::connection_commit", skip(self))]
    fn handle_connection_commit(&self, _conn_handle: ConnectionHandle) -> thrift::Result<()> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::connection_rollback", skip(self))]
    fn handle_connection_rollback(&self, _conn_handle: ConnectionHandle) -> thrift::Result<()> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::statement_new", skip(self))]
    fn handle_statement_new(
        &self,
        conn_handle: ConnectionHandle,
    ) -> thrift::Result<StatementHandle> {
        let handle = conn_handle.into();
        match CONN_HANDLE_MANAGER.get_obj(handle) {
            Some(conn_ptr) => {
                let stmt = Mutex::new(Statement::new(conn_ptr));
                let handle = STMT_HANDLE_MANAGER.add_handle(stmt);
                Ok(handle.into())
            }
            None => Err(Self::invalid_argument("Connection handle not found")),
        }
    }

    #[instrument(name = "DatabaseDriverV1::statement_release", skip(self))]
    fn handle_statement_release(&self, stmt_handle: StatementHandle) -> thrift::Result<()> {
        match STMT_HANDLE_MANAGER.delete_handle(stmt_handle.into()) {
            true => Ok(()),
            false => Err(Self::invalid_argument("Failed to release statement handle")),
        }
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_sql_query", skip(self))]
    fn handle_statement_set_sql_query(
        &self,
        stmt_handle: StatementHandle,
        query: String,
    ) -> thrift::Result<()> {
        let handle = stmt_handle.into();
        match STMT_HANDLE_MANAGER.get_obj(handle) {
            Some(stmt_ptr) => {
                let mut stmt = stmt_ptr.lock().unwrap();
                stmt.query = Some(query);
                Ok(())
            }
            None => Err(Self::invalid_argument("Statement handle not found")),
        }
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_substrait_plan", skip(self))]
    fn handle_statement_set_substrait_plan(
        &self,
        _stmt_handle: StatementHandle,
        _plan: Vec<u8>,
    ) -> thrift::Result<()> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::statement_prepare", skip(self))]
    fn handle_statement_prepare(&self, _stmt_handle: StatementHandle) -> thrift::Result<()> {
        Ok(())
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_option_string", skip(self))]
    fn handle_statement_set_option_string(
        &self,
        stmt_handle: StatementHandle,
        key: String,
        value: String,
    ) -> thrift::Result<()> {
        self.statement_set_option(stmt_handle, key, Setting::String(value))
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_option_bytes", skip(self))]
    fn handle_statement_set_option_bytes(
        &self,
        stmt_handle: StatementHandle,
        key: String,
        value: Vec<u8>,
    ) -> thrift::Result<()> {
        self.statement_set_option(stmt_handle, key, Setting::Bytes(value))
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_option_int", skip(self))]
    fn handle_statement_set_option_int(
        &self,
        stmt_handle: StatementHandle,
        key: String,
        value: i64,
    ) -> thrift::Result<()> {
        self.statement_set_option(stmt_handle, key, Setting::Int(value))
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_option_double", skip(self))]
    fn handle_statement_set_option_double(
        &self,
        stmt_handle: StatementHandle,
        key: String,
        value: OrderedFloat<f64>,
    ) -> thrift::Result<()> {
        self.statement_set_option(stmt_handle, key, Setting::Double(value.into_inner()))
    }

    #[instrument(name = "DatabaseDriverV1::statement_get_parameter_schema", skip(self))]
    fn handle_statement_get_parameter_schema(
        &self,
        _stmt_handle: StatementHandle,
    ) -> thrift::Result<ArrowSchemaPtr> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::statement_bind", skip(self))]
    fn handle_statement_bind(
        &self,
        stmt_handle: StatementHandle,
        schema: ArrowSchemaPtr,
        array: ArrowArrayPtr,
    ) -> thrift::Result<()> {
        let schema = unsafe { FFI_ArrowSchema::from_raw(schema.into()) };
        let array = unsafe { FFI_ArrowArray::from_raw(array.into()) };
        let array = unsafe { arrow::ffi::from_ffi(array, &schema) }
            .map_err(|e| Self::internal_error(format!("Failed to convert ArrowArray: {e}")))?;
        let record_batch = RecordBatch::from(StructArray::from(array));
        self.with_statement(stmt_handle, |mut stmt| {
            stmt.bind_parameters(record_batch).map_err(snafu_to_thrift)
        })
    }

    #[instrument(name = "DatabaseDriverV1::statement_bind_stream", skip(self))]
    fn handle_statement_bind_stream(
        &self,
        _stmt_handle: StatementHandle,
        _stream: Vec<u8>,
    ) -> thrift::Result<()> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::statement_execute_query", skip(self))]
    fn handle_statement_execute_query(
        &self,
        stmt_handle: StatementHandle,
    ) -> thrift::Result<ExecuteResult> {
        let handle = stmt_handle.into();
        let stmt_ptr = STMT_HANDLE_MANAGER
            .get_obj(handle)
            .ok_or_else(|| Self::invalid_argument("Statement handle not found"))?;

        let mut stmt = stmt_ptr.lock().unwrap();
        let query = stmt
            .query
            .take()
            .ok_or_else(|| Self::invalid_argument("Query not found"))?;

        // Run within the async runtime
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Self::internal_error(format!("Failed to create runtime: {e}")))?;

        let (query_parameters, session_token) = {
            let conn = stmt.conn.lock().unwrap();
            (
                QueryParameters::from_settings(&conn.settings).map_err(snafu_to_thrift)?,
                conn.session_token
                    .clone()
                    .ok_or_else(|| Self::invalid_argument("Session token not found"))?,
            )
        };

        let response = rt
            .block_on(snowflake_query(
                query_parameters,
                session_token,
                query,
                stmt.get_query_parameter_bindings()
                    .map_err(snafu_to_thrift)?,
            ))
            .map_err(snafu_to_thrift)?;

        let response_reader = rt
            .block_on(process_query_response(&response.data))
            .map_err(snafu_to_thrift)?;

        let rowset_stream = Box::new(FFI_ArrowArrayStream::new(response_reader));

        // Serialize pointer into integer
        let stream_ptr = Box::into_raw(rowset_stream);
        stmt.state = StatementState::Executed;
        Ok(ExecuteResult::new(Box::new(stream_ptr.into()), 0))
    }

    #[instrument(name = "DatabaseDriverV1::statement_execute_partitions", skip(self))]
    fn handle_statement_execute_partitions(
        &self,
        _stmt_handle: StatementHandle,
    ) -> thrift::Result<PartitionedResult> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::statement_read_partition", skip(self))]
    fn handle_statement_read_partition(
        &self,
        _stmt_handle: StatementHandle,
        _partition_descriptor: Vec<u8>,
    ) -> thrift::Result<i64> {
        todo!()
    }
}

// TODO: Implement a function that prints a SNAFU error with location info for easier debugging
pub fn generate_error_report<E>(error: E) -> String
where
    E: std::error::Error + Send + Sync + 'static,
{
    // Convert the given error into a snafu::Report.
    let report = Report::from_error(error);
    // Use `to_string()` to get the human-readable report string.
    report.to_string()
}

pub fn snafu_to_thrift<E>(error: E) -> Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    Error::from(DriverException::new(
        error.to_string(),
        StatusCode::GENERIC_ERROR,
        DriverError::GenericError(GenericError::new()),
        generate_error_report(error),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_utils::setup_logging, thrift_apis::client::create_client};

    // Database operation tests
    #[test]
    fn test_database_new_and_release() {
        setup_logging();
        let mut client = create_client::<DatabaseDriverV1>();

        let db = client.database_new().unwrap();
        client.database_release(db).unwrap();
    }

    #[test]
    fn test_database_set_option_string() {
        setup_logging();
        let mut client = create_client::<DatabaseDriverV1>();

        let db = client.database_new().unwrap();
        client
            .database_set_option_string(
                db.clone(),
                "test_option".to_string(),
                "test_value".to_string(),
            )
            .unwrap();
        client.database_release(db).unwrap();
    }

    #[test]
    fn test_database_set_option_bytes() {
        setup_logging();
        let mut client = create_client::<DatabaseDriverV1>();

        let db = client.database_new().unwrap();
        let test_bytes = vec![1, 2, 3, 4, 5];
        client
            .database_set_option_bytes(db.clone(), "test_option".to_string(), test_bytes)
            .unwrap();
        client.database_release(db).unwrap();
    }

    #[test]
    fn test_database_set_option_int() {
        setup_logging();
        let mut client = create_client::<DatabaseDriverV1>();

        let db = client.database_new().unwrap();
        client
            .database_set_option_int(db.clone(), "test_option".to_string(), 42)
            .unwrap();
        client.database_release(db).unwrap();
    }

    #[test]
    fn test_database_set_option_double() {
        setup_logging();
        let mut client = create_client::<DatabaseDriverV1>();

        let db = client.database_new().unwrap();
        client
            .database_set_option_double(
                db.clone(),
                "test_option".to_string(),
                std::f64::consts::PI.into(),
            )
            .unwrap();
        client.database_release(db).unwrap();
    }

    #[test]
    fn test_database_init() {
        setup_logging();
        let mut client = create_client::<DatabaseDriverV1>();

        let db = client.database_new().unwrap();
        client.database_init(db.clone()).unwrap();
        client.database_release(db).unwrap();
    }

    #[test]
    fn test_database_lifecycle() {
        setup_logging();
        let mut client = create_client::<DatabaseDriverV1>();

        // Create database
        let db = client.database_new().unwrap();

        // Set various options
        client
            .database_set_option_string(db.clone(), "driver".to_string(), "test_driver".to_string())
            .unwrap();
        client
            .database_set_option_int(db.clone(), "timeout".to_string(), 30)
            .unwrap();

        // Initialize database
        client.database_init(db.clone()).unwrap();

        // Release database
        client.database_release(db).unwrap();
    }

    // Connection operation tests
    #[test]
    fn test_connection_new_and_release() {
        setup_logging();
        let mut client = create_client::<DatabaseDriverV1>();

        let conn = client.connection_new().unwrap();

        client.connection_release(conn).unwrap();
    }

    #[test]
    fn test_connection_set_option_string() {
        setup_logging();
        let mut client = create_client::<DatabaseDriverV1>();

        let conn = client.connection_new().unwrap();
        client
            .connection_set_option_string(
                conn.clone(),
                "username".to_string(),
                "test_user".to_string(),
            )
            .unwrap();
        client.connection_release(conn).unwrap();
    }

    #[test]
    fn test_connection_set_option_bytes() {
        setup_logging();
        let mut client = create_client::<DatabaseDriverV1>();

        let conn = client.connection_new().unwrap();
        let test_bytes = vec![0xDE, 0xAD, 0xBE, 0xEF];
        client
            .connection_set_option_bytes(conn.clone(), "cert".to_string(), test_bytes)
            .unwrap();
        client.connection_release(conn).unwrap();
    }

    #[test]
    fn test_connection_set_option_int() {
        setup_logging();
        let mut client = create_client::<DatabaseDriverV1>();

        let conn = client.connection_new().unwrap();
        client
            .connection_set_option_int(conn.clone(), "port".to_string(), 5432)
            .unwrap();
        client.connection_release(conn).unwrap();
    }

    #[test]
    fn test_connection_set_option_double() {
        setup_logging();
        let mut client = create_client::<DatabaseDriverV1>();

        let conn = client.connection_new().unwrap();
        client
            .connection_set_option_double(conn.clone(), "timeout_seconds".to_string(), 30.5.into())
            .unwrap();
        client.connection_release(conn).unwrap();
    }
}
