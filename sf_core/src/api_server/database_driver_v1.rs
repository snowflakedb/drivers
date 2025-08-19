use crate::api_server::query::process_query_response;
use crate::config::ConfigError;
use crate::config::rest_parameters::{LoginParameters, QueryParameters};
use crate::config::settings::Setting;
use crate::driver::{Connection, Database, Statement, StatementError};
use crate::handle_manager::{Handle, HandleManager};
use crate::rest::error::RestError;

use crate::thrift_gen::database_driver_v1::{
    ArrowArrayPtr, ArrowSchemaPtr, ConnectionHandle, DatabaseDriverSyncHandler,
    DatabaseDriverSyncProcessor, DatabaseHandle, DriverException, ExecuteResult, InfoCode,
    PartitionedResult, StatementHandle, StatusCode,
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

impl From<RestError> for thrift::Error {
    fn from(error: RestError) -> thrift::Error {
        thrift::Error::from(DriverException::new(
            error.to_string(),
            StatusCode::INVALID_STATE,
            None,
            None,
            None,
        ))
    }
}

impl From<ConfigError> for Error {
    fn from(error: ConfigError) -> Self {
        Error::from(DriverException::new(
            format!("Configuration error: {error:?}"),
            StatusCode::INVALID_STATE,
            None,
            None,
            None,
        ))
    }
}

pub struct DatabaseDriverV1 {
    db_handle_manager: HandleManager<Mutex<Database>>,
    conn_handle_manager: HandleManager<Mutex<Connection>>,
    stmt_handle_manager: HandleManager<Mutex<Statement>>,
}

impl Default for DatabaseDriverV1 {
    fn default() -> Self {
        Self::new()
    }
}

impl From<StatementError> for Error {
    fn from(error: StatementError) -> Self {
        Error::from(DriverException::new(
            format!("{error:?}"),
            StatusCode::INVALID_STATE,
            None,
            None,
            None,
        ))
    }
}

impl DatabaseDriverV1 {
    /// Helper to create a standard DriverException with commonly used defaults
    fn driver_error(message: impl Into<String>, status: StatusCode) -> Error {
        Error::from(DriverException::new(
            message.into(),
            status,
            None,
            None,
            None,
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
    fn unknown_error(message: impl Into<String>) -> Error {
        Self::driver_error(message, StatusCode::UNKNOWN)
    }

    fn with_statement<T>(
        &self,
        handle: StatementHandle,
        f: impl FnOnce(MutexGuard<Statement>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let handle = handle.into();
        let stmt = self
            .stmt_handle_manager
            .get_obj(handle)
            .ok_or_else(|| Self::invalid_argument("Statement handle not found"))?;
        let guard = stmt
            .lock()
            .map_err(|_| Self::invalid_state("Statement cannot be locked"))?;
        f(guard)
    }

    pub fn new() -> DatabaseDriverV1 {
        DatabaseDriverV1 {
            db_handle_manager: HandleManager::new(),
            conn_handle_manager: HandleManager::new(),
            stmt_handle_manager: HandleManager::new(),
        }
    }

    pub fn processor() -> Box<dyn TProcessor + Send + Sync> {
        Box::new(DatabaseDriverSyncProcessor::new(DatabaseDriverV1::new()))
    }

    pub fn database_set_option(
        &self,
        db_handle: DatabaseHandle,
        key: String,
        value: Setting,
    ) -> thrift::Result<()> {
        let handle = db_handle.into();
        match self.db_handle_manager.get_obj(handle) {
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
        match self.conn_handle_manager.get_obj(handle) {
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
        match self.stmt_handle_manager.get_obj(handle) {
            Some(stmt_ptr) => {
                let mut stmt = stmt_ptr.lock().unwrap();
                stmt.settings.insert(key, value);
                Ok(())
            }
            None => Err(Self::invalid_argument("Statement handle not found")),
        }
    }
}

impl DatabaseDriverSyncHandler for DatabaseDriverV1 {
    #[instrument(name = "DatabaseDriverV1::database_new", skip(self))]
    fn handle_database_new(&self) -> thrift::Result<DatabaseHandle> {
        let handle = self
            .db_handle_manager
            .add_handle(Mutex::new(Database::new()));
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
        match self.db_handle_manager.get_obj(handle) {
            Some(_db_ptr) => Ok(()),
            None => Err(Self::invalid_argument("Database handle not found")),
        }
    }

    #[instrument(name = "DatabaseDriverV1::database_release", skip(self))]
    fn handle_database_release(&self, db_handle: DatabaseHandle) -> thrift::Result<()> {
        match self.db_handle_manager.delete_handle(db_handle.into()) {
            true => Ok(()),
            false => Err(Self::invalid_argument("Failed to release database handle")),
        }
    }

    #[instrument(name = "DatabaseDriverV1::connection_new", skip(self))]
    fn handle_connection_new(&self) -> thrift::Result<ConnectionHandle> {
        let handle = self
            .conn_handle_manager
            .add_handle(Mutex::new(Connection::new()));
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
        _db_handle: DatabaseHandle,
    ) -> thrift::Result<()> {
        let handle = conn_handle.into();
        match self.conn_handle_manager.get_obj(handle) {
            Some(conn_ptr) => {
                // Create a blocking runtime for the login process
                let rt = tokio::runtime::Runtime::new()
                    .map_err(|e| Self::unknown_error(format!("Failed to create runtime: {e}")))?;

                let login_parameters =
                    LoginParameters::from_settings(&conn_ptr.lock().unwrap().settings)?;

                let login_result = rt.block_on(async {
                    crate::rest::snowflake::snowflake_login(&login_parameters).await
                });

                match login_result {
                    Ok(session_token) => {
                        conn_ptr.lock().unwrap().session_token = Some(session_token);
                        Ok(())
                    }
                    Err(e) => Err(e.into()),
                }
            }
            None => Err(Self::invalid_argument("Connection handle not found")),
        }
    }

    #[instrument(name = "DatabaseDriverV1::connection_release", skip(self))]
    fn handle_connection_release(&self, conn_handle: ConnectionHandle) -> thrift::Result<()> {
        match self.conn_handle_manager.delete_handle(conn_handle.into()) {
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
        match self.conn_handle_manager.get_obj(handle) {
            Some(conn_ptr) => {
                let stmt = Mutex::new(Statement::new(conn_ptr));
                let handle = self.stmt_handle_manager.add_handle(stmt);
                Ok(handle.into())
            }
            None => Err(Self::invalid_argument("Connection handle not found")),
        }
    }

    #[instrument(name = "DatabaseDriverV1::statement_release", skip(self))]
    fn handle_statement_release(&self, stmt_handle: StatementHandle) -> thrift::Result<()> {
        match self.stmt_handle_manager.delete_handle(stmt_handle.into()) {
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
        match self.stmt_handle_manager.get_obj(handle) {
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
            .map_err(|e| Self::unknown_error(format!("Failed to convert ArrowArray: {e}")))?;
        let record_batch = RecordBatch::from(StructArray::from(array));
        self.with_statement(stmt_handle, |mut stmt| {
            stmt.bind_parameters(record_batch).map_err(Error::from)
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
        let stmt_ptr = self
            .stmt_handle_manager
            .get_obj(handle)
            .ok_or_else(|| Self::invalid_argument("Statement handle not found"))?;

        let mut stmt = stmt_ptr.lock().unwrap();
        let query = stmt
            .query
            .take()
            .ok_or(RestError::Internal("Query not found".to_string()))?;

        // Run within the async runtime
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Self::unknown_error(format!("Failed to create runtime: {e}")))?;

        let (query_parameters, session_token) = {
            let conn = stmt.conn.lock().unwrap();
            (
                QueryParameters::from_settings(&conn.settings)?,
                conn.session_token
                    .clone()
                    .ok_or(RestError::Internal("Session token not found".to_string()))?,
            )
        };

        let response = rt.block_on(crate::rest::snowflake::snowflake_query(
            query_parameters,
            session_token,
            query,
            stmt.get_query_parameter_bindings().map_err(Error::from)?,
        ))?;

        if !response.success {
            // TODO: Add proper error handling
            return Err(Self::unknown_error(
                response
                    .message
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        let response_reader = rt
            .block_on(process_query_response(&response.data))
            .map_err(|e| Self::unknown_error(format!("Failed to process query response: {e}")))?;

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
