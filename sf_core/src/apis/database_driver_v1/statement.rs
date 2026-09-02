use snafu::{OptionExt, ResultExt, Snafu};
use tokio::sync::Mutex;
use tracing::Instrument;

use super::connection::{Connection, RefreshContext, with_valid_session};
use super::error::*;
use super::global_state::DatabaseDriverV1;
use super::multistatement;
use super::query::{StageInfoRefreshContext, perform_put_get_transfer};
use super::result_set::{
    ColumnMetadata, ExecuteQueryResult, fetch_query_response_data, resolve_reader_ctx,
    response_to_descriptor,
};
use super::validation::{
    ValidationIssue, ValidationSeverity, canonicalize_setting_key, resolve_options,
    validate_statement_option_write,
};
use crate::apis::operation_ctx::{OperationCtx, run_opt, with_cleanup_opt};
use crate::config::ParamStore;
use crate::config::param_registry::ParamKey;
use crate::config::param_registry::param_names;
use crate::config::settings::Setting;
use crate::handle_manager::Handle;
use crate::rest::snowflake::{
    AbortOutcome, MissingQueryIdSnafu, QueryExecutionMode, QueryIds, QueryInput, QueryOptions,
    query_request, query_response, snowflake_abort_query, snowflake_cancel_query,
    snowflake_query_with_client,
};
use crate::utils::sync::MutexRecoverExt;

use crate::config::rest_parameters::QueryParameters;
use crate::config::retry::RetryPolicy;
use crate::rest::snowflake::async_exec::submit_statement_async;
use arrow::array::RecordBatchReader;
use serde_json::value::RawValue;
use std::sync::atomic::Ordering;
use std::{collections::HashMap, sync::Arc};

/// Upper bound on how long [`abort_query_by_request_id`] waits for the
/// abort-request to be processed before giving up — applied to both callers (the
/// operation-cancellation cleanup and the client-side query-timeout path). The
/// abort is best-effort — the executing thread observes the actual cancellation —
/// so a generous-but-finite bound is enough to keep the caller from stalling on a
/// slow/hung server.
///
/// Distinct from `OperationCtx`'s cleanup wait, which bounds how long a
/// *cancelled caller* blocks rather than the abort POST itself.
const ABORT_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Pointer to raw bytes in memory - used by query bindings
#[derive(Debug)]
pub struct DataPtr<'a> {
    /// Pointer to the data
    value: *const u8,
    /// Length of data in bytes
    length: i64,
    /// Phantom data to enforce lifetime
    _phantom: std::marker::PhantomData<&'a [u8]>,
}

// Safety: DataPtr semantically represents a &[u8] (immutable borrowed slice),
// which is Send. The raw pointer is only used for FFI interop and is always
// accessed immutably within the lifetime 'a.
//
// Callers must ensure the backing memory is not freed or mutated while
// any DataPtr (or Future holding one) is alive — including across .await
// points. All current production paths run the entire async execution
// synchronously via block_on, keeping the source data on the stack for
// the full duration, which satisfies this requirement.
unsafe impl Send for DataPtr<'_> {}

