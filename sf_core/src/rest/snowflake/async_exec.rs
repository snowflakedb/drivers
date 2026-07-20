use crate::config::rest_parameters::{ClientInfo, QueryParameters};
use crate::config::retry::{BackoffConfig, RetryPolicy};
use crate::http::retry::{HttpContext, execute_with_retry};
use crate::rest::snowflake::error::{SfError, current_location, map_http_error};
use crate::rest::snowflake::{
    QUERY_REQUEST_PATH, QueryInput, apply_json_content_type, apply_query_headers, query_log_fields,
    query_request, query_response,
};
use reqwest::{Method, StatusCode};
use std::time::{Duration, Instant};
use tracing::{debug, info};
use url::Url;

const INLINE_SHORT_POLL_DELAYS: &[Duration] = &[
    Duration::from_millis(5),
    Duration::from_millis(10),
    Duration::from_millis(20),
    Duration::from_millis(40),
];
const QUERY_SEQUENCE_ID: u64 = 1;

/// Maximum wall-clock time the driver will spend polling for a single
/// statement's completion. `None` means unbounded — poll until Snowflake
/// returns data or a terminal error.
///
/// TODO: plumb a caller-supplied `statement_timeout` (e.g. configuration) so users can cap long-running queries.
const STATEMENT_POLL_DEADLINE: Option<Duration> = None;

/// Metrics for async query execution phases, logged for monitoring and debugging.
///
/// Async execution follows this flow:
/// 1. **Submit**: Initial statement submission to Snowflake (always occurs)
/// 2. **Inline Poll**: Quick polls with short delays (5-40ms) hoping for fast completion
/// 3. **Wait**: Exponential backoff polling if inline polling didn't complete
///
/// Either inline_poll completes the query, or we fall through to wait phase.
#[derive(Debug, Default)]
struct AsyncExecutionMetrics {
    /// Time spent submitting the initial async statement request.
    submit: Duration,
    /// Metrics from the inline polling phase (short delays, hoping for quick completion).
    inline_poll: Option<InlinePollMetrics>,
    /// Metrics from the wait phase (exponential backoff, used for longer queries).
    wait: Option<WaitMetrics>,
}

/// Metrics from the inline polling phase.
#[derive(Debug)]
struct InlinePollMetrics {
    /// Total time spent in inline polling.
    duration: Duration,
    /// Whether the query completed during inline polling (true) or fell through to wait (false).
    completed: bool,
}

/// Metrics from the exponential backoff wait phase.
#[derive(Debug)]
struct WaitMetrics {
    /// Total time spent waiting for completion.
    duration: Duration,
    /// Number of poll requests made during the wait phase.
    polls: usize,
}

impl AsyncExecutionMetrics {
    fn record_submit(&mut self, duration: Duration) {
        self.submit = duration;
    }

    fn record_inline(&mut self, duration: Duration, completed: bool) {
        self.inline_poll = Some(InlinePollMetrics {
            duration,
            completed,
        });
    }

    fn record_wait(&mut self, duration: Duration, polls: usize) {
        self.wait = Some(WaitMetrics { duration, polls });
    }

    fn emit(&self, label: &str) {
        fn ms(d: Duration) -> f64 {
            d.as_secs_f64() * 1000.0
        }

        let inline_ms = self.inline_poll.as_ref().map(|m| ms(m.duration));
        let inline_completed = self.inline_poll.as_ref().map(|m| m.completed);
        let wait_ms = self.wait.as_ref().map(|w| ms(w.duration));
        let wait_polls = self.wait.as_ref().map(|w| w.polls);

        debug!(
            submit_ms = ms(self.submit),
            inline_ms, inline_completed, wait_ms, wait_polls, label,
        );
    }
}

fn join_server_path(server_url: &str, path: &str) -> Result<String, SfError> {
    Url::parse(server_url)
        .and_then(|base| base.join(path))
        .map(|joined| joined.to_string())
        .map_err(|source| SfError::ResultUrlParse {
            url: format!("{server_url}{path}"),
            source,
            location: current_location(),
        })
}

pub struct SubmitOk {
    pub query_id: Option<String>,
    pub get_result_url: Option<String>,
    pub response: query_response::Response,
}

fn build_async_query_request<'a>(query_input: &QueryInput<'a>) -> query_request::Request<'a> {
    query_request::Request {
        sql_text: query_input.sql.clone(),
        async_exec: true,
        sequence_id: QUERY_SEQUENCE_ID,
        query_submission_time: current_epoch_millis(),
        is_internal: false,
        describe_only: query_input.describe_only,
        parameters: query_input.query_parameters.clone(),
        bindings: query_input.bindings,
        bind_stage: query_input.bind_stage.clone(),
        query_context: query_request::QueryContext { entries: None },
    }
}

