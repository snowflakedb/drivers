use crate::apis::database_driver_v1::ApiError;
use crate::apis::database_driver_v1::Handle;
use crate::apis::database_driver_v1::Setting;
use crate::apis::database_driver_v1::error::ConfigError;
use crate::apis::database_driver_v1::error::RestError;
use crate::apis::database_driver_v1::{
    connection_init, connection_new, connection_release, connection_set_option,
};
use crate::apis::database_driver_v1::{
    database_init, database_new, database_release, database_set_option,
};
use crate::apis::database_driver_v1::{
    statement_bind, statement_execute_query, statement_new, statement_prepare, statement_release,
    statement_set_option, statement_set_sql_query,
};
use crate::thrift_apis::ThriftApi;
use snafu::{self, Report};
use thrift::protocol::{TInputProtocol, TOutputProtocol};

use crate::thrift_gen::database_driver_v1::{
    ArrowArrayPtr, ArrowSchemaPtr, AuthenticationError, ConnectionHandle, DatabaseDriverSyncClient,
    DatabaseDriverSyncHandler, DatabaseDriverSyncProcessor, DatabaseHandle, DriverError,
    DriverException, ExecuteResult, GenericError, InfoCode, InternalError, InvalidParameterValue,
    LoginError, MissingParameter, PartitionedResult, StatementHandle, StatusCode,
    TDatabaseDriverSyncClient,
};

use crate::thrift_gen::database_driver_v1::ArrowArrayStreamPtr;
use arrow::ffi::{FFI_ArrowArray, FFI_ArrowSchema};
use arrow::ffi_stream::FFI_ArrowArrayStream;
use std::mem::size_of;
use thrift::server::TProcessor;
use thrift::{Error, OrderedFloat};
use tracing::instrument;

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
        Box::new(DatabaseDriverSyncProcessor::new(DatabaseDriverV1Server {}))
    }
}

