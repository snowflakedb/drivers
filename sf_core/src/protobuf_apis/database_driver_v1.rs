use crate::apis::database_driver_v1::ApiError;
use crate::apis::database_driver_v1::ConnectionInfo;
use crate::apis::database_driver_v1::Handle;
use crate::apis::database_driver_v1::Setting;
use crate::apis::database_driver_v1::error::ConfigError;
use crate::apis::database_driver_v1::error::RestError;
use crate::apis::database_driver_v1::statement_bind;
use crate::apis::database_driver_v1::{BindingType, DataPtr};
use crate::apis::database_driver_v1::{
    connection_get_info, connection_get_parameter, connection_init, connection_new,
    connection_release, connection_set_option, connection_set_session_parameters,
};
use crate::apis::database_driver_v1::{
    database_init, database_new, database_release, database_set_option,
};
use crate::apis::database_driver_v1::{
    statement_execute_query, statement_new, statement_prepare, statement_release,
    statement_set_option, statement_set_sql_query,
};
use crate::config::config_manager;
use crate::protobuf_gen::database_driver_v1::*;
use arrow::ffi::FFI_ArrowArray;
use arrow::ffi::FFI_ArrowSchema;
use arrow::ffi_stream::FFI_ArrowArrayStream;
use error_trace::ErrorTrace;
use tracing::instrument;

