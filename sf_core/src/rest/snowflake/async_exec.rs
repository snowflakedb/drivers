use crate::chunks::ChunkDownloadData;
use crate::config::rest_parameters::{ClientInfo, QueryParameters};
use crate::config::retry::{BackoffConfig, RetryPolicy};
use crate::http::retry::{HttpContext, HttpError, execute_with_retry};
use crate::rest::snowflake::error::SfError;
use crate::rest::snowflake::{
    QUERY_REQUEST_PATH, apply_json_content_type, apply_query_headers, query_request, query_response,
};
use reqwest::{Method, StatusCode};
use snafu::Location;
use std::collections::HashMap;
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
    parameter_bindings: Option<&HashMap<String, query_request::BindParameter>>,
) -> query_request::Request {
    query_request::Request {
        sql_text: sql,
        async_exec: true,
        sequence_id: QUERY_SEQUENCE_ID,
        query_submission_time: current_epoch_millis(),
        is_internal: false,
        describe_only: None,
        parameters: None,
        bindings: parameter_bindings.cloned(),
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

    let body_text = response
        .text()
        .await
        .map_err(|source| transport_error(source))?;
    let parsed: query_response::Response =
        serde_json::from_str(&body_text).map_err(|source| body_parse_error(source))?;
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
    parameter_bindings: Option<&HashMap<String, query_request::BindParameter>>,
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
    let body_text = response
        .text()
        .await
        .map_err(|source| transport_error(source))?;
    let parsed: query_response::Response =
        serde_json::from_str(&body_text).map_err(|source| body_parse_error(source))?;
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
    parameter_bindings: Option<HashMap<String, query_request::BindParameter>>,
    request_id: uuid::Uuid,
    policy: &RetryPolicy,
) -> Result<query_response::Response, SfError> {
    let client_info = &params.client_info;
    let submitted = submit_statement_async(
        client,
        params,
        session_token,
        sql,
        parameter_bindings.as_ref(),
        request_id,
        policy,
    )
    .await?;

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

        if let Some(inline) =
            inline_poll_for_completion(client, client_info, session_token, result_url, policy)
                .await?
        {
            response = inline;
        } else {
            response =
                wait_for_completion(client, client_info, session_token, result_url, policy).await?;
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
    handle_poll_response(response)
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
) -> Result<query_response::Response, SfError> {
    let start = Instant::now();
    let mut attempt: usize = 0;
    let mut sleep_ms = policy.backoff.base.as_millis() as f64;

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

        if let Some(done) = handle_poll_response(response)? {
            return Ok(done);
        }
    }
}

fn should_continue_after_success(resp: &query_response::Response) -> bool {
    resp.data.get_result_url.is_some() && !response_has_tabular_data(resp)
}

fn should_continue_after_failure(resp: &query_response::Response) -> bool {
    resp.data.get_result_url.is_some()
}

fn snowflake_failure(resp: &query_response::Response) -> SfError {
    let code = resp
        .code
        .as_deref()
        .and_then(|c| c.parse::<i32>().ok())
        .unwrap_or(-1);
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

fn handle_poll_response(
    resp: query_response::Response,
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

    Err(snowflake_failure(&resp))
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
}
