use crate::config::retry::{BackoffConfig, HttpPolicy, Jitter, RetryPolicy};
use rand::{Rng, rng};
use reqwest::{Method, Response, StatusCode};
use snafu::{IntoError, Location, ResultExt, Snafu};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct HttpContext {
    pub method: Method,
    pub path: String,
    /// Whether the request is known idempotent server-side (e.g., PUT/DELETE semantics).
    pub idempotent: bool,
    /// Whether to allow POST/PATCH retries for this request (overrides global default).
    pub allow_post_retry: bool,
}

impl HttpContext {
    /// Construct a context with sensible defaults for the supplied method and path.
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        let method_clone = method.clone();
        Self {
            idempotent: matches!(method_clone, Method::PUT | Method::DELETE),
            allow_post_retry: false,
            method,
            path: path.into(),
        }
    }

    /// Mark this context as explicitly idempotent (useful for DELETE, PUT, or POST overrides).
    pub fn with_idempotent(mut self, idempotent: bool) -> Self {
        self.idempotent = idempotent;
        self
    }

    /// Allow POST/PATCH retries for this particular request.
    pub fn allow_post_retry(mut self) -> Self {
        self.allow_post_retry = true;
        self
    }
}

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
pub enum HttpError {
    #[snafu(display("transport error"))]
    #[snafu(visibility(pub(crate)))]
    Transport {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("deadline exceeded after {elapsed:?} (budget {configured:?})"))]
    DeadlineExceeded {
        configured: Duration,
        elapsed: Duration,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("max attempts ({attempts}) reached; last status {last_status}"))]
    MaxAttempts {
        attempts: u32,
        last_status: StatusCode,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("retry-after {retry_after:?} exceeds remaining budget {remaining:?}"))]
    RetryAfterExceeded {
        retry_after: Duration,
        remaining: Duration,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("response body of {size} bytes exceeds max {max_size} bytes"))]
    ResponseTooLarge {
        size: u64,
        max_size: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("operation cancelled"))]
    #[snafu(visibility(pub(crate)))]
    Cancelled {
        #[snafu(implicit)]
        location: Location,
    },
}

impl HttpError {
    pub(crate) fn is_cancelled(&self) -> bool {
        matches!(self, HttpError::Cancelled { .. })
    }
}

/// Calculate effective timeout for a request attempt.
///
/// When `max_elapsed` is set, returns `min(per_request_timeout, remaining_budget)`.
/// When `max_elapsed` is `None`, returns `per_request_timeout` directly (or `None`
/// meaning no per-attempt timeout — the outer operation timeout is the only guard).
fn calculate_request_timeout(
    per_request_timeout: Option<Duration>,
    remaining: Option<Duration>,
) -> Option<Duration> {
    match (per_request_timeout, remaining) {
        (Some(configured), Some(rem)) => Some(configured.min(rem)),
        (Some(configured), None) => Some(configured),
        (None, Some(rem)) => Some(rem),
        (None, None) => None,
    }
}

