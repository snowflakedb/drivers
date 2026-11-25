//! `oauth2::AsyncHttpClient` adapter over the shared `reqwest::Client`.
//!
//! The OAuth crate is HTTP-client agnostic; it accepts any
//! [`oauth2::AsyncHttpClient`] implementation. We reuse the existing
//! `sf_core` `reqwest::Client` instead of building a separate one so that
//! TLS configuration, proxy settings, and connection pooling all match
//! the rest of the driver.
//!
//! DPoP support is layered here because the token-endpoint hop is the
//! only place a DPoP header needs to be attached during the AC/CC flows
//! (RFC 9449 §5). On a `400 use_dpop_nonce` response the adapter stashes
//! the `DPoP-Nonce` header value and performs exactly one replay with
//! the nonce embedded in a freshly signed proof JWT (RFC 9449 §8). That
//! replay does not count as a transient-retry attempt, but it shares the
//! same authentication budget.
//!
//! Transient failures are retried with the attempt count and backoff of the
//! caller's [`RetryPolicy`](crate::config::retry::RetryPolicy). What counts as
//! transient depends on the [`TokenGrant`]. Client credentials and an
//! authorization code retry HTTP 408, 429, and every 5xx, plus timeouts,
//! connection errors, and errors while reading the body. A client-credentials
//! request can be sent again without side effects. An authorization code is
//! used up by the first request that reaches the IdP, so a resend after a lost
//! response is answered with `invalid_grant`; when that happens after an
//! earlier failure, the call returns
//! [`OAuthError::AuthorizationCodeRetryRejected`] instead of the bare
//! `invalid_grant`. A refresh token may rotate, and a replayed rotating token
//! revokes the whole token family, so it retries only failures where the IdP
//! did not use it: a connection that was never established, HTTP 429, and HTTP
//! 503. TLS failures, redirects, decode errors, and other statuses are never
//! retried. Each attempt rebuilds the DPoP proof.
//!
//! When retries end on a retryable status, that last response goes to the
//! `oauth2` crate, so the caller sees the IdP's own `error`. A refresh-token
//! exchange that ends on HTTP 429 or 5xx returns
//! [`OAuthError::TokenEndpointStatus`] instead, so the caller can tell from
//! the status whether the cached refresh token is still usable. Running out
//! of the authentication budget returns [`OAuthError::AuthenticationTimeout`]
//! with the time since the flow started. A client-credentials or
//! authorization-code attempt is capped at half of the time left when the
//! exchange starts, so a token endpoint that accepts the connection and then
//! stalls leaves time for another attempt.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use http::header::HeaderValue;
use http::{Request, Response};
use oauth2::AsyncHttpClient;
use reqwest::StatusCode;
use url::Url;

use crate::config::retry::RetryPolicy;
use crate::crypto::dpop::{self, DPoPKey};
use crate::http::retry::{
    self, HttpContext, HttpError, LastStatus, RetryRules, RetryState, RetryableSendErrors,
    RetryableStatuses, body_error_to_http, read_body_capped,
};
use crate::utils::sync::MutexRecoverExt;

use super::deadline::FlowDeadline;
use super::error::{AuthorizationCodeRetryRejectedSnafu, OAuthError, TokenEndpointStatusSnafu};

/// Memory guard, not a protocol limit: token responses are a few KiB. The cap
/// equals the one on the other control-plane bodies (CRL download, browser
/// callback, GCP metadata), so no legitimate response is rejected.
const MAX_OAUTH_RESPONSE_BODY_BYTES: usize = 20 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TokenGrant {
    ClientCredentials,
    AuthorizationCode,
    RefreshToken,
}

impl TokenGrant {
    fn retries_lost_responses(self) -> bool {
        matches!(self, Self::ClientCredentials | Self::AuthorizationCode)
    }

    fn retry_rules(self) -> RetryRules {
        if self.retries_lost_responses() {
            RetryRules {
                statuses: RetryableStatuses::TimeoutThrottledOrServerError,
                send_errors: RetryableSendErrors::TransportExceptDecode,
                retry_body_errors: true,
                return_exhausted_response: true,
            }
        } else {
            RetryRules {
                statuses: RetryableStatuses::ThrottledOrUnavailable,
                send_errors: RetryableSendErrors::ConnectOnly,
                retry_body_errors: false,
                return_exhausted_response: true,
            }
        }
    }
}