impl<'a> DataPtr<'a> {
    /// Create a new DataPtr from a raw pointer and length
    pub fn new(value: *const u8, length: i64) -> Self {
        Self {
            value,
            length,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get a slice view of the data
    pub fn slice(&self) -> &'a [u8] {
        // Safety: The caller must ensure the pointer is valid for the lifetime 'a
        unsafe { std::slice::from_raw_parts(self.value, self.length as usize) }
    }
}

#[derive(Debug)]
pub enum BindingType<'a> {
    /// JSON bindings - pointer to UTF-8 encoded JSON bytes.
    /// The bytes must represent valid UTF-8 JSON.
    Json(DataPtr<'a>),
    /// CSV bindings - pointer to raw CSV data bytes for bulk upload.
    Csv(DataPtr<'a>),
}

/// Channel by which a cancelled operation's abort-cleanup reports what it
/// achieved, so the `Cancelled` error can carry the acknowledgement out to the
/// caller.
///
/// A shared slot rather than the cleanup's return value because there is no way
/// to read that: the cleanup runs on a task spawned by
/// [`OperationCtx::with_cleanup`](crate::apis::operation_ctx::OperationCtx::with_cleanup),
/// deliberately outliving the execute future that armed it, and the future is
/// gone by the time it finishes.
///
/// `std::sync::Mutex`, not `tokio`'s: the critical sections are a single
/// assignment and a single read, and the read happens on a path that must not
/// have to `.await` to see the value.
#[derive(Clone, Debug, Default)]
struct AbortReport(Arc<std::sync::Mutex<Option<CancellationAbortResult>>>);

impl AbortReport {
    /// Record what the abort achieved. Later calls overwrite earlier ones, which
    /// is how the cleanup upgrades its own pessimistic `NotConfirmed` to the real
    /// outcome.
    fn set(&self, outcome: CancellationAbortResult) {
        *self.0.lock_recover() = Some(outcome);
    }

    /// Attach this report to `error` if it is a cancellation, leaving every other
    /// error untouched.
    ///
    /// Applied on the way out of the operation rather than where the error is
    /// raised, because the raiser ([`OperationCtx::run`]) is generic machinery
    /// that knows nothing about abort-requests.
    fn attach(&self, error: ApiError) -> ApiError {
        match error {
            ApiError::Cancelled { location, .. } => ApiError::Cancelled {
                abort: *self.0.lock_recover(),
                location,
            },
            other => other,
        }
    }
}

/// Run a query-submitting operation under `operation_ctx`, attaching to a resulting
/// cancellation whatever its abort-cleanup reported.
///
/// Shared by both entry points into `execute_query_internal` so the
/// "cancelled → attach the acknowledgement" step cannot be wired into one and
/// forgotten in the other. Safe to call after `run_opt`: `OperationCtx::run`
/// awaits registered cleanup before reporting, so in the healthy case the report
/// is already final by the time this reads it.
async fn run_reporting_abort<T, F>(
    operation_ctx: Option<&OperationCtx>,
    method: &str,
    report: &AbortReport,
    fut: F,
) -> Result<T, ApiError>
where
    F: Future<Output = Result<T, ApiError>>,
{
    run_opt(operation_ctx, method, fut)
        .await
        .map_err(|e| report.attach(e))
}

/// Result returned from async query submission (non-blocking).
pub struct AsyncExecuteResult {
    pub query_id: String,
    /// Client-generated UUID v4 sent as `?requestId=` on the submission request.
    pub request_id: uuid::Uuid,
}

impl DatabaseDriverV1 {
    pub fn statement_new(&self, conn_handle: Handle) -> Result<Handle, ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let stmt = Mutex::new(Statement::new(conn_ptr));
                let handle = self.statements.add_handle(stmt);
                Ok(handle)
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub fn statement_release(&self, stmt_handle: Handle) -> Result<(), ApiError> {
        match self.statements.delete_handle(stmt_handle) {
            true => Ok(()),
            false => InvalidArgumentSnafu {
                argument: "Failed to release statement handle".to_string(),
            }
            .fail(),
        }
    }

    pub async fn statement_set_option(
        &self,
        handle: Handle,
        key: String,
        value: Setting,
    ) -> Result<(), ApiError> {
        match self.statements.get_obj(handle) {
            Some(stmt_ptr) => {
                let mut stmt = stmt_ptr.lock().await;
                let (canonical, def) =
                    canonicalize_setting_key(self.wrapper_presets.configuration_flavor, &key);
                validate_statement_option_write(def)?;
                stmt.settings.insert(canonical, value);
                Ok(())
            }
            None => InvalidArgumentSnafu {
                argument: "Statement handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub async fn statement_set_options(
        &self,
        handle: Handle,
        options: HashMap<String, Setting>,
    ) -> Result<Vec<ValidationIssue>, ApiError> {
        match self.statements.get_obj(handle) {
            Some(stmt_ptr) => {
                let mut stmt = stmt_ptr.lock().await;
                let (resolved, issues) =
                    resolve_options(self.wrapper_presets.configuration_flavor, options);
                let error_messages: Vec<String> = issues
                    .iter()
                    .filter(|i| i.severity == ValidationSeverity::Error)
                    .map(|i| i.to_string())
                    .collect();
                if !error_messages.is_empty() {
                    return InvalidArgumentSnafu {
                        argument: error_messages.join("; "),
                    }
                    .fail();
                }
                for key in resolved.keys() {
                    let def = crate::config::param_registry::registry().resolve(key.as_str());
                    validate_statement_option_write(def)?;
                }
                for (key, value) in resolved {
                    stmt.settings.insert(key, value);
                }
                Ok(issues
                    .into_iter()
                    .filter(|i| i.severity == ValidationSeverity::Warning)
                    .collect())
            }
            None => InvalidArgumentSnafu {
                argument: "Statement handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub async fn statement_set_sql_query(
        &self,
        stmt_handle: Handle,
        query: String,
    ) -> Result<(), ApiError> {
        match self.statements.get_obj(stmt_handle) {
            Some(stmt_ptr) => {
                let mut stmt = stmt_ptr.lock().await;
                stmt.query = Some(query);
                Ok(())
            }
            None => InvalidArgumentSnafu {
                argument: "Statement handle not found".to_string(),
            }
            .fail(),
        }
    }
}

pub struct PrepareResult {
    pub stream: Box<dyn RecordBatchReader + Send>,
    pub query_id: String,
    pub columns: Vec<ColumnMetadata>,
    pub number_of_binds: i32,
    pub query: String,
    pub sql_state: Option<String>,
    pub array_bind_supported: bool,
    pub binds: Vec<ColumnMetadata>,
    pub request_id: uuid::Uuid,
}

impl DatabaseDriverV1 {
    /// Describe a statement without producing rows (`describe_only`), returning
    /// the column and bind metadata.
    pub async fn statement_prepare(
        &self,
        operation_ctx: Option<&OperationCtx>,
        stmt_handle: Handle,
    ) -> Result<PrepareResult, ApiError> {
        let stmt_ptr =
            self.statements
                .get_obj(stmt_handle)
                .with_context(|| InvalidArgumentSnafu {
                    argument: "Statement handle not found".to_string(),
                })?;
        // keep the lock for the whole execution
        let mut stmt = stmt_ptr.lock().await;

        let session_id = stmt.conn.lock().await.session_id;
        let report = AbortReport::default();
        let prepare = Box::pin(async {
            // Multi-statement query prepare is not supported. `request_id` is
            // always `Some` here — `execute_query_internal` mints one on every
            // path — so binding `Some` keeps `PrepareResult.request_id`
            // non-optional without inventing a fallback value.
            let ExecuteQueryResult::Single {
                info: rs_info,
                request_id: Some(request_id),
            } = self
                .execute_query_internal(operation_ctx, &report, &mut stmt, None, Some(true), None)
                .await?
            else {
                return InvalidArgumentSnafu {
                    argument: "Multi-statement queries cannot be prepared".to_string(),
                }
                .fail();
            };
            let stream = self.result_set_get_stream(rs_info.handle).await?;
            self.result_set_release(rs_info.handle)?;
            let query = stmt.query.clone().unwrap_or_default();

            Ok(PrepareResult {
                stream,
                query_id: rs_info.descriptor.query_id,
                columns: rs_info.descriptor.columns,
                number_of_binds: rs_info.descriptor.number_of_binds,
                query,
                sql_state: rs_info.descriptor.sql_state,
                array_bind_supported: rs_info.descriptor.array_bind_supported,
                binds: rs_info.descriptor.binds,
                request_id,
            })
        });
        run_reporting_abort(operation_ctx, "statement_prepare", &report, prepare)
            .instrument(crate::snowflake_op_span!("statement_prepare", session_id))
            .await
    }
}

impl DatabaseDriverV1 {
    pub async fn statement_execute_query<'a>(
        &self,
        operation_ctx: Option<&OperationCtx>,
        stmt_handle: Handle,
        bindings: Option<BindingType<'a>>,
        timeout_seconds: Option<u32>,
    ) -> Result<ExecuteQueryResult, ApiError> {
        let stmt_ptr =
            self.statements
                .get_obj(stmt_handle)
                .with_context(|| InvalidArgumentSnafu {
                    argument: "Statement handle not found".to_string(),
                })?;
        // keep the lock for the whole execution
        let mut stmt = stmt_ptr.lock().await;

        let session_id = stmt.conn.lock().await.session_id;
        let report = AbortReport::default();
        run_reporting_abort(
            operation_ctx,
            "statement_execute_query",
            &report,
            Box::pin(self.execute_query_internal(
                operation_ctx,
                &report,
                &mut stmt,
                bindings,
                None,
                timeout_seconds,
            )),
        )
        .instrument(crate::snowflake_op_span!(
            "statement_execute_query",
            session_id
        ))
        .await
    }

    /// Uploads a large-bind CSV payload to the bind stage, returning its stage
    /// path. `operation_ctx` matters only for a payload large enough to take the multipart
    /// path, which has an abort to register.
    async fn upload_csv_bindings_to_stage(
        &self,
        operation_ctx: Option<&OperationCtx>,
        conn_arc: &Arc<Mutex<super::connection::Connection>>,
        http_client: &reqwest::Client,
        query_parameters: &QueryParameters,
        retry_policy: &RetryPolicy,
        csv_bytes: &[u8],
    ) -> Result<String, ApiError> {
        let (use_s3_regional_url_session_param, flags, put_get_policy) = {
            let conn = conn_arc.lock().await;
            let regional = conn.use_s3_regional_url_session_param().await;
            let flags = crate::stage_binding::StageBindingFlags {
                stage_state: conn.stage_state.clone(),
            };
            let put_get_policy = RetryPolicy::put_get(&conn.connection_seed);
            (regional, flags, put_get_policy)
        };

        let mut upload_refresh = RefreshContext::from_arc(conn_arc).await?;
        let session_token = upload_refresh.refresh_token(None).await?;

        let stage_ctx = crate::stage_binding::StageBindingContext {
            client: http_client,
            query_parameters,
            session_token: &session_token,
            retry_policy,
            put_get_policy: &put_get_policy,
            use_s3_regional_url_session_param,
            crl_worker: self.crl_worker.clone(),
            cleanup: operation_ctx.map(OperationCtx::cleanup_scope),
        };
        let request_id = uuid::Uuid::new_v4();
        crate::stage_binding::upload_csv_bindings(&stage_ctx, &flags, request_id, csv_bytes)
            .await
            .context(StageBindingSnafu)
    }

    /// PUT/GET disable gate (legacy `enablePutGet` + `JDBC_ENABLE_PUT_GET`
    /// parity): reject file transfers before any server dispatch when the client
    /// property or the server session parameter has disabled them. Both flags are
    /// JDBC-specific, so only wrappers that opt in via `honor_put_get_disable`
    /// (JDBC) consult them.
    async fn ensure_file_transfer_allowed(
        &self,
        query: &str,
        conn: &Arc<Mutex<Connection>>,
    ) -> Result<(), ApiError> {
        if self.wrapper_presets.honor_put_get_disable && is_file_transfer(query) {
            let conn = conn.lock().await;
            if !conn.enable_put_get().await {
                return FileTransfersDisabledSnafu.fail();
            }
        }
        Ok(())
    }

    async fn execute_query_internal<'a>(
        &self,
        operation_ctx: Option<&OperationCtx>,
        report: &AbortReport,
        stmt: &mut Statement,
        bindings: Option<BindingType<'a>>,
        describe_only: Option<bool>,
        timeout_seconds: Option<u32>,
    ) -> Result<ExecuteQueryResult, ApiError> {
        let query = extract_query(stmt)?;

        self.ensure_file_transfer_allowed(&query, &stmt.conn)
            .await?;

        let (query_parameters, http_client, retry_policy) = query_context(&stmt.conn).await?;

        let execution_mode = stmt.execution_mode(Some(&query));

        let mut query_parameter_map =
            build_query_parameters_with_timeout(&stmt.settings, timeout_seconds);

        let conn_arc = stmt.conn.clone();

        // Mint the requestId, the whole function shares one identity for the query:
        // the submission below, the abort fired on cancellation, and the abort fired on client-side
        // timeout all key on this one value. It stays a plain local.
        let request_id = uuid::Uuid::new_v4();

        let (query_bindings, csv_bytes) = split_bindings(&bindings)?;

        let bind_stage_path = if let Some(bytes) = csv_bytes {
            let path = self
                .upload_csv_bindings_to_stage(
                    operation_ctx,
                    &conn_arc,
                    &http_client,
                    &query_parameters,
                    &retry_policy,
                    bytes,
                )
                .await?;
            inject_timestamp_input_format_auto(&mut query_parameter_map);
            Some(path)
        } else {
            None
        };

        let (query_context, query_deadline) = {
            let conn = conn_arc.lock().await;
            let qctx = conn.query_context_cache.get_query_context_snapshot().await;
            let deadline = conn
                .timeout_config
                .query_timeout
                .map(|budget| (budget, tokio::time::Instant::now() + budget));
            (qctx, deadline)
        };

        let query_input = QueryInput {
            sql: query.clone(),
            bindings: query_bindings,
            bind_stage: bind_stage_path,
            describe_only,
            query_parameters: query_parameter_map,
            query_context,
        };

        // Abort the query server-side if the operation is cancelled while the
        // request below is in flight. The identity is moved into the cleanup
        // rather than looked up when it runs: the cleanup task outlives this
        // future by design, so it cannot borrow anything from it.
        //
        // Armed only here, once the query is actually about to be sent. Until
        // then there is nothing for the server to abort, so a cancel arriving
        // during (say) a large bind-variable upload correctly issues no abort.
        let abort_cleanup = {
            let (conn_arc, req, sql) = (conn_arc.clone(), request_id.to_string(), query.clone());
            let report = report.clone();
            async move {
                // Record "issued, result unknown" *before* awaiting, so that a
                // cleanup which never gets to finish (it errored, or the caller
                // was released after `CLEANUP_WAIT`) is still distinguishable
                // from one that was never armed at all. That is what lets an
                // unset report mean exactly "no abort was issued".
                report.set(CancellationAbortResult::NotConfirmed);
                match abort_query_by_request_id(&conn_arc, req, sql).await {
                    Ok(outcome) => {
                        report.set(match outcome {
                            AbortOutcome::Aborted => CancellationAbortResult::Aborted,
                            AbortOutcome::NotRunning => CancellationAbortResult::NotRunning,
                        });
                        tracing::debug!(?outcome, "aborted query after cancellation");
                    }
                    // Best-effort: the caller is already being told the operation
                    // was cancelled, so there is nobody to return this to — the
                    // report keeps `NotConfirmed` set above.
                    Err(error) => {
                        tracing::warn!(%error, "failed to abort query after cancellation")
                    }
                }
            }
        };

        // Boxed for the same reason as the outer `run_opt` call — see clippy.toml.
        let response = with_cleanup_opt(
            operation_ctx,
            abort_cleanup,
            Box::pin(async {
                // Named `refresh_ctx`, not `operation_ctx`: shadowing the operation operation_ctx here
                // would silently hide it from anything added inside this loop.
                let mut refresh_ctx = RefreshContext::from_arc(&conn_arc).await?;
                let mut last_error = None;
                loop {
                    let session_token = refresh_ctx.refresh_token(last_error).await?;
                    let query_call = snowflake_query_with_client(
                        &http_client,
                        query_parameters.clone(),
                        session_token.reveal(),
                        query_input.clone(),
                        QueryOptions {
                            retry_policy: retry_policy.clone(),
                            execution_mode,
                            request_id: Some(request_id),
                        },
                    );
                    let result = if let Some((budget, deadline)) = query_deadline {
                        match tokio::time::timeout_at(deadline, query_call).await {
                            Ok(inner) => inner,
                            Err(_) => {
                                // Aborted inline rather than by cancelling the token so
                                // the caller still gets `QueryTimeout` (distinct from
                                // `Cancelled`; ODBC maps them to different SQLSTATEs),
                                // and so it also works on the sync paths that have no
                                // operation_ctx.
                                // Fire-and-forget the abort to avoid blocking the timeout error
                                // return on slow/hung abort requests. Spawn it rather than awaiting
                                // inline so the test can observe a timeout at the configured
                                // threshold rather than timeout + abort latency.
                                let conn_arc_clone = conn_arc.clone();
                                let request_id_clone = request_id.to_string();
                                let query_clone = query.clone();
                                tokio::spawn(async move {
                                    match abort_query_by_request_id(
                                        &conn_arc_clone,
                                        request_id_clone,
                                        query_clone,
                                    )
                                    .await
                                    {
                                        Ok(_) => {
                                            tracing::debug!(
                                                "successfully aborted query after timeout"
                                            );
                                        }
                                        Err(error) => {
                                            tracing::warn!(
                                                %error,
                                                "failed to abort query after client-side timeout"
                                            );
                                        }
                                    }
                                });
                                return Err(QueryTimeoutSnafu { budget, request_id }.build());
                            }
                        }
                    } else {
                        query_call.await
                    };
                    match result {
                        Ok(result) => break Ok(result),
                        Err(e) => {
                            // Update QCC from failed queries — the server may include
                            // queryContext even in error responses.
                            if let RestError::QueryFailed {
                                query_context: Some(qctx),
                                ..
                            } = &e
                            {
                                let mut conn = conn_arc.lock().await;
                                conn.query_context_cache
                                    .update_query_context_cache(Some(qctx), None)
                                    .await;
                            }
                            last_error = Some(e);
                        }
                    }
                }
            }),
        )
        .await?;

        if response.success {
            let mut conn = conn_arc.lock().await;
            conn.update_session_params_cache(
                &query,
                response.data.parameters.as_ref(),
                &super::connection::FinalSessionNames {
                    database: response.data.final_database_name.clone(),
                    schema: response.data.final_schema_name.clone(),
                    warehouse: response.data.final_warehouse_name.clone(),
                    role: response.data.final_role_name.clone(),
                },
            )
            .await;
            conn.query_context_cache
                .update_query_context_cache(
                    response.data.query_context.as_ref(),
                    response.data.parameters.as_ref(),
                )
                .await;
        }

        stmt.state = StatementState::Executed;
        let skip_upload_on_content_match = stmt
            .settings
            .get_bool(param_names::SKIP_UPLOAD_ON_CONTENT_MATCH)
            .unwrap_or(false);
        // `PUT_FASTFAIL`/`GET_FASTFAIL` have no registry default, so this is `None`
        // unless set on the statement. Session/connection value and the wrapper
        // preset are applied later in `extract_rowset_data`, once `conn` is locked.
        let put_fastfail = stmt.settings.get_bool(param_names::PUT_FASTFAIL);
        let get_fastfail = stmt.settings.get_bool(param_names::GET_FASTFAIL);

        let data = response.data;
        let descriptor = response_to_descriptor(&data, &self.wrapper_presets);
        if let Some(multi) =
            multistatement::try_into_multi_result(&data, descriptor.clone(), Some(request_id))
        {
            return Ok(multi);
        }
        let rowset_data = self
            .extract_rowset_data(
                operation_ctx,
                &conn_arc,
                data,
                Some((query, query_parameters)),
                skip_upload_on_content_match,
                put_fastfail,
                get_fastfail,
            )
            .await?;
        let reader_ctx = resolve_reader_ctx(&conn_arc).await?;
        Ok(self.build_execute_result(rowset_data, descriptor, reader_ctx, Some(request_id)))
    }

    /// Decodes the row data from a query response, dispatching PUT/GET when
    /// `data.command` is set. `refresh_sql` (the original SQL + parameters)
    /// lets the file manager re-issue the PUT/GET to refresh stage
    /// credentials mid-transfer. Callers pass `Some` when they can supply the
    /// originating SQL (the sync execute path always can; the async
    /// result-fetch path only when the response carries `sqlText`) and `None`
    /// otherwise, which disables stage-info refresh for that transfer.
    ///
    /// `operation_ctx` is forwarded to the PUT/GET transfer so a cancel aborts the in-flight
    /// cloud transfer rather than only dropping it locally.
    // One arg over the 7-arg clippy threshold, since `operation_ctx` joined the three
    // statement-level PUT/GET overrides; mirrors `upload_to_gcs_or_skip`.
    #[allow(clippy::too_many_arguments)]
    async fn extract_rowset_data(
        &self,
        operation_ctx: Option<&OperationCtx>,
        conn: &Arc<Mutex<Connection>>,
        data: query_response::Data,
        refresh_sql: Option<(String, QueryParameters)>,
        skip_upload_on_content_match: bool,
        put_fastfail_override: Option<bool>,
        get_fastfail_override: Option<bool>,
    ) -> Result<query_response::RowsetData, ApiError> {
        match data.command.as_deref() {
            Some(command) => {
                // PUT/GET refresh context: lets the file manager re-issue
                // this SQL when stage credentials expire mid-transfer.
                let stage_info_refresh_context =
                    refresh_sql.map(|(sql, query_parameters)| StageInfoRefreshContext {
                        sql,
                        query_parameters,
                        conn: conn.clone(),
                    });
                // Late-bind connection params so post-init `set_option`
                // overrides take effect (mirrors `LogoutConfig`).
                let (
                    retry_policy,
                    use_s3_regional_url_session_param,
                    unsafe_file_write,
                    tls_config,
                    proxy_config,
                    put_fastfail,
                    get_fastfail,
                ) = {
                    let conn = conn.lock().await;
                    (
                        crate::config::retry::RetryPolicy::put_get(&conn.connection_seed),
                        conn.use_s3_regional_url_session_param().await,
                        conn.unsafe_file_write(),
                        conn.tls_config(),
                        conn.proxy_config(),
                        // precedence: statement setting > session override > connection seed > wrapper preset
                        put_fastfail_override
                            .or_else(|| conn.put_fastfail())
                            .unwrap_or(self.wrapper_presets.put_get_fastfail_default),
                        get_fastfail_override
                            .or_else(|| conn.get_fastfail())
                            .unwrap_or(self.wrapper_presets.put_get_fastfail_default),
                    )
                };
                perform_put_get_transfer(
                    operation_ctx,
                    command,
                    &data,
                    &self.wrapper_presets,
                    &retry_policy,
                    stage_info_refresh_context,
                    use_s3_regional_url_session_param,
                    skip_upload_on_content_match,
                    put_fastfail,
                    get_fastfail,
                    unsafe_file_write,
                    tls_config,
                    proxy_config,
                    self.crl_worker.clone(),
                )
                .await
                .context(QueryResponseProcessSnafu)
            }
            None => Ok(data.into_rowset_data()),
        }
    }

    /// Execute query asynchronously (non-blocking) — returns immediately with query_id.
    ///
    /// Cancelling aborts the submitted query server-side, for the same reason
    /// `statement_execute_query` does, and one specific to this path: the caller
    /// never receives the `query_id`, so a query left running is one they cannot
    /// poll, abort, or even name. The abort is armed only around the submission, so
    /// a cancel during the bind-stage upload issues none — nothing was sent yet.
    pub async fn statement_execute_async<'a>(
        &self,
        operation_ctx: Option<&OperationCtx>,
        stmt_handle: Handle,
        bindings: Option<BindingType<'a>>,
    ) -> Result<AsyncExecuteResult, ApiError> {
        let report = AbortReport::default();
        // `async move`, so `bindings` is owned by this future rather than borrowed
        // from the frame: `BindingType` wraps a raw pointer and is `Send` but not
        // `Sync`, and the generated server trait requires a `Send` future — which a
        // borrow of a non-`Sync` value would not satisfy.
        let cleanup_report = report.clone();
        let submit = Box::pin(async move {
            let stmt_ptr =
                self.statements
                    .get_obj(stmt_handle)
                    .with_context(|| InvalidArgumentSnafu {
                        argument: "Statement handle not found".to_string(),
                    })?;

            let mut stmt = stmt_ptr.lock().await;

            let query = extract_query(&stmt)?;

            self.ensure_file_transfer_allowed(&query, &stmt.conn)
                .await?;

            let (query_parameters, http_client, retry_policy) = query_context(&stmt.conn).await?;
            let mut query_parameter_map = build_query_parameters(&stmt.settings);
            let conn_arc = stmt.conn.clone();

            let (query_bindings, csv_bytes) = split_bindings(&bindings)?;

            let bind_stage_path = if let Some(bytes) = csv_bytes {
                let path = self
                    .upload_csv_bindings_to_stage(
                        operation_ctx,
                        &conn_arc,
                        &http_client,
                        &query_parameters,
                        &retry_policy,
                        bytes,
                    )
                    .await?;
                inject_timestamp_input_format_auto(&mut query_parameter_map);
                Some(path)
            } else {
                None
            };

            let query_context = {
                let conn = stmt.conn.lock().await;
                conn.query_context_cache.get_query_context_snapshot().await
            };
            let query_input = QueryInput {
                sql: query.clone(),
                bindings: query_bindings,
                bind_stage: bind_stage_path,
                describe_only: None,
                query_parameters: query_parameter_map,
                query_context,
            };
            let request_id = uuid::Uuid::new_v4();

            // Same shape as `execute_query_internal`'s: the identity is moved in
            // rather than looked up when it runs, because the cleanup task
            // outlives this future by design.
            let abort_cleanup = {
                let (conn_arc, req, sql) =
                    (conn_arc.clone(), request_id.to_string(), query.clone());
                let report = cleanup_report;
                async move {
                    // Recorded before awaiting so a cleanup that never finishes is
                    // still distinguishable from one that was never armed.
                    report.set(CancellationAbortResult::NotConfirmed);
                    match abort_query_by_request_id(&conn_arc, req, sql).await {
                        Ok(outcome) => {
                            report.set(match outcome {
                                AbortOutcome::Aborted => CancellationAbortResult::Aborted,
                                AbortOutcome::NotRunning => CancellationAbortResult::NotRunning,
                            });
                            tracing::debug!(
                                ?outcome,
                                "aborted async submission after cancellation"
                            );
                        }
                        Err(error) => {
                            tracing::warn!(
                                %error,
                                "failed to abort async submission after cancellation"
                            )
                        }
                    }
                }
            };

            let result = with_cleanup_opt(
                operation_ctx,
                abort_cleanup,
                Box::pin(async {
                    // Named `refresh_ctx`, not `operation_ctx`: shadowing the operation operation_ctx
                    // here would silently hide it from anything added inside this
                    // loop.
                    let mut refresh_ctx = RefreshContext::from_arc(&conn_arc).await?;
                    let mut last_error = None;
                    loop {
                        let session_token = refresh_ctx.refresh_token(last_error).await?;
                        match submit_statement_async(
                            &http_client,
                            &query_parameters,
                            session_token.reveal(),
                            &query_input,
                            request_id,
                            &retry_policy,
                        )
                        .await
                        {
                            Ok(submit_result) => break Ok(submit_result),
                            Err(e) => {
                                last_error = Some(e);
                            }
                        }
                    }
                }),
            )
            .await?;

            // A 200 that carries no usable id means GS accepted the submission but
            // the caller cannot name the query it created. `with_cleanup_opt`'s
            // cleanup covers cancellation only and is already disarmed by the time
            // the submit future returns, so the abort has to be fired here or the
            // query is orphaned — the same leak a cancelled submit would cause,
            // reached without any cancellation.
            //
            // `filter` rather than a bare `None` check: an empty string is not a
            // usable id either, and would otherwise be handed back as a valid one.
            let query_id = match result.query_id.clone().filter(|id| !id.is_empty()) {
                Some(id) => id,
                None => {
                    abort_unnameable_submission(&conn_arc, request_id, &query).await;
                    return MissingQueryIdSnafu {
                        ids: QueryIds {
                            request_id: Some(request_id),
                            query_id: result.query_id,
                        },
                    }
                    .fail()
                    .context(QuerySnafu);
                }
            };

            {
                let mut conn = stmt.conn.lock().await;
                conn.query_context_cache
                    .update_query_context_cache(
                        result.response.data.query_context.as_ref(),
                        result.response.data.parameters.as_ref(),
                    )
                    .await;
            }

            stmt.state = StatementState::Executed;

            Ok(AsyncExecuteResult {
                query_id,
                request_id,
            })
        });
        run_reporting_abort(operation_ctx, "statement_execute_async", &report, submit).await
    }

    /// Cancelling abandons the fetch and, on the PUT/GET branch, aborts the
    /// in-flight cloud transfer through `operation_ctx`. Nothing is aborted server-side:
    /// the query this reads has already finished, so there is no running query
    /// left to abort as there is for `statement_execute_query`.
    pub async fn connection_get_query_result(
        &self,
        operation_ctx: Option<&OperationCtx>,
        conn_handle: Handle,
        query_id: String,
    ) -> Result<ExecuteQueryResult, ApiError> {
        let session_id = self.session_id_for_conn(conn_handle).await;
        let fetch = Box::pin(async {
            let conn_ptr = self.connections.get_obj(conn_handle).with_context(|| {
                InvalidArgumentSnafu {
                    argument: "Connection handle not found".to_string(),
                }
            })?;

            let data = fetch_query_response_data(&conn_ptr, &query_id).await?;
            let descriptor = response_to_descriptor(&data, &self.wrapper_presets);
            // No submission UUID on this path: the result is fetched by query ID
            // via an endpoint that sends no `?requestId=`.
            if let Some(multi) =
                multistatement::try_into_multi_result(&data, descriptor.clone(), None)
            {
                return Ok(multi);
            }

            // Build a refresh context when the response carries the original
            // SQL (`sqlText`). Async PUT/GET retrieval goes through
            // `monitoring/queries/{queryId}/result` which populates `sqlText`;
            // without it we cannot re-issue the command, so stage-info refresh
            // is disabled for this path.
            let refresh_sql = match data.command.as_deref() {
                Some(_) => {
                    let (query_parameters, _http_client, _retry_policy) =
                        query_context(&conn_ptr).await?;
                    match data.sql_text.clone() {
                        Some(sql) => Some((sql, query_parameters)),
                        None => {
                            tracing::debug!(
                                "async PUT/GET response missing sqlText; stage-info refresh disabled"
                            );
                            None
                        }
                    }
                }
                None => None,
            };
            // This is the path a PUT/GET submitted earlier is retrieved on, so
            // `operation_ctx` is forwarded: a cancel here tears down the cloud transfer
            // rather than only abandoning this request. The three `None`s that
            // follow `data` are unrelated to it — there is no `Statement` on this
            // path to carry per-statement PUT/GET overrides, so
            // `extract_rowset_data` falls back to connection/session and then the
            // wrapper preset. `skip_upload_on_content_match` is defensively false.
            let rowset_data = self
                .extract_rowset_data(operation_ctx, &conn_ptr, data, refresh_sql, false, None, None)
                .await?;
            let reader_ctx = resolve_reader_ctx(&conn_ptr).await?;
            Ok(self.build_execute_result(rowset_data, descriptor, reader_ctx, None))
        }
        .instrument(crate::snowflake_op_span!(
            "connection_get_query_result",
            session_id
        )));
        run_opt(operation_ctx, "connection_get_query_result", fetch).await
    }

    pub async fn connection_abort_query(
        &self,
        conn_handle: Handle,
        query_id: String,
    ) -> Result<AbortOutcome, ApiError> {
        let session_id = self.session_id_for_conn(conn_handle).await;
        async {
            let conn_ptr =
                self.connections
                    .get_obj(conn_handle)
                    .with_context(|| InvalidArgumentSnafu {
                        argument: "Connection handle not found".to_string(),
                    })?;

            let (query_parameters, http_client, _) = query_context(&conn_ptr).await?;

            with_valid_session(&conn_ptr, |token| {
                let http_client = &http_client;
                let query_parameters = &query_parameters;
                let query_id = &query_id;
                async move {
                    snowflake_abort_query(http_client, query_parameters, token.reveal(), query_id)
                        .await
                }
            })
            .await
        }
        .instrument(crate::snowflake_op_span!(
            "connection_abort_query",
            session_id
        ))
        .await
    }
}

/// Abort a submitted query by the `requestId` it was sent with, via
/// `POST /queries/v1/abort-request`.
///
/// Both callers are inside
/// [`execute_query_internal`](DatabaseDriverV1::execute_query_internal), which is
/// what makes "at most one abort per query" structural rather than something to
/// coordinate: the cancellation cleanup and the client-side query-timeout path
/// are mutually exclusive (whichever ends the query is the one that aborts it),
/// and each holds the `requestId` as a local rather than reading it from shared
/// state a second caller could also find.
///
/// Aborts are idempotent server-side anyway — a repeat for the same `requestId`
/// comes back `000605`/not-executing — so a retry costs a round trip, not
/// correctness.
/// Abort a submission GS accepted but whose `query_id` never reached the caller.
///
/// Distinct from the cancellation cleanup: that one is armed against the operation
/// token and disarmed as soon as the submit future returns, so it cannot cover a
/// submit that *succeeded* at the HTTP level while yielding nothing the caller can
/// abort with. Best-effort and bounded by [`ABORT_REQUEST_TIMEOUT`]; the outcome is
/// logged rather than returned, because the error the caller receives is not a
/// cancellation and has nowhere to carry an abort acknowledgement.
async fn abort_unnameable_submission(
    conn_arc: &Arc<Mutex<Connection>>,
    request_id: uuid::Uuid,
    sql: &str,
) {
    match abort_query_by_request_id(conn_arc, request_id.to_string(), sql.to_string()).await {
        Ok(outcome) => tracing::warn!(
            ?outcome,
            %request_id,
            "aborted an async submission that returned no query id"
        ),
        Err(error) => tracing::warn!(
            %error,
            %request_id,
            "failed to abort an async submission that returned no query id"
        ),
    }
}

async fn abort_query_by_request_id(
    conn_arc: &Arc<Mutex<Connection>>,
    request_id: String,
    sql_text: String,
) -> Result<AbortOutcome, ApiError> {
    let (query_parameters, http_client, _) = query_context(conn_arc).await?;

    let cancel = with_valid_session(conn_arc, |token| {
        let http_client = &http_client;
        let query_parameters = &query_parameters;
        let request_id = &request_id;
        let sql_text = &sql_text;
        async move {
            snowflake_cancel_query(
                http_client,
                query_parameters,
                token.reveal(),
                request_id,
                sql_text,
            )
            .await
        }
    });

    // Bound the abort POST so a slow/hung server cannot stall the caller. A
    // cancel only requires the request to be processed, not awaited to
    // completion.
    tokio::time::timeout(ABORT_REQUEST_TIMEOUT, cancel)
        .await
        .unwrap_or_else(|_elapsed| {
            tracing::warn!(
                timeout_secs = ABORT_REQUEST_TIMEOUT.as_secs(),
                "abort-request timed out"
            );
            Err(CancelTimeoutSnafu {
                timeout: ABORT_REQUEST_TIMEOUT,
                request_id,
            }
            .build())
        })
}

/// Lock the connection and extract the transport parameters, HTTP client, and retry policy
/// needed to issue a query.
///
/// Also rejects query execution if close() has already been called on the connection.
pub(super) async fn query_context(
    conn: &Arc<Mutex<Connection>>,
) -> Result<(QueryParameters, reqwest::Client, RetryPolicy), ApiError> {
    let conn = conn.lock().await;
    // Reject query execution if close() has been called
    if conn.is_closed.load(Ordering::SeqCst) {
        return ConnectionClosedSnafu {}.fail();
    }
    Ok((
        conn.query_transport_parameters()?,
        conn.http_client
            .clone()
            .context(ConnectionNotInitializedSnafu)?,
        RetryPolicy::query(&conn.connection_seed),
    ))
}

/// Return the SQL text attached to a statement, or an error if none has been set.
fn extract_query(stmt: &Statement) -> Result<String, ApiError> {
    stmt.query.clone().with_context(|| InvalidArgumentSnafu {
        argument: "Query not found".to_string(),
    })
}

pub struct Statement {
    pub state: StatementState,
    pub(crate) settings: ParamStore,
    pub query: Option<String>,
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
            settings: ParamStore::new(),
            state: StatementState::Initialized,
            query: None,
            conn,
        }
    }

    pub(crate) fn execution_mode(&self, query: Option<&str>) -> QueryExecutionMode {
        let async_requested = self
            .settings
            .get(param_names::ASYNC_EXECUTION)
            .and_then(parse_bool_setting)
            .unwrap_or(false);

        if async_requested && !query.is_some_and(is_file_transfer) {
            return QueryExecutionMode::Async;
        }
        QueryExecutionMode::Blocking
    }
}

fn setting_to_json_value(setting: &Setting) -> serde_json::Value {
    match setting {
        Setting::String(s) => serde_json::Value::String(s.clone()),
        Setting::Int(i) => serde_json::json!(i),
        Setting::Double(d) => serde_json::json!(d),
        Setting::Bool(b) => serde_json::Value::Bool(*b),
        Setting::Bytes(b) => serde_json::Value::String(String::from_utf8_lossy(b).into_owned()),
    }
}

/// Server-side parameter names forwarded in the query request.
/// Each entry maps a local `ParamKey` to the Snowflake server-side parameter name.
/// Statement-scoped options forwarded to GS in the query-request `parameters`
/// map. Maps the local `ParamKey` to the server-side parameter name. Only
/// registry-recognized, server-meaningful options belong here (the client-only
/// `skip_upload_on_content_match` is intentionally excluded).
const QUERY_PARAMETER_NAMES: &[(ParamKey, &str)] = &[
    (param_names::MULTI_STATEMENT_COUNT, "MULTI_STATEMENT_COUNT"),
    (param_names::QUERY_TAG, "QUERY_TAG"),
];

fn build_query_parameters(settings: &ParamStore) -> Option<HashMap<String, serde_json::Value>> {
    let mut params = HashMap::new();
    for (key, server_name) in QUERY_PARAMETER_NAMES {
        if let Some(setting) = settings.get(*key) {
            params.insert(server_name.to_string(), setting_to_json_value(setting));
        }
    }
    if params.is_empty() {
        None
    } else {
        Some(params)
    }
}

fn build_query_parameters_with_timeout(
    settings: &ParamStore,
    timeout_seconds: Option<u32>,
) -> Option<HashMap<String, serde_json::Value>> {
    let mut params = build_query_parameters(settings).unwrap_or_default();
    if let Some(t) = timeout_seconds.filter(|&t| t > 0) {
        params.insert(
            "STATEMENT_TIMEOUT_IN_SECONDS".to_string(),
            serde_json::Value::Number(t.into()),
        );
    }
    if params.is_empty() {
        None
    } else {
        Some(params)
    }
}

fn parse_bool_setting(setting: &Setting) -> Option<bool> {
    match setting {
        Setting::Bool(v) => Some(*v),
        Setting::String(s) => {
            let s = s.trim();
            if s.eq_ignore_ascii_case("true") || s.eq_ignore_ascii_case("yes") || s == "1" {
                Some(true)
            } else if s.eq_ignore_ascii_case("false") || s.eq_ignore_ascii_case("no") || s == "0" {
                Some(false)
            } else {
                None
            }
        }
        Setting::Int(v) => Some(*v != 0),
        _ => None,
    }
}

/// Best-effort detection of file transfer commands (PUT/GET) from SQL text.
///
/// Snowflake's async API does not support file transfers. Submitting PUT/GET with
/// asyncExec=true returns a poll URL, but polling returns error 612 "Result not found"
/// because file transfer metadata is only available synchronously.
///
/// We parse SQL to detect PUT/GET and force sync mode. If detection fails, error 612
/// triggers a retry with sync mode (see snowflake_query_with_client).
fn is_file_transfer(sql: &str) -> bool {
    let s = skip_leading_whitespace_and_comments(sql);
    if s.len() < 4 {
        return false;
    }
    let prefix = &s[..3];
    let next_char = s.as_bytes()[3];
    let is_put_or_get = prefix.eq_ignore_ascii_case("PUT") || prefix.eq_ignore_ascii_case("GET");
    // Must be followed by whitespace or comment start (-- or /*)
    let valid_separator = next_char.is_ascii_whitespace() || next_char == b'/' || next_char == b'-';
    is_put_or_get && valid_separator
}

/// Strips leading whitespace, line comments (--), and block comments (/* */)
pub(super) fn skip_leading_whitespace_and_comments(s: &str) -> &str {
    let mut s = s;
    loop {
        s = s.trim_start();

        // Skip line comments: -- ... \n
        if s.starts_with("--") {
            match s.find('\n') {
                Some(pos) => s = &s[pos + 1..],
                None => return "", // Comment extends to end
            }
            continue;
        }

        // Skip block comments: /* ... */
        if s.starts_with("/*") {
            match s.find("*/") {
                Some(pos) => s = &s[pos + 2..],
                None => return "", // Unterminated comment
            }
            continue;
        }

        break;
    }
    s
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
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

/// Extract JSON bindings from a `DataPtr` -- **zero-copy with validation**.
///
/// Returns a reference directly into the language wrapper's memory with **no allocation**.
/// The lifetime is tied to the DataPtr, ensuring the slice doesn't outlive the pointed data.
///
/// ## Memory / allocation details
///
/// Total allocations: **ZERO**
/// 1. **DataPtr.slice()**: creates a `&[u8]` slice over wrapper memory (no allocation)
/// 2. **UTF-8 validation**: validates bytes are valid UTF-8 (no allocation)
/// 3. **JSON syntax validation**: RawValue::from_string checks JSON syntax (no allocation)
///
/// ## Validation
///
/// This function performs validation to catch errors early:
/// - **UTF-8 validation**: Ensures the bytes are valid UTF-8
/// - **JSON syntax validation**: RawValue validates basic JSON structure
///
/// The Snowflake server still validates the full JSON structure, types, and formats.
///
/// ## Safety contract
///
/// The caller (language wrapper) MUST guarantee:
/// 1. The pointer points to memory that remains valid for the entire `statement_execute_query` call
/// 2. `statement_execute_query` is called synchronously (blocks until HTTP completes)
pub(crate) fn parse_json_bindings<'a>(
    data_ptr: &'a DataPtr<'a>,
) -> Result<&'a RawValue, StatementError> {
    // Get the byte slice from the pointer - zero allocation.
    // The slice lifetime is tied to DataPtr, ensuring safety.
    let json_bytes: &'a [u8] = data_ptr.slice();