pub async fn execute_with_retry<T, B, F, H>(
    build_request: B,
    ctx: &HttpContext,
    policy: &RetryPolicy,
    on_response: H,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<T, HttpError>
where
    B: Fn() -> reqwest::RequestBuilder,
    F: std::future::Future<Output = Result<T, HttpError>>,
    H: Fn(Response) -> F,
{
    let mut attempt: u32 = 0;
    let mut sleep_ms: f64 = policy.backoff.base.as_millis() as f64;
    let start = Instant::now();

    let backoff = &policy.backoff;
    let max_attempts = policy.max_attempts;

    loop {
        attempt += 1;

        // When max_elapsed is set, enforce the internal deadline.
        let remaining = if let Some(budget) = policy.max_elapsed {
            let elapsed = start.elapsed();
            if elapsed >= budget {
                return DeadlineExceededSnafu {
                    configured: budget,
                    elapsed,
                }
                .fail();
            }
            Some(budget - elapsed)
        } else {
            None
        };

        let timeout = calculate_request_timeout(policy.per_request_timeout, remaining);
        let mut req_builder = build_request();
        if let Some(t) = timeout {
            tracing::debug!(
                attempt,
                timeout_secs = t.as_secs(),
                remaining_secs = remaining.map(|r| r.as_secs()),
                "Applying per-request timeout"
            );
            req_builder = req_builder.timeout(t);
        }

        // Every outbound HTTP call is logged at INFO so driver-generated traffic
        // is always visible (ud-log-every-http-call-at-info). `ctx.path` may be a
        // full URL for some callers (e.g. chunk downloads), so strip any query
        // string / fragment — those can carry presigned auth tokens and the rule
        // permits only host + path.
        let log_path = ctx.path.split(['?', '#']).next().unwrap_or("");
        tracing::info!(
            method = %ctx.method,
            path = %log_path,
            attempt,
            "outbound HTTP call"
        );

        let result = tokio::select! {
            biased;
            _ = cancel.cancelled() => return CancelledSnafu.fail(),
            sent = req_builder.send() => sent,
        };

        match result {
            Ok(resp) => {
                if resp.status().is_success() {
                    return tokio::select! {
                        biased;
                        _ = cancel.cancelled() => CancelledSnafu.fail(),
                        res = on_response(resp) => res,
                    };
                }
                if !should_retry_status(resp.status(), &policy.extra_retryable_statuses)
                    || !allow_retry(ctx, &policy.http)
                {
                    return tokio::select! {
                        biased;
                        _ = cancel.cancelled() => CancelledSnafu.fail(),
                        res = on_response(resp) => res,
                    };
                }

                if attempt >= max_attempts {
                    return MaxAttemptsSnafu {
                        attempts: attempt,
                        last_status: resp.status(),
                    }
                    .fail();
                }

                let retry_after = parse_retry_after(&resp);
                sleep_ms = next_delay_ms(sleep_ms, backoff);
                let delay = retry_after.unwrap_or(Duration::from_millis(sleep_ms as u64));
                if let Some(rem) = remaining
                    && delay > rem
                {
                    return RetryAfterExceededSnafu {
                        retry_after: delay,
                        remaining: rem,
                    }
                    .fail();
                }
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => return CancelledSnafu.fail(),
                    _ = tokio::time::sleep(delay) => {}
                }
                continue;
            }
            Err(e) => {
                if !is_retryable_transport(&e) || !allow_retry(ctx, &policy.http) {
                    return Err(TransportSnafu.into_error(e));
                }
                if attempt >= max_attempts {
                    return Err(TransportSnafu.into_error(e));
                }
                sleep_ms = next_delay_ms(sleep_ms, backoff);
                let delay = Duration::from_millis(sleep_ms as u64);
                if let Some(rem) = remaining
                    && delay > rem
                {
                    return RetryAfterExceededSnafu {
                        retry_after: delay,
                        remaining: rem,
                    }
                    .fail();
                }
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => return CancelledSnafu.fail(),
                    _ = tokio::time::sleep(delay) => {}
                }
                continue;
            }
        }
    }
}

fn should_retry_status(status: StatusCode, extra: &BTreeSet<u16>) -> bool {
    extra.contains(&status.as_u16())
        || status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
        || status == StatusCode::TEMPORARY_REDIRECT
        || status == StatusCode::PERMANENT_REDIRECT
        || status.is_server_error()
}

fn allow_retry(ctx: &HttpContext, http: &HttpPolicy) -> bool {
    match ctx.method {
        Method::GET | Method::HEAD | Method::OPTIONS => http.retry_safe_reads,
        Method::PUT | Method::DELETE => http.retry_idempotent_writes || ctx.idempotent,
        Method::POST | Method::PATCH => http.retry_post_patch || ctx.allow_post_retry,
        _ => false,
    }
}

fn next_delay_ms(prev_ms: f64, backoff: &BackoffConfig) -> f64 {
    match backoff.jitter {
        Jitter::None => ((prev_ms.max(backoff.base.as_millis() as f64)) * backoff.factor)
            .min(backoff.cap.as_millis() as f64),
        Jitter::Full => {
            let max = ((prev_ms.max(backoff.base.as_millis() as f64)) * backoff.factor)
                .min(backoff.cap.as_millis() as f64);
            let mut rng = rng();
            rng.random_range(0.0..=max)
        }
        Jitter::Decorrelated => {
            // decorrelated jitter: new = rand(base, prev*3) capped
            let upper = (prev_ms.max(backoff.base.as_millis() as f64) * 3.0)
                .min(backoff.cap.as_millis() as f64);
            let mut rng = rng();
            rng.random_range(backoff.base.as_millis() as f64..=upper)
        }
    }
}

fn parse_retry_after(resp: &Response) -> Option<Duration> {
    let h = resp.headers().get(reqwest::header::RETRY_AFTER)?;
    let s = h.to_str().ok()?;
    let secs = s.parse::<u64>().ok()?;
    Some(Duration::from_secs(secs))
}

fn is_retryable_transport(e: &reqwest::Error) -> bool {
    e.is_timeout() || e.is_connect() || e.is_request() || e.is_body() || e.is_decode()
}