fn build_submit_request(
    client: &reqwest::Client,
    endpoint: &str,
    client_info: &ClientInfo,
    session_token: &str,
    request_id: uuid::Uuid,
    payload: &query_request::Request,
) -> reqwest::RequestBuilder {
    let builder = client.post(endpoint);
    apply_json_content_type(apply_query_headers(builder, client_info, session_token))
        .query(&[("requestId", request_id.to_string())])
        .json(payload)
}

async fn parse_submit_response(
    server_url: &str,
    response: reqwest::Response,
) -> Result<SubmitOk, SfError> {
    let status = response.status();
    if !status.is_success() {
        return Err(http_status_error(status));
    }

    let body_bytes = response
        .bytes()
        .await
        .map_err(|source| transport_error(source))?;
    let parsed: query_response::Response =
        serde_json::from_slice(&body_bytes).map_err(|source| body_parse_error(source))?;
    let query_id = parsed.data.query_id.clone();
    let get_result_url = extract_result_url_from_response(server_url, &parsed)?;
    debug!(
        success = parsed.success,
        rowset_present = parsed.data.rowset.is_some(),
        rowset_base64_present = parsed.data.rowset_base64.is_some(),
        chunks = parsed
            .data
            .chunks
            .as_ref()
            .map(|c| c.len())
            .unwrap_or_default(),
        query_id = query_id.as_deref().unwrap_or_default(),
        get_result_url = get_result_url.as_deref().unwrap_or_default(),
        "submitted async query"
    );
    Ok(SubmitOk {
        query_id,
        get_result_url,
        response: parsed,
    })
}

fn current_epoch_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

pub async fn submit_statement_async<'a>(
    client: &reqwest::Client,
    params: &QueryParameters,
    session_token: &str,
    query_input: &QueryInput<'a>,
    request_id: uuid::Uuid,
    policy: &RetryPolicy,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<SubmitOk, SfError> {
    let server_url = &params.server_url;
    let client_info = &params.client_info;
    let endpoint = join_server_path(server_url, QUERY_REQUEST_PATH)?;
    // query logging guarded with: log_query_text, log_query_parameters
    let (sql, bindings) = query_log_fields(params, query_input);
    info!(
        request_id = %request_id,
        sql = sql,
        bindings = bindings,
        "Executing async query"
    );
    let request_body = build_async_query_request(query_input);
    let submit_request = || {
        build_submit_request(
            client,
            &endpoint,
            client_info,
            session_token,
            request_id,
            &request_body,
        )
    };

    let ctx = HttpContext::new(Method::POST, QUERY_REQUEST_PATH).allow_post_retry();
    let response = execute_with_retry(
        submit_request,
        &ctx,
        policy,
        |r| async move { Ok(r) },
        cancel.clone(),
    )
    .await
    .map_err(map_http_error)?;

    tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(SfError::Cancelled {
            location: current_location(),
        }),
        parsed = parse_submit_response(server_url, response) => parsed,
    }
}

pub(super) async fn poll_query_status(
    client: &reqwest::Client,
    client_info: &ClientInfo,
    session_token: &str,
    get_result_url: &str,
    policy: &RetryPolicy,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<query_response::Response, SfError> {
    let result_url = get_result_url.to_string();
    let poll_request =
        move || apply_query_headers(client.get(result_url.clone()), client_info, session_token);
    let ctx = HttpContext::new(Method::GET, get_result_url.to_string());
    let response =
        execute_with_retry(poll_request, &ctx, policy, |r| async move { Ok(r) }, cancel.clone())
            .await
            .map_err(map_http_error)?;
    let status = response.status();
    if !status.is_success() {
        return Err(http_status_error(status));
    }
    let body_bytes = tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            return Err(SfError::Cancelled {
                location: current_location(),
            });
        }
        bytes = response.bytes() => bytes.map_err(|source| transport_error(source))?,
    };
    let parsed: query_response::Response =
        serde_json::from_slice(&body_bytes).map_err(|source| body_parse_error(source))?;
    debug!(
        success = parsed.success,
        rowset_present = parsed.data.rowset.is_some(),
        rowset_base64_present = parsed.data.rowset_base64.is_some(),
        chunks = parsed
            .data
            .chunks
            .as_ref()
            .map(|c| c.len())
            .unwrap_or_default(),
        code = parsed.code.as_deref().unwrap_or_default(),
        message = parsed.message.as_deref().unwrap_or_default(),
        "polled query status"
    );
    Ok(parsed)
}