    // Validate UTF-8 encoding - zero allocation.
    let json_str: &'a str = std::str::from_utf8(json_bytes).map_err(|_| {
        UnsupportedBindParameterTypeSnafu {
            type_: "Bindings data is not valid UTF-8".to_string(),
        }
        .build()
    })?;

    // Validate JSON syntax - zero allocation.
    // RawValue::from_string checks that the string is valid JSON without parsing it fully.
    let raw: &'a RawValue = serde_json::from_str(json_str).map_err(|e| {
        UnsupportedBindParameterTypeSnafu {
            type_: format!("Bindings data is not valid JSON: {}", e),
        }
        .build()
    })?;

    Ok(raw)
}

/// Splits an optional `BindingType` into its two mutually-exclusive components:
/// - `query_bindings`: parsed JSON `RawValue` for inline JSON bindings.
/// - `csv_bytes`: raw CSV byte slice for stage-based CSV bindings.
///
/// Exactly one of the two is `Some` when `bindings` is `Some`; both are `None`
/// when `bindings` is `None`. The two fields being separate is load-bearing:
/// the `QueryInput` struct requires them as distinct optional fields.
fn split_bindings<'a>(
    bindings: &'a Option<BindingType<'a>>,
) -> Result<(Option<&'a RawValue>, Option<&'a [u8]>), ApiError> {
    Ok(match bindings {
        None => (None, None),
        Some(BindingType::Json(ptr)) => (
            Some(parse_json_bindings(ptr).context(StatementSnafu)?),
            None,
        ),
        Some(BindingType::Csv(ptr)) => (None, Some(ptr.slice())),
    })
}

