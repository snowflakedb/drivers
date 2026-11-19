use snafu::{OptionExt, ResultExt};
use std::future::Future;
use std::sync::{Arc, atomic::Ordering};
use std::time::Duration;

use crate::rest::snowflake::async_exec::{
    INLINE_SHORT_POLL_DELAYS, next_poll_delay_ms, poll_detached_once,
};
use tokio::sync::Mutex;

use super::connection::{
    CloseState, Connection, FinalSessionNames, with_session_refresh, with_valid_session,
};
use super::error::*;
use crate::apis::operation_ctx::{OperationCtx, run_opt};
use crate::config::rest_parameters::QueryParameters;
use crate::config::retry::RetryPolicy;
use crate::rest::snowflake::query_response::Response;
use crate::rest::snowflake::{
    AbortOutcome, DetachableQueryOutcome, DetachedContext, QueryExecutionMode, QueryInput,
    QueryOptions, RestError, snowflake_cancel_query, snowflake_query_with_client,
    snowflake_query_with_client_detachable,
};
use crate::utils::sync::MutexRecoverExt;
use crate::xp_backend::SnowflakeBackend;

/// Upper bound on the abort POST itself. Distinct from `OperationCtx`'s cleanup
/// wait, which bounds how long a cancelled caller blocks.
pub(super) const ABORT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Connection fields a query submission reads under one lock.
pub(super) struct QueryTransport {
    pub(super) query_parameters: QueryParameters,
    pub(super) http_client: reqwest::Client,
    pub(super) retry_policy: RetryPolicy,
    pub(super) xp_backend: Option<Arc<dyn SnowflakeBackend>>,
}

/// Slot the abort-cleanup writes so `Cancelled` can carry the acknowledgement.
/// The cleanup is spawned by `OperationCtx::with_cleanup` and outlives the
/// execute future, so its return value is unreadable. `std::sync::Mutex` because
/// the read must not `.await`.
#[derive(Clone, Debug, Default)]
pub(super) struct AbortReport(Arc<std::sync::Mutex<Vec<CancellationAbortResult>>>);

/// One in-flight abort inside an [`AbortReport`]. Replacing this slot leaves
/// every sibling abort's outcome intact.
struct AbortSlot {
    report: AbortReport,
    index: usize,
}

impl AbortReport {
    /// Record that an abort has been issued and is not yet confirmed.
    ///
    /// Called from the cleanup task, not when the cleanup is armed: a query
    /// that finishes disarms its cleanup, and that must stay "no abort was
    /// issued".
    fn begin(&self) -> AbortSlot {
        let mut slots = self.0.lock_recover();
        let index = slots.len();
        slots.push(CancellationAbortResult::NotConfirmed);
        AbortSlot {
            report: self.clone(),
            index,
        }
    }

    /// Attach the combined abort outcome to a `Cancelled` error; leave every
    /// other error as-is. Several aborts (a metadata call with concurrent
    /// `SHOW`s) collapse to one acknowledgement: all aborted, all not running,
    /// or not confirmed when the results disagree or any of them is unknown.
    pub(super) fn attach(&self, error: ApiError) -> ApiError {
        match error {
            ApiError::Cancelled { location, .. } => ApiError::Cancelled {
                abort: self.outcome(),
                location,
            },
            other => other,
        }
    }

    fn outcome(&self) -> Option<CancellationAbortResult> {
        let slots = self.0.lock_recover();
        if slots.is_empty() {
            return None;
        }
        if slots
            .iter()
            .all(|outcome| *outcome == CancellationAbortResult::Aborted)
        {
            return Some(CancellationAbortResult::Aborted);
        }
        if slots
            .iter()
            .all(|outcome| *outcome == CancellationAbortResult::NotRunning)
        {
            return Some(CancellationAbortResult::NotRunning);
        }
        Some(CancellationAbortResult::NotConfirmed)
    }
}

impl AbortSlot {
    fn set(&self, outcome: CancellationAbortResult) {
        let mut slots = self.report.0.lock_recover();
        if let Some(slot) = slots.get_mut(self.index) {
            *slot = outcome;
        }
    }
}

/// Run a query-submitting operation under `operation_ctx`, attaching to a
/// resulting cancellation whatever its abort-cleanup reported.
///
/// `OperationCtx::run` awaits registered cleanup before reporting, so in the
/// healthy case the report is already final by the time this reads it.
pub(super) async fn run_reporting_abort<T, F>(
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
        .map_err(|error| report.attach(error))
}