/// Adapter that lets the `oauth2` crate drive token-endpoint requests
/// through our shared `reqwest::Client`.
///
/// Owns its inputs (cheaply cloneable `reqwest::Client`, `Arc`-shared
/// DPoP key + URL) so the resulting `AsyncHttpClient` impl is free of
/// caller-supplied lifetime parameters. This matters because the wiring
/// layer drives the OAuth flows from inside `async fn`s whose `Send`
/// bounds require a higher-rank `for<'c> AsyncHttpClient<'c>` impl.
pub(super) struct OAuthHttpClient {
    inner: reqwest::Client,
    dpop: Option<DPoPContext>,
    grant: TokenGrant,
    retry_policy: RetryPolicy,
    deadline: FlowDeadline,
}

/// Per-flow DPoP state: the key used to sign proofs, the token endpoint
/// the proof's `htu` claim must point at, and a mutable slot for the
/// most-recent server-supplied nonce.
pub(super) struct DPoPContext {
    key: Arc<DPoPKey>,
    token_url: Arc<Url>,
    last_nonce: Mutex<Option<String>>,
}

impl DPoPContext {
    pub(super) fn new(key: &DPoPKey, token_url: &Url) -> Self {
        Self {
            key: Arc::new(key.clone()),
            token_url: Arc::new(token_url.clone()),
            last_nonce: Mutex::new(None),
        }
    }
}

impl OAuthHttpClient {
    pub(super) fn new(
        inner: &reqwest::Client,
        grant: TokenGrant,
        retry_policy: &RetryPolicy,
        deadline: FlowDeadline,
    ) -> Self {
        Self {
            inner: inner.clone(),
            dpop: None,
            grant,
            retry_policy: retry_policy.clone(),
            deadline,
        }
    }

    pub(super) fn with_dpop(mut self, dpop: DPoPContext) -> Self {
        self.dpop = Some(dpop);
        self
    }

    fn attach_dpop_header(
        &self,
        request: &mut Request<Vec<u8>>,
        nonce: Option<&str>,
    ) -> Result<(), OAuthError> {
        let Some(dpop) = self.dpop.as_ref() else {
            return Ok(());
        };
        let proof = dpop::proof_jwt(&dpop.key, request.method().as_str(), &dpop.token_url, nonce)?;
        // The proof JWT is base64url-encoded JWS by construction, so it's
        // ASCII and always a valid HTTP header value.
        let value = HeaderValue::from_str(proof.reveal())
            .expect("DPoP proof JWT must be a valid HTTP header value");
        request.headers_mut().insert("DPoP", value);
        Ok(())
    }

    fn stored_nonce(&self) -> Option<String> {
        self.dpop
            .as_ref()
            .and_then(|dpop| dpop.last_nonce.lock_recover().clone())
    }

    fn request_builder(
        &self,
        request: &Request<Vec<u8>>,
    ) -> Result<reqwest::RequestBuilder, OAuthError> {
        let mut owned = clone_request(request);
        self.attach_dpop_header(&mut owned, self.stored_nonce().as_deref())?;
        let method = owned.method().clone();
        let uri = owned.uri().to_string();
        let headers = owned.headers().clone();
        let body = owned.body().clone();
        Ok(self.inner.request(method, uri).headers(headers).body(body))
    }

    fn per_request_cap(&self, remaining: Duration, configured: Option<Duration>) -> Duration {
        let grant_cap = if self.grant.retries_lost_responses() {
            let half = remaining / 2;
            if half.is_zero() { remaining } else { half }
        } else {
            remaining
        };
        match configured {
            Some(configured) if !configured.is_zero() => configured.min(grant_cap),
            _ => grant_cap,
        }
    }

    fn with_flow_clock(&self, err: OAuthError) -> OAuthError {
        match err {
            OAuthError::AuthenticationTimeout { .. } => self.deadline.timeout_error(),
            OAuthError::Transport { source, .. }
                if source.is_timeout() && self.deadline.remaining().is_none() =>
            {
                self.deadline.timeout_error()
            }
            other => other,
        }
    }