const TIMESTAMP_INPUT_FORMAT_KEY: &str = "TIMESTAMP_INPUT_FORMAT";
const TIMESTAMP_INPUT_FORMAT_VALUE_AUTO: &str = "AUTO";

fn inject_timestamp_input_format_auto(
    query_parameters: &mut Option<HashMap<String, serde_json::Value>>,
) {
    let map = query_parameters.get_or_insert_with(HashMap::new);
    map.insert(
        TIMESTAMP_INPUT_FORMAT_KEY.to_string(),
        serde_json::Value::String(TIMESTAMP_INPUT_FORMAT_VALUE_AUTO.to_string()),
    );
}

#[cfg(test)]
mod tests {
    use super::super::result_set;
    use super::*;
    use crate::rest::snowflake::query_response::Data;

    #[test]
    fn parse_bool_setting_accepts_native_bool_values() {
        assert_eq!(parse_bool_setting(&Setting::Bool(true)), Some(true));
        assert_eq!(parse_bool_setting(&Setting::Bool(false)), Some(false));
    }

    /// `skip_upload_on_content_match` is the first Statement-scoped param that
    /// must stay client-only. Adding it to `QUERY_PARAMETER_NAMES` would forward
    /// it to GS where it has no meaning.
    #[test]
    fn skip_upload_on_content_match_is_not_forwarded_to_gs() {
        for (key, _server_name) in QUERY_PARAMETER_NAMES {
            assert_ne!(key.as_str(), "skip_upload_on_content_match");
        }
    }

