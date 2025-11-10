use crate::chunks::ChunkDownloadData;
use crate::config::retry::RetryPolicy;
use crate::http::retry::{HttpContext, execute_with_retry};
use crate::rest::snowflake::error::SfError;
use crate::rest::snowflake::{query_request, query_response, user_agent};
use once_cell::sync::Lazy;
use reqwest::Method;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct SubmitOk {
    pub query_id: String,
    pub response: query_response::Response,
}

pub async fn submit_statement_async(
    client: &reqwest::Client,
    server_url: &str,
    session_token: &str,
    sql: String,
    parameter_bindings: Option<std::collections::HashMap<String, query_request::BindParameter>>,
    request_id: uuid::Uuid,
    policy: &RetryPolicy,
) -> Result<SubmitOk, SfError> {
    let query_url = format!("{server_url}/queries/v1/query-request");

    let query_request = query_request::Request {
        sql_text: sql,
        async_exec: true,
        sequence_id: 1,
        query_submission_time: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
        is_internal: false,
        describe_only: None,
        parameters: None,
        bindings: parameter_bindings,
        bind_stage: None,
        query_context: query_request::QueryContext { entries: None },
    };

    let build = || {
        client
            .post(&query_url)
            .header(
                "Authorization",
                format!("Snowflake Token=\"{session_token}\""),
            )
            .header("Accept", "application/json")
            .header(
                "User-Agent",
                user_agent(&crate::rest::snowflake::client_info::ClientInfo::default()),
            )
            .query(&[("requestId", request_id.to_string())])
            .json(&query_request)
    };

    let ctx = HttpContext {
        method: Method::POST,
        path: "/queries/v1/query-request".to_string(),
        idempotent: false,
        allow_post_retry: true, // safe due to stable requestId
    };

    let resp = execute_with_retry(client, build, &ctx, policy, |r| async move { Ok(r) })
        .await
        .map_err(SfError::from)?;

    let status = resp.status();
    if !status.is_success() {
        return Err(SfError::HttpStatus { status });
    }

    let body_text = resp.text().await.map_err(SfError::from)?;
    let parsed: query_response::Response =
        serde_json::from_str(&body_text).map_err(|e| SfError::BodyParse { source: e })?;
    let qid = parsed
        .data
        ._query_id
        .clone()
        .unwrap_or_else(|| "".to_string());
    if !qid.is_empty() {
        REQUEST_TO_QUERY.insert(request_id, qid.clone());
    }
    Ok(SubmitOk {
        query_id: qid,
        response: parsed,
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PollState {
    NotReady,
    Success,
    Failure,
    Cancelled,
}

pub async fn poll_query_status(
    client: &reqwest::Client,
    get_result_url: &str,
    policy: &RetryPolicy,
) -> Result<(PollState, query_response::Response), SfError> {
    let build = || client.get(get_result_url);
    let ctx = HttpContext {
        method: Method::GET,
        path: get_result_url.to_string(),
        idempotent: true,
        allow_post_retry: false,
    };
    let resp = execute_with_retry(client, build, &ctx, policy, |r| async move { Ok(r) })
        .await
        .map_err(SfError::from)?;
    let status = resp.status();
    if !status.is_success() {
        return Err(SfError::HttpStatus { status });
    }
    let body_text = resp.text().await.map_err(SfError::from)?;
    let parsed: query_response::Response =
        serde_json::from_str(&body_text).map_err(|e| SfError::BodyParse { source: e })?;
    if parsed.data._async_result.is_some() || parsed.data._async_rows.is_some() {
        // Simplified: rely on success flag for terminal; extend with detailed code mapping later
    }
    if parsed.success {
        Ok((PollState::Success, parsed))
    } else {
        Ok((PollState::Failure, parsed))
    }
}

pub async fn execute_blocking_with_async(
    client: &reqwest::Client,
    server_url: &str,
    session_token: &str,
    sql: String,
    parameter_bindings: Option<std::collections::HashMap<String, query_request::BindParameter>>,
    request_id: uuid::Uuid,
    policy: &RetryPolicy,
) -> Result<query_response::Response, SfError> {
    let submitted = submit_statement_async(
        client,
        server_url,
        session_token,
        sql,
        parameter_bindings,
        request_id,
        policy,
    )
    .await?;

    // Inline short-poll
    let start = Instant::now();
    for d in &policy.inline_short_poll.delays {
        if start.elapsed() > policy.inline_short_poll.budget {
            break;
        }
        if d.as_millis() > 0 {
            tokio::time::sleep(*d).await;
        }
        // If get_result_url existed we'd poll it; for now assume success response is terminal
        if submitted.response.success {
            return Ok(submitted.response);
        }
    }

    Ok(submitted.response)
}

static REQUEST_TO_QUERY: Lazy<RequestQueryMap> = Lazy::new(|| RequestQueryMap::default());

#[derive(Default)]
struct RequestQueryMap(Mutex<HashMap<uuid::Uuid, String>>);

impl RequestQueryMap {
    fn insert(&self, rid: uuid::Uuid, qid: String) {
        if let Ok(mut g) = self.0.lock() {
            g.insert(rid, qid);
        }
    }
    #[allow(dead_code)]
    fn get(&self, rid: &uuid::Uuid) -> Option<String> {
        self.0.lock().ok().and_then(|g| g.get(rid).cloned())
    }
}

pub async fn refresh_chunk_download_data_from_get_result(
    client: &reqwest::Client,
    get_result_url: &str,
    policy: &RetryPolicy,
) -> Result<Option<Vec<ChunkDownloadData>>, SfError> {
    let (state, resp) = poll_query_status(client, get_result_url, policy).await?;
    match state {
        PollState::Success => Ok(resp.data.to_chunk_download_data()),
        _ => Ok(None),
    }
}