pub(super) async fn execute_blocking_with_async<'a>(
    client: &reqwest::Client,
    params: &QueryParameters,
    session_token: &str,
    query_input: &QueryInput<'a>,
    request_id: uuid::Uuid,
    policy: &RetryPolicy,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<query_response::Response, SfError> {
    // query logging guarded with: log_query_text, log_query_parameters
    let (sql, bindings) = query_log_fields(params, query_input);
    info!(
        request_id = %request_id,
        sql = sql,
        bindings = bindings,
        "Executing sync query"
    );
    let client_info = &params.client_info;
    let mut metrics = AsyncExecutionMetrics::default();
    let submit_start = Instant::now();
    let submitted = submit_statement_async(
        client,
        params,
        session_token,
        query_input,
        request_id,
        policy,
        cancel.clone(),
    )
    .await?;
    metrics.record_submit(submit_start.elapsed());

    let SubmitOk {
        query_id,
        get_result_url,
        mut response,
    } = submitted;

    if should_poll_for_completion(&response) {
        let result_url = get_result_url
            .as_deref()
            .ok_or_else(|| SfError::MissingResultUrl {
                location: current_location(),
            })?;

        response = poll_for_result(
            client,
            client_info,
            session_token,
            result_url,
            policy,
            &mut metrics,
            cancel,
        )
        .await?
    };

    response
        .data
        .query_id
        .clone()
        .or(query_id)
        .ok_or_else(|| SfError::MissingQueryId {
            location: current_location(),
        })?;

    metrics.emit("async execution timings");
    Ok(response)
}

#[track_caller]
fn transport_error(source: reqwest::Error) -> SfError {
    SfError::Transport {
        source,
        location: current_location(),
    }
}

#[track_caller]
fn body_parse_error(source: serde_json::Error) -> SfError {
    SfError::BodyParse {
        source,
        location: current_location(),
    }
}

#[track_caller]
fn http_status_error(status: StatusCode) -> SfError {
    // Return SessionExpired so caller can refresh and retry
    if status == StatusCode::UNAUTHORIZED {
        return SfError::SessionExpired {
            location: current_location(),
        };
    }
    SfError::HttpStatus {
        status,
        location: current_location(),
    }
}

fn extract_result_url_from_response(
    server_url: &str,
    response: &query_response::Response,
) -> Result<Option<String>, SfError> {
    response
        .data
        .get_result_url
        .as_deref()
        .map(|u| normalize_get_result_url(server_url, u))
        .transpose()
}

fn normalize_get_result_url(base: &str, url: &str) -> Result<String, SfError> {
    if url.starts_with("http://") || url.starts_with("https://") {
        return Ok(url.to_string());
    }
    let base_url = Url::parse(base).map_err(|source| SfError::ResultUrlParse {
        url: base.to_string(),
        source,
        location: current_location(),
    })?;
    let joined = base_url
        .join(url)
        .map_err(|source| SfError::ResultUrlParse {
            url: url.to_string(),
            source,
            location: current_location(),
        })?;
    Ok(joined.to_string())
}

pub(super) fn should_poll_for_completion(resp: &query_response::Response) -> bool {
    resp.data
        .get_result_url
        .as_ref()
        .is_some_and(|_| !response_has_tabular_data(resp))
}

fn response_has_tabular_data(resp: &query_response::Response) -> bool {
    resp.data.rowset.is_some()
        || resp.data.rowset_base64.is_some()
        || resp
            .data
            .chunks
            .as_ref()
            .map(|c| !c.is_empty())
            .unwrap_or(false)
}

