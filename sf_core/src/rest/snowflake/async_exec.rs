use crate::chunks::ChunkDownloadData;
use crate::config::rest_parameters::{ClientInfo, QueryParameters};
use crate::config::retry::{BackoffConfig, RetryPolicy};
use crate::http::retry::{HttpContext, HttpError, execute_with_retry};
use crate::rest::snowflake::error::SfError;
use crate::rest::snowflake::{
    QUERY_REQUEST_PATH, apply_json_content_type, apply_query_headers, query_request, query_response,
};
use reqwest::{Method, StatusCode};
use serde_json::value::RawValue;
use snafu::Location;
use std::panic::Location as StdLocation;
use std::time::{Duration, Instant};
use tracing::debug;
use url::Url;

const INLINE_SHORT_POLL_DELAYS: &[Duration] = &[
    Duration::from_millis(5),
    Duration::from_millis(10),
    Duration::from_millis(20),
    Duration::from_millis(40),
];
const QUERY_SEQUENCE_ID: u64 = 1;

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

    fn emit(&self) {
        fn ms(d: Duration) -> f64 {
            d.as_secs_f64() * 1000.0
        }

        let inline_ms = self.inline_poll.as_ref().map(|m| ms(m.duration));
        let inline_completed = self.inline_poll.as_ref().map(|m| m.completed);
        let wait_ms = self.wait.as_ref().map(|w| ms(w.duration));
        let wait_polls = self.wait.as_ref().map(|w| w.polls);

        debug!(
            submit_ms = ms(self.submit),
            inline_ms, inline_completed, wait_ms, wait_polls, "async execution timings"
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

fn build_async_query_request(
    sql: String,
    parameter_bindings: Option<&RawValue>,
) -> query_request::Request<'_> {
    query_request::Request {
        sql_text: sql,
        async_exec: true,
        sequence_id: QUERY_SEQUENCE_ID,
        query_submission_time: current_epoch_millis(),
        is_internal: false,
        describe_only: None,
        parameters: None,
        bindings: parameter_bindings,
        bind_stage: None,
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
    let get_result_url = parsed
        .data
        .get_result_url
        .as_deref()
        .map(|u| normalize_get_result_url(server_url, u))
        .transpose()?;
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

pub async fn submit_statement_async(
    client: &reqwest::Client,
    params: &QueryParameters,
    session_token: &str,
    sql: String,
    parameter_bindings: Option<&RawValue>,
    request_id: uuid::Uuid,
    policy: &RetryPolicy,
) -> Result<SubmitOk, SfError> {
    let server_url = &params.server_url;
    let client_info = &params.client_info;
    let endpoint = join_server_path(server_url, QUERY_REQUEST_PATH)?;
    let request_body = build_async_query_request(sql, parameter_bindings);
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
    let response = execute_with_retry(submit_request, &ctx, policy, |r| async move { Ok(r) })
        .await
        .map_err(map_http_error)?;

    parse_submit_response(server_url, response).await
}

pub async fn poll_query_status(
    client: &reqwest::Client,
    client_info: &ClientInfo,
    session_token: &str,
    get_result_url: &str,
    policy: &RetryPolicy,
) -> Result<query_response::Response, SfError> {
    let result_url = get_result_url.to_string();
    let poll_request =
        move || apply_query_headers(client.get(result_url.clone()), client_info, session_token);
    let ctx = HttpContext::new(Method::GET, get_result_url.to_string());
    let response = execute_with_retry(poll_request, &ctx, policy, |r| async move { Ok(r) })
        .await
        .map_err(map_http_error)?;
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

pub async fn execute_blocking_with_async(
    client: &reqwest::Client,
    params: &QueryParameters,
    session_token: &str,
    sql: String,
    parameter_bindings: Option<&RawValue>,
    request_id: uuid::Uuid,
    policy: &RetryPolicy,
) -> Result<query_response::Response, SfError> {
    let client_info = &params.client_info;
    let mut metrics = AsyncExecutionMetrics::default();
    let submit_start = Instant::now();
    let submitted = submit_statement_async(
        client,
        params,
        session_token,
        sql,
        parameter_bindings,
        request_id,
        policy,
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

        let inline_start = Instant::now();
        match inline_poll_for_completion(client, client_info, session_token, result_url, policy)
            .await?
        {
            Some(inline) => {
                metrics.record_inline(inline_start.elapsed(), true);
                response = inline;
            }
            None => {
                metrics.record_inline(inline_start.elapsed(), false);
                let wait_start = Instant::now();
                let (waited, polls) =
                    wait_for_completion(client, client_info, session_token, result_url, policy)
                        .await?;
                metrics.record_wait(wait_start.elapsed(), polls);
                response = waited;
            }
        }
    }

    response
        .data
        .query_id
        .clone()
        .or(query_id)
        .ok_or_else(|| SfError::MissingQueryId {
            location: current_location(),
        })?;

    metrics.emit();
    Ok(response)
}

#[track_caller]
fn current_location() -> Location {
    let caller = StdLocation::caller();
    Location::new(caller.file(), caller.line(), caller.column())
}

#[track_caller]
fn map_http_error(err: HttpError) -> SfError {
    let location = current_location();
    match err {
        HttpError::Transport { source, .. } => SfError::Transport { source, location },
        HttpError::DeadlineExceeded {
            configured,
            elapsed,
            ..
        } => SfError::DeadlineExceeded {
            configured,
            elapsed,
            location,
        },
        HttpError::MaxAttempts {
            attempts,
            last_status,
            ..
        } => SfError::RetryAttemptsExhausted {
            attempts,
            last_status,
            location,
        },
        HttpError::RetryAfterExceeded {
            retry_after,
            remaining,
            ..
        } => SfError::RetryBudgetExceeded {
            retry_after,
            remaining,
            location,
        },
    }
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

pub async fn refresh_chunk_download_data_from_get_result(
    client: &reqwest::Client,
    client_info: &ClientInfo,
    session_token: &str,
    get_result_url: &str,
    policy: &RetryPolicy,
) -> Result<Option<Vec<ChunkDownloadData>>, SfError> {
    let resp =
        poll_query_status(client, client_info, session_token, get_result_url, policy).await?;
    if resp.success {
        Ok(resp.data.to_chunk_download_data())
    } else {
        Ok(None)
    }
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

fn should_poll_for_completion(resp: &query_response::Response) -> bool {
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
) -> Result<Option<query_response::Response>, SfError> {
    let response =
        poll_query_status(client, client_info, session_token, result_url, policy).await?;
    handle_poll_response(response, true) // First poll
}

/// Poll Snowflake for completion, starting with a burst of short delays
/// and then degrading into retry-policy-driven exponential backoff.
/// Each HTTP poll flows through the shared retry helper so transport
/// or retryable status failures are retried automatically. We stop
/// polling once tabular data arrives, Snowflake returns a terminal
/// error, or the overall deadline / retry budget is exhausted.
async fn wait_for_completion(
    client: &reqwest::Client,
    client_info: &ClientInfo,
    session_token: &str,
    result_url: &str,
    policy: &RetryPolicy,
) -> Result<(query_response::Response, usize), SfError> {
    let start = Instant::now();
    let mut attempt: usize = 0;
    let mut sleep_ms = policy.backoff.base.as_millis() as f64;
    let mut polls: usize = 0;

    loop {
        let elapsed = start.elapsed();
        if elapsed >= policy.max_elapsed {
            return Err(SfError::DeadlineExceeded {
                configured: policy.max_elapsed,
                elapsed,
                location: current_location(),
            });
        }

        let delay = if attempt < INLINE_SHORT_POLL_DELAYS.len() {
            INLINE_SHORT_POLL_DELAYS[attempt]
        } else {
            sleep_ms = next_poll_delay_ms(sleep_ms, &policy.backoff);
            Duration::from_millis(sleep_ms as u64)
        };
        attempt += 1;

        if !delay.is_zero() {
            let sleep_deadline = start.elapsed() + delay;
            if sleep_deadline >= policy.max_elapsed {
                return Err(SfError::DeadlineExceeded {
                    configured: policy.max_elapsed,
                    elapsed,
                    location: current_location(),
                });
            }
            tokio::time::sleep(delay).await;
        }

        let elapsed_after_sleep = start.elapsed();
        if elapsed_after_sleep >= policy.max_elapsed {
            return Err(SfError::DeadlineExceeded {
                configured: policy.max_elapsed,
                elapsed: elapsed_after_sleep,
                location: current_location(),
            });
        }

        let remaining = policy
            .max_elapsed
            .checked_sub(elapsed_after_sleep)
            .unwrap_or_default()
            .max(Duration::from_millis(1));

        let mut poll_policy = policy.clone();
        poll_policy.max_elapsed = remaining;
        let response =
            poll_query_status(client, client_info, session_token, result_url, &poll_policy).await?;
        polls += 1;

        if let Some(done) = handle_poll_response(response, false)? {
            return Ok((done, polls));
        }
    }
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

    fn response_from_json(value: serde_json::Value) -> query_response::Response {
        serde_json::from_value(value).expect("valid response JSON")
    }

    #[test]
    fn should_not_poll_when_failure_has_no_result_url() {
        let resp = response_from_json(json!({
            "success": false,
            "data": {
                "rowset": null,
                "rowsetBase64": null
            }
        }));

        assert!(!should_poll_for_completion(&resp));
    }

    #[test]
    fn should_poll_when_result_url_present_and_no_data() {
        let resp = response_from_json(json!({
            "success": true,
            "data": {
                "getResultUrl": "https://example.test",
                "rowset": null,
                "rowsetBase64": null,
                "chunks": null
            }
        }));

        assert!(should_poll_for_completion(&resp));
    }

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
}