/// Abort `request_id` server-side and record the outcome. Owns its inputs because
/// the cleanup task outlives the future that arms it.
pub(super) fn abort_on_cancel(
    conn_arc: Arc<Mutex<Connection>>,
    request_id: uuid::Uuid,
    sql: String,
    report: &AbortReport,
    what: &'static str,
) -> impl Future<Output = ()> + Send + 'static {
    let report = report.clone();
    let request_id = request_id.to_string();
    async move {
        let slot = report.begin();
        match abort_query_by_request_id(&conn_arc, request_id, sql).await {
            Ok(outcome) => {
                slot.set(match outcome {
                    AbortOutcome::Aborted => CancellationAbortResult::Aborted,
                    AbortOutcome::NotRunning => CancellationAbortResult::NotRunning,
                });
                tracing::debug!(?outcome, "aborted {what} after cancellation");
            }
            Err(error) => tracing::warn!(%error, "failed to abort {what} after cancellation"),
        }
    }
}

/// Submit `query_input` with session refresh. Cancellation and client-side
/// timeout abort the query server-side. `cancellation` is `None` when the
/// caller cannot be cancelled. `request_id` is minted by the caller so submit
/// and abort share one identity. `query_timeout` bounds the whole
/// submit-and-refresh loop; `None` leaves only the retry policy.
pub(super) async fn submit_query(
    cancellation: Option<(&OperationCtx, &AbortReport)>,
    conn_arc: &Arc<Mutex<Connection>>,
    transport: &QueryTransport,
    query_input: QueryInput<'_>,
    execution_mode: QueryExecutionMode,
    request_id: uuid::Uuid,
    query_timeout: Option<Duration>,
) -> Result<Response, ApiError> {
    let sql = query_input.sql.clone();

    let submit = Box::pin(async {
        let attempt = post_then_poll(
            conn_arc,
            transport,
            query_input.clone(),
            execution_mode,
            request_id,
        );

        match query_timeout {
            Some(budget) => match tokio::time::timeout(budget, attempt).await {
                Ok(result) => result,
                Err(_elapsed) => {
                    spawn_abort_after_timeout(conn_arc, request_id, query_input.sql.clone());
                    Err(QueryTimeoutSnafu { budget, request_id }.build())
                }
            },
            None => attempt.await,
        }
    });

    match cancellation {
        Some((operation_ctx, report)) => {
            // Armed here so a cancel during bind-stage upload issues no abort.
            let cleanup = abort_on_cancel(conn_arc.clone(), request_id, sql, report, "query");
            operation_ctx.with_cleanup(cleanup, submit).await
        }
        None => submit.await,
    }
}

enum Posted {
    Done(Box<Response>),
    Poll(DetachedContext),
}

#[derive(Clone, Copy)]
pub(super) enum PollRefresh {
    Submit,
    ValidSession,
}

pub(super) async fn poll_until_complete(
    conn_arc: &Arc<Mutex<Connection>>,
    http_client: &reqwest::Client,
    query_parameters: &QueryParameters,
    retry_policy: &RetryPolicy,
    detached_ctx: DetachedContext,
    refresh: PollRefresh,
) -> Result<Response, ApiError> {
    let mut is_first_poll = true;
    let mut delay_i = 0;
    let mut sleep_ms = retry_policy.backoff.base.as_millis() as f64;
    loop {
        if !is_first_poll {
            let delay = if delay_i < INLINE_SHORT_POLL_DELAYS.len() {
                INLINE_SHORT_POLL_DELAYS[delay_i]
            } else {
                sleep_ms = next_poll_delay_ms(sleep_ms, &retry_policy.backoff);
                Duration::from_millis(sleep_ms as u64)
            };
            delay_i += 1;
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
        }

        let poll = {
            let http_client = http_client.clone();
            let query_parameters = query_parameters.clone();
            let retry_policy = retry_policy.clone();
            let detached_ctx = detached_ctx.clone();
            move |token: crate::sensitive::SensitiveString| {
                let http_client = http_client.clone();
                let query_parameters = query_parameters.clone();
                let retry_policy = retry_policy.clone();
                let detached_ctx = detached_ctx.clone();
                async move {
                    poll_detached_once(
                        &http_client,
                        &query_parameters,
                        token.reveal(),
                        &detached_ctx,
                        &retry_policy,
                        is_first_poll,
                    )
                    .await
                }
            }
        };
        let step = match refresh {
            PollRefresh::Submit => with_session_refresh(conn_arc, poll).await?,
            PollRefresh::ValidSession => with_valid_session(conn_arc, poll).await?,
        };
        is_first_poll = false;
        if let Some(response) = step {
            return Ok(response);
        }
    }
}

