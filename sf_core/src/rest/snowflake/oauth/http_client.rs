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
//! the `DPoP-Nonce` header value and performs exactly one retry with
//! the nonce embedded in the proof JWT (RFC 9449 §8).

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use http::header::HeaderValue;
use http::{Request, Response};
use oauth2::AsyncHttpClient;
use snafu::{IntoError, ResultExt};
use url::Url;

use crate::http::retry::{CappedBodyError, read_body_capped};

use super::dpop::{self, DPoPKey};
use super::error::{OAuthError, ResponseBodyTooLargeSnafu, TransportSnafu};

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
    pub(super) fn new(inner: &reqwest::Client) -> Self {
        Self {
            inner: inner.clone(),
            dpop: None,
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

    async fn send_once(
        &self,
        mut request: Request<Vec<u8>>,
        nonce: Option<&str>,
    ) -> Result<Response<Vec<u8>>, OAuthError> {
        self.attach_dpop_header(&mut request, nonce)?;

        let reqwest_req: reqwest::Request = request.try_into().context(TransportSnafu)?;
        let response = self
            .inner
            .execute(reqwest_req)
            .await
            .context(TransportSnafu)?;

        const MAX_OAUTH_RESPONSE_BODY_BYTES: u64 = 20 * 1024 * 1024;
        let status = response.status();
        let mut builder = http::Response::builder().status(status);
        for (name, value) in response.headers().iter() {
            builder = builder.header(name, value);
        }
        // Stream the body against a fixed size limit so an oversized response is
        // bounded even under chunked transfer-encoding or an omitted
        // `Content-Length`, which a bare `content_length()` check does not cover.
        let body = match read_body_capped(response, MAX_OAUTH_RESPONSE_BODY_BYTES as usize).await {
            Ok(bytes) => bytes,
            Err(CappedBodyError::TooLarge { .. }) => {
                return ResponseBodyTooLargeSnafu {
                    max_bytes: MAX_OAUTH_RESPONSE_BODY_BYTES,
                }
                .fail();
            }
            Err(CappedBodyError::Transport(source)) => {
                return Err(TransportSnafu.into_error(source));
            }
        };
        Ok(builder
            .body(body)
            .expect("http::Response builder must accept reqwest-provided headers and body"))
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
            let nonce = self
                .dpop
                .as_ref()
                .and_then(|d| d.last_nonce.lock().ok().and_then(|g| g.clone()));
            let first_request = clone_request(&request);
            let response = self.send_once(first_request, nonce.as_deref()).await?;

            // DPoP nonce-retry per RFC 9449 §8.
            if let Some(dpop) = self.dpop.as_ref()
                && let Ok(body) = std::str::from_utf8(response.body())
                && let Some(server_nonce) = dpop::check_use_dpop_nonce(response.headers(), body)
            {
                tracing::debug!("retrying OAuth token request with DPoP nonce");
                if let Ok(mut guard) = dpop.last_nonce.lock() {
                    *guard = Some(server_nonce.clone());
                }
                let retry = clone_request(&request);
                return self.send_once(retry, Some(server_nonce.as_str())).await;
            }

            Ok(response)
        })
    }
}

/// Build an [`OAuthHttpClient`] from the caller-injected `reqwest::Client`,
/// optionally layering DPoP proof signing when a key is provided.
///
/// The `reqwest::Client` comes from the core login logic (connection pool,
/// TLS, proxy settings are all shared). The [`OAuthHttpClient`] wrapper is
/// required by the `oauth2` crate's `AsyncHttpClient` trait system.
pub(super) fn make_http_client(
    client: &reqwest::Client,
    dpop_key: Option<&DPoPKey>,
    token_url: &Url,
) -> OAuthHttpClient {
    let adapter = OAuthHttpClient::new(client);
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
        let client = OAuthHttpClient::new(&reqwest::Client::new());
        let request = http::Request::builder()
            .method("GET")
            .uri(url)
            .body(Vec::new())
            .expect("request builder should accept a well-formed GET");

        let err = client
            .send_once(request, None)
            .await
            .expect_err("an oversized chunked token response must be rejected");
        assert!(
            matches!(err, OAuthError::ResponseBodyTooLarge { max_bytes, .. } if max_bytes == CAP as u64),
            "expected ResponseBodyTooLarge {{ max_bytes: 20 MiB }}, got {err:?}"
        );
    }
}