/// Convenience helper: execute with retries and return the response body as bytes.
/// Status is validated; non-2xx statuses surface as `HttpError::Transport`.
pub async fn execute_bytes_with_retry<B>(
    build: B,
    ctx: &HttpContext,
    policy: &RetryPolicy,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<Vec<u8>, HttpError>
where
    B: Fn() -> reqwest::RequestBuilder,
{
    execute_with_retry(
        build,
        ctx,
        policy,
        |r| async move {
            match r.error_for_status() {
                Ok(ok) => {
                    let bytes = ok.bytes().await.context(TransportSnafu)?;
                    Ok(bytes.to_vec())
                }
                Err(e) => Err(TransportSnafu.into_error(e)),
            }
        },
        cancel,
    )
    .await
}

/// Like [`execute_bytes_with_retry`] but streams the response body and aborts
/// with [`HttpError::ResponseTooLarge`] the moment the advertised
/// `Content-Length` or the accumulated body size exceeds `max_size`. The size
/// rejection is a property of the payload (not a transient failure), so it is
/// surfaced without further retries. Transport/status failures still flow
/// through the normal retry path.
pub async fn execute_bytes_with_retry_capped<B>(
    build: B,
    ctx: &HttpContext,
    policy: &RetryPolicy,
    max_size: usize,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<Vec<u8>, HttpError>
where
    B: Fn() -> reqwest::RequestBuilder,
{
    use futures::StreamExt;
    execute_with_retry(
        build,
        ctx,
        policy,
        |resp| async move {
            let resp = resp.error_for_status().context(TransportSnafu)?;
            if let Some(len) = resp.content_length()
                && len > max_size as u64
            {
                return ResponseTooLargeSnafu {
                    size: len,
                    max_size,
                }
                .fail();
            }
            let mut stream = resp.bytes_stream();
            let mut buf: Vec<u8> = Vec::new();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.context(TransportSnafu)?;
                if buf.len() + chunk.len() > max_size {
                    return ResponseTooLargeSnafu {
                        size: (buf.len() + chunk.len()) as u64,
                        max_size,
                    }
                    .fail();
                }
                buf.extend_from_slice(&chunk);
            }
            Ok(buf)
        },
        cancel,
    )
    .await
}

#[cfg(test)]
mod timeout_tests {
    use super::*;

    #[test]
    fn no_timeout_when_both_none() {
        assert_eq!(calculate_request_timeout(None, None), None);
    }

    #[test]
    fn falls_back_to_remaining_when_per_request_not_configured() {
        let result = calculate_request_timeout(None, Some(Duration::from_secs(10)));
        assert_eq!(result, Some(Duration::from_secs(10)));
    }

    #[test]
    fn uses_per_request_when_no_budget() {
        let result = calculate_request_timeout(Some(Duration::from_secs(5)), None);
        assert_eq!(result, Some(Duration::from_secs(5)));
    }

    #[test]
    fn uses_configured_when_more_time_available() {
        let result =
            calculate_request_timeout(Some(Duration::from_secs(5)), Some(Duration::from_secs(15)));
        assert_eq!(result, Some(Duration::from_secs(5)));
    }

    #[test]
    fn uses_remaining_when_less_than_configured() {
        let result =
            calculate_request_timeout(Some(Duration::from_secs(10)), Some(Duration::from_secs(3)));
        assert_eq!(result, Some(Duration::from_secs(3)));
    }
}

#[cfg(test)]
mod cancel_tests {
    use super::*;

    #[tokio::test]
    async fn returns_cancelled_when_token_already_cancelled() {
        let cancel = tokio_util::sync::CancellationToken::new();
        cancel.cancel();
        let client = reqwest::Client::new();
        let ctx = HttpContext::new(Method::GET, "/cancel-test");
        let policy = RetryPolicy::default();
        let result: Result<(), HttpError> = execute_with_retry(
            || client.get("http://127.0.0.1:9/"),
            &ctx,
            &policy,
            |_resp| async move { Ok::<(), HttpError>(()) },
            cancel,
        )
        .await;
        assert!(matches!(result, Err(HttpError::Cancelled { .. })));
    }

    #[tokio::test]
    async fn returns_cancelled_when_cancelled_while_reading_body() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            if let Ok((mut socket, _)) = listener.accept().await {
                let _ = socket
                    .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n")
                    .await;
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });

        let cancel = tokio_util::sync::CancellationToken::new();
        let body_cancel = cancel.clone();
        let client = reqwest::Client::new();
        let ctx = HttpContext::new(Method::GET, "/slow-body");
        let policy = RetryPolicy::default();
        let url = format!("http://{addr}/");

        let result: Result<(), HttpError> = execute_with_retry(
            || client.get(url.clone()),
            &ctx,
            &policy,
            move |_resp| {
                let body_cancel = body_cancel.clone();
                async move {
                    // Cancellation lands mid-body: on_response must lose the race
                    // and surface Cancelled instead of a slow success.
                    body_cancel.cancel();
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    Ok::<(), HttpError>(())
                }
            },
            cancel,
        )
        .await;

        assert!(
            matches!(result, Err(HttpError::Cancelled { .. })),
            "expected Cancelled, got {result:?}"
        );
    }
}