async fn inline_poll_for_completion(
    client: &reqwest::Client,
    client_info: &ClientInfo,
    session_token: &str,
    result_url: &str,
    policy: &RetryPolicy,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<Option<query_response::Response>, SfError> {
    let response = poll_query_status(
        client,
        client_info,
        session_token,
        result_url,
        policy,
        cancel,
    )
    .await?;
    handle_poll_response(response, true) // First poll
}

/// Poll Snowflake for completion, starting with a burst of short delays
/// and then degrading into retry-policy-driven exponential backoff.
/// Each HTTP poll flows through the shared retry helper so transport
/// or retryable status failures are retried automatically.
///
/// The outer loop is bounded only by [`STATEMENT_POLL_DEADLINE`] (unbounded
/// by default). `policy.max_elapsed` bounds each individual HTTP poll's
/// internal retry budget — it does NOT cap total query wall-clock time.
async fn wait_for_completion(
    client: &reqwest::Client,
    client_info: &ClientInfo,
    session_token: &str,
    result_url: &str,
    policy: &RetryPolicy,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<(query_response::Response, usize), SfError> {
    let start = Instant::now();
    let mut attempt: usize = 0;
    let mut sleep_ms = policy.backoff.base.as_millis() as f64;
    let mut polls: usize = 0;

    loop {
        if cancel.is_cancelled() {
            return Err(SfError::Cancelled {
                location: current_location(),
            });
        }
        if let Some(deadline) = STATEMENT_POLL_DEADLINE {
            let elapsed = start.elapsed();
            if elapsed >= deadline {
                return Err(SfError::DeadlineExceeded {
                    configured: deadline,
                    elapsed,
                    location: current_location(),
                });
            }
        }

        let delay = if attempt < INLINE_SHORT_POLL_DELAYS.len() {
            INLINE_SHORT_POLL_DELAYS[attempt]
        } else {
            sleep_ms = next_poll_delay_ms(sleep_ms, &policy.backoff);
            Duration::from_millis(sleep_ms as u64)
        };
        attempt += 1;

        if !delay.is_zero() {
            if let Some(deadline) = STATEMENT_POLL_DEADLINE {
                let would_wake = start.elapsed() + delay;
                if would_wake >= deadline {
                    return Err(SfError::DeadlineExceeded {
                        configured: deadline,
                        elapsed: start.elapsed(),
                        location: current_location(),
                    });
                }
            }
            tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    return Err(SfError::Cancelled {
                        location: current_location(),
                    });
                }
                _ = tokio::time::sleep(delay) => {}
            }
        }

        let response = poll_query_status(
            client,
            client_info,
            session_token,
            result_url,
            policy,
            cancel.clone(),
        )
        .await?;
        polls += 1;

        if let Some(done) = handle_poll_response(response, false)? {
            return Ok((done, polls));
        }
    }
}