    #[test]
    fn query_tag_statement_option_is_forwarded_as_query_parameter() {
        // Stored under the canonical key (as `statement_set_options` resolves it).
        let mut settings = ParamStore::new();
        settings.insert(
            "query_tag".to_string(),
            Setting::String("stmt_tag".to_string()),
        );
        let params = build_query_parameters(&settings).expect("QUERY_TAG should be forwarded");
        assert_eq!(
            params.get("QUERY_TAG"),
            Some(&serde_json::Value::String("stmt_tag".to_string()))
        );
    }

    #[test]
    fn registered_client_only_statement_option_is_not_forwarded() {
        let mut settings = ParamStore::new();
        settings.insert(
            "skip_upload_on_content_match".to_string(),
            Setting::Bool(true),
        );
        assert!(
            build_query_parameters(&settings).is_none(),
            "registered client-only statement option must not be forwarded to GS"
        );
    }

    #[test]
    fn execution_mode_uses_native_bool_async_setting() {
        let conn = Arc::new(Mutex::new(Connection::new()));
        let mut stmt = Statement::new(conn);
        stmt.settings
            .insert("async_execution".to_string(), Setting::Bool(true));

        assert_eq!(
            stmt.execution_mode(Some("SELECT 1")),
            QueryExecutionMode::Async
        );
    }

