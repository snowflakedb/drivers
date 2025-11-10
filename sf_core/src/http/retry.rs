use crate::config::retry::{BackoffStrategy, HttpPolicy, Jitter, RetryPolicy};
use rand::{Rng, thread_rng};
use reqwest::{Method, Response, StatusCode};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct HttpContext {
    pub method: Method,
    pub path: String,
    /// Whether the request is known idempotent server-side (e.g., PUT/DELETE semantics)
    pub idempotent: bool,
    /// Whether to allow POST/PATCH retries for this request (overrides global default)
    pub allow_post_retry: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum HttpError {
    #[error("transport error")]
    Transport(#[from] reqwest::Error),
    #[error("http status {status}: {body}")]
    HttpStatus { status: StatusCode, body: String },
    #[error("deadline exceeded")]
    DeadlineExceeded,
}

pub async fn execute_with_retry<T, B, F, H>(
    _client: &reqwest::Client,
    build: B,
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
    let mut sleep_ms: f64 = policy.submission.backoff.base.as_millis() as f64; // seed; updated per category if needed by caller
    let start = Instant::now();

    let cat = select_category(ctx, policy);
    let Category { max_attempts, bo } = cat;

    loop {
        attempt += 1;
        if start.elapsed() > policy.deadline {
            return Err(HttpError::DeadlineExceeded);
        }

        let req = build();
        let result = req.try_clone().unwrap_or_else(&build).send().await;

        match result {
            Ok(resp) => {
                if resp.status().is_success() {
                    return on_response(resp).await;
                }

                if should_retry_status(resp.status()) && allow_retry(ctx, &policy.http) {
                    // Honor Retry-After if present
                    let retry_after = parse_retry_after(&resp);
                    sleep_ms = next_delay_ms(sleep_ms, &bo);
                    let delay = retry_after.unwrap_or(Duration::from_millis(sleep_ms as u64));
                    if attempt >= max_attempts {
                        // Return the response to let caller decide how to surface status/body
                        return on_response(resp).await;
                    }
                    tokio::time::sleep(delay).await;
                    continue;
                } else {
                    // Non-retryable status: surface response to caller
                    return on_response(resp).await;
                }
            }
            Err(e) => {
                if is_retryable_transport(&e) && allow_retry(ctx, &policy.http) {
                    if attempt >= max_attempts {
                        return Err(HttpError::Transport(e));
                    }
                    sleep_ms = next_delay_ms(sleep_ms, &bo);
                    tokio::time::sleep(Duration::from_millis(sleep_ms as u64)).await;
                    continue;
                } else {
                    return Err(HttpError::Transport(e));
                }
            }
        }
    }
}

struct Category {
    max_attempts: u32,
    bo: BackoffStrategy,
}

fn select_category(ctx: &HttpContext, policy: &RetryPolicy) -> Category {
    // Map method to a default category; callers can pass a different policy by wrapping this helper if needed.
    match ctx.method {
        Method::GET | Method::HEAD | Method::OPTIONS => Category {
            max_attempts: policy
                .chunk
                .max_attempts
                .max(policy.poll.max_attempts.min(16)),
            bo: policy.chunk.backoff.clone(),
        },
        Method::PUT | Method::DELETE => Category {
            max_attempts: policy.submission.max_attempts,
            bo: policy.submission.backoff.clone(),
        },
        _ => Category {
            max_attempts: policy.submission.max_attempts,
            bo: policy.submission.backoff.clone(),
        },
    }
}

fn should_retry_status(status: StatusCode) -> bool {
    status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
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

fn next_delay_ms(prev_ms: f64, bo: &BackoffStrategy) -> f64 {
    match bo.jitter {
        Jitter::None => {
            ((prev_ms.max(bo.base.as_millis() as f64)) * bo.factor).min(bo.cap.as_millis() as f64)
        }
        Jitter::Full => {
            let max = ((prev_ms.max(bo.base.as_millis() as f64)) * bo.factor)
                .min(bo.cap.as_millis() as f64);
            thread_rng().gen_range(0.0..=max)
        }
        Jitter::Decorrelated => {
            // decorrelated jitter: new = rand(base, prev*3) capped
            let upper =
                (prev_ms.max(bo.base.as_millis() as f64) * 3.0).min(bo.cap.as_millis() as f64);
            thread_rng().gen_range(bo.base.as_millis() as f64..=upper)
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
