use crate::driver::{Connection, Database, Setting, Statement};
use crate::file_manager::transfer_file;
use crate::handle_manager::{Handle, HandleManager};
use crate::rest::error::RestError;
use crate::thrift_gen::database_driver_v1::{
    ArrowSchemaPtr, ConnectionHandle, DatabaseDriverSyncHandler, DatabaseDriverSyncProcessor,
    DatabaseHandle, DriverException, ExecuteResult, InfoCode, PartitionedResult, StatementHandle,
    StatusCode,
};
use arrow::array::{Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::ffi_stream::FFI_ArrowArrayStream;
use arrow::record_batch::RecordBatch;
use arrow_ipc::reader::StreamReader;
use arrow_ipc::writer::StreamWriter;
use base64::Engine;
use std::mem::size_of;
use std::sync::Mutex;
use thrift::server::TProcessor;
use thrift::{Error, OrderedFloat};

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

impl DatabaseDriverV1 {
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
            None => Err(Error::from(DriverException::new(
                String::from("Database handle not found"),
                StatusCode::INVALID_ARGUMENT,
                None,
                None,
                None,
            ))),
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
            None => Err(Error::from(DriverException::new(
                String::from("Connection handle not found"),
                StatusCode::INVALID_ARGUMENT,
                None,
                None,
                None,
            ))),
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
            None => Err(Error::from(DriverException::new(
                String::from("Statement handle not found"),
                StatusCode::INVALID_ARGUMENT,
                None,
                None,
                None,
            ))),
        }
    }
}

impl DatabaseDriverSyncHandler for DatabaseDriverV1 {
    fn handle_database_new(&self) -> thrift::Result<DatabaseHandle> {
        let handle = self
            .db_handle_manager
            .add_handle(Mutex::new(Database::new()));
        Ok(DatabaseHandle::from(handle))
    }

    fn handle_database_set_option_string(
        &self,
        db_handle: DatabaseHandle,
        key: String,
        value: String,
    ) -> thrift::Result<()> {
        self.database_set_option(db_handle, key, Setting::String(value))
    }

    fn handle_database_set_option_bytes(
        &self,
        db_handle: DatabaseHandle,
        key: String,
        value: Vec<u8>,
    ) -> thrift::Result<()> {
        self.database_set_option(db_handle, key, Setting::Bytes(value))
    }

    fn handle_database_set_option_int(
        &self,
        db_handle: DatabaseHandle,
        key: String,
        value: i64,
    ) -> thrift::Result<()> {
        self.database_set_option(db_handle, key, Setting::Int(value))
    }

    fn handle_database_set_option_double(
        &self,
        db_handle: DatabaseHandle,
        key: String,
        value: OrderedFloat<f64>,
    ) -> thrift::Result<()> {
        self.database_set_option(db_handle, key, Setting::Double(value.into_inner()))
    }

    fn handle_database_init(&self, db_handle: DatabaseHandle) -> thrift::Result<()> {
        let handle = db_handle.into();
        match self.db_handle_manager.get_obj(handle) {
            Some(_db_ptr) => Ok(()),
            None => Err(Error::from(DriverException::new(
                String::from("Database handle not found"),
                StatusCode::INVALID_ARGUMENT,
                None,
                None,
                None,
            ))),
        }
    }

    fn handle_database_release(&self, db_handle: DatabaseHandle) -> thrift::Result<()> {
        match self.db_handle_manager.delete_handle(db_handle.into()) {
            true => Ok(()),
            false => Err(Error::from(DriverException::new(
                String::from("Failed to release database handle"),
                StatusCode::INVALID_ARGUMENT,
                None,
                None,
                None,
            ))),
        }
    }

    fn handle_connection_new(&self) -> thrift::Result<ConnectionHandle> {
        let handle = self
            .conn_handle_manager
            .add_handle(Mutex::new(Connection::new()));
        Ok(ConnectionHandle::from(handle))
    }