fn to_driver_error(error: &ApiError) -> DriverError {
    match error {
        ApiError::GenericError { .. } => DriverError::GenericError(GenericError::new()),
        ApiError::RuntimeCreation { .. } => DriverError::InternalError(InternalError::new()),
        ApiError::Configuration {
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
        ApiError::Configuration {
            source: ConfigError::MissingParameter { parameter, .. },
            ..
        } => DriverError::MissingParameter(MissingParameter::new(parameter.clone())),
        ApiError::InvalidArgument { .. } => DriverError::InternalError(InternalError::new()),
        ApiError::Login {
            source: RestError::LoginError { message, code, .. },
            ..
        } => DriverError::LoginError(LoginError {
            message: message.clone(),
            code: *code,
        }),
        ApiError::Login { source, .. } => {
            DriverError::AuthError(AuthenticationError::new(source.to_string()))
        }
        ApiError::ConnectionLocking { .. } => DriverError::InternalError(InternalError::new()),
        ApiError::StatementLocking { .. } => DriverError::InternalError(InternalError::new()),
        ApiError::DatabaseLocking { .. } => DriverError::InternalError(InternalError::new()),
        ApiError::QueryResponseProcessing { .. } => {
            DriverError::InternalError(InternalError::new())
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
        let driver_error = to_driver_error(&error);
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

trait ToThriftResult<T> {
    fn to_thrift(self) -> thrift::Result<T>;
}

impl<E, V> ToThriftResult<V> for Result<V, E>
where
    E: Into<thrift::Error>,
{
    fn to_thrift(self) -> thrift::Result<V> {
        self.map_err(|e| e.into())
    }
}

impl DatabaseDriverSyncHandler for DatabaseDriverV1Server {
    #[instrument(name = "DatabaseDriverV1::database_new", skip(self))]
    fn handle_database_new(&self) -> thrift::Result<DatabaseHandle> {
        let handle = database_new();
        Ok(DatabaseHandle::from(handle))
    }

    #[instrument(name = "DatabaseDriverV1::database_set_option_string", skip(self))]
    fn handle_database_set_option_string(
        &self,
        db_handle: DatabaseHandle,
        key: String,
        value: String,
    ) -> thrift::Result<()> {
        database_set_option(db_handle.into(), key, Setting::String(value)).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::database_set_option_bytes", skip(self))]
    fn handle_database_set_option_bytes(
        &self,
        db_handle: DatabaseHandle,
        key: String,
        value: Vec<u8>,
    ) -> thrift::Result<()> {
        database_set_option(db_handle.into(), key, Setting::Bytes(value)).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::database_set_option_int", skip(self))]
    fn handle_database_set_option_int(
        &self,
        db_handle: DatabaseHandle,
        key: String,
        value: i64,
    ) -> thrift::Result<()> {
        database_set_option(db_handle.into(), key, Setting::Int(value)).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::database_set_option_double", skip(self))]
    fn handle_database_set_option_double(
        &self,
        db_handle: DatabaseHandle,
        key: String,
        value: OrderedFloat<f64>,
    ) -> thrift::Result<()> {
        database_set_option(db_handle.into(), key, Setting::Double(value.into_inner())).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::database_init", skip(self))]
    fn handle_database_init(&self, db_handle: DatabaseHandle) -> thrift::Result<()> {
        database_init(db_handle.into()).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::database_release", skip(self))]
    fn handle_database_release(&self, db_handle: DatabaseHandle) -> thrift::Result<()> {
        database_release(db_handle.into()).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::connection_new", skip(self))]
    fn handle_connection_new(&self) -> thrift::Result<ConnectionHandle> {
        let handle = connection_new();
        Ok(ConnectionHandle::from(handle))
    }

    #[instrument(name = "DatabaseDriverV1::connection_set_option_string", skip(self))]
    fn handle_connection_set_option_string(
        &self,
        conn_handle: ConnectionHandle,
        key: String,
        value: String,
    ) -> thrift::Result<()> {
        connection_set_option(conn_handle.into(), key, Setting::String(value)).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::connection_set_option_bytes", skip(self))]
    fn handle_connection_set_option_bytes(
        &self,
        conn_handle: ConnectionHandle,
        key: String,
        value: Vec<u8>,
    ) -> thrift::Result<()> {
        connection_set_option(conn_handle.into(), key, Setting::Bytes(value)).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::connection_set_option_int", skip(self))]
    fn handle_connection_set_option_int(
        &self,
        conn_handle: ConnectionHandle,
        key: String,
        value: i64,
    ) -> thrift::Result<()> {
        connection_set_option(conn_handle.into(), key, Setting::Int(value)).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::connection_set_option_double", skip(self))]
    fn handle_connection_set_option_double(
        &self,
        conn_handle: ConnectionHandle,
        key: String,
        value: OrderedFloat<f64>,
    ) -> thrift::Result<()> {
        connection_set_option(conn_handle.into(), key, Setting::Double(value.into_inner()))
            .to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::connection_init", skip(self))]
    fn handle_connection_init(
        &self,
        conn_handle: ConnectionHandle,
        db_handle: DatabaseHandle,
    ) -> thrift::Result<()> {
        connection_init(conn_handle.into(), db_handle.into()).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::connection_release", skip(self))]
    fn handle_connection_release(&self, conn_handle: ConnectionHandle) -> thrift::Result<()> {
        connection_release(conn_handle.into()).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_info", skip(self))]
    fn handle_connection_get_info(
        &self,
        conn_handle: ConnectionHandle,
        info_codes: Vec<InfoCode>,
    ) -> thrift::Result<Vec<u8>> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_objects", skip(self))]
    fn handle_connection_get_objects(
        &self,
        conn_handle: ConnectionHandle,
        depth: i32,
        catalog: String,
        db_schema: String,
        table_name: String,
        table_type: Vec<String>,
        column_name: String,
    ) -> thrift::Result<Vec<u8>> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_table_schema", skip(self))]
    fn handle_connection_get_table_schema(
        &self,
        conn_handle: ConnectionHandle,
        catalog: String,
        db_schema: String,
        table_name: String,
    ) -> thrift::Result<Vec<u8>> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_table_types", skip(self))]
    fn handle_connection_get_table_types(
        &self,
        conn_handle: ConnectionHandle,
    ) -> thrift::Result<Vec<u8>> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::connection_commit", skip(self))]
    fn handle_connection_commit(&self, conn_handle: ConnectionHandle) -> thrift::Result<()> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::connection_rollback", skip(self))]
    fn handle_connection_rollback(&self, conn_handle: ConnectionHandle) -> thrift::Result<()> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::statement_new", skip(self))]
    fn handle_statement_new(
        &self,
        conn_handle: ConnectionHandle,
    ) -> thrift::Result<StatementHandle> {
        let handle =
            statement_new(conn_handle.into()).map_err(|e: ApiError| thrift::Error::from(e))?;
        Ok(handle.into())
    }

    #[instrument(name = "DatabaseDriverV1::statement_release", skip(self))]
    fn handle_statement_release(&self, stmt_handle: StatementHandle) -> thrift::Result<()> {
        statement_release(stmt_handle.into()).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_sql_query", skip(self))]
    fn handle_statement_set_sql_query(
        &self,
        stmt_handle: StatementHandle,
        query: String,
    ) -> thrift::Result<()> {
        statement_set_sql_query(stmt_handle.into(), query).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_substrait_plan", skip(self))]
    fn handle_statement_set_substrait_plan(
        &self,
        stmt_handle: StatementHandle,
        plan: Vec<u8>,
    ) -> thrift::Result<()> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::statement_prepare", skip(self))]
    fn handle_statement_prepare(&self, stmt_handle: StatementHandle) -> thrift::Result<()> {
        statement_prepare(stmt_handle.into()).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_option_string", skip(self))]
    fn handle_statement_set_option_string(
        &self,
        stmt_handle: StatementHandle,
        key: String,
        value: String,
    ) -> thrift::Result<()> {
        statement_set_option(stmt_handle.into(), key, Setting::String(value)).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_option_bytes", skip(self))]
    fn handle_statement_set_option_bytes(
        &self,
        stmt_handle: StatementHandle,
        key: String,
        value: Vec<u8>,
    ) -> thrift::Result<()> {
        statement_set_option(stmt_handle.into(), key, Setting::Bytes(value)).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_option_int", skip(self))]
    fn handle_statement_set_option_int(
        &self,
        stmt_handle: StatementHandle,
        key: String,
        value: i64,
    ) -> thrift::Result<()> {
        statement_set_option(stmt_handle.into(), key, Setting::Int(value)).to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_option_double", skip(self))]
    fn handle_statement_set_option_double(
        &self,
        stmt_handle: StatementHandle,
        key: String,
        value: OrderedFloat<f64>,
    ) -> thrift::Result<()> {
        statement_set_option(stmt_handle.into(), key, Setting::Double(value.into_inner()))
            .to_thrift()
    }

    #[instrument(name = "DatabaseDriverV1::statement_get_parameter_schema", skip(self))]
    fn handle_statement_get_parameter_schema(
        &self,
        stmt_handle: StatementHandle,
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
        unsafe { statement_bind(stmt_handle.into(), schema.into(), array.into()).to_thrift() }
    }

    #[instrument(name = "DatabaseDriverV1::statement_bind_stream", skip(self))]
    fn handle_statement_bind_stream(
        &self,
        stmt_handle: StatementHandle,
        stream: Vec<u8>,
    ) -> thrift::Result<()> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::statement_execute_query", skip(self))]
    fn handle_statement_execute_query(
        &self,
        stmt_handle: StatementHandle,
    ) -> thrift::Result<ExecuteResult> {
        let result = statement_execute_query(stmt_handle.into()).to_thrift()?;
        let stream_ptr: ArrowArrayStreamPtr = Box::into_raw(result.stream).into();
        Ok(ExecuteResult::new(
            Box::new(stream_ptr),
            result.rows_affected,
        ))
    }

    #[instrument(name = "DatabaseDriverV1::statement_execute_partitions", skip(self))]
    fn handle_statement_execute_partitions(
        &self,
        stmt_handle: StatementHandle,
    ) -> thrift::Result<PartitionedResult> {
        todo!()
    }

    #[instrument(name = "DatabaseDriverV1::statement_read_partition", skip(self))]
    fn handle_statement_read_partition(
        &self,
        stmt_handle: StatementHandle,
        partition_descriptor: Vec<u8>,
    ) -> thrift::Result<i64> {
        todo!()
    }
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