    fn finish(&self, response: Response<Vec<u8>>) -> Result<Response<Vec<u8>>, OAuthError> {
        let status = response.status();
        if self.grant == TokenGrant::RefreshToken
            && (status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error())
        {
            return TokenEndpointStatusSnafu {
                status: status.as_u16(),
            }
            .fail();
        }
        Ok(response)
    }

    async fn exchange(&self, request: &Request<Vec<u8>>) -> Result<Response<Vec<u8>>, OAuthError> {
        let remaining = self
            .deadline
            .remaining()
            .ok_or_else(|| self.deadline.timeout_error())?;
        let mut policy = self.retry_policy.clone();
        policy.max_elapsed = Some(remaining);
        policy.per_request_timeout =
            Some(self.per_request_cap(remaining, policy.per_request_timeout));

        let http_ctx = HttpContext::new(request.method().clone(), log_target(request))
            .allow_post_retry()
            .with_retry_rules(self.grant.retry_rules());

        let lost_response = Mutex::new(None::<String>);
        let response = retry::try_execute_with_retry_state(
            |state: &RetryState| {
                if self.grant == TokenGrant::AuthorizationCode {
                    note_possibly_used_code(&lost_response, state.get_last_status());
                }
                self.request_builder(request)
            },
            &http_ctx,
            &policy,
            read_token_response,
        )
        .await
        .map_err(|err| self.with_flow_clock(err))?;
        if let Some(previous) = lost_response.lock_recover().take()
            && is_invalid_grant(&response)
        {
            return AuthorizationCodeRetryRejectedSnafu { previous }.fail();
        }
        self.finish(response)
    }

    async fn replay_dpop_nonce_once(
        &self,
        request: &Request<Vec<u8>>,
        response: Response<Vec<u8>>,
    ) -> Result<Response<Vec<u8>>, OAuthError> {
        let Some(dpop) = self.dpop.as_ref() else {
            return Ok(response);
        };
        let Ok(body) = std::str::from_utf8(response.body()) else {
            return Ok(response);
        };
        let Some(server_nonce) = check_use_dpop_nonce(response.headers(), body) else {
            return Ok(response);
        };
        tracing::debug!("retrying OAuth token request with DPoP nonce");
        *dpop.last_nonce.lock_recover() = Some(server_nonce);
        self.exchange(request).await
    }
}

impl<'c> AsyncHttpClient<'c> for OAuthHttpClient {
    type Error = OAuthError;
    type Future = Pin<Box<dyn Future<Output = Result<Response<Vec<u8>>, Self::Error>> + Send + 'c>>;

    #[tracing::instrument(
        skip(self, request),
        fields(
            method = %request.method(),
            authority = request.uri().authority().map(|a| a.as_str()).unwrap_or("").to_string(),
        ),
    )]
    fn call(&'c self, request: Request<Vec<u8>>) -> Self::Future {
        Box::pin(async move {
            let response = self.exchange(&request).await?;
            self.replay_dpop_nonce_once(&request, response).await
        })
    }
}

fn log_target(request: &Request<Vec<u8>>) -> String {
    let host = request.uri().host().unwrap_or("");
    let path = request.uri().path();
    if host.is_empty() {
        path.to_string()
    } else {
        format!("{host}{path}")
    }
}

fn note_possibly_used_code(slot: &Mutex<Option<String>>, previous_attempt: LastStatus) {
    let previous = match previous_attempt {
        LastStatus::Transport => "no response".to_string(),
        LastStatus::Code(code)
            if code != StatusCode::TOO_MANY_REQUESTS.as_u16()
                && code != StatusCode::SERVICE_UNAVAILABLE.as_u16() =>
        {
            format!("HTTP {code}")
        }
        LastStatus::Code(_) | LastStatus::None => return,
    };
    slot.lock_recover().get_or_insert(previous);
}

fn is_invalid_grant(response: &Response<Vec<u8>>) -> bool {
    response.status() == StatusCode::BAD_REQUEST
        && serde_json::from_slice::<serde_json::Value>(response.body())
            .ok()
            .and_then(|body| {
                body.get("error")
                    .and_then(|e| e.as_str())
                    .map(str::to_owned)
            })
            .as_deref()
            == Some("invalid_grant")
}

