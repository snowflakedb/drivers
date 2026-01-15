//! Unified protobuf handler for the database driver API.
//!
//! This module provides a single implementation of the protobuf RPC interface
//! that works for both native and WASM builds by using the unified API.

#[cfg(feature = "native")]
use crate::apis::unified::statement_bind_ffi;
use crate::apis::unified::{
    ApiError, Setting, connection_init, connection_new, connection_release, connection_set_option,
    database_init, database_new, database_release, database_set_option, statement_bind_stream,
    statement_execute_query, statement_new, statement_prepare, statement_release,
    statement_set_option, statement_set_sql_query,
};
use crate::config::ConfigError;
use crate::protobuf_gen::database_driver_v1::*;
use crate::rest::RestClientError;
use snafu::Report;
use tracing::instrument;

// Note: Type conversions are in the `conversions` module

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
        ApiError::InvalidArgument { .. } => DriverError {
            error_type: Some(driver_error::ErrorType::InternalError(InternalError {})),
        },
        ApiError::Login {
            source: RestClientError::LoginError { message, code, .. },
            ..
        } => DriverError {
            error_type: Some(driver_error::ErrorType::LoginError(LoginError {
                message: message.clone(),
                code: *code as i32,
            })),
        },
        ApiError::Login { source, .. } => DriverError {
            error_type: Some(driver_error::ErrorType::AuthError(AuthenticationError {
                detail: source.to_string(),
            })),
        },
        ApiError::Query {
            source: _source, ..
        } => DriverError {
            error_type: Some(driver_error::ErrorType::GenericError(GenericError {})),
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
        ApiError::InvalidArgument { .. } => StatusCode::InvalidArgument,
        ApiError::Login {
            source: RestClientError::LoginError { .. },
            ..
        } => StatusCode::LoginError,
        ApiError::Login { .. } => StatusCode::AuthenticationError,
        ApiError::Query { .. } => StatusCode::GenericError,
        ApiError::ConnectionLocking { .. } => StatusCode::InternalError,
        ApiError::StatementLocking { .. } => StatusCode::InternalError,
        ApiError::DatabaseLocking { .. } => StatusCode::InternalError,
        ApiError::QueryResponseProcessing { .. } => StatusCode::InternalError,
        ApiError::ConnectionNotInitialized { .. } => StatusCode::InternalError,
    };

    let message = error.to_string();
    let driver_error = to_driver_error(&error);
    let report = Report::from_error(error).to_string();
    DriverException {
        message,
        status_code: status_code as i32,
        error: Some(driver_error),
        report,
    }
}

#[allow(clippy::result_large_err)]
fn required<T>(value: Option<T>, message: &str) -> Result<T, DriverException> {
    value.ok_or_else(|| DriverException {
        message: message.to_string(),
        status_code: StatusCode::InvalidArgument as i32,
        error: None,
        report: message.to_string(),
    })
}

