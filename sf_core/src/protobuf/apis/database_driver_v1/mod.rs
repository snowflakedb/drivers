mod converter;

use crate::apis::database_driver_v1::BindingType;
use crate::apis::database_driver_v1::DatabaseDriverV1;
use crate::apis::database_driver_v1::FetchChunkInput;
use crate::apis::database_driver_v1::error::ConfigurationSnafu;
use crate::config::config_manager;
use crate::config::path_resolver;
use crate::handle_manager::Handle;
use crate::protobuf::generated::database_driver_v1::*;
use converter::{
    ToProtobuf, core_validation_issue_to_proto, flat_sections_to_nested_json,
    proto_options_to_hashmap,
};
use error_trace::ErrorTrace;
use snafu::ResultExt;
use std::future::Future;
use std::sync::LazyLock;
use tracing::instrument;

#[allow(clippy::result_large_err)]
fn required<T>(value: Option<T>, message: &str) -> Result<T, DriverException> {
    value.ok_or_else(|| DriverException {
        message: message.to_string(),
        status_code: StatusCode::InvalidArgument as i32,
        ..Default::default()
    })
}

fn not_implemented(message: &str) -> DriverException {
    DriverException {
        message: message.to_string(),
        status_code: StatusCode::NotImplemented as i32,
        ..Default::default()
    }
}

pub struct DatabaseDriverImpl {
    driver: DatabaseDriverV1,
}

impl Default for DatabaseDriverImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl DatabaseDriverImpl {
    pub fn new() -> Self {
        Self::new_with(DriverProviders::default())
    }

    pub fn new_with(providers: DriverProviders) -> Self {
        Self {
            driver: DatabaseDriverV1::with_providers(providers),
        }
    }
}

impl DatabaseDriver for DatabaseDriverImpl {
    #[instrument(name = "DatabaseDriverV1::database_new", skip(self, _input))]
    async fn database_new(
        &self,
        _input: DatabaseNewRequest,
    ) -> Result<DatabaseNewResponse, DriverException> {
        let handle = self.driver.database_new();
        Ok(DatabaseNewResponse {
            db_handle: Some(DatabaseHandle::from(handle)),
        })
    }

    #[instrument(name = "DatabaseDriverV1::database_set_options", skip(self, input))]
    async fn database_set_options(
        &self,
        input: DatabaseSetOptionsRequest,
    ) -> Result<DatabaseSetOptionsResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;
        let options = proto_options_to_hashmap(input.options);

        let warnings = self
            .driver
            .database_set_options(db_handle.into(), options)
            .await
            .to_protobuf()?;