async fn read_token_response(response: reqwest::Response) -> Result<Response<Vec<u8>>, HttpError> {
    let status = response.status();
    let headers = response.headers().clone();
    let body = read_body_capped(response, MAX_OAUTH_RESPONSE_BODY_BYTES)
        .await
        .map_err(body_error_to_http)?;
    let mut token_response = Response::new(body);
    *token_response.status_mut() = status;
    *token_response.headers_mut() = headers;
    Ok(token_response)
}

/// Build an [`OAuthHttpClient`] from the caller-injected `reqwest::Client`,
/// optionally layering DPoP proof signing when a key is provided.
///
/// The `reqwest::Client` comes from the core login logic (connection pool,
/// TLS, proxy settings are all shared). The [`OAuthHttpClient`] wrapper is
/// required by the `oauth2` crate's `AsyncHttpClient` trait system.
/// `retry_policy` is the connection's shared HTTP retry policy; only its
/// attempt count, backoff, and per-request timeout are used. `deadline` bounds
/// every attempt and the DPoP nonce replay.
pub(super) fn make_http_client(
    client: &reqwest::Client,
    dpop_key: Option<&DPoPKey>,
    token_url: &Url,
    grant: TokenGrant,
    retry_policy: &RetryPolicy,
    deadline: FlowDeadline,
) -> OAuthHttpClient {
    let adapter = OAuthHttpClient::new(client, grant, retry_policy, deadline);
    if let Some(key) = dpop_key {
        adapter.with_dpop(DPoPContext::new(key, token_url))
    } else {
        adapter
    }
}

fn clone_request(request: &Request<Vec<u8>>) -> Request<Vec<u8>> {
    let mut builder = http::Request::builder()
        .method(request.method().clone())
        .uri(request.uri().clone())
        .version(request.version());
    if let Some(headers) = builder.headers_mut() {
        *headers = request.headers().clone();
    }
    builder
        .body(request.body().clone())
        .expect("cloning a well-formed http::Request must succeed")
}

const DPOP_NONCE_HEADER: &str = "DPoP-Nonce";