/// Poll a result URL until the query completes, using an immediate inline
/// poll followed by exponential-backoff waiting if needed.
///
/// Shared by both the async execution path (after async submit) and the
/// detached query path (after sync submit returned an "in progress" code).
async fn poll_for_result(
    client: &reqwest::Client,
    client_info: &ClientInfo,
    session_token: &str,
    result_url: &str,
    policy: &RetryPolicy,
    metrics: &mut AsyncExecutionMetrics,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<query_response::Response, SfError> {
    let inline_start = Instant::now();
    let inline_result = inline_poll_for_completion(
        client,
        client_info,
        session_token,
        result_url,
        policy,
        cancel.clone(),
    )
    .await?;

    match inline_result {
        Some(response) => {
            metrics.record_inline(inline_start.elapsed(), true);
            Ok(response)
        }
        None => {
            metrics.record_inline(inline_start.elapsed(), false);
            let wait_start = Instant::now();
            let (response, polls) = wait_for_completion(
                client,
                client_info,
                session_token,
                result_url,
                policy,
                cancel,
            )
            .await?;
            metrics.record_wait(wait_start.elapsed(), polls);
            Ok(response)
        }
    }
}

/// Poll for the result of a detached query — a sync submission that returned
/// a `get_result_url` without tabular data, indicating the server is still
/// processing.
pub(super) async fn poll_detached_query(
    client: &reqwest::Client,
    query_parameters: &QueryParameters,
    session_token: &str,
    response: &query_response::Response,
    policy: &RetryPolicy,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<query_response::Response, SfError> {
    let result_url = extract_result_url_from_response(&query_parameters.server_url, response)?
        .ok_or_else(|| SfError::MissingResultUrl {
            location: current_location(),
        })?;

    let mut metrics = AsyncExecutionMetrics::default();
    let result = poll_for_result(
        client,
        &query_parameters.client_info,
        session_token,
        &result_url,
        policy,
        &mut metrics,
        cancel,
    )
    .await;
    metrics.emit("detached query poll timings");
    result
}

/// Error code 612 indicates "Result not found" - typically returned when
/// trying to poll for file transfer (PUT/GET) results in async mode.
const SNOWFLAKE_ERROR_RESULT_NOT_FOUND: i32 = 612;

fn snowflake_failure(resp: &query_response::Response, is_first_poll: bool) -> SfError {
    let code = resp
        .code
        .as_deref()
        .and_then(|c| c.parse::<i32>().ok())
        .unwrap_or(-1);

    // Error 612 "Result not found" occurs when polling for PUT/GET results.
    // File transfer commands don't support async mode.
    if code == SNOWFLAKE_ERROR_RESULT_NOT_FOUND {
        return SfError::AsyncPollResultNotFound {
            is_first_poll,
            location: current_location(),
        };
    }

    let message = resp
        .message
        .clone()
        .unwrap_or_else(|| "Snowflake reported failure".to_string());
    SfError::SnowflakeBody {
        code,
        message,
        location: current_location(),
    }
}

fn next_poll_delay_ms(prev_ms: f64, backoff: &BackoffConfig) -> f64 {
    let base = backoff.base.as_millis() as f64;
    let mut next = if prev_ms <= 0.0 {
        base
    } else {
        prev_ms.max(base) * backoff.factor
    };
    let cap = backoff.cap.as_millis() as f64;
    if next > cap {
        next = cap;
    }
    next
}

/// Returns true if a successful response still requires more polling.
/// This occurs when we have a result URL but no tabular data yet.
fn should_continue_after_success(resp: &query_response::Response) -> bool {
    resp.data.get_result_url.is_some() && !response_has_tabular_data(resp)
}

/// Returns true if a failed response should continue polling.
/// This occurs when the response has a result URL (query still running).
fn should_continue_after_failure(resp: &query_response::Response) -> bool {
    resp.data.get_result_url.is_some()
}

fn handle_poll_response(
    resp: query_response::Response,
    is_first_poll: bool,
) -> Result<Option<query_response::Response>, SfError> {
    if resp.success {
        if should_continue_after_success(&resp) {
            return Ok(None);
        }
        return Ok(Some(resp));
    }

    if should_continue_after_failure(&resp) {
        return Ok(None);
    }

    Err(snowflake_failure(&resp, is_first_poll))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn wait_for_completion_cancels_when_token_already_cancelled() {
        let cancel = tokio_util::sync::CancellationToken::new();
        cancel.cancel();
        let client = reqwest::Client::new();
        let client_info = crate::config::rest_parameters::test_fixtures::test_client_info();
        let policy = crate::config::retry::RetryPolicy::default();
        let result = wait_for_completion(
            &client,
            &client_info,
            "session-token",
            "http://127.0.0.1:9/result",
            &policy,
            cancel,
        )
        .await;
        assert!(matches!(result, Err(SfError::Cancelled { .. })));
    }

    fn response_from_json(value: serde_json::Value) -> query_response::Response {
        serde_json::from_value(value).expect("valid response JSON")
    }

    fn response_with_result_url(url: &str) -> serde_json::Value {
        json!({
            "success": true,
            "data": {
                "getResultUrl": url,
                "rowset": null,
                "rowsetBase64": null,
                "chunks": null,
            }
        })
    }

    // ── should_poll_for_completion ──────────────────────────────────

    #[test]
    fn should_not_poll_when_no_result_url() {
        let resp = response_from_json(json!({
            "success": true,
            "data": { "rowset": null, "rowsetBase64": null }
        }));
        assert!(!should_poll_for_completion(&resp));
    }

    #[test]
    fn should_poll_when_result_url_present_and_no_data() {
        let resp = response_from_json(response_with_result_url("https://example.test"));
        assert!(should_poll_for_completion(&resp));
    }

    #[test]
    fn should_not_poll_when_result_url_present_but_rowset_exists() {
        let resp = response_from_json(json!({
            "success": true,
            "data": {
                "getResultUrl": "https://example.test",
                "rowset": [["1"]],
                "rowsetBase64": null,
            }
        }));
        assert!(!should_poll_for_completion(&resp));
    }

    #[test]
    fn should_not_poll_when_result_url_present_but_rowset_base64_exists() {
        let resp = response_from_json(json!({
            "success": true,
            "data": {
                "getResultUrl": "https://example.test",
                "rowset": null,
                "rowsetBase64": "AAAA",
            }
        }));
        assert!(!should_poll_for_completion(&resp));
    }

    #[test]
    fn should_not_poll_when_result_url_present_but_chunks_exist() {
        let resp = response_from_json(json!({
            "success": true,
            "data": {
                "getResultUrl": "https://example.test",
                "rowset": null,
                "rowsetBase64": null,
                "chunks": [{"url": "https://chunk.test", "rowCount": 10, "uncompressedSize": 100, "compressedSize": 50}],
            }
        }));
        assert!(!should_poll_for_completion(&resp));
    }

    #[test]
    fn should_poll_when_chunks_is_empty_array() {
        let resp = response_from_json(json!({
            "success": true,
            "data": {
                "getResultUrl": "https://example.test",
                "rowset": null,
                "rowsetBase64": null,
                "chunks": [],
            }
        }));
        assert!(should_poll_for_completion(&resp));
    }

    #[test]
    fn should_not_poll_when_failure_has_no_result_url() {
        let resp = response_from_json(json!({
            "success": false,
            "data": { "rowset": null, "rowsetBase64": null }
        }));
        assert!(!should_poll_for_completion(&resp));
    }

    // ── extract_result_url_from_response ───────────────────────────

    #[test]
    fn extract_result_url_returns_none_when_absent() {
        let resp = response_from_json(json!({
            "success": true,
            "data": { "rowset": null, "rowsetBase64": null }
        }));
        assert!(
            extract_result_url_from_response("https://base.test", &resp)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn extract_result_url_normalizes_relative_path() {
        let resp = response_from_json(response_with_result_url("/queries/abc/result"));
        let url = extract_result_url_from_response("https://base.test", &resp)
            .unwrap()
            .unwrap();
        assert_eq!(url, "https://base.test/queries/abc/result");
    }

    #[test]
    fn extract_result_url_passes_through_absolute_url() {
        let resp = response_from_json(response_with_result_url("https://other.test/result"));
        let url = extract_result_url_from_response("https://base.test", &resp)
            .unwrap()
            .unwrap();
        assert_eq!(url, "https://other.test/result");
    }

    // ── http_status_error ──────────────────────────────────────────

    #[test]
    fn http_401_returns_session_expired() {
        let err = http_status_error(StatusCode::UNAUTHORIZED);
        assert!(
            matches!(err, SfError::SessionExpired { .. }),
            "expected SessionExpired, got {:?}",
            err
        );
    }

    #[test]
    fn http_503_returns_http_status() {
        let err = http_status_error(StatusCode::SERVICE_UNAVAILABLE);
        match err {
            SfError::HttpStatus { status, .. } => {
                assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
            }
            other => panic!("expected HttpStatus, got {:?}", other),
        }
    }

    // ── snowflake error ──────────────────────────────────────────

    #[test]
    fn error_612_returns_async_poll_result_not_found() {
        // Error 612 "Result not found" is returned when polling for PUT/GET results
        let resp = response_from_json(json!({
            "success": false,
            "code": "612",
            "message": "Result not found",
            "data": {
                "rowset": null,
                "rowsetBase64": null
            }
        }));

        let err = snowflake_failure(&resp, true);
        assert!(
            matches!(
                err,
                SfError::AsyncPollResultNotFound {
                    is_first_poll: true,
                    ..
                }
            ),
            "expected AsyncPollResultNotFound with is_first_poll=true, got {:?}",
            err
        );

        let err = snowflake_failure(&resp, false);
        assert!(
            matches!(
                err,
                SfError::AsyncPollResultNotFound {
                    is_first_poll: false,
                    ..
                }
            ),
            "expected AsyncPollResultNotFound with is_first_poll=false, got {:?}",
            err
        );
    }

    // ── handle_poll_response ───────────────────────────────────────

    #[test]
    fn handle_poll_success_with_data_returns_response() {
        let resp = response_from_json(json!({
            "success": true,
            "data": { "rowset": [["1"]], "rowsetBase64": null }
        }));
        let result = handle_poll_response(resp, true).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn handle_poll_success_with_result_url_but_no_data_continues() {
        let resp = response_from_json(response_with_result_url("https://example.test"));
        let result = handle_poll_response(resp, true).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn handle_poll_failure_with_result_url_continues() {
        let resp = response_from_json(json!({
            "success": false,
            "data": {
                "getResultUrl": "https://example.test",
                "rowset": null,
                "rowsetBase64": null,
            }
        }));
        let result = handle_poll_response(resp, false).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn handle_poll_failure_without_result_url_returns_error() {
        let resp = response_from_json(json!({
            "success": false,
            "code": "1003",
            "message": "Syntax error",
            "data": { "rowset": null, "rowsetBase64": null }
        }));
        match handle_poll_response(resp, false) {
            Err(SfError::SnowflakeBody { code: 1003, .. }) => {}
            Err(other) => panic!("expected SnowflakeBody(1003), got {other:?}"),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }
}