    fn handle_connection_set_option_string(
        &self,
        conn_handle: ConnectionHandle,
        key: String,
        value: String,
    ) -> thrift::Result<()> {
        self.connection_set_option(conn_handle, key, Setting::String(value))
    }

    fn handle_connection_set_option_bytes(
        &self,
        conn_handle: ConnectionHandle,
        key: String,
        value: Vec<u8>,
    ) -> thrift::Result<()> {
        self.connection_set_option(conn_handle, key, Setting::Bytes(value))
    }

    fn handle_connection_set_option_int(
        &self,
        conn_handle: ConnectionHandle,
        key: String,
        value: i64,
    ) -> thrift::Result<()> {
        self.connection_set_option(conn_handle, key, Setting::Int(value))
    }

    fn handle_connection_set_option_double(
        &self,
        conn_handle: ConnectionHandle,
        key: String,
        value: OrderedFloat<f64>,
    ) -> thrift::Result<()> {
        self.connection_set_option(conn_handle, key, Setting::Double(value.into_inner()))
    }

    fn handle_connection_init(
        &self,
        conn_handle: ConnectionHandle,
        _db_handle: DatabaseHandle,
    ) -> thrift::Result<()> {
        let handle = conn_handle.into();
        match self.conn_handle_manager.get_obj(handle) {
            Some(conn_ptr) => {
                // Create a blocking runtime for the login process
                let rt = tokio::runtime::Runtime::new().map_err(|e| {
                    Error::from(DriverException::new(
                        format!("Failed to create runtime: {e}"),
                        StatusCode::UNKNOWN,
                        None,
                        None,
                        None,
                    ))
                })?;

                let login_result =
                    rt.block_on(async { crate::rest::snowflake::snowflake_login(&conn_ptr).await });

                match login_result {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e.into()),
                }
            }
            None => Err(Error::from(DriverException::new(
                String::from("Connection handle not found"),
                StatusCode::INVALID_ARGUMENT,
                None,
                None,
                None,
            ))),
        }
    }

    fn handle_connection_release(&self, conn_handle: ConnectionHandle) -> thrift::Result<()> {
        match self.conn_handle_manager.delete_handle(conn_handle.into()) {
            true => Ok(()),
            false => Err(DriverException::new(
                String::from("Failed to release connection handle"),
                StatusCode::INVALID_ARGUMENT,
                None,
                None,
                None,
            )
            .into()),
        }
    }

    fn handle_connection_get_info(
        &self,
        _conn_handle: ConnectionHandle,
        _info_codes: Vec<InfoCode>,
    ) -> thrift::Result<Vec<u8>> {
        todo!()
    }

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

    fn handle_connection_get_table_schema(
        &self,
        _conn_handle: ConnectionHandle,
        _catalog: String,
        _db_schema: String,
        _table_name: String,
    ) -> thrift::Result<Vec<u8>> {
        todo!()
    }

    fn handle_connection_get_table_types(
        &self,
        _conn_handle: ConnectionHandle,
    ) -> thrift::Result<Vec<u8>> {
        todo!()
    }

    fn handle_connection_commit(&self, _conn_handle: ConnectionHandle) -> thrift::Result<()> {
        todo!()
    }

    fn handle_connection_rollback(&self, _conn_handle: ConnectionHandle) -> thrift::Result<()> {
        todo!()
    }

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
            None => Err(Error::from(DriverException::new(
                String::from("Connection handle not found"),
                StatusCode::INVALID_ARGUMENT,
                None,
                None,
                None,
            ))),
        }
    }

    fn handle_statement_release(&self, stmt_handle: StatementHandle) -> thrift::Result<()> {
        match self.stmt_handle_manager.delete_handle(stmt_handle.into()) {
            true => Ok(()),
            false => Err(DriverException::new(
                String::from("Failed to release statement handle"),
                StatusCode::INVALID_ARGUMENT,
                None,
                None,
                None,
            )
            .into()),
        }
    }

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
            None => Err(Error::from(DriverException::new(
                String::from("Statement handle not found"),
                StatusCode::INVALID_ARGUMENT,
                None,
                None,
                None,
            ))),
        }
    }

    fn handle_statement_set_substrait_plan(
        &self,
        _stmt_handle: StatementHandle,
        _plan: Vec<u8>,
    ) -> thrift::Result<()> {
        todo!()
    }

    fn handle_statement_prepare(&self, _stmt_handle: StatementHandle) -> thrift::Result<()> {
        todo!()
    }

    fn handle_statement_set_option_string(
        &self,
        stmt_handle: StatementHandle,
        key: String,
        value: String,
    ) -> thrift::Result<()> {
        self.statement_set_option(stmt_handle, key, Setting::String(value))
    }

    fn handle_statement_set_option_bytes(
        &self,
        stmt_handle: StatementHandle,
        key: String,
        value: Vec<u8>,
    ) -> thrift::Result<()> {
        self.statement_set_option(stmt_handle, key, Setting::Bytes(value))
    }

    fn handle_statement_set_option_int(
        &self,
        stmt_handle: StatementHandle,
        key: String,
        value: i64,
    ) -> thrift::Result<()> {
        self.statement_set_option(stmt_handle, key, Setting::Int(value))
    }

    fn handle_statement_set_option_double(
        &self,
        stmt_handle: StatementHandle,
        key: String,
        value: OrderedFloat<f64>,
    ) -> thrift::Result<()> {
        self.statement_set_option(stmt_handle, key, Setting::Double(value.into_inner()))
    }

    fn handle_statement_get_parameter_schema(
        &self,
        _stmt_handle: StatementHandle,
    ) -> thrift::Result<ArrowSchemaPtr> {
        todo!()
    }

    fn handle_statement_bind(
        &self,
        _stmt_handle: StatementHandle,
        _values: Vec<u8>,
    ) -> thrift::Result<()> {
        todo!()
    }

    fn handle_statement_bind_stream(
        &self,
        _stmt_handle: StatementHandle,
        _stream: Vec<u8>,
    ) -> thrift::Result<()> {
        todo!()
    }

    fn handle_statement_execute_query(
        &self,
        stmt_handle: StatementHandle,
    ) -> thrift::Result<ExecuteResult> {
        let handle = stmt_handle.into();
        match self.stmt_handle_manager.get_obj(handle) {
            Some(stmt_ptr) => {
                use base64::engine::general_purpose;
                let mut stmt = stmt_ptr.lock().unwrap();
                let query = stmt
                    .query
                    .take()
                    .ok_or(RestError::Internal("Query not found".to_string()))?;
                // Run within the async runtime
                let rt = tokio::runtime::Runtime::new().map_err(|e| {
                    Error::from(DriverException::new(
                        format!("Failed to create runtime: {e}"),
                        StatusCode::UNKNOWN,
                        None,
                        None,
                        None,
                    ))
                })?;

                let response =
                    rt.block_on(crate::rest::snowflake::snowflake_query(&stmt.conn, query))?;

                if !response.success {
                    // TODO: Add proper error handling
                    return Err(Error::from(DriverException::new(
                        response
                            .message
                            .unwrap_or_else(|| "Unknown error".to_string()),
                        StatusCode::UNKNOWN,
                        None,
                        None,
                        None,
                    )));
                }

                if let Some(ref command) = response.data.command {
                    if command == "UPLOAD" {
                        let file_transfer_data = response.data.to_file_transfer_data()?;
                        rt.block_on(transfer_file(file_transfer_data))
                            .map_err(|e| {
                                Error::from(RestError::Internal(format!(
                                    "Failed to transfer file: {e}"
                                )))
                            })?;
                        stmt.state = StatementState::Executed;
                        return Ok(ExecuteResult::new(
                            Box::new(ArrowArrayStreamPtr::new(Vec::new())),
                            0,
                        ));
                    } else if command == "DOWNLOAD" {
                        return Err(Error::from(RestError::Internal(
                            "Handling GET queries is not yet implemented".to_string(),
                        )));
                    }
                }

                let rowset_bytes = match response.data.rowset_base64 {
                    Some(rowset_base64) => {
                        // Decode the base64 string to bytes
                        general_purpose::STANDARD
                            .decode(rowset_base64)
                            .map_err(|e| {
                                Error::from(RestError::InvalidSnowflakeResponse(format!(
                                    "Failed to decode base64 rowset: {e}"
                                )))
                            })?
                    }
                    None => {
                        match response.data.rowset {
                            Some(rowset) => {
                                // Simple conversion assuming single row with string values
                                convert_single_row_to_arrow(rowset)?
                            }
                            None => {
                                return Err(Error::from(RestError::InvalidSnowflakeResponse(
                                    "Rowset not found in response".to_string(),
                                )));
                            }
                        }
                    }
                };

                let cursor = std::io::Cursor::new(rowset_bytes);
                // Parse rowset from arrow ipc format
                let reader = Box::new(StreamReader::try_new(cursor, None).map_err(|e| {
                    RestError::Internal(format!("Failed to create stream reader: {e}"))
                })?);
                let stream = Box::new(arrow::ffi_stream::FFI_ArrowArrayStream::new(reader));
                // Serialize pointer into integer
                let stream_ptr = Box::into_raw(stream);
                stmt.state = StatementState::Executed;
                Ok(ExecuteResult::new(Box::new(stream_ptr.into()), 0))
            }
            None => Err(Error::from(DriverException::new(
                String::from("Statement handle not found"),
                StatusCode::INVALID_ARGUMENT,
                None,
                None,
                None,
            ))),
        }
    }

    fn handle_statement_execute_partitions(
        &self,
        _stmt_handle: StatementHandle,
    ) -> thrift::Result<PartitionedResult> {
        todo!()
    }

    fn handle_statement_read_partition(
        &self,
        _stmt_handle: StatementHandle,
        _partition_descriptor: Vec<u8>,
    ) -> thrift::Result<i64> {
        todo!()
    }
}