/// If the IdP responded with an `error == "use_dpop_nonce"` body and a
/// `DPoP-Nonce` header, return the nonce so the caller can retry once
/// with the nonce embedded in the proof (mirrors JDBC
/// `RestRequest.checkForDPoPNonceError`).
fn check_use_dpop_nonce(headers: &http::HeaderMap, body: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    let error = parsed.get("error").and_then(|v| v.as_str())?;
    if error != "use_dpop_nonce" {
        return None;
    }
    let nonce = headers
        .get(DPOP_NONCE_HEADER)
        .or_else(|| headers.get("dpop-nonce"))?
        .to_str()
        .ok()?;
    Some(nonce.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::retry::{BackoffConfig, Jitter};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn check_use_dpop_nonce_extracts_nonce_when_signaled() {
        let body = r#"{"error":"use_dpop_nonce","error_description":"DPoP nonce required"}"#;
        let mut headers = http::HeaderMap::new();
        headers.insert("DPoP-Nonce", "server-nonce-123".parse().unwrap());
        let n = check_use_dpop_nonce(&headers, body).expect("nonce found");
        assert_eq!(n, "server-nonce-123");
    }

    #[test]
    fn check_use_dpop_nonce_is_none_for_unrelated_errors() {
        let body = r#"{"error":"invalid_request"}"#;
        let mut headers = http::HeaderMap::new();
        headers.insert("DPoP-Nonce", "x".parse().unwrap());
        assert!(check_use_dpop_nonce(&headers, body).is_none());
    }

    #[test]
    fn check_use_dpop_nonce_is_none_when_header_missing() {
        let body = r#"{"error":"use_dpop_nonce"}"#;
        let headers = http::HeaderMap::new();
        assert!(check_use_dpop_nonce(&headers, body).is_none());
    }

    /// Serve one HTTP/1.1 response of `body_len` bytes via chunked
    /// transfer-encoding with **no** `Content-Length`, then close, so the test
    /// exercises the streaming size limit where no advertised length is present.
    async fn serve_chunked_once(body_len: usize) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            if let Ok((mut sock, _)) = listener.accept().await {
                let mut scratch = [0u8; 1024];
                let _ = sock.read(&mut scratch).await;
                let head =
                    "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.write_all(format!("{body_len:x}\r\n").as_bytes()).await;
                let _ = sock.write_all(&vec![b'x'; body_len]).await;
                let _ = sock.write_all(b"\r\n0\r\n\r\n").await;
                let _ = sock.flush().await;
            }
        });
        format!("http://{addr}/")
    }

    /// A token-endpoint response that streams past the 20 MiB cap using chunked
    /// encoding (no `Content-Length`) must be rejected. Before the streaming cap
    /// the header-only guard would have let this through and read it all into
    /// memory.
    #[tokio::test]
    async fn send_once_rejects_chunked_body_exceeding_cap_without_content_length() {
        const CAP: usize = 20 * 1024 * 1024;
        let url = serve_chunked_once(CAP + 1).await;
        let client = OAuthHttpClient::new(
            &reqwest::Client::new(),
            TokenGrant::ClientCredentials,
            &RetryPolicy::default(),
            FlowDeadline::new(30),
        );
        let request = http::Request::builder()
            .method("GET")
            .uri(url)
            .body(Vec::new())
            .expect("request builder should accept a well-formed GET");

        let err = client
            .call(request)
            .await
            .expect_err("an oversized chunked token response must be rejected");
        assert!(
            matches!(err, OAuthError::ResponseBodyTooLarge { max_bytes, .. } if max_bytes == CAP as u64),
            "expected ResponseBodyTooLarge {{ max_bytes: 20 MiB }}, got {err:?}"
        );
    }

    enum Reply {
        Send(Vec<u8>),
        SendAfter(Duration, Vec<u8>),
    }

    const TOKEN_JSON: &str = r#"{"access_token":"AT","token_type":"Bearer"}"#;

    fn http_response(status_line: &str, extra_headers: &str, body: &str) -> Vec<u8> {
        format!(
            "HTTP/1.1 {status_line}\r\n{extra_headers}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()
    }

    fn token_ok() -> Reply {
        Reply::Send(http_response("200 OK", "", TOKEN_JSON))
    }

    fn token_cut_mid_body() -> Reply {
        let mut bytes = http_response("200 OK", "", TOKEN_JSON);
        bytes.truncate(bytes.len() - TOKEN_JSON.len() / 2);
        Reply::Send(bytes)
    }

    fn invalid_grant() -> Reply {
        Reply::Send(http_response(
            "400 Bad Request",
            "",
            r#"{"error":"invalid_grant"}"#,
        ))
    }

    fn status_only(status_line: &str) -> Reply {
        Reply::Send(http_response(status_line, "", ""))
    }

    async fn read_request(sock: &mut tokio::net::TcpStream) {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let Ok(n) = sock.read(&mut chunk).await else {
                return;
            };
            if n == 0 {
                return;
            }
            buf.extend_from_slice(&chunk[..n]);
            let Some(head_end) = buf.windows(4).position(|w| w == b"\r\n\r\n") else {
                continue;
            };
            let head = String::from_utf8_lossy(&buf[..head_end]).to_ascii_lowercase();
            let body_len = head
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|v| v.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if buf.len() >= head_end + 4 + body_len {
                return;
            }
        }
    }

    async fn serve(replies: Vec<Reply>) -> (String, Arc<AtomicUsize>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        let accepts = Arc::new(AtomicUsize::new(0));
        let accepts_bg = Arc::clone(&accepts);
        let replies = Arc::new(replies);
        tokio::spawn(async move {
            while let Ok((mut sock, _)) = listener.accept().await {
                let index = accepts_bg.fetch_add(1, Ordering::SeqCst);
                let replies = Arc::clone(&replies);
                tokio::spawn(async move {
                    read_request(&mut sock).await;
                    let (delay, bytes) = match replies.get(index) {
                        Some(Reply::Send(bytes)) => (Duration::ZERO, bytes),
                        Some(Reply::SendAfter(delay, bytes)) => (*delay, bytes),
                        None => {
                            tokio::time::sleep(Duration::from_secs(60)).await;
                            return;
                        }
                    };
                    tokio::time::sleep(delay).await;
                    let _ = sock.write_all(bytes).await;
                    let _ = sock.flush().await;
                });
            }
        });
        (format!("http://{addr}/token"), accepts)
    }

    fn fast_policy(max_attempts: u32) -> RetryPolicy {
        RetryPolicy {
            max_attempts,
            backoff: BackoffConfig {
                base: Duration::ZERO,
                factor: 1.0,
                cap: Duration::ZERO,
                jitter: Jitter::None,
            },
            ..RetryPolicy::default()
        }
    }

    fn token_request(url: &str) -> Request<Vec<u8>> {
        http::Request::builder()
            .method("POST")
            .uri(url)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(b"grant_type=test".to_vec())
            .expect("token request")
    }

    async fn call(
        grant: TokenGrant,
        url: &str,
        policy: &RetryPolicy,
        budget_secs: u64,
    ) -> Result<Response<Vec<u8>>, OAuthError> {
        let client = OAuthHttpClient::new(
            &reqwest::Client::new(),
            grant,
            policy,
            FlowDeadline::new(budget_secs),
        );
        tokio::time::timeout(Duration::from_secs(20), client.call(token_request(url)))
            .await
            .expect("the token call must finish")
    }

    #[tokio::test]
    async fn client_credentials_is_retried_after_body_read_fails() {
        let (url, accepts) = serve(vec![token_cut_mid_body(), token_ok()]).await;
        let response = call(TokenGrant::ClientCredentials, &url, &fast_policy(3), 10)
            .await
            .expect("the second attempt succeeds");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(accepts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn authorization_code_is_retried_after_body_read_fails() {
        let (url, accepts) = serve(vec![token_cut_mid_body(), token_ok()]).await;
        let response = call(TokenGrant::AuthorizationCode, &url, &fast_policy(3), 10)
            .await
            .expect("the second attempt succeeds");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(accepts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn authorization_code_is_retried_after_bad_gateway() {
        let (url, accepts) = serve(vec![status_only("502 Bad Gateway"), token_ok()]).await;
        let response = call(TokenGrant::AuthorizationCode, &url, &fast_policy(3), 10)
            .await
            .expect("the second attempt succeeds");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(accepts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn authorization_code_is_retried_after_a_stalled_attempt() {
        let (url, accepts) = serve(vec![
            Reply::SendAfter(
                Duration::from_secs(30),
                http_response("200 OK", "", TOKEN_JSON),
            ),
            token_ok(),
        ])
        .await;
        let response = call(TokenGrant::AuthorizationCode, &url, &fast_policy(3), 2)
            .await
            .expect("the second attempt succeeds");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(accepts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn authorization_code_rejected_after_a_lost_response_reports_the_first_failure() {
        let (url, accepts) = serve(vec![token_cut_mid_body(), invalid_grant()]).await;
        let err = call(TokenGrant::AuthorizationCode, &url, &fast_policy(3), 10)
            .await
            .expect_err("the retry reuses a code the first request may have used");
        assert!(
            matches!(
                &err,
                OAuthError::AuthorizationCodeRetryRejected { previous, .. } if previous == "no response"
            ),
            "expected AuthorizationCodeRetryRejected, got {err:?}"
        );
        assert_eq!(accepts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn authorization_code_rejected_after_a_bad_gateway_reports_the_status() {
        let (url, _) = serve(vec![status_only("502 Bad Gateway"), invalid_grant()]).await;
        let err = call(TokenGrant::AuthorizationCode, &url, &fast_policy(3), 10)
            .await
            .expect_err("the retry reuses a code the first request may have used");
        assert!(
            matches!(
                &err,
                OAuthError::AuthorizationCodeRetryRejected { previous, .. } if previous == "HTTP 502"
            ),
            "expected AuthorizationCodeRetryRejected, got {err:?}"
        );
    }

    #[tokio::test]
    async fn invalid_grant_without_an_earlier_failure_is_left_to_the_idp_error() {
        let (url, accepts) = serve(vec![invalid_grant()]).await;
        let response = call(TokenGrant::AuthorizationCode, &url, &fast_policy(3), 10)
            .await
            .expect("the IdP's own answer goes to the oauth2 crate");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(accepts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn invalid_grant_after_service_unavailable_is_left_to_the_idp_error() {
        let (url, accepts) = serve(vec![
            status_only("503 Service Unavailable"),
            invalid_grant(),
        ])
        .await;
        let response = call(TokenGrant::AuthorizationCode, &url, &fast_policy(3), 10)
            .await
            .expect("a 503 means the IdP did not use the code");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(accepts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn refresh_token_is_not_sent_again_after_body_read_fails() {
        let (url, accepts) = serve(vec![token_cut_mid_body(), token_ok()]).await;
        let err = call(TokenGrant::RefreshToken, &url, &fast_policy(3), 10)
            .await
            .expect_err("a cut body must not be replayed");
        assert!(
            matches!(err, OAuthError::Transport { .. }),
            "expected Transport, got {err:?}"
        );
        assert_eq!(accepts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn refresh_token_is_not_sent_again_after_bad_gateway() {
        let (url, accepts) = serve(vec![status_only("502 Bad Gateway"), token_ok()]).await;
        let err = call(TokenGrant::RefreshToken, &url, &fast_policy(3), 10)
            .await
            .expect_err("a 502 must not be replayed");
        assert!(
            matches!(err, OAuthError::TokenEndpointStatus { status: 502, .. }),
            "expected TokenEndpointStatus 502, got {err:?}"
        );
        assert_eq!(accepts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn refresh_token_is_retried_after_service_unavailable() {
        let (url, accepts) = serve(vec![status_only("503 Service Unavailable"), token_ok()]).await;
        let response = call(TokenGrant::RefreshToken, &url, &fast_policy(3), 10)
            .await
            .expect("the second attempt succeeds");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(accepts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn refresh_token_ending_on_server_error_reports_the_status() {
        let (url, accepts) = serve(vec![status_only("502 Bad Gateway")]).await;
        let err = call(TokenGrant::RefreshToken, &url, &fast_policy(3), 10)
            .await
            .expect_err("a 502 fails the refresh");
        assert!(
            matches!(err, OAuthError::TokenEndpointStatus { status: 502, .. }),
            "expected TokenEndpointStatus 502, got {err:?}"
        );
        assert_eq!(accepts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn stalled_token_endpoint_reports_authentication_timeout() {
        let (url, accepts) = serve(vec![]).await;
        let err = call(
            TokenGrant::ClientCredentials,
            &url,
            &RetryPolicy::default(),
            1,
        )
        .await
        .expect_err("every attempt stalls");
        assert!(
            matches!(err, OAuthError::AuthenticationTimeout { .. }),
            "expected AuthenticationTimeout, got {err:?}"
        );
        assert!(accepts.load(Ordering::SeqCst) >= 1);
    }

    #[tokio::test]
    async fn dpop_nonce_replay_shares_the_authentication_budget() {
        let nonce_reply = http_response(
            "400 Bad Request",
            "DPoP-Nonce: nonce-1\r\n",
            r#"{"error":"use_dpop_nonce"}"#,
        );
        let (url, accepts) = serve(vec![Reply::SendAfter(
            Duration::from_millis(1500),
            nonce_reply,
        )])
        .await;
        let token_url = Url::parse(&url).expect("token url");
        let key = DPoPKey::generate().expect("DPoP key");
        let client = OAuthHttpClient::new(
            &reqwest::Client::new(),
            TokenGrant::AuthorizationCode,
            &fast_policy(6),
            FlowDeadline::new(2),
        )
        .with_dpop(DPoPContext::new(&key, &token_url));

        let err = tokio::time::timeout(Duration::from_secs(3), client.call(token_request(&url)))
            .await
            .expect("a replay with a fresh 2s budget would still be waiting here")
            .expect_err("the replay stalls");

        assert!(
            matches!(err, OAuthError::AuthenticationTimeout { .. }),
            "expected AuthenticationTimeout, got {err:?}"
        );
        assert_eq!(accepts.load(Ordering::SeqCst), 2);
    }
}
