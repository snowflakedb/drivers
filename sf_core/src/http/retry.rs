use crate::config::retry::{BackoffConfig, HttpPolicy, Jitter, RetryPolicy};
use rand::{Rng, rng};
use reqwest::{Method, Response, StatusCode};
use snafu::{IntoError, Location, ResultExt, Snafu};
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

#[derive(Debug, Snafu)]
pub enum HttpError {
    #[snafu(display("transport error"))]
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
}

pub async fn execute_with_retry<T, B, F, H>(
    build_request: B,
    ctx: &HttpContext,
    policy: &RetryPolicy,
    on_response: H,
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
        let elapsed = start.elapsed();
        if elapsed >= policy.max_elapsed {
            return DeadlineExceededSnafu {
                configured: policy.max_elapsed,
                elapsed,
            }
            .fail();
        }
        let remaining = policy.max_elapsed - elapsed;

        let result = build_request().send().await;

        match result {
            Ok(resp) => {
                if resp.status().is_success() {
                    return on_response(resp).await;
                }
                if !should_retry_status(resp.status()) || !allow_retry(ctx, &policy.http) {
                    // Non-retryable status: surface response to caller
                    return on_response(resp).await;
                }

                if attempt >= max_attempts {
                    return MaxAttemptsSnafu {
                        attempts: attempt,
                        last_status: resp.status(),
                    }
                    .fail();
                }

                // Honor Retry-After if present
                let retry_after = parse_retry_after(&resp);
                sleep_ms = next_delay_ms(sleep_ms, backoff);
                let delay = retry_after.unwrap_or(Duration::from_millis(sleep_ms as u64));
                if delay > remaining {
                    return RetryAfterExceededSnafu {
                        retry_after: delay,
                        remaining,
                    }
                    .fail();
                }
                tokio::time::sleep(delay).await;
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
                if delay > remaining {
                    return RetryAfterExceededSnafu {
                        retry_after: delay,
                        remaining,
                    }
                    .fail();
                }
                tokio::time::sleep(delay).await;
                continue;
            }
        }
    }
}

fn should_retry_status(status: StatusCode) -> bool {
    status == StatusCode::REQUEST_TIMEOUT
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
) -> Result<Vec<u8>, HttpError>
where
    B: Fn() -> reqwest::RequestBuilder,
{
    let resp = execute_with_retry(build, ctx, policy, |r| async move { Ok(r) }).await?;
    match resp.error_for_status() {
        Ok(ok) => {
            let bytes = ok.bytes().await.context(TransportSnafu)?;
            Ok(bytes.to_vec())
        }
        Err(e) => Err(TransportSnafu.into_error(e)),
    }
}