        Ok(DatabaseSetOptionsResponse {
            warnings: warnings
                .into_iter()
                .map(core_validation_issue_to_proto)
                .collect(),
        })
    }

    #[instrument(name = "DatabaseDriverV1::database_init", skip(self, input))]
    async fn database_init(
        &self,
        input: DatabaseInitRequest,
    ) -> Result<DatabaseInitResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;

        self.driver.database_init(db_handle.into()).to_protobuf()?;
        Ok(DatabaseInitResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::database_release", skip(self, input))]
    async fn database_release(
        &self,
        input: DatabaseReleaseRequest,
    ) -> Result<DatabaseReleaseResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;

        self.driver
            .database_release(db_handle.into())
            .to_protobuf()?;
        Ok(DatabaseReleaseResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::database_fetch_chunk", skip(self, input))]
    async fn database_fetch_chunk(
        &self,
        input: DatabaseFetchChunkRequest,
    ) -> Result<DatabaseFetchChunkResponse, DriverException> {
        let db_handle = required(input.db_handle, "Database handle is required")?;
        let chunk = required(input.chunk, "Chunk is required")?;
        let chunk_data = required(chunk.data, "Chunk data is required")?;
        let fetch_input: FetchChunkInput = chunk_data.into();

        let stream = self
            .driver
            .database_fetch_chunk(db_handle.into(), fetch_input)
            .await
            .to_protobuf()?;

        let stream_ptr: ArrowArrayStreamPtr = Box::into_raw(stream).into();
        Ok(DatabaseFetchChunkResponse {
            stream: Some(stream_ptr),
        })
    }

    #[instrument(name = "DatabaseDriverV1::connection_new", skip(self, _input))]
    async fn connection_new(
        &self,
        _input: ConnectionNewRequest,
    ) -> Result<ConnectionNewResponse, DriverException> {
        let handle = self.driver.connection_new();
        Ok(ConnectionNewResponse {
            conn_handle: Some(ConnectionHandle::from(handle)),
        })
    }

    #[instrument(name = "DatabaseDriverV1::connection_set_options", skip(self, input))]
    async fn connection_set_options(
        &self,
        input: ConnectionSetOptionsRequest,
    ) -> Result<ConnectionSetOptionsResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;
        let options = proto_options_to_hashmap(input.options);

        let warnings = self
            .driver
            .connection_set_options(conn_handle.into(), options)
            .await
            .to_protobuf()?;

        Ok(ConnectionSetOptionsResponse {
            warnings: warnings
                .into_iter()
                .map(core_validation_issue_to_proto)
                .collect(),
        })
    }

    #[instrument(name = "DatabaseDriverV1::connection_init", skip(self, input))]
    async fn connection_init(
        &self,
        input: ConnectionInitRequest,
    ) -> Result<ConnectionInitResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;
        let db_handle = required(input.db_handle, "Database handle is required")?;

        // If wrapper identity is provided, store it before connection_init
        // so the session_init telemetry event carries the identity.
        if let Some(ref wrapper) = input.wrapper_identity
            && let Some(ref driver_name) = wrapper.driver_name
            && !driver_name.is_empty()
        {
            let identity = crate::apis::database_driver_v1::connection::WrapperIdentity {
                driver_name: driver_name.clone(),
                driver_version: wrapper.driver_version.clone().unwrap_or_default(),
                language_runtime: wrapper.language_runtime.clone().unwrap_or_default(),
                language_version: wrapper.language_version.clone().unwrap_or_default(),
                language_compiler: wrapper.language_compiler.clone(),
            };

            tracing::debug!(
                driver_name = %identity.driver_name,
                driver_version = %identity.driver_version,
                language_runtime = %identity.language_runtime,
                language_version = %identity.language_version,
                "Wrapper identity stored via connection_init"
            );

            self.driver
                .set_wrapper_identity(Handle::from(conn_handle), identity)
                .await
                .to_protobuf()?;
        }

        self.driver
            .connection_init(conn_handle.into(), db_handle.into())
            .await
            .to_protobuf()?;
        Ok(ConnectionInitResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::connection_release", skip(self, input))]
    async fn connection_release(
        &self,
        input: ConnectionReleaseRequest,
    ) -> Result<ConnectionReleaseResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        self.driver
            .connection_release(conn_handle.into())
            .to_protobuf()?;
        Ok(ConnectionReleaseResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::connection_close", skip(self, input))]
    async fn connection_close(
        &self,
        input: ConnectionCloseRequest,
    ) -> Result<ConnectionCloseResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        self.driver
            .connection_close(conn_handle.into())
            .await
            .to_protobuf()?;
        Ok(ConnectionCloseResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::connection_is_closed", skip(self, input))]
    async fn connection_is_closed(
        &self,
        input: ConnectionIsClosedRequest,
    ) -> Result<ConnectionIsClosedResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let is_closed = self
            .driver
            .connection_is_closed(conn_handle.into())
            .await
            .to_protobuf()?;
        Ok(ConnectionIsClosedResponse { is_closed })
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_info", skip(self, input))]
    async fn connection_get_info(
        &self,
        input: ConnectionGetInfoRequest,
    ) -> Result<ConnectionGetInfoResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let info = self
            .driver
            .connection_get_info(conn_handle.into())
            .await
            .to_protobuf()?;

        Ok(ConnectionGetInfoResponse::from_info(
            info,
            input.include_master_token,
        ))
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_objects", skip(self, _input))]
    async fn connection_get_objects(
        &self,
        _input: ConnectionGetObjectsRequest,
    ) -> Result<ConnectionGetObjectsResponse, DriverException> {
        Err(not_implemented(
            "connection_get_objects is not yet implemented",
        ))
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_get_table_schema",
        skip(self, _input)
    )]
    async fn connection_get_table_schema(
        &self,
        _input: ConnectionGetTableSchemaRequest,
    ) -> Result<ConnectionGetTableSchemaResponse, DriverException> {
        Err(not_implemented(
            "connection_get_table_schema is not yet implemented",
        ))
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_get_table_types",
        skip(self, _input)
    )]
    async fn connection_get_table_types(
        &self,
        _input: ConnectionGetTableTypesRequest,
    ) -> Result<ConnectionGetTableTypesResponse, DriverException> {
        Err(not_implemented(
            "connection_get_table_types is not yet implemented",
        ))
    }

    #[instrument(name = "DatabaseDriverV1::connection_commit", skip(self, _input))]
    async fn connection_commit(
        &self,
        _input: ConnectionCommitRequest,
    ) -> Result<ConnectionCommitResponse, DriverException> {
        Err(not_implemented("connection_commit is not yet implemented"))
    }

    #[instrument(name = "DatabaseDriverV1::connection_rollback", skip(self, _input))]
    async fn connection_rollback(
        &self,
        _input: ConnectionRollbackRequest,
    ) -> Result<ConnectionRollbackResponse, DriverException> {
        Err(not_implemented(
            "connection_rollback is not yet implemented",
        ))
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_set_autocommit",
        skip(self, input),
        fields(conn_handle = tracing::field::Empty, autocommit = tracing::field::Empty)
    )]
    async fn connection_set_autocommit(
        &self,
        input: ConnectionSetAutocommitRequest,
    ) -> Result<ConnectionSetAutocommitResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;
        tracing::Span::current().record("conn_handle", tracing::field::debug(&conn_handle));
        tracing::Span::current().record("autocommit", input.autocommit);
        self.driver
            .connection_set_autocommit(conn_handle.into(), input.autocommit)
            .await
            .to_protobuf()?;
        Ok(ConnectionSetAutocommitResponse {})
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_use_database",
        skip(self, input),
        fields(conn_handle = tracing::field::Empty, database = tracing::field::Empty)
    )]
    async fn connection_use_database(
        &self,
        input: ConnectionUseDatabaseRequest,
    ) -> Result<ConnectionUseDatabaseResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;
        // Record the trimmed form so the span matches what actually executes — the
        // driver's connection_use_database normalises whitespace before running the SQL.
        tracing::Span::current().record("conn_handle", tracing::field::debug(&conn_handle));
        tracing::Span::current().record("database", input.database.trim());
        self.driver
            .connection_use_database(conn_handle.into(), &input.database)
            .await
            .to_protobuf()?;
        Ok(ConnectionUseDatabaseResponse {})
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_set_session_parameters",
        skip(self, input)
    )]
    async fn connection_set_session_parameters(
        &self,
        input: ConnectionSetSessionParametersRequest,
    ) -> Result<ConnectionSetSessionParametersResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        self.driver
            .connection_set_session_parameters(conn_handle.into(), input.parameters)
            .await
            .to_protobuf()?;

        Ok(ConnectionSetSessionParametersResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::connection_get_parameter", skip(self, input))]
    async fn connection_get_parameter(
        &self,
        input: ConnectionGetParameterRequest,
    ) -> Result<ConnectionGetParameterResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let value = self
            .driver
            .connection_get_parameter(conn_handle.into(), input.key)
            .await
            .to_protobuf()?;

        Ok(ConnectionGetParameterResponse { value })
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_validate_options",
        skip(self, input)
    )]
    async fn connection_validate_options(
        &self,
        input: ConnectionValidateOptionsRequest,
    ) -> Result<ConnectionValidateOptionsResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let issues = self
            .driver
            .connection_validate_options(conn_handle.into())
            .await
            .to_protobuf()?;

        Ok(ConnectionValidateOptionsResponse {
            issues: issues
                .into_iter()
                .map(core_validation_issue_to_proto)
                .collect(),
        })
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_get_query_result",
        skip(self, input)
    )]
    async fn connection_get_query_result(
        &self,
        input: ConnectionGetQueryResultRequest,
    ) -> Result<ExecuteQueryResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let result = self
            .driver
            .connection_get_query_result(conn_handle.into(), input.query_id)
            .await
            .to_protobuf()?;

        Ok(result.into())
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_get_result_set",
        skip(self, input)
    )]
    async fn connection_get_result_set(
        &self,
        input: ConnectionGetResultSetRequest,
    ) -> Result<ResultSetResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let result = self
            .driver
            .connection_get_result_set(conn_handle.into(), input.query_id)
            .await
            .to_protobuf()?;

        Ok(result.into())
    }

    #[instrument(
        name = "DatabaseDriverV1::connection_get_query_status",
        skip(self, input)
    )]
    async fn connection_get_query_status(
        &self,
        input: ConnectionGetQueryStatusRequest,
    ) -> Result<ConnectionGetQueryStatusResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let result = self
            .driver
            .connection_get_query_status(conn_handle.into(), &input.query_id)
            .await
            .to_protobuf()?;

        Ok(result.into())
    }

    #[instrument(name = "DatabaseDriverV1::connection_abort_query", skip(self, input))]
    async fn connection_abort_query(
        &self,
        input: ConnectionAbortQueryRequest,
    ) -> Result<ConnectionAbortQueryResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let success = match self
            .driver
            .connection_abort_query(conn_handle.into(), input.query_id)
            .await
        {
            Ok(()) => true,
            Err(_) => false,
        };

        Ok(ConnectionAbortQueryResponse { success })
    }

    #[instrument(name = "DatabaseDriverV1::connection_send_http", skip(self, input))]
    async fn connection_send_http(
        &self,
        input: ConnectionSendHttpRequest,
    ) -> Result<ConnectionSendHttpResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let result = self
            .driver
            .connection_send_http_request(
                conn_handle.into(),
                input.method,
                input.url,
                input.headers,
                input.body,
            )
            .await
            .to_protobuf()?;

        Ok(ConnectionSendHttpResponse {
            status_code: result.status_code,
            headers: result.headers,
            body: result.body,
        })
    }

    #[instrument(name = "DatabaseDriverV1::connection_request_token", skip(self, input))]
    async fn connection_request_token(
        &self,
        input: ConnectionTokenRequest,
    ) -> Result<ConnectionTokenResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let request_type = match TokenRequestType::try_from(input.request_type) {
            Ok(TokenRequestType::Issue) => "ISSUE",
            Ok(TokenRequestType::Renew) => "RENEW",
            _ => {
                return Err(DriverException {
                    message: "request_type must be ISSUE or RENEW".to_string(),
                    status_code: StatusCode::InvalidArgument as i32,
                    ..Default::default()
                });
            }
        };

        let result = self
            .driver
            .connection_token_request(conn_handle.into(), request_type.to_string())
            .await
            .to_protobuf()?;

        Ok(ConnectionTokenResponse {
            session_token: result.session_token.reveal().to_string(),
            validity_in_seconds: result.validity_in_seconds,
        })
    }

    #[instrument(name = "DatabaseDriverV1::connection_heartbeat", skip(self, input))]
    async fn connection_heartbeat(
        &self,
        input: ConnectionHeartbeatRequest,
    ) -> Result<ConnectionHeartbeatResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;
        let valid = self
            .driver
            .connection_heartbeat(conn_handle.into())
            .await
            .to_protobuf()?;
        Ok(ConnectionHeartbeatResponse { valid })
    }

    #[instrument(name = "DatabaseDriverV1::statement_new", skip(self, input))]
    async fn statement_new(
        &self,
        input: StatementNewRequest,
    ) -> Result<StatementNewResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;

        let handle = self
            .driver
            .statement_new(conn_handle.into())
            .to_protobuf()?;
        Ok(StatementNewResponse {
            stmt_handle: Some(StatementHandle::from(handle)),
        })
    }

    #[instrument(name = "DatabaseDriverV1::statement_release", skip(self, input))]
    async fn statement_release(
        &self,
        input: StatementReleaseRequest,
    ) -> Result<StatementReleaseResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        self.driver
            .statement_release(stmt_handle.into())
            .to_protobuf()?;
        Ok(StatementReleaseResponse {})
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_sql_query", skip(self, input))]
    async fn statement_set_sql_query(
        &self,
        input: StatementSetSqlQueryRequest,
    ) -> Result<StatementSetSqlQueryResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        self.driver
            .statement_set_sql_query(stmt_handle.into(), input.query)
            .await
            .to_protobuf()?;
        Ok(StatementSetSqlQueryResponse {})
    }

    #[instrument(
        name = "DatabaseDriverV1::statement_set_substrait_plan",
        skip(self, _input)
    )]
    async fn statement_set_substrait_plan(
        &self,
        _input: StatementSetSubstraitPlanRequest,
    ) -> Result<StatementSetSubstraitPlanResponse, DriverException> {
        // TODO: Implement when corresponding API method is available
        Err(not_implemented(
            "statement_set_substrait_plan is not yet implemented",
        ))
    }

    #[instrument(name = "DatabaseDriverV1::statement_prepare", skip(self, input))]
    async fn statement_prepare(
        &self,
        input: StatementPrepareRequest,
    ) -> Result<StatementPrepareResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;
        let result = self
            .driver
            .statement_prepare(stmt_handle.into())
            .await
            .to_protobuf()?;
        let result_ptr: ArrowArrayStreamPtr = Box::into_raw(result.stream).into();
        Ok(StatementPrepareResponse {
            result: Some(PrepareResult {
                stream: Some(result_ptr),
                query_id: result.query_id,
                columns: result.columns.into_iter().map(|cm| cm.into()).collect(),
                number_of_binds: result.number_of_binds,
                query: result.query,
                sql_state: result.sql_state,
            }),
        })
    }

    #[instrument(name = "DatabaseDriverV1::statement_set_options", skip(self, input))]
    async fn statement_set_options(
        &self,
        input: StatementSetOptionsRequest,
    ) -> Result<StatementSetOptionsResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;
        let options = proto_options_to_hashmap(input.options);

        let warnings = self
            .driver
            .statement_set_options(stmt_handle.into(), options)
            .await
            .to_protobuf()?;

        Ok(StatementSetOptionsResponse {
            warnings: warnings
                .into_iter()
                .map(core_validation_issue_to_proto)
                .collect(),
        })
    }

    #[instrument(
        name = "DatabaseDriverV1::statement_get_parameter_schema",
        skip(self, _input)
    )]
    async fn statement_get_parameter_schema(
        &self,
        _input: StatementGetParameterSchemaRequest,
    ) -> Result<StatementGetParameterSchemaResponse, DriverException> {
        Err(not_implemented(
            "statement_get_parameter_schema is not yet implemented",
        ))
    }

    #[instrument(name = "DatabaseDriverV1::statement_execute_query", skip(self, input))]
    async fn statement_execute_query(
        &self,
        input: StatementExecuteQueryRequest,
    ) -> Result<ExecuteQueryResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        let bindings_opt = input
            .bindings
            .and_then(|b| b.binding_type)
            .map(BindingType::from);

        let result = self
            .driver
            .statement_execute_query(stmt_handle.into(), bindings_opt)
            .await
            .to_protobuf()?;

        Ok(result.into())
    }

    #[instrument(name = "DatabaseDriverV1::statement_get_result_set", skip(self, input))]
    async fn statement_get_result_set(
        &self,
        input: StatementGetResultSetRequest,
    ) -> Result<ResultSetResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        let result = self
            .driver
            .statement_get_result_set(stmt_handle.into(), input.query_id)
            .await
            .to_protobuf()?;

        Ok(result.into())
    }

    #[instrument(name = "DatabaseDriverV1::statement_execute_async", skip(self, input))]
    async fn statement_execute_async(
        &self,
        input: StatementExecuteAsyncRequest,
    ) -> Result<StatementExecuteAsyncResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        let bindings_opt = input
            .bindings
            .and_then(|b| b.binding_type)
            .map(BindingType::from);

        let result = self
            .driver
            .statement_execute_async(stmt_handle.into(), bindings_opt)
            .await
            .to_protobuf()?;

        Ok(StatementExecuteAsyncResponse {
            query_id: result.query_id,
        })
    }

    #[instrument(
        name = "DatabaseDriverV1::statement_execute_partitions",
        skip(self, _input)
    )]
    async fn statement_execute_partitions(
        &self,
        _input: StatementExecutePartitionsRequest,
    ) -> Result<StatementExecutePartitionsResponse, DriverException> {
        Err(not_implemented(
            "statement_execute_partitions is not yet implemented",
        ))
    }

    #[instrument(
        name = "DatabaseDriverV1::statement_read_partition",
        skip(self, _input)
    )]
    async fn statement_read_partition(
        &self,
        _input: StatementReadPartitionRequest,
    ) -> Result<StatementReadPartitionResponse, DriverException> {
        Err(not_implemented(
            "statement_read_partition is not yet implemented",
        ))
    }

    #[instrument(name = "DatabaseDriverV1::statement_result_chunks", skip(self, input))]
    async fn statement_result_chunks(
        &self,
        input: StatementResultChunksRequest,
    ) -> Result<StatementResultChunksResponse, DriverException> {
        let stmt_handle = required(input.stmt_handle, "Statement handle is required")?;

        let chunk_info = self
            .driver
            .statement_result_chunks(stmt_handle.into(), &input.query_id)
            .await
            .to_protobuf()?;

        let mut chunks = Vec::new();

        if let Some(base64_data) = chunk_info.initial_chunk_base64 {
            // Calculate inline chunk row count: total from descriptor minus remote chunks
            let remote_rows: i32 = chunk_info.chunks.iter().map(|c| c.row_count).sum();
            let inline_row_count = chunk_info
                .descriptor
                .rows_affected
                .map(|total| (total as i32).saturating_sub(remote_rows))
                .unwrap_or(0);

            chunks.push(ResultChunk {
                format: ChunkFormat::ArrowIpc as i32,
                data: Some(result_chunk::Data::Inline(base64_data)),
                row_count: inline_row_count,
            });
        }

        for c in &chunk_info.chunks {
            chunks.push(ResultChunk {
                format: ChunkFormat::ArrowIpc as i32,
                data: Some(result_chunk::Data::Remote(RemoteChunk {
                    url: c.url.clone(),
                    headers: c.headers.clone(),
                    compressed_size: c.compressed_size,
                    uncompressed_size: c.uncompressed_size,
                })),
                row_count: c.row_count,
            });
        }

        Ok(StatementResultChunksResponse {
            result: Some(ResultChunksResult { chunks }),
        })
    }

    #[instrument(name = "DatabaseDriverV1::config_load_all_sections", skip(self, input))]
    async fn config_load_all_sections(
        &self,
        input: ConfigLoadAllSectionsRequest,
    ) -> Result<ConfigLoadAllSectionsResponse, DriverException> {
        let all_sections = if input.config_file.is_some() || input.connections_file.is_some() {
            let paths = path_resolver::ConfigPaths {
                config_file: input.config_file.map(std::path::PathBuf::from),
                connections_file: input.connections_file.map(std::path::PathBuf::from),
            };
            config_manager::load_all_config_sections_with_paths(&paths)
        } else {
            config_manager::load_all_config_sections()
        }
        .context(ConfigurationSnafu)
        .to_protobuf()?;

        let nested_json = flat_sections_to_nested_json(all_sections);
        let config_json = serde_json::to_string(&nested_json).map_err(|e| DriverException {
            message: format!("Failed to serialize config to JSON: {e}"),
            status_code: StatusCode::InternalError as i32,
            ..Default::default()
        })?;

        Ok(ConfigLoadAllSectionsResponse { config_json })
    }

    #[instrument(name = "DatabaseDriverV1::config_get_paths", skip(self, _input))]
    async fn config_get_paths(
        &self,
        _input: ConfigGetPathsRequest,
    ) -> Result<ConfigGetPathsResponse, DriverException> {
        let paths = path_resolver::get_config_paths()
            .context(ConfigurationSnafu)
            .to_protobuf()?;

        let config_file = paths.config_file.ok_or_else(|| DriverException {
            message: "Configuration path for config file is unavailable".to_string(),
            status_code: StatusCode::InternalError as i32,
            ..Default::default()
        })?;

        let connections_file = paths.connections_file.ok_or_else(|| DriverException {
            message: "Configuration path for connections file is unavailable".to_string(),
            status_code: StatusCode::InternalError as i32,
            ..Default::default()
        })?;

        Ok(ConfigGetPathsResponse {
            config_file: config_file.to_string_lossy().into_owned(),
            connections_file: connections_file.to_string_lossy().into_owned(),
        })
    }

    // -- Telemetry operations --

    #[instrument(name = "DatabaseDriverV1::telemetry_send_api_usage", skip(self, input))]
    async fn telemetry_send_api_usage(
        &self,
        input: TelemetrySendApiUsageRequest,
    ) -> Result<TelemetrySendResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;
        let handle = Handle::from(conn_handle);

        if let Some(conn_span) = self.driver.telemetry_span(handle).await {
            let _guard = conn_span.enter();
            crate::telemetry::record_api_call(&input.api_method);
        }

        Ok(TelemetrySendResponse {})
    }

    #[instrument(
        name = "DatabaseDriverV1::telemetry_send_wrapper_error",
        skip(self, input)
    )]
    async fn telemetry_send_wrapper_error(
        &self,
        input: TelemetrySendWrapperErrorRequest,
    ) -> Result<TelemetrySendResponse, DriverException> {
        let conn_handle = required(input.conn_handle, "Connection handle is required")?;
        let handle = Handle::from(conn_handle);

        if let Some(conn_span) = self.driver.telemetry_span(handle).await {
            let _guard = conn_span.enter();
            crate::telemetry::record_exception(&input.exception_type, &input.error_source);
        }

        Ok(TelemetrySendResponse {})
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

pub type DatabaseDriverClient =
    crate::protobuf::generated::database_driver_v1::DatabaseDriverClient<
        crate::protobuf::apis::RustTransport,
    >;

pub use crate::apis::database_driver_v1::DriverProviders;

pub fn database_driver_client() -> DatabaseDriverClient {
    database_driver_client_with(DriverProviders::default())
}

pub fn database_driver_client_with(providers: DriverProviders) -> DatabaseDriverClient {
    DatabaseDriverClient::new(crate::protobuf::apis::RustTransport::new_with(providers))
}

// Synchronous convenience wrappers used by Rust test helpers and small
// in-process smoke tests. Production callers should prefer the async
// client methods directly.
static BLOCKING_CLIENT_RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create blocking protobuf client runtime")
});