fn convert_single_row_to_arrow(rowset: Vec<Vec<Option<String>>>) -> Result<Vec<u8>, RestError> {
    if rowset.is_empty() {
        return Ok(Vec::new());
    }

    // Assumption: single row with string values
    let row = &rowset[0];
    let num_columns = row.len();

    // Create simple string schema with generic column names
    let fields: Vec<Field> = (0..num_columns)
        .map(|i| Field::new(format!("column_{i}"), DataType::Utf8, true))
        .collect();
    let schema = Schema::new(fields);

    // Create string arrays from the single row
    let arrow_arrays: Vec<std::sync::Arc<dyn Array>> = (0..num_columns)
        .map(|col_idx| {
            let values = vec![row[col_idx].clone()]; // Single value per column
            std::sync::Arc::new(StringArray::from(values)) as std::sync::Arc<dyn Array>
        })
        .collect();

    // Create RecordBatch
    let batch = RecordBatch::try_new(std::sync::Arc::new(schema), arrow_arrays).map_err(|e| {
        RestError::Internal(format!("Failed to create RecordBatch from rowset: {e}"))
    })?;

    // Serialize to Arrow IPC format
    let mut bytes = Vec::new();
    let mut writer = StreamWriter::try_new(&mut bytes, &batch.schema())
        .map_err(|e| RestError::Internal(format!("Failed to create Arrow StreamWriter: {e}")))?;

    writer
        .write(&batch)
        .map_err(|e| RestError::Internal(format!("Failed to write Arrow batch: {e}")))?;

    writer
        .finish()
        .map_err(|e| RestError::Internal(format!("Failed to finish Arrow writing: {e}")))?;

    Ok(bytes)
}