async fn post_then_poll(
    conn_arc: &Arc<Mutex<Connection>>,
    transport: &QueryTransport,
    query_input: QueryInput<'_>,
    execution_mode: QueryExecutionMode,
    request_id: uuid::Uuid,
) -> Result<Response, ApiError> {
    let posted = with_session_refresh(conn_arc, |token| {
        let query_input = query_input.clone();
        async move {
            match snowflake_query_with_client_detachable(
                &transport.http_client,
                &transport.query_parameters,
                token.reveal(),
                query_input,
                QueryOptions {
                    retry_policy: transport.retry_policy.clone(),
                    execution_mode,
                    request_id: Some(request_id),
                },
                transport.xp_backend.as_deref(),
            )
            .await
            {
                DetachableQueryOutcome::Completed(result) => {
                    result.map(|response| Posted::Done(Box::new(response)))
                }
                DetachableQueryOutcome::Detached(detached_ctx) => Ok(Posted::Poll(*detached_ctx)),
            }
        }
    })
    .await;

    let result = match posted {
        Ok(Posted::Done(response)) => Ok(*response),
        Ok(Posted::Poll(detached_ctx)) => {
            let polled = poll_until_complete(
                conn_arc,
                &transport.http_client,
                &transport.query_parameters,
                &transport.retry_policy,
                detached_ctx,
                PollRefresh::Submit,
            )
            .await;
            match (execution_mode, &polled) {
                (QueryExecutionMode::Async, Err(ApiError::Query { source, .. }))
                    if matches!(
                        source.as_ref(),
                        RestError::AsyncPollResultNotFound {
                            is_first_poll: true,
                            ..
                        }
                    ) =>
                {
                    with_session_refresh(conn_arc, |token| {
                        let query_input = query_input.clone();
                        async move {
                            snowflake_query_with_client(
                                &transport.http_client,
                                transport.query_parameters.clone(),
                                token.reveal(),
                                query_input,
                                QueryOptions {
                                    retry_policy: transport.retry_policy.clone(),
                                    execution_mode: QueryExecutionMode::Blocking,
                                    request_id: Some(request_id),
                                },
                                transport.xp_backend.as_deref(),
                            )
                            .await
                        }
                    })
                    .await
                }
                _ => polled,
            }
        }
        Err(error) => Err(error),
    };

    if let Err(ApiError::Query { source, .. }) = &result
        && let RestError::QueryFailed {
            query_context: Some(qctx),
            ..
        } = source.as_ref()
    {
        let mut conn = conn_arc.lock().await;
        conn.query_context_cache
            .update_query_context_cache(Some(qctx), None)
            .await;
    }
    result
}

/// Spawned so `QueryTimeout` surfaces at the deadline, not deadline + abort latency.
fn spawn_abort_after_timeout(
    conn_arc: &Arc<Mutex<Connection>>,
    request_id: uuid::Uuid,
    sql: String,
) {
    let conn_arc = conn_arc.clone();
    let request_id = request_id.to_string();
    tokio::spawn(async move {
        match abort_query_by_request_id(&conn_arc, request_id, sql).await {
            Ok(_) => tracing::debug!("successfully aborted query after timeout"),
            Err(error) => {
                tracing::warn!(%error, "failed to abort query after client-side timeout")
            }
        }
    });
}