type BlockingProtoError = Box<proto_utils::ProtoError<DriverException>>;
type BlockingProtoResult<T> = Result<T, BlockingProtoError>;

fn block_on_client_call<F, T>(future: F) -> BlockingProtoResult<T>
where
    F: Future<Output = Result<T, proto_utils::ProtoError<DriverException>>>,
{
    BLOCKING_CLIENT_RUNTIME.block_on(future).map_err(Box::new)
}

/// Blocking adapters for synchronous Rust test/support code that drives
/// the in-process protobuf client. Production async paths should call
/// the generated async client methods directly.
#[allow(clippy::result_large_err)]
pub trait DatabaseDriverClientBlockingExt {
    fn database_new_blocking(
        &self,
        input: DatabaseNewRequest,
    ) -> BlockingProtoResult<DatabaseNewResponse>;
    fn database_init_blocking(
        &self,
        input: DatabaseInitRequest,
    ) -> BlockingProtoResult<DatabaseInitResponse>;
    fn connection_new_blocking(
        &self,
        input: ConnectionNewRequest,
    ) -> BlockingProtoResult<ConnectionNewResponse>;
    fn connection_init_blocking(
        &self,
        input: ConnectionInitRequest,
    ) -> BlockingProtoResult<ConnectionInitResponse>;
    fn connection_set_options_blocking(
        &self,
        input: ConnectionSetOptionsRequest,
    ) -> BlockingProtoResult<ConnectionSetOptionsResponse>;
    fn statement_new_blocking(
        &self,
        input: StatementNewRequest,
    ) -> BlockingProtoResult<StatementNewResponse>;
    fn statement_execute_query_blocking(
        &self,
        input: StatementExecuteQueryRequest,
    ) -> BlockingProtoResult<ExecuteQueryResponse>;
    fn statement_set_sql_query_blocking(
        &self,
        input: StatementSetSqlQueryRequest,
    ) -> BlockingProtoResult<StatementSetSqlQueryResponse>;
    fn statement_set_options_blocking(
        &self,
        input: StatementSetOptionsRequest,
    ) -> BlockingProtoResult<StatementSetOptionsResponse>;
    fn statement_release_blocking(
        &self,
        input: StatementReleaseRequest,
    ) -> BlockingProtoResult<StatementReleaseResponse>;
    fn statement_result_chunks_blocking(
        &self,
        input: StatementResultChunksRequest,
    ) -> BlockingProtoResult<StatementResultChunksResponse>;
    fn database_fetch_chunk_blocking(
        &self,
        input: DatabaseFetchChunkRequest,
    ) -> BlockingProtoResult<DatabaseFetchChunkResponse>;
    fn connection_release_blocking(
        &self,
        input: ConnectionReleaseRequest,
    ) -> BlockingProtoResult<ConnectionReleaseResponse>;
    fn database_release_blocking(
        &self,
        input: DatabaseReleaseRequest,
    ) -> BlockingProtoResult<DatabaseReleaseResponse>;
    fn connection_close_blocking(
        &self,
        input: ConnectionCloseRequest,
    ) -> BlockingProtoResult<ConnectionCloseResponse>;
    fn connection_is_closed_blocking(
        &self,
        input: ConnectionIsClosedRequest,
    ) -> BlockingProtoResult<ConnectionIsClosedResponse>;
    fn connection_get_info_blocking(
        &self,
        input: ConnectionGetInfoRequest,
    ) -> BlockingProtoResult<ConnectionGetInfoResponse>;
    fn statement_get_result_set_blocking(
        &self,
        input: StatementGetResultSetRequest,
    ) -> Result<ResultSetResponse, proto_utils::ProtoError<DriverException>>;
    fn connection_get_result_set_blocking(
        &self,
        input: ConnectionGetResultSetRequest,
    ) -> Result<ResultSetResponse, proto_utils::ProtoError<DriverException>>;
}