    #[tokio::test]
    async fn statement_rejects_connection_scoped_param() {
        let ds = DatabaseDriverV1::new();
        let ch = ds.connection_new();
        let sh = ds.statement_new(ch).unwrap();
        let err = ds
            .statement_set_option(sh, "host".into(), Setting::String("h".into()))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("not statement-scoped"),
            "unexpected: {err}"
        );
        ds.statement_release(sh).unwrap();
        ds.connection_release(ch).unwrap();
    }

    #[test]
    fn is_file_transfer_detects_put_statements() {
        assert!(is_file_transfer("PUT file://local @stage"));
        assert!(is_file_transfer("put file://local @stage"));
        assert!(is_file_transfer("Put file://local @stage"));
    }

    #[test]
    fn is_file_transfer_detects_get_statements() {
        assert!(is_file_transfer("GET @stage file://local"));
        assert!(is_file_transfer("get @stage file://local"));
        assert!(is_file_transfer("Get @stage file://local"));
    }

    #[test]
    fn is_file_transfer_handles_whitespace_after_command() {
        // Space
        assert!(is_file_transfer("PUT file://local"));
        // Tab
        assert!(is_file_transfer("PUT\tfile://local"));
        // Newline
        assert!(is_file_transfer("PUT\nfile://local"));
        assert!(is_file_transfer("GET\n@stage"));
    }

    #[test]
    fn is_file_transfer_handles_comment_after_command() {
        // Block comment immediately after PUT/GET
        assert!(is_file_transfer("PUT/* comment */file://local"));
        assert!(is_file_transfer("GET/**/file://local"));
        // Line comment immediately after PUT/GET
        assert!(is_file_transfer("PUT-- comment\nfile://local"));
        assert!(is_file_transfer("GET--\n@stage"));
    }

    #[test]
    fn is_file_transfer_handles_leading_whitespace() {
        assert!(is_file_transfer("  PUT file://local @stage"));
        assert!(is_file_transfer("\t\nGET @stage file://local"));
    }

    #[test]
    fn is_file_transfer_handles_line_comments() {
        assert!(is_file_transfer("-- comment\nPUT file://local @stage"));
        assert!(is_file_transfer(
            "-- line1\n-- line2\nGET @stage file://local"
        ));
        assert!(is_file_transfer("  -- indented comment\nPUT file://local"));
    }

    #[test]
    fn is_file_transfer_handles_block_comments() {
        assert!(is_file_transfer("/* comment */PUT file://local @stage"));
        assert!(is_file_transfer("/* comment */ PUT file://local @stage"));
        assert!(is_file_transfer(
            "/* c1 */ /* c2 */ GET @stage file://local"
        ));
        assert!(is_file_transfer("/*\nmultiline\n*/PUT file://local"));
    }

    #[test]
    fn is_file_transfer_handles_mixed_comments() {
        assert!(is_file_transfer("-- line\n/* block */PUT file://local"));
        assert!(is_file_transfer("/* block */-- line\nGET @stage"));
        assert!(is_file_transfer(
            "  /* block */ -- line\n  PUT file://local"
        ));
    }

    #[test]
    fn is_file_transfer_rejects_comment_only() {
        assert!(!is_file_transfer("-- just a comment"));
        assert!(!is_file_transfer("/* unterminated comment"));
        assert!(!is_file_transfer("-- comment\n-- another"));
    }

    #[test]
    fn is_file_transfer_rejects_bare_commands() {
        // PUT or GET alone is not a valid command
        assert!(!is_file_transfer("PUT"));
        assert!(!is_file_transfer("GET"));
        assert!(!is_file_transfer("put"));
        assert!(!is_file_transfer("get"));
    }

    #[test]
    fn is_file_transfer_rejects_non_blocking_statements() {
        assert!(!is_file_transfer("SELECT * FROM table"));
        assert!(!is_file_transfer("INSERT INTO table VALUES (1)"));
        assert!(!is_file_transfer("UPDATE table SET x = 1"));
        assert!(!is_file_transfer("DELETE FROM table"));
        assert!(!is_file_transfer("CREATE TABLE t (id INT)"));
    }

    #[test]
    fn is_file_transfer_rejects_similar_prefixes() {
        // Should not match words that start with PUT/GET but aren't commands
        assert!(!is_file_transfer("PUTTING"));
        assert!(!is_file_transfer("GETTING"));
        assert!(!is_file_transfer("PUTTER"));
        assert!(!is_file_transfer("GETAWAY"));
    }

    #[test]
    fn is_file_transfer_handles_edge_cases() {
        assert!(!is_file_transfer(""));
        assert!(!is_file_transfer("   "));
        assert!(!is_file_transfer("PU"));
        assert!(!is_file_transfer("GE"));
        assert!(!is_file_transfer("P"));
    }

    #[test]
    fn test_parse_json_bindings() {
        // Test simple bindings
        let json =
            r#"{"1": {"type": "FIXED", "value": "123"}, "2": {"type": "TEXT", "value": "hello"}}"#;

        // Create a pointer to the JSON bytes (simulating Python's no-copy scheme)
        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let raw = parse_json_bindings(&data_ptr).unwrap();
        let params: serde_json::Value = serde_json::from_str(raw.get()).unwrap();

        // Verify it's a JSON object with 2 keys
        assert!(params.is_object());
        let obj = params.as_object().unwrap();
        assert_eq!(obj.len(), 2);

        // Verify parameter 1
        let param1 = obj.get("1").unwrap();
        assert_eq!(param1["type"], "FIXED");
        assert_eq!(param1["value"], "123");

        // Verify parameter 2
        let param2 = obj.get("2").unwrap();
        assert_eq!(param2["type"], "TEXT");
        assert_eq!(param2["value"], "hello");
    }

    #[test]
    fn test_parse_json_bindings_with_array() {
        // Test array bindings (multi-row)
        let json = r#"{"1": {"type": "FIXED", "value": ["1", "2", "3"]}, "2": {"type": "TEXT", "value": ["a", "b", "c"]}}"#;

        // Create a pointer to the JSON bytes (simulating Python's no-copy scheme)
        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let raw = parse_json_bindings(&data_ptr).unwrap();
        let params: serde_json::Value = serde_json::from_str(raw.get()).unwrap();

        // Verify it's a JSON object with 2 keys
        assert!(params.is_object());
        let obj = params.as_object().unwrap();
        assert_eq!(obj.len(), 2);

        // Verify parameter 1
        let param1 = obj.get("1").unwrap();
        assert_eq!(param1["type"], "FIXED");
        assert!(param1["value"].is_array());

        // Verify parameter 2
        let param2 = obj.get("2").unwrap();
        assert_eq!(param2["type"], "TEXT");
        assert!(param2["value"].is_array());
    }

    // ---------------------------------------------------------------
    // parse_json_bindings: error cases
    // ---------------------------------------------------------------

    // Note: These tests are removed as pointer validation is now handled at construction time
    // by the caller (language wrapper), not in parse_json_bindings

    #[test]
    fn test_parse_json_bindings_rejects_invalid_utf8() {
        // Create a byte buffer with invalid UTF-8 (0xFF is never valid in UTF-8).
        // With validation, this should be rejected early.
        let bad_bytes: Vec<u8> = vec![0xFF, 0xFE, 0x7B, 0x7D]; // invalid followed by "{}"
        let ptr = bad_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, bad_bytes.len() as i64);

        let result = parse_json_bindings(&data_ptr);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("not valid UTF-8"),
            "Expected UTF-8 validation error, got: {err_msg}"
        );
    }

    #[test]
    fn test_parse_json_bindings_rejects_invalid_json() {
        // Valid UTF-8 but not valid JSON.
        // With validation, this should be rejected early.
        let bad_json = "{ this is not json }";
        let json_bytes = bad_json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, bad_json.len() as i64);

        let result = parse_json_bindings(&data_ptr);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("not valid JSON"),
            "Expected JSON validation error, got: {err_msg}"
        );
    }

    #[test]
    fn test_parse_json_bindings_rejects_truncated_json() {
        // JSON that starts valid but is cut short.
        // With validation, this should be rejected early.
        let truncated = r#"{"1": {"type": "FIXED""#;
        let json_bytes = truncated.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, truncated.len() as i64);

        let result = parse_json_bindings(&data_ptr);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("not valid JSON"),
            "Expected JSON validation error, got: {err_msg}"
        );
    }

    // ---------------------------------------------------------------
    // parse_json_bindings: zero-copy verification
    // ---------------------------------------------------------------

    #[test]
    fn test_parse_json_bindings_zero_copy() {
        let json = r#"{"1": {"type": "TEXT", "value": "abc"}}"#;
        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let result = parse_json_bindings(&data_ptr).unwrap();

        // Verify the returned reference points into the original buffer (zero-copy)
        let raw_ptr = result.get().as_ptr() as usize;
        let original_start = json_bytes.as_ptr() as usize;
        let original_end = original_start + json_bytes.len();
        assert!(
            raw_ptr >= original_start && raw_ptr < original_end,
            "Zero-copy: RawValue should point into original buffer"
        );

        // Verify the content is correct
        let params: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(params["1"]["type"], "TEXT");
        assert_eq!(params["1"]["value"], "abc");
    }

    // ---------------------------------------------------------------
    // parse_json_bindings: additional happy-path cases
    // ---------------------------------------------------------------

    #[test]
    fn test_parse_json_bindings_single_parameter() {
        let json = r#"{"1": {"type": "FIXED", "value": "42"}}"#;
        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let result = parse_json_bindings(&data_ptr).unwrap();
        let params: serde_json::Value = serde_json::from_str(result.get()).unwrap();

        let obj = params.as_object().unwrap();
        assert_eq!(obj.len(), 1);
        assert_eq!(obj["1"]["type"], "FIXED");
        assert_eq!(obj["1"]["value"], "42");
    }

    #[test]
    fn test_parse_json_bindings_with_null_values() {
        let json = r#"{"1": {"type": "TEXT", "value": null}}"#;
        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let result = parse_json_bindings(&data_ptr).unwrap();
        let params: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert!(params["1"]["value"].is_null());
    }

    #[test]
    fn test_parse_json_bindings_with_unicode_values() {
        let json = r#"{"1": {"type": "TEXT", "value": "日本語テスト 🎉"}}"#;
        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let result = parse_json_bindings(&data_ptr).unwrap();
        let params: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(params["1"]["value"], "日本語テスト 🎉");
    }

    #[test]
    fn test_parse_json_bindings_with_special_characters() {
        let json = r#"{"1": {"type": "TEXT", "value": "line1\nline2\ttab\"quote"}}"#;
        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let result = parse_json_bindings(&data_ptr).unwrap();
        let params: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert!(params["1"]["value"].is_string());
    }

    #[test]
    fn test_parse_json_bindings_empty_object() {
        // An empty JSON object is valid -- zero bindings
        let json = r#"{}"#;
        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let result = parse_json_bindings(&data_ptr).unwrap();
        let params: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert!(params.is_object());
        assert_eq!(params.as_object().unwrap().len(), 0);
    }

    #[test]
    fn test_parse_json_bindings_many_parameters() {
        // Build a JSON object with 20 parameters
        let mut entries: Vec<String> = Vec::new();
        for i in 1..=20 {
            entries.push(format!(r#""{i}": {{"type": "FIXED", "value": "{i}"}}"#));
        }
        let json = format!("{{{}}}", entries.join(", "));

        let json_bytes = json.as_bytes();
        let ptr = json_bytes.as_ptr();

        let data_ptr = DataPtr::new(ptr, json.len() as i64);

        let result = parse_json_bindings(&data_ptr).unwrap();
        let params: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(params.as_object().unwrap().len(), 20);
    }

    // ---------------------------------------------------------------
    // Request serialization round-trip
    // ---------------------------------------------------------------

    #[test]
    fn test_request_serialization_with_bindings() {
        let json = r#"{"1":{"type":"FIXED","value":"7"}}"#;
        let raw: &RawValue = serde_json::from_str(json).unwrap();

        let request = query_request::Request {
            sql_text: "SELECT ?".to_string(),
            async_exec: false,
            sequence_id: 1,
            query_submission_time: 0,
            is_internal: false,
            describe_only: None,
            parameters: None,
            bindings: Some(raw),
            bind_stage: None,
            query_context: query_request::QueryContext { entries: None },
        };

        let serialized = serde_json::to_string(&request).unwrap();

        // The bindings JSON should appear verbatim in the serialized output
        assert!(
            serialized.contains(json),
            "Serialized request must contain the raw JSON verbatim.\nSerialized: {serialized}"
        );
    }

    #[test]
    fn test_request_serialization_with_owned_bindings() {
        // Simulate the Arrow path: Box<RawValue> (owned, passed by reference)
        let json = r#"{"1":{"type":"TEXT","value":"test"}}"#;
        let raw = RawValue::from_string(json.to_string()).unwrap();

        let request = query_request::Request {
            sql_text: "SELECT ?".to_string(),
            async_exec: false,
            sequence_id: 1,
            query_submission_time: 0,
            is_internal: false,
            describe_only: None,
            parameters: None,
            bindings: Some(&*raw),
            bind_stage: None,
            query_context: query_request::QueryContext { entries: None },
        };

        let serialized = serde_json::to_string(&request).unwrap();
        assert!(
            serialized.contains(json),
            "Serialized request must contain the raw JSON verbatim.\nSerialized: {serialized}"
        );
    }

    #[test]
    fn test_request_serialization_without_bindings() {
        // When bindings is None, the "bindings" key should be omitted entirely
        let request = query_request::Request {
            sql_text: "SELECT 1".to_string(),
            async_exec: false,
            sequence_id: 1,
            query_submission_time: 0,
            is_internal: false,
            describe_only: None,
            parameters: None,
            bindings: None,
            bind_stage: None,
            query_context: query_request::QueryContext { entries: None },
        };

        let serialized = serde_json::to_string(&request).unwrap();
        assert!(
            !serialized.contains("bindings"),
            "None bindings should be omitted from serialized output.\nSerialized: {serialized}"
        );
    }

    #[test]
    fn test_request_serialization_with_bind_stage_emits_bindstage_field() {
        // When the CSV stage uploader has written a path into
        // `QueryInput::bind_stage`, the serialized request must carry
        // the `bindStage` field.
        let request = query_request::Request {
            sql_text: "INSERT INTO t VALUES (?, ?)".to_string(),
            async_exec: false,
            sequence_id: 1,
            query_submission_time: 0,
            is_internal: false,
            describe_only: None,
            parameters: None,
            bindings: None,
            bind_stage: Some("@SYSTEM$BIND/abc-123".to_string()),
            query_context: query_request::QueryContext { entries: None },
        };

        let serialized = serde_json::to_string(&request).unwrap();
        assert!(
            serialized.contains(r#""bindStage":"@SYSTEM$BIND/abc-123""#),
            "serialized request must carry bindStage:\n{serialized}"
        );
        assert!(
            !serialized.contains(r#""bindings""#),
            "bindings must be omitted when bind_stage is set:\n{serialized}"
        );
    }

    #[test]
    fn test_request_serialization_without_bind_stage_omits_bindstage_field() {
        // Symmetric to the previous test: the inline-JSON path must not
        // leak `bindStage` onto the wire. `skip_serializing_if` on the
        // field is what enforces this; the test is a regression guard.
        let request = query_request::Request {
            sql_text: "SELECT 1".to_string(),
            async_exec: false,
            sequence_id: 1,
            query_submission_time: 0,
            is_internal: false,
            describe_only: None,
            parameters: None,
            bindings: None,
            bind_stage: None,
            query_context: query_request::QueryContext { entries: None },
        };

        let serialized = serde_json::to_string(&request).unwrap();
        assert!(
            !serialized.contains("bindStage"),
            "bindStage must be omitted when None:\n{serialized}"
        );
    }

    #[test]
    fn inject_timestamp_input_format_auto_allocates_map_when_none() {
        // The inline-bindings path keeps `query_parameters: None` when
        // no other server-side parameter is needed. The CSV path must
        // therefore be able to lazily allocate the map.
        let mut params = None;
        inject_timestamp_input_format_auto(&mut params);
        let map = params.expect("map must be allocated");
        assert_eq!(
            map.get("TIMESTAMP_INPUT_FORMAT"),
            Some(&serde_json::Value::String("AUTO".to_string()))
        );
        assert_eq!(map.len(), 1, "no other keys should be added");
    }

    #[test]
    fn inject_timestamp_input_format_auto_preserves_other_keys() {
        // Pre-existing entries (e.g. MULTI_STATEMENT_COUNT,
        // STATEMENT_TIMEOUT_IN_SECONDS) must survive the injection.
        let mut params = Some({
            let mut m = HashMap::new();
            m.insert(
                "MULTI_STATEMENT_COUNT".to_string(),
                serde_json::Value::Number(3.into()),
            );
            m
        });
        inject_timestamp_input_format_auto(&mut params);
        let map = params.unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get("TIMESTAMP_INPUT_FORMAT"),
            Some(&serde_json::Value::String("AUTO".to_string()))
        );
        assert_eq!(
            map.get("MULTI_STATEMENT_COUNT"),
            Some(&serde_json::Value::Number(3.into()))
        );
    }

    #[test]
    fn inject_timestamp_input_format_auto_overrides_existing_value() {
        // If the user has set their own
        // TIMESTAMP_INPUT_FORMAT in session params and *also* requested
        // stage binding, the stage path wins for that query so the
        // server can parse staged timestamps. A user who needs a
        // specific session-level format must `ALTER SESSION SET` after
        // the binding completes.
        let mut params = Some({
            let mut m = HashMap::new();
            m.insert(
                "TIMESTAMP_INPUT_FORMAT".to_string(),
                serde_json::Value::String("YYYY-MM-DD HH:MI:SS".to_string()),
            );
            m
        });
        inject_timestamp_input_format_auto(&mut params);
        assert_eq!(
            params.unwrap().get("TIMESTAMP_INPUT_FORMAT"),
            Some(&serde_json::Value::String("AUTO".to_string()))
        );
    }

    fn deserialize_query_response(json: &str) -> Data {
        serde_json::from_str(json).expect("test JSON must be valid query response Data")
    }

    fn rows_affected_of(data: &Data) -> Option<i64> {
        // Default (Python) flavor: exercises the non-COPY paths shared by every
        // wrapper. COPY's JDBC-specific rows_loaded summation is covered by the
        // flavor-keyed tests in result_set.rs.
        result_set::calculate_rows_affected(
            data,
            data.statement_type_id,
            &super::super::global_state::PutGetResultsetFlavor::default(),
        )
    }

    #[test]
    fn calculate_rows_affected_sums_dml_columns() {
        let data = deserialize_query_response(
            r#"{
                "statementTypeId": 12544,
                "rowset": [["10", "3"]],
                "rowtype": [
                    {"name": "number of rows inserted", "type": "FIXED", "nullable": false, "scale": 0, "precision": 10},
                    {"name": "number of rows updated", "type": "FIXED", "nullable": false, "scale": 0, "precision": 10}
                ]
            }"#,
        );
        assert_eq!(rows_affected_of(&data), Some(13));
    }

    #[test]
    fn calculate_rows_affected_skips_null_cells() {
        let data = deserialize_query_response(
            r#"{
                "statementTypeId": 12544,
                "rowset": [["5", null]],
                "rowtype": [
                    {"name": "number of rows inserted", "type": "FIXED", "nullable": false, "scale": 0, "precision": 10},
                    {"name": "number of rows deleted", "type": "FIXED", "nullable": true, "scale": 0, "precision": 10}
                ]
            }"#,
        );
        assert_eq!(rows_affected_of(&data), Some(5));
    }

    #[test]
    fn calculate_rows_affected_all_null_cells() {
        let data = deserialize_query_response(
            r#"{
                "statementTypeId": 12544,
                "rowset": [[null]],
                "rowtype": [
                    {"name": "number of rows inserted", "type": "FIXED", "nullable": true, "scale": 0, "precision": 10}
                ]
            }"#,
        );
        assert_eq!(rows_affected_of(&data), Some(0));
    }

    #[test]
    fn calculate_rows_affected_select_uses_total() {
        let data = deserialize_query_response(
            r#"{
                "statementTypeId": 4096,
                "total": 42
            }"#,
        );
        assert_eq!(rows_affected_of(&data), Some(42));
    }

    #[test]
    fn calculate_rows_affected_ddl_is_none_not_total() {
        // DDL (0x6000) is a no-result statement. Snowflake returns total: 1 as a
        // generic success marker; we must report None rather than a misleading 1.
        let data = deserialize_query_response(
            r#"{
                "statementTypeId": 24576,
                "total": 1
            }"#,
        );
        assert_eq!(rows_affected_of(&data), None);
    }

    #[test]
    fn calculate_rows_affected_unknown_type_is_none() {
        // No statementTypeId -> unknown -> no-result -> None (not data.total).
        let data = deserialize_query_response(
            r#"{
                "total": 1
            }"#,
        );
        assert_eq!(rows_affected_of(&data), None);
    }

    #[test]
    fn calculate_rows_affected_file_transfer_fallback_uses_total() {
        use super::super::global_state::WrapperPresets;
        use crate::query_types::statement_type::QueryType;

        // PUT/GET responses can omit statementTypeId. Exercise the full wiring:
        // response_to_descriptor must derive the effective type from
        // command=UPLOAD via effective_statement_type_id, then rows_affected must
        // use that same type (Cursor -> total), not fall back to None.
        let upload = deserialize_query_response(
            r#"{
                "command": "UPLOAD",
                "total": 3
            }"#,
        );
        let descriptor = result_set::response_to_descriptor(&upload, &WrapperPresets::default());
        assert_eq!(
            descriptor.statement_type_id,
            Some(QueryType::PUT_FILES.raw())
        );
        assert_eq!(descriptor.rows_affected, Some(3));
    }

    #[test]
    fn extract_query_returns_sql_when_set() {
        let conn = Arc::new(Mutex::new(Connection::new()));
        let mut stmt = Statement::new(conn);
        stmt.query = Some("SELECT 1".to_string());

        let result = extract_query(&stmt).unwrap();
        assert_eq!(result, "SELECT 1");
    }

    #[test]
    fn extract_query_errors_when_no_query() {
        let conn = Arc::new(Mutex::new(Connection::new()));
        let stmt = Statement::new(conn);

        let err = extract_query(&stmt).unwrap_err();
        assert!(
            err.to_string().contains("Query not found"),
            "unexpected: {err}"
        );
    }

    #[tokio::test]
    async fn query_context_returns_transport_fields() {
        let mut conn = Connection::new();
        conn.server_url = Some("https://account.snowflakecomputing.com".to_string());
        conn.client_info = Some(crate::config::rest_parameters::test_fixtures::test_client_info());
        conn.http_client = Some(reqwest::Client::new());
        conn.retry_policy = RetryPolicy::default();
        let conn = Arc::new(Mutex::new(conn));

        let (params, _client, _retry) = query_context(&conn).await.unwrap();
        assert_eq!(params.server_url, "https://account.snowflakecomputing.com");
    }

    #[tokio::test]
    async fn query_context_errors_when_not_initialized() {
        let conn = Arc::new(Mutex::new(Connection::new()));

        let err = query_context(&conn).await.err().unwrap();
        assert!(
            err.to_string().contains("not initialized"),
            "unexpected: {err}"
        );
    }
}