impl From<ArrowArrayStreamPtr> for *mut FFI_ArrowArrayStream {
    fn from(ptr: ArrowArrayStreamPtr) -> Self {
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

impl From<*mut FFI_ArrowArrayStream> for ArrowArrayStreamPtr {
    fn from(raw: *mut FFI_ArrowArrayStream) -> Self {
        let len = size_of::<*mut FFI_ArrowArrayStream>();
        let buf_ptr = std::ptr::addr_of!(raw) as *const u8;
        let slice = unsafe { std::slice::from_raw_parts(buf_ptr, len) };
        let vec = slice.to_vec();
        ArrowArrayStreamPtr { value: vec }
    }
}

// Convert protobuf BinaryDataPtr to internal DataPtr.
// Both represent a raw pointer + length; this avoids leaking protobuf types into core.
impl<'a> From<BinaryDataPtr> for DataPtr<'a> {
    fn from(proto_ptr: BinaryDataPtr) -> Self {
        let ptr_bytes: [u8; 8] = proto_ptr
            .value
            .as_slice()
            .try_into()
            .expect("Pointer must be 8 bytes");
        let ptr_value = usize::from_le_bytes(ptr_bytes);
        let ptr = ptr_value as *const u8;
        DataPtr::new(ptr, proto_ptr.length)
    }
}

// Convert protobuf QueryBindings variant to internal BindingType.
impl<'a> From<query_bindings::BindingType> for BindingType<'a> {
    fn from(proto: query_bindings::BindingType) -> Self {
        match proto {
            query_bindings::BindingType::Json(ptr) => BindingType::Json(ptr.into()),
            query_bindings::BindingType::Csv(ptr) => BindingType::Csv(ptr.into()),
        }
    }
}

// Handle conversions from protobuf types to internal Handle type
impl From<DatabaseHandle> for Handle {
    fn from(handle: DatabaseHandle) -> Self {
        Handle {
            id: handle.id as u64,
            magic: handle.magic as u64,
        }
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

impl From<ConnectionHandle> for Handle {
    fn from(handle: ConnectionHandle) -> Self {
        Handle {
            id: handle.id as u64,
            magic: handle.magic as u64,
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

impl From<StatementHandle> for Handle {
    fn from(handle: StatementHandle) -> Self {
        Handle {
            id: handle.id as u64,
            magic: handle.magic as u64,
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

impl From<ConnectionInfo> for ConnectionGetInfoResponse {
    fn from(info: ConnectionInfo) -> Self {
        ConnectionGetInfoResponse {
            host: info.host,
            port: info.port,
            server_url: info.server_url,
            session_token: info.session_token,
            session_id: info.session_id,
        }
    }
}

// Convert ApiError to DriverException
fn to_driver_error(error: &ApiError) -> DriverError {
    match error {
        ApiError::GenericError { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::GenericError(GenericError {})),
        },
        ApiError::RuntimeCreation { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::Configuration {
            source:
                ConfigError::InvalidParameterValue {
                    parameter,
                    value,
                    explanation,
                    ..
                },
            ..
        } => DriverError {
            error_type: Some(driver_error::ErrorType::InvalidParameterValue(
                InvalidParameterValue {
                    parameter: parameter.clone(),
                    value: value.clone(),
                    explanation: Some(explanation.clone()),
                },
            )),
        },
        ApiError::Configuration {
            source: ConfigError::MissingParameter { parameter, .. },
            ..
        } => DriverError {
            error_type: Some(driver_error::ErrorType::MissingParameter(
                MissingParameter {
                    parameter: parameter.clone(),
                },
            )),
        },
        ApiError::Configuration {
            source: ConfigError::ConflictingParameters { explanation, .. },
            ..
        } => DriverError {
            error_type: Some(driver_error::ErrorType::InvalidParameterValue(
                InvalidParameterValue {
                    parameter: "private_key/private_key_file".to_string(),
                    value: "(both set)".to_string(),
                    explanation: Some(explanation.clone()),
                },
            )),
        },
        ApiError::InvalidArgument { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::Login {
            source: RestError::LoginError { message, code, .. },
            ..
        } => DriverError {
            error_type: Some(driver_error::ErrorType::LoginError(LoginError {
                message: message.clone(),
                code: *code,
            })),
        },
        ApiError::Login { source, .. } => DriverError {
            error_type: Some(driver_error::ErrorType::AuthError(AuthenticationError {
                detail: source.to_string(),
            })),
        },
        ApiError::ConnectionLocking { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::StatementLocking { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::DatabaseLocking { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::QueryResponseProcessing { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::ConnectionNotInitialized { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::TlsClientCreation { source, .. } => DriverError {
            error_type: Some(driver_error::ErrorType::AuthError(AuthenticationError {
                detail: source.to_string(),
            })),
        },
        ApiError::SessionRefresh { source, .. } => DriverError {
            error_type: Some(driver_error::ErrorType::AuthError(AuthenticationError {
                detail: source.to_string(),
            })),
        },
        ApiError::Statement { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::Query { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::MasterTokenExpired { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::AuthError(AuthenticationError {
                detail: "Master token expired, full re-authentication required".to_string(),
            })),
        },
        ApiError::InvalidRefreshState { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::Configuration {
            source: ConfigError::ConfigFileRead { .. },
            ..
        }
        | ApiError::Configuration {
            source: ConfigError::TomlParse { .. },
            ..
        }
        | ApiError::Configuration {
            source: ConfigError::InsecurePermissions { .. },
            ..
        }
        | ApiError::Configuration {
            source: ConfigError::ConfigDirNotFound { .. },
            ..
        } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::Configuration {
            source: ConfigError::ConnectionNotFound { name, .. },
            ..
        } => DriverError {
            error_type: Some(driver_error::ErrorType::MissingParameter(
                MissingParameter {
                    parameter: format!("connection: {}", name),
                },
            )),
        },
    }
}

fn to_driver_exception(error: ApiError) -> DriverException {
    let status_code = match &error {
        ApiError::GenericError { .. } => StatusCode::GenericError,
        ApiError::RuntimeCreation { .. } => StatusCode::InternalError,
        ApiError::Configuration {
            source: ConfigError::InvalidParameterValue { .. },
            ..
        } => StatusCode::InvalidParameterValue,
        ApiError::Configuration {
            source: ConfigError::MissingParameter { .. },
            ..
        } => StatusCode::MissingParameter,
        ApiError::Configuration {
            source: ConfigError::ConflictingParameters { .. },
            ..
        } => StatusCode::InvalidParameterValue,
        ApiError::Configuration {
            source: ConfigError::ConfigFileRead { .. },
            ..
        } => StatusCode::InternalError,
        ApiError::Configuration {
            source: ConfigError::TomlParse { .. },
            ..
        } => StatusCode::InternalError,
        ApiError::Configuration {
            source: ConfigError::InsecurePermissions { .. },
            ..
        } => StatusCode::InternalError,
        ApiError::Configuration {
            source: ConfigError::ConfigDirNotFound { .. },
            ..
        } => StatusCode::InternalError,
        ApiError::Configuration {
            source: ConfigError::ConnectionNotFound { .. },
            ..
        } => StatusCode::MissingParameter,
        ApiError::InvalidArgument { .. } => StatusCode::InvalidArgument,
        ApiError::Login {
            source: RestError::LoginError { .. },
            ..
        } => StatusCode::LoginError,
        ApiError::Login { .. } => StatusCode::AuthenticationError,
        ApiError::ConnectionLocking { .. } => StatusCode::InternalError,
        ApiError::StatementLocking { .. } => StatusCode::InternalError,
        ApiError::DatabaseLocking { .. } => StatusCode::InternalError,
        ApiError::QueryResponseProcessing { .. } => StatusCode::InternalError,
        ApiError::ConnectionNotInitialized { .. } => StatusCode::InternalError,
        ApiError::TlsClientCreation { .. } => StatusCode::AuthenticationError,
        ApiError::SessionRefresh { .. } => StatusCode::AuthenticationError,
        ApiError::Statement { .. } => StatusCode::InternalError,
        ApiError::Query { .. } => StatusCode::InternalError,
        ApiError::MasterTokenExpired { .. } => StatusCode::AuthenticationError,
        ApiError::InvalidRefreshState { .. } => StatusCode::InternalError,
    };

    let message = error.to_string();
    let driver_error = to_driver_error(&error);
    let error_trace = error
        .error_trace()
        .into_iter()
        .map(|entry| ErrorTraceEntry {
            file: entry.location.file,
            line: entry.location.line,
            column: entry.location.column,
            message: entry.message,
        })
        .collect();
    DriverException {
        message,
        status_code: status_code as i32,
        error: Some(driver_error),
        error_trace,
    }
}

#[allow(clippy::result_large_err)]
fn required<T>(value: Option<T>, message: &str) -> Result<T, DriverException> {
    value.ok_or_else(|| DriverException {
        message: message.to_string(),
        status_code: StatusCode::InvalidArgument as i32,
        error: None,
        error_trace: vec![],
    })
}

fn not_implemented(message: &str) -> DriverException {
    DriverException {
        message: message.to_string(),
        status_code: StatusCode::NotImplemented as i32,
        error: None,
        error_trace: vec![],
    }
}

// Trait for converting ApiError results to protobuf results
trait ToProtobuf<T> {
    #[allow(clippy::result_large_err)]
    fn to_protobuf(self) -> Result<T, DriverException>;
}

impl<T> ToProtobuf<T> for Result<T, ApiError> {
    #[allow(clippy::result_large_err)]
    fn to_protobuf(self) -> Result<T, DriverException> {
        self.map_err(to_driver_exception)
    }
}

pub struct DatabaseDriverImpl {}

impl DatabaseDriver for DatabaseDriverImpl {
    #[instrument(name = "DatabaseDriverV1::database_new", skip(_input))]
    fn database_new(_input: DatabaseNewRequest) -> Result<DatabaseNewResponse, DriverException> {
        let handle = database_new();
        Ok(DatabaseNewResponse {
            db_handle: Some(DatabaseHandle::from(handle)),
        })
    }

    #[instrument(name = "DatabaseDriverV1::database_set_option_string", skip(input))]
    fn database_set_option_string(
        input: DatabaseSetOptionStringRequest,
    ) -> Result<DatabaseSetOptionStringResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;

        database_set_option(db_handle.into(), input.key, Setting::String(input.value))
            .to_protobuf()?;

        Ok(DatabaseSetOptionStringResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::database_set_option_bytes", skip(input))]
    fn database_set_option_bytes(
        input: DatabaseSetOptionBytesRequest,
    ) -> Result<DatabaseSetOptionBytesResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;

        database_set_option(db_handle.into(), input.key, Setting::Bytes(input.value))
            .to_protobuf()?;

        Ok(DatabaseSetOptionBytesResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::database_set_option_int", skip(input))]
    fn database_set_option_int(
        input: DatabaseSetOptionIntRequest,
    ) -> Result<DatabaseSetOptionIntResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;

        database_set_option(db_handle.into(), input.key, Setting::Int(input.value))
            .to_protobuf()?;

        Ok(DatabaseSetOptionIntResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::database_set_option_double", skip(input))]
    fn database_set_option_double(
        input: DatabaseSetOptionDoubleRequest,
    ) -> Result<DatabaseSetOptionDoubleResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;

        database_set_option(db_handle.into(), input.key, Setting::Double(input.value))
            .to_protobuf()?;

        Ok(DatabaseSetOptionDoubleResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::database_init", skip(input))]
    fn database_init(input: DatabaseInitRequest) -> Result<DatabaseInitResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;

        database_init(db_handle.into()).to_protobuf()?;
        Ok(DatabaseInitResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::database_release", skip(input))]
    fn database_release(
        input: DatabaseReleaseRequest,
    ) -> Result<DatabaseReleaseResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;

        database_release(db_handle.into()).to_protobuf()?;
        Ok(DatabaseReleaseResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::connection_new", skip(_input))]
    fn connection_new(
        _input: ConnectionNewRequest,
    ) -> Result<ConnectionNewResponse, DriverException> {
        let handle = connection_new();
        Ok(ConnectionNewResponse {
            conn_handle: Some(ConnectionHandle::from(handle)),
        })
    }

    #[instrument(name = "DatabaseDriverV1::connection_set_option_string", skip(input))]
    fn connection_set_option_string(
        input: ConnectionSetOptionStringRequest,
    ) -> Result<ConnectionSetOptionStringResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        connection_set_option(conn_handle.into(), input.key, Setting::String(input.value))
            .to_protobuf()?;

        Ok(ConnectionSetOptionStringResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::connection_set_option_bytes", skip(input))]
    fn connection_set_option_bytes(
        input: ConnectionSetOptionBytesRequest,
    ) -> Result<ConnectionSetOptionBytesResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        connection_set_option(conn_handle.into(), input.key, Setting::Bytes(input.value))
            .to_protobuf()?;

        Ok(ConnectionSetOptionBytesResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::connection_set_option_int", skip(input))]
    fn connection_set_option_int(
        input: ConnectionSetOptionIntRequest,
    ) -> Result<ConnectionSetOptionIntResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        connection_set_option(conn_handle.into(), input.key, Setting::Int(input.value))
            .to_protobuf()?;

        Ok(ConnectionSetOptionIntResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::connection_set_option_double", skip(input))]
    fn connection_set_option_double(
        input: ConnectionSetOptionDoubleRequest,
    ) -> Result<ConnectionSetOptionDoubleResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        connection_set_option(conn_handle.into(), input.key, Setting::Double(input.value))
            .to_protobuf()?;

        Ok(ConnectionSetOptionDoubleResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::connection_init", skip(input))]
    fn connection_init(
        input: ConnectionInitRequest,
    ) -> Result<ConnectionInitResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let db_handle = required(input.db_handle, "Database handle is required")?;

        connection_init(conn_handle.into(), db_handle.into()).to_protobuf()?;
        Ok(ConnectionInitResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::connection_release", skip(input))]
    fn connection_release(
        input: ConnectionReleaseRequest,
    ) -> Result<ConnectionReleaseResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        connection_release(conn_handle.into()).to_protobuf()?;
        Ok(ConnectionReleaseResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_info", skip(input))]
    fn connection_get_info(
        input: ConnectionGetInfoRequest,
    ) -> Result<ConnectionGetInfoResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let info = connection_get_info(conn_handle.into()).to_protobuf()?;

        Ok(ConnectionGetInfoResponse::from(info))
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_objects", skip(_input))]
    fn connection_get_objects(
        _input: ConnectionGetObjectsRequest,
    ) -> Result<ConnectionGetObjectsResponse, DriverException> {
        Err(not_implemented(
            "connection_get_objects is not yet implemented",
        ))
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_table_schema", skip(_input))]
    fn connection_get_table_schema(
        _input: ConnectionGetTableSchemaRequest,
    ) -> Result<ConnectionGetTableSchemaResponse, DriverException> {
        Err(not_implemented(
            "connection_get_table_schema is not yet implemented",
        ))
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_table_types", skip(_input))]
    fn connection_get_table_types(
        _input: ConnectionGetTableTypesRequest,
    ) -> Result<ConnectionGetTableTypesResponse, DriverException> {
        Err(not_implemented(
            "connection_get_table_types is not yet implemented",
        ))
    }

    #[instrument(name = "DatabaseDriverV1::connection_commit", skip(_input))]
    fn connection_commit(
        _input: ConnectionCommitRequest,
    ) -> Result<ConnectionCommitResponse, DriverException> {
        Err(not_implemented("connection_commit is not yet implemented"))
    }

    #[instrument(name = "DatabaseDriverV1::connection_rollback", skip(_input))]
    fn connection_rollback(
        _input: ConnectionRollbackRequest,
    ) -> Result<ConnectionRollbackResponse, DriverException> {
        Err(not_implemented(
            "connection_rollback is not yet implemented",
        ))
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_set_session_parameters",
        skip(input)
    )]
    fn connection_set_session_parameters(
        input: ConnectionSetSessionParametersRequest,
    ) -> Result<ConnectionSetSessionParametersResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        connection_set_session_parameters(conn_handle.into(), input.parameters).to_protobuf()?;

        Ok(ConnectionSetSessionParametersResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_parameter", skip(input))]
    fn connection_get_parameter(
        input: ConnectionGetParameterRequest,
    ) -> Result<ConnectionGetParameterResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let value = connection_get_parameter(conn_handle.into(), input.key).to_protobuf()?;

        Ok(ConnectionGetParameterResponse { value })
    }

    #[instrument(name = "DatabaseDriverV1::statement_new", skip(input))]
    fn statement_new(input: StatementNewRequest) -> Result<StatementNewResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let handle = statement_new(conn_handle.into()).to_protobuf()?;
        Ok(StatementNewResponse {
            stmt_handle: Some(StatementHandle::from(handle)),
        })
    }

    #[instrument(name = "DatabaseDriverV1::statement_release", skip(input))]
    fn statement_release(
        input: StatementReleaseRequest,
    ) -> Result<StatementReleaseResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        statement_release(stmt_handle.into()).to_protobuf()?;
        Ok(StatementReleaseResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_sql_query", skip(input))]
    fn statement_set_sql_query(
        input: StatementSetSqlQueryRequest,
    ) -> Result<StatementSetSqlQueryResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        statement_set_sql_query(stmt_handle.into(), input.query).to_protobuf()?;
        Ok(StatementSetSqlQueryResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_substrait_plan", skip(_input))]
    fn statement_set_substrait_plan(
        _input: StatementSetSubstraitPlanRequest,
    ) -> Result<StatementSetSubstraitPlanResponse, DriverException> {
        // TODO: Implement when corresponding API method is available
        Err(not_implemented(
            "statement_set_substrait_plan is not yet implemented",
        ))
    }

    #[instrument(name = "DatabaseDriverV1::statement_prepare", skip(input))]
    fn statement_prepare(
        input: StatementPrepareRequest,
    ) -> Result<StatementPrepareResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        statement_prepare(stmt_handle.into()).to_protobuf()?;
        Ok(StatementPrepareResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_option_string", skip(input))]
    fn statement_set_option_string(
        input: StatementSetOptionStringRequest,
    ) -> Result<StatementSetOptionStringResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        statement_set_option(stmt_handle.into(), input.key, Setting::String(input.value))
            .to_protobuf()?;

        Ok(StatementSetOptionStringResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_option_bytes", skip(input))]
    fn statement_set_option_bytes(
        input: StatementSetOptionBytesRequest,
    ) -> Result<StatementSetOptionBytesResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        statement_set_option(stmt_handle.into(), input.key, Setting::Bytes(input.value))
            .to_protobuf()?;

        Ok(StatementSetOptionBytesResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_option_int", skip(input))]
    fn statement_set_option_int(
        input: StatementSetOptionIntRequest,
    ) -> Result<StatementSetOptionIntResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        statement_set_option(stmt_handle.into(), input.key, Setting::Int(input.value))
            .to_protobuf()?;

        Ok(StatementSetOptionIntResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_option_double", skip(input))]
    fn statement_set_option_double(
        input: StatementSetOptionDoubleRequest,
    ) -> Result<StatementSetOptionDoubleResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        statement_set_option(stmt_handle.into(), input.key, Setting::Double(input.value))
            .to_protobuf()?;

        Ok(StatementSetOptionDoubleResponse {})
    }

    #[instrument(
        name = "DatabaseDriverV1::statement_get_parameter_schema",
        skip(_input)
    )]
    fn statement_get_parameter_schema(
        _input: StatementGetParameterSchemaRequest,
    ) -> Result<StatementGetParameterSchemaResponse, DriverException> {
        Err(not_implemented(
            "statement_get_parameter_schema is not yet implemented",
        ))
    }

    #[instrument(name = "DatabaseDriverV1::statement_bind", skip(input))]
    fn statement_bind(
        input: StatementBindRequest,
    ) -> Result<StatementBindResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;
        let schema = required(input.schema, "Schema is required")?;
        let array = required(input.array, "Array is required")?;
        unsafe { statement_bind(stmt_handle.into(), schema.into(), array.into()).to_protobuf()? };
        Ok(StatementBindResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::statement_bind_stream", skip(_input))]
    fn statement_bind_stream(
        _input: StatementBindStreamRequest,
    ) -> Result<StatementBindStreamResponse, DriverException> {
        // TODO: Implement when corresponding API method is available
        Err(not_implemented(
            "statement_bind_stream is not yet implemented",
        ))
    }

    #[instrument(name = "DatabaseDriverV1::statement_execute_query", skip(input))]
    fn statement_execute_query(
        input: StatementExecuteQueryRequest,
    ) -> Result<StatementExecuteQueryResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        let bindings_opt = input
            .bindings
            .and_then(|b| b.binding_type)
            .map(BindingType::from);

        let result = statement_execute_query(stmt_handle.into(), bindings_opt).to_protobuf()?;
        let stream_ptr: ArrowArrayStreamPtr = Box::into_raw(result.stream).into();

        Ok(StatementExecuteQueryResponse {
            result: Some(ExecuteResult {
                stream: Some(stream_ptr),
                rows_affected: result.rows_affected,
                query_id: result.query_id,
                columns: result.columns,
                statement_type_id: result.statement_type_id,
                query: result.query,
            }),
        })
    }

    #[instrument(name = "DatabaseDriverV1::statement_execute_partitions", skip(_input))]
    fn statement_execute_partitions(
        _input: StatementExecutePartitionsRequest,
    ) -> Result<StatementExecutePartitionsResponse, DriverException> {
        Err(not_implemented(
            "statement_execute_partitions is not yet implemented",
        ))
    }

    #[instrument(name = "DatabaseDriverV1::statement_read_partition", skip(_input))]
    fn statement_read_partition(
        _input: StatementReadPartitionRequest,
    ) -> Result<StatementReadPartitionResponse, DriverException> {
        Err(not_implemented(
            "statement_read_partition is not yet implemented",
        ))
    }

    #[instrument(name = "DatabaseDriverV1::config_load_all_sections", skip(_input))]
    fn config_load_all_sections(
        _input: ConfigLoadAllSectionsRequest,
    ) -> Result<ConfigLoadAllSectionsResponse, DriverException> {
        let all_sections = config_manager::load_all_config_sections().map_err(|e| {
            to_driver_exception(ApiError::Configuration {
                source: e,
                location: snafu::Location::new(file!(), line!(), 0),
            })
        })?;

        let sections = all_sections
            .into_iter()
            .map(|(section_name, settings)| {
                let proto_settings = settings
                    .into_iter()
                    .map(|(key, value)| {
                        let proto_value = match value {
                            Setting::String(s) => ConfigSetting {
                                value: Some(config_setting::Value::StringValue(s)),
                            },
                            Setting::Int(i) => ConfigSetting {
                                value: Some(config_setting::Value::IntValue(i)),
                            },
                            Setting::Double(d) => ConfigSetting {
                                value: Some(config_setting::Value::DoubleValue(d)),
                            },
                            Setting::Bytes(b) => ConfigSetting {
                                value: Some(config_setting::Value::BytesValue(b)),
                            },
                        };
                        (key, proto_value)
                    })
                    .collect();
                (
                    section_name,
                    ConfigSection {
                        settings: proto_settings,
                    },
                )
            })
            .collect();

        Ok(ConfigLoadAllSectionsResponse { sections })
    }
}

impl DatabaseDriverServer for DatabaseDriverImpl {}

impl ErrorTrace for DriverException {
    fn error_trace(&self) -> Vec<error_trace::ErrorTraceEntry> {
        self.error_trace
            .iter()
            .map(|entry| error_trace::ErrorTraceEntry {
                location: error_trace::Location::new(entry.file.clone(), entry.line, entry.column),
                message: entry.message.clone(),
            })
            .collect()
    }
}

pub type DatabaseDriverClient = crate::protobuf_gen::database_driver_v1::DatabaseDriverClient<
    crate::protobuf_apis::RustTransport,
>;