#[allow(clippy::result_large_err)]
impl DatabaseDriverClientBlockingExt for DatabaseDriverClient {
    fn database_new_blocking(
        &self,
        input: DatabaseNewRequest,
    ) -> BlockingProtoResult<DatabaseNewResponse> {
        block_on_client_call(self.database_new(input))
    }

    fn database_init_blocking(
        &self,
        input: DatabaseInitRequest,
    ) -> BlockingProtoResult<DatabaseInitResponse> {
        block_on_client_call(self.database_init(input))
    }

    fn connection_new_blocking(
        &self,
        input: ConnectionNewRequest,
    ) -> BlockingProtoResult<ConnectionNewResponse> {
        block_on_client_call(self.connection_new(input))
    }

    fn connection_init_blocking(
        &self,
        input: ConnectionInitRequest,
    ) -> BlockingProtoResult<ConnectionInitResponse> {
        block_on_client_call(self.connection_init(input))
    }

    fn connection_set_options_blocking(
        &self,
        input: ConnectionSetOptionsRequest,
    ) -> BlockingProtoResult<ConnectionSetOptionsResponse> {
        block_on_client_call(self.connection_set_options(input))
    }

    fn statement_new_blocking(
        &self,
        input: StatementNewRequest,
    ) -> BlockingProtoResult<StatementNewResponse> {
        block_on_client_call(self.statement_new(input))
    }

