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
//! the nonce embedded in the proof JWT (RFC 9449 §8 + analysis §5.1).

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use http::header::HeaderValue;
use http::{Request, Response};
use oauth2::AsyncHttpClient;
use snafu::ResultExt;
use url::Url;

use super::dpop::{self, DPoPKey};
use super::error::{OAuthError, TransportSnafu};

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

        let status = response.status();
        let mut builder = http::Response::builder().status(status);
        for (name, value) in response.headers().iter() {
            builder = builder.header(name, value);
        }
        let body = response.bytes().await.context(TransportSnafu)?.to_vec();
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

            // DPoP nonce-retry per RFC 9449 §8 + analysis §5.1.
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