fn not_implemented(message: &str) -> DriverException {
    DriverException {
        message: message.to_string(),
        status_code: StatusCode::NotImplemented as i32,
        error: None,
        report: message.to_string(),
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

/// Unified database driver implementation.
pub struct UnifiedDatabaseDriverImpl {}

impl DatabaseDriver for UnifiedDatabaseDriverImpl {
    #[instrument(name = "UnifiedDriver::database_new", skip(_input))]
    fn database_new(_input: DatabaseNewRequest) -> Result<DatabaseNewResponse, DriverException> {
        let handle = database_new();
        Ok(DatabaseNewResponse {
            db_handle: Some(DatabaseHandle::from(handle)),
        })
    }

    #[instrument(name = "UnifiedDriver::database_set_option_string", skip(input))]
    fn database_set_option_string(
        input: DatabaseSetOptionStringRequest,
    ) -> Result<DatabaseSetOptionStringResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;
        database_set_option(db_handle.into(), input.key, Setting::String(input.value))
            .to_protobuf()?;
        Ok(DatabaseSetOptionStringResponse {})
    }

    #[instrument(name = "UnifiedDriver::database_set_option_bytes", skip(input))]
    fn database_set_option_bytes(
        input: DatabaseSetOptionBytesRequest,
    ) -> Result<DatabaseSetOptionBytesResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;
        database_set_option(db_handle.into(), input.key, Setting::Bytes(input.value))
            .to_protobuf()?;
        Ok(DatabaseSetOptionBytesResponse {})
    }

    #[instrument(name = "UnifiedDriver::database_set_option_int", skip(input))]
    fn database_set_option_int(
        input: DatabaseSetOptionIntRequest,
    ) -> Result<DatabaseSetOptionIntResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;
        database_set_option(db_handle.into(), input.key, Setting::Int(input.value))
            .to_protobuf()?;
        Ok(DatabaseSetOptionIntResponse {})
    }

    #[instrument(name = "UnifiedDriver::database_set_option_double", skip(input))]
    fn database_set_option_double(
        input: DatabaseSetOptionDoubleRequest,
    ) -> Result<DatabaseSetOptionDoubleResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;
        database_set_option(db_handle.into(), input.key, Setting::Double(input.value))
            .to_protobuf()?;
        Ok(DatabaseSetOptionDoubleResponse {})
    }

    #[instrument(name = "UnifiedDriver::database_init", skip(input))]
    fn database_init(input: DatabaseInitRequest) -> Result<DatabaseInitResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;
        database_init(db_handle.into()).to_protobuf()?;
        Ok(DatabaseInitResponse {})
    }

    #[instrument(name = "UnifiedDriver::database_release", skip(input))]
    fn database_release(
        input: DatabaseReleaseRequest,
    ) -> Result<DatabaseReleaseResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;
        database_release(db_handle.into()).to_protobuf()?;
        Ok(DatabaseReleaseResponse {})
    }

    #[instrument(name = "UnifiedDriver::connection_new", skip(_input))]
    fn connection_new(
        _input: ConnectionNewRequest,
    ) -> Result<ConnectionNewResponse, DriverException> {
        let handle = connection_new();
        Ok(ConnectionNewResponse {
            conn_handle: Some(ConnectionHandle::from(handle)),
        })
    }

    #[instrument(name = "UnifiedDriver::connection_set_option_string", skip(input))]
    fn connection_set_option_string(
        input: ConnectionSetOptionStringRequest,
    ) -> Result<ConnectionSetOptionStringResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;
        connection_set_option(conn_handle.into(), input.key, Setting::String(input.value))
            .to_protobuf()?;
        Ok(ConnectionSetOptionStringResponse {})
    }

    #[instrument(name = "UnifiedDriver::connection_set_option_bytes", skip(input))]
    fn connection_set_option_bytes(
        input: ConnectionSetOptionBytesRequest,
    ) -> Result<ConnectionSetOptionBytesResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;
        connection_set_option(conn_handle.into(), input.key, Setting::Bytes(input.value))
            .to_protobuf()?;
        Ok(ConnectionSetOptionBytesResponse {})
    }

    #[instrument(name = "UnifiedDriver::connection_set_option_int", skip(input))]
    fn connection_set_option_int(
        input: ConnectionSetOptionIntRequest,
    ) -> Result<ConnectionSetOptionIntResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;
        connection_set_option(conn_handle.into(), input.key, Setting::Int(input.value))
            .to_protobuf()?;
        Ok(ConnectionSetOptionIntResponse {})
    }

    #[instrument(name = "UnifiedDriver::connection_set_option_double", skip(input))]
    fn connection_set_option_double(
        input: ConnectionSetOptionDoubleRequest,
    ) -> Result<ConnectionSetOptionDoubleResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;
        connection_set_option(conn_handle.into(), input.key, Setting::Double(input.value))
            .to_protobuf()?;
        Ok(ConnectionSetOptionDoubleResponse {})
    }

    #[instrument(name = "UnifiedDriver::connection_init", skip(input))]
    fn connection_init(
        input: ConnectionInitRequest,
    ) -> Result<ConnectionInitResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;
        let db_handle = required(input.db_handle, "Database handle is required")?;
        connection_init(conn_handle.into(), db_handle.into()).to_protobuf()?;
        Ok(ConnectionInitResponse {})
    }

    #[instrument(name = "UnifiedDriver::connection_release", skip(input))]
    fn connection_release(
        input: ConnectionReleaseRequest,
    ) -> Result<ConnectionReleaseResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;
        connection_release(conn_handle.into()).to_protobuf()?;
        Ok(ConnectionReleaseResponse {})
    }

    #[instrument(name = "UnifiedDriver::connection_get_info", skip(_input))]
    fn connection_get_info(
        _input: ConnectionGetInfoRequest,
    ) -> Result<ConnectionGetInfoResponse, DriverException> {
        Err(not_implemented(
            "connection_get_info is not yet implemented",
        ))
    }

    #[instrument(name = "UnifiedDriver::connection_get_objects", skip(_input))]
    fn connection_get_objects(
        _input: ConnectionGetObjectsRequest,
    ) -> Result<ConnectionGetObjectsResponse, DriverException> {
        Err(not_implemented(
            "connection_get_objects is not yet implemented",
        ))
    }

    #[instrument(name = "UnifiedDriver::connection_get_table_schema", skip(_input))]
    fn connection_get_table_schema(
        _input: ConnectionGetTableSchemaRequest,
    ) -> Result<ConnectionGetTableSchemaResponse, DriverException> {
        Err(not_implemented(
            "connection_get_table_schema is not yet implemented",
        ))
    }

    #[instrument(name = "UnifiedDriver::connection_get_table_types", skip(_input))]
    fn connection_get_table_types(
        _input: ConnectionGetTableTypesRequest,
    ) -> Result<ConnectionGetTableTypesResponse, DriverException> {
        Err(not_implemented(
            "connection_get_table_types is not yet implemented",
        ))
    }

    #[instrument(name = "UnifiedDriver::connection_commit", skip(_input))]
    fn connection_commit(
        _input: ConnectionCommitRequest,
    ) -> Result<ConnectionCommitResponse, DriverException> {
        Err(not_implemented("connection_commit is not yet implemented"))
    }

    #[instrument(name = "UnifiedDriver::connection_rollback", skip(_input))]
    fn connection_rollback(
        _input: ConnectionRollbackRequest,
    ) -> Result<ConnectionRollbackResponse, DriverException> {
        Err(not_implemented(
            "connection_rollback is not yet implemented",
        ))
    }

    #[instrument(name = "UnifiedDriver::statement_new", skip(input))]
    fn statement_new(input: StatementNewRequest) -> Result<StatementNewResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;
        let handle = statement_new(conn_handle.into()).to_protobuf()?;
        Ok(StatementNewResponse {
            stmt_handle: Some(StatementHandle::from(handle)),
        })
    }

    #[instrument(name = "UnifiedDriver::statement_release", skip(input))]
    fn statement_release(
        input: StatementReleaseRequest,
    ) -> Result<StatementReleaseResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;
        statement_release(stmt_handle.into()).to_protobuf()?;
        Ok(StatementReleaseResponse {})
    }

    #[instrument(name = "UnifiedDriver::statement_set_sql_query", skip(input))]
    fn statement_set_sql_query(
        input: StatementSetSqlQueryRequest,
    ) -> Result<StatementSetSqlQueryResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;
        statement_set_sql_query(stmt_handle.into(), input.query).to_protobuf()?;
        Ok(StatementSetSqlQueryResponse {})
    }

    #[instrument(name = "UnifiedDriver::statement_set_substrait_plan", skip(_input))]
    fn statement_set_substrait_plan(
        _input: StatementSetSubstraitPlanRequest,
    ) -> Result<StatementSetSubstraitPlanResponse, DriverException> {
        Err(not_implemented(
            "statement_set_substrait_plan is not yet implemented",
        ))
    }

    #[instrument(name = "UnifiedDriver::statement_prepare", skip(input))]
    fn statement_prepare(
        input: StatementPrepareRequest,
    ) -> Result<StatementPrepareResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;
        statement_prepare(stmt_handle.into()).to_protobuf()?;
        Ok(StatementPrepareResponse {})
    }

    #[instrument(name = "UnifiedDriver::statement_set_option_string", skip(input))]
    fn statement_set_option_string(
        input: StatementSetOptionStringRequest,
    ) -> Result<StatementSetOptionStringResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;
        statement_set_option(stmt_handle.into(), input.key, Setting::String(input.value))
            .to_protobuf()?;
        Ok(StatementSetOptionStringResponse {})
    }

    #[instrument(name = "UnifiedDriver::statement_set_option_bytes", skip(input))]
    fn statement_set_option_bytes(
        input: StatementSetOptionBytesRequest,
    ) -> Result<StatementSetOptionBytesResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;
        statement_set_option(stmt_handle.into(), input.key, Setting::Bytes(input.value))
            .to_protobuf()?;
        Ok(StatementSetOptionBytesResponse {})
    }

    #[instrument(name = "UnifiedDriver::statement_set_option_int", skip(input))]
    fn statement_set_option_int(
        input: StatementSetOptionIntRequest,
    ) -> Result<StatementSetOptionIntResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;
        statement_set_option(stmt_handle.into(), input.key, Setting::Int(input.value))
            .to_protobuf()?;
        Ok(StatementSetOptionIntResponse {})
    }

    #[instrument(name = "UnifiedDriver::statement_set_option_double", skip(input))]
    fn statement_set_option_double(
        input: StatementSetOptionDoubleRequest,
    ) -> Result<StatementSetOptionDoubleResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;
        statement_set_option(stmt_handle.into(), input.key, Setting::Double(input.value))
            .to_protobuf()?;
        Ok(StatementSetOptionDoubleResponse {})
    }

    #[instrument(name = "UnifiedDriver::statement_get_parameter_schema", skip(_input))]
    fn statement_get_parameter_schema(
        _input: StatementGetParameterSchemaRequest,
    ) -> Result<StatementGetParameterSchemaResponse, DriverException> {
        Err(not_implemented(
            "statement_get_parameter_schema is not yet implemented",
        ))
    }

    #[instrument(name = "UnifiedDriver::statement_bind", skip(input))]
    fn statement_bind(
        input: StatementBindRequest,
    ) -> Result<StatementBindResponse, DriverException> {
        #[cfg(feature = "native")]
        {
            let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;
            let schema = required(input.schema, "Schema is required")?;
            let array = required(input.array, "Array is required")?;
            unsafe {
                statement_bind_ffi(stmt_handle.into(), schema.into(), array.into()).to_protobuf()?
            };
            Ok(StatementBindResponse {})
        }

        #[cfg(all(feature = "wasm", not(feature = "native")))]
        {
            // FFI parameter binding not available for WASM - use statement_bind_stream instead
            let _ = input;
            Err(not_implemented(
                "FFI parameter binding not available for WASM, use statement_bind_stream",
            ))
        }
    }

    #[instrument(name = "UnifiedDriver::statement_bind_stream", skip(input))]
    fn statement_bind_stream(
        input: StatementBindStreamRequest,
    ) -> Result<StatementBindStreamResponse, DriverException> {
        // JSON-based parameter binding works for both native and WASM
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;
        statement_bind_stream(stmt_handle.into(), &input.stream).to_protobuf()?;
        Ok(StatementBindStreamResponse {})
    }

    #[instrument(name = "UnifiedDriver::statement_execute_query", skip(input))]
    fn statement_execute_query(
        input: StatementExecuteQueryRequest,
    ) -> Result<StatementExecuteQueryResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;
        let result = statement_execute_query(stmt_handle.into()).to_protobuf()?;

        #[cfg(feature = "native")]
        {
            // Native: use FFI pointer
            let stream_ptr: ArrowArrayStreamPtr = Box::into_raw(result.stream).into();
            Ok(StatementExecuteQueryResponse {
                result: Some(ExecuteResult {
                    stream: Some(stream_ptr),
                    rows_affected: result.rows_affected,
                    wasm_result: None, // Not used for native
                }),
            })
        }

        #[cfg(all(feature = "wasm", not(feature = "native")))]
        {
            // WASM: serialize full Arrow IPC for simpler host integration
            // Note: This sacrifices zero-copy for compatibility with database/sql interfaces
            let ipc_bytes = crate::arrow_wasm::serialize_reader_to_full_ipc(result.stream)
                .map_err(|e| {
                    to_driver_exception(ApiError::GenericError {
                        message: format!("Failed to serialize Arrow to IPC: {}", e),
                        location: snafu::Location::new(file!(), line!(), column!()),
                    })
                })?;

            // Put the full IPC in schema_ipc field (which is normally just schema)
            // The host reads this as a complete IPC stream
            let wasm_result = WasmArrowResult {
                schema_ipc: ipc_bytes,
                batches: vec![],   // Not needed - data is in IPC
                total_rows: 0,     // Will be determined from IPC
                release_handle: 0, // No WASM memory to release
            };

            Ok(StatementExecuteQueryResponse {
                result: Some(ExecuteResult {
                    stream: None, // Not used for WASM
                    rows_affected: result.rows_affected,
                    wasm_result: Some(wasm_result),
                }),
            })
        }
    }

    #[instrument(name = "UnifiedDriver::statement_execute_partitions", skip(_input))]
    fn statement_execute_partitions(
        _input: StatementExecutePartitionsRequest,
    ) -> Result<StatementExecutePartitionsResponse, DriverException> {
        Err(not_implemented(
            "statement_execute_partitions is not yet implemented",
        ))
    }

    #[instrument(name = "UnifiedDriver::statement_read_partition", skip(_input))]
    fn statement_read_partition(
        _input: StatementReadPartitionRequest,
    ) -> Result<StatementReadPartitionResponse, DriverException> {
        Err(not_implemented(
            "statement_read_partition is not yet implemented",
        ))
    }
}

impl DatabaseDriverServer for UnifiedDatabaseDriverImpl {}

/// Client type for the unified database driver.
pub type UnifiedDatabaseDriverClient =
    crate::protobuf_gen::database_driver_v1::DatabaseDriverClient<
        crate::protobuf_apis::RustTransport,
    >;