    fn statement_execute_query_blocking(
        &self,
        input: StatementExecuteQueryRequest,
    ) -> BlockingProtoResult<ExecuteQueryResponse> {
        block_on_client_call(self.statement_execute_query(input))
    }

    fn statement_set_sql_query_blocking(
        &self,
        input: StatementSetSqlQueryRequest,
    ) -> BlockingProtoResult<StatementSetSqlQueryResponse> {
        block_on_client_call(self.statement_set_sql_query(input))
    }

    fn statement_set_options_blocking(
        &self,
        input: StatementSetOptionsRequest,
    ) -> BlockingProtoResult<StatementSetOptionsResponse> {
        block_on_client_call(self.statement_set_options(input))
    }

    fn statement_release_blocking(
        &self,
        input: StatementReleaseRequest,
    ) -> BlockingProtoResult<StatementReleaseResponse> {
        block_on_client_call(self.statement_release(input))
    }

    fn statement_result_chunks_blocking(
        &self,
        input: StatementResultChunksRequest,
    ) -> BlockingProtoResult<StatementResultChunksResponse> {
        block_on_client_call(self.statement_result_chunks(input))
    }

    fn database_fetch_chunk_blocking(
        &self,
        input: DatabaseFetchChunkRequest,
    ) -> BlockingProtoResult<DatabaseFetchChunkResponse> {
        block_on_client_call(self.database_fetch_chunk(input))
    }