/// `POST /queries/v1/abort-request`. Aborts are idempotent server-side.
pub(super) async fn abort_query_by_request_id(
    conn_arc: &Arc<Mutex<Connection>>,
    request_id: String,
    sql_text: String,
) -> Result<AbortOutcome, ApiError> {
    let transport = query_context(conn_arc).await?;

    let cancel = with_valid_session(conn_arc, |token| {
        let transport = &transport;
        let request_id = &request_id;
        let sql_text = &sql_text;
        async move {
            snowflake_cancel_query(
                &transport.http_client,
                &transport.query_parameters,
                token.reveal(),
                request_id,
                sql_text,
                transport.xp_backend.as_deref(),
            )
            .await
        }
    });

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

/// Transport for a query, or `ConnectionClosed` if the connection is not open
/// (`Closing` included: logout is already in flight).
pub(super) async fn query_context(
    conn: &Arc<Mutex<Connection>>,
) -> Result<QueryTransport, ApiError> {
    let conn = conn.lock().await;
    if conn.close_state.load(Ordering::SeqCst) != CloseState::Open {
        return ConnectionClosedSnafu {}.fail();
    }
    Ok(QueryTransport {
        query_parameters: conn.query_transport_parameters().await?,
        http_client: conn
            .http_client
            .clone()
            .context(ConnectionNotInitializedSnafu)?,
        retry_policy: RetryPolicy::query(&conn.effective_settings()),
        xp_backend: conn.xp_backend_arc().context(QuerySnafu)?,
    })
}

/// Write session names/params on success, and the query-context cache from any
/// response. `queryContext` can arrive on a failed envelope; session names from
/// a failed response are not applied.
pub(super) async fn update_caches(
    conn_arc: &Arc<Mutex<Connection>>,
    query: &str,
    response: &Response,
) {
    let mut conn = conn_arc.lock().await;
    if response.success {
        conn.update_session_params_cache(
            query,
            response.data.parameters.as_ref(),
            &FinalSessionNames {
                database: response.data.final_database_name.clone(),
                schema: response.data.final_schema_name.clone(),
                warehouse: response.data.final_warehouse_name.clone(),
                role: response.data.final_role_name.clone(),
            },
        )
        .await;
        conn.start_heartbeat_if_session_keeps_alive().await;
    }
    conn.query_context_cache
        .update_query_context_cache(
            response.data.query_context.as_ref(),
            response.data.parameters.as_ref(),
        )
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apis::database_driver_v1::global_state::WrapperPresets;
    use crate::config::ParamStore;
    use crate::rest::snowflake::query_response::{Data, QueryContext, QueryContextEntry};

    #[tokio::test]
    async fn query_context_returns_transport_fields() {
        let mut conn = Connection::new();
        conn.server_url = Some("https://account.snowflakecomputing.com".to_string());
        conn.client_info = Some(crate::config::rest_parameters::test_fixtures::test_client_info());
        conn.http_client = Some(reqwest::Client::new());
        conn.retry_policy = RetryPolicy::default();
        let conn = Arc::new(Mutex::new(conn));

        let transport = query_context(&conn).await.unwrap();
        assert_eq!(
            transport.query_parameters.server_url,
            "https://account.snowflakecomputing.com"
        );
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

    fn response_with_qcc(success: bool, database: Option<&str>) -> Response {
        let mut data = Data::default();
        data.query_context = Some(QueryContext {
            entries: Some(vec![QueryContextEntry {
                id: 7,
                timestamp: 100,
                priority: 0,
                context: Some("ctx".into()),
            }]),
        });
        data.final_database_name = database.map(str::to_string);
        Response {
            success,
            code: None,
            message: None,
            data,
        }
    }

    async fn conn_with_qcc_enabled() -> Arc<Mutex<Connection>> {
        let mut conn = Connection::new();
        conn.query_context_cache
            .init(Some(&ParamStore::new()), &WrapperPresets::default());
        Arc::new(Mutex::new(conn))
    }

    #[tokio::test]
    async fn update_caches_writes_qcc_on_failed_response() {
        let conn = conn_with_qcc_enabled().await;
        let response = response_with_qcc(false, Some("should_not_apply"));

        update_caches(&conn, "SELECT 1", &response).await;

        let conn = conn.lock().await;
        let snap = conn.query_context_cache.get_query_context_snapshot().await;
        let entries = snap.entries.expect("QCC entries on a failed envelope");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, 7);
        assert!(
            conn.final_session_names.read().unwrap().database.is_none(),
            "failed response must not write session names"
        );
    }

    #[tokio::test]
    async fn update_caches_writes_qcc_and_session_names_on_success() {
        let conn = conn_with_qcc_enabled().await;
        let response = response_with_qcc(true, Some("applied_db"));

        update_caches(&conn, "SELECT 1", &response).await;

        let conn = conn.lock().await;
        let snap = conn.query_context_cache.get_query_context_snapshot().await;
        assert_eq!(snap.entries.as_ref().map(|e| e[0].id), Some(7));
        assert_eq!(
            conn.final_session_names.read().unwrap().database.as_deref(),
            Some("applied_db")
        );
    }
}