    fn connection_release_blocking(
        &self,
        input: ConnectionReleaseRequest,
    ) -> BlockingProtoResult<ConnectionReleaseResponse> {
        block_on_client_call(self.connection_release(input))
    }

    fn database_release_blocking(
        &self,
        input: DatabaseReleaseRequest,
    ) -> BlockingProtoResult<DatabaseReleaseResponse> {
        block_on_client_call(self.database_release(input))
    }

    fn connection_close_blocking(
        &self,
        input: ConnectionCloseRequest,
    ) -> BlockingProtoResult<ConnectionCloseResponse> {
        block_on_client_call(self.connection_close(input))
    }

    fn connection_is_closed_blocking(
        &self,
        input: ConnectionIsClosedRequest,
    ) -> BlockingProtoResult<ConnectionIsClosedResponse> {
        block_on_client_call(self.connection_is_closed(input))
    }

    fn connection_get_info_blocking(
        &self,
        input: ConnectionGetInfoRequest,
    ) -> BlockingProtoResult<ConnectionGetInfoResponse> {
        block_on_client_call(self.connection_get_info(input))
    }

    fn statement_get_result_set_blocking(
        &self,
        input: StatementGetResultSetRequest,
    ) -> Result<ResultSetResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.statement_get_result_set(input))
    }

    fn connection_get_result_set_blocking(
        &self,
        input: ConnectionGetResultSetRequest,
    ) -> Result<ResultSetResponse, proto_utils::ProtoError<DriverException>> {
        BLOCKING_CLIENT_RUNTIME.block_on(self.connection_get_result_set(input))
    }
}
