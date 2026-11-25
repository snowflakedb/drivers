//! OAuth 2.0 Client Credentials flow (external IdP only).
//!
//! Snowflake GS does not currently issue tokens for
//! `grant_type=client_credentials`, so this module always requires an
//! explicit `token_url`. Tokens obtained here are intentionally not
//! persisted to the OS token cache (matches JDBC/.NET/Python/Node).
//!
//! HTTP authentication defaults to RFC 6749 §2.3.1 Basic auth, mirroring
//! JDBC, ODBC, .NET, Python (default), and Go. When
//! `credentials_in_body` is set (Python's `oauth_credentials_in_body`),
//! the `client_id`/`client_secret` are sent in the request body instead
//! (`client_secret_post`, matching Node's `ClientSecretPost` shape) for
//! IdPs that require the body form. Unlike legacy `snowflake-connector-python`
//! — which emits the Basic header *and* duplicates the credentials in the
//! body — UD sends them in the body only. The token exchange is driven
//! through the `oauth2` crate via [`OAuthHttpClient`], which also injects
//! the optional DPoP proof header.

use oauth2::basic::BasicClient;
use oauth2::{AuthType, ClientId, ClientSecret, Scope, TokenResponse, TokenUrl};

use super::authorization_code::{AcquiredOAuthToken, map_request_token_error};
use super::deadline::FlowDeadline;
use super::error::{MissingAccessTokenSnafu, OAuthError};
use super::http_client::{TokenGrant, make_http_client};
use crate::config::rest_parameters::OAuthClientCredentialsConfig;
use crate::config::retry::RetryPolicy;
use crate::crypto::dpop::DPoPKey;
use crate::sensitive::SensitiveString;

#[tracing::instrument(
    skip(client, config),
    fields(token_url = %config.token_url, username = %config.username),
)]
pub(crate) async fn acquire_client_credentials(
    // TODO(SNOW-XXX): build a no-redirect sibling reqwest client for OAuth
    // token calls (see https://docs.rs/oauth2/5.0.0/oauth2/#security-warning).
    client: &reqwest::Client,
    config: &OAuthClientCredentialsConfig,
    retry_policy: &RetryPolicy,
) -> Result<AcquiredOAuthToken, OAuthError> {
    tracing::info!("Starting OAuth client credentials flow");

    let dpop_key = if config.flow_options.enable_dpop {
        Some(DPoPKey::generate()?)
    } else {
        None
    };

    // Drift B.1: `LOCAL_APPLICATION` substitution is intentionally NOT
    // performed for CC. CC is external-IdP only (Snowflake's GS does not
    // issue tokens for `grant_type=client_credentials`), and
    // `LoginMethod::from_settings` enforces non-empty `client_id` /
    // `client_secret` before the config ever reaches this function — so
    // the Snowflake-IdP substitution path documented for AC has no
    // analogue here.
    //
    let oauth_client = BasicClient::new(ClientId::new(config.client_id.clone()))
        .set_client_secret(ClientSecret::new(config.client_secret.reveal().to_string()))
        .set_token_uri(TokenUrl::from_url(config.token_url.clone()));

    // `credentials_in_body` selects RFC 6749 `client_secret_post` (client
    // credentials in the form body) over the default `client_secret_basic`
    // (Authorization: Basic header). Some IdPs require the body form.
    let oauth_client = if config.credentials_in_body {
        oauth_client.set_auth_type(AuthType::RequestBody)
    } else {
        oauth_client
    };

    let mut request = oauth_client.exchange_client_credentials();
    if let Some(scope) = config.scope.as_deref() {
        request = request.add_scope(Scope::new(scope.to_string()));
    }

    let http = make_http_client(
        client,
        dpop_key.as_ref(),
        &config.token_url,
        TokenGrant::ClientCredentials,
        retry_policy,
        FlowDeadline::new(config.flow_options.authentication_timeout_secs),
    );
    let response = request
        .request_async(&http)
        .await
        .map_err(map_request_token_error)?;

    let access_token = SensitiveString::from(response.access_token().secret().clone());
    if access_token.reveal().is_empty() {
        return MissingAccessTokenSnafu.fail();
    }

    tracing::info!("OAuth client credentials flow completed");
    // CC tokens MUST NOT be persisted (matches JDBC/.NET/Python/Node);
    // we hand them straight back to the caller.
    Ok(AcquiredOAuthToken {
        access_token,
        refresh_token: None,
        dpop_key,
        expires_in: response.expires_in(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::param_registry::param_names;
    use crate::config::param_store::ParamStore;
    use crate::config::rest_parameters::{DEFAULT_AUTHENTICATION_TIMEOUT_SECS, OAuthFlowOptions};
    use crate::config::retry::RetryPolicy;
    use crate::config::settings::Setting;
    use std::time::Duration;
    use url::Url;
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn cfg(token_url: Url) -> OAuthClientCredentialsConfig {
        OAuthClientCredentialsConfig {
            username: "alice".to_string(),
            client_id: "cid".to_string(),
            client_secret: "shh".into(),
            token_url,
            scope: None,
            credentials_in_body: false,
            flow_options: OAuthFlowOptions {
                enable_dpop: false,
                authentication_timeout_secs: DEFAULT_AUTHENTICATION_TIMEOUT_SECS,
            },
        }
    }

    #[tokio::test]
    async fn happy_path_returns_access_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(header("authorization", "Basic Y2lkOnNoaA=="))
            .and(body_string_contains("grant_type=client_credentials"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"access_token":"AT-CC","token_type":"Bearer","expires_in":900}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/token", server.uri())).unwrap();
        let client = reqwest::Client::new();
        let acquired =
            acquire_client_credentials(&client, &cfg(token_url), &RetryPolicy::default())
                .await
                .expect("CC flow succeeds");
        assert_eq!(acquired.access_token.reveal(), "AT-CC");
        assert!(acquired.refresh_token.is_none());
        assert_eq!(acquired.expires_in, Some(Duration::from_secs(900)));
        assert!(acquired.dpop_key.is_none());
    }

    #[tokio::test]
    async fn credentials_in_body_sends_client_secret_post() {
        // With `credentials_in_body = true`, the client_id/client_secret
        // are form-encoded into the body (`client_secret_post`) and the
        // RFC 6749 §2.3.1 `Authorization: Basic` header is NOT sent —
        // UD's body-only shape, unlike legacy Python which sends both.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(HeaderIsAbsent("authorization"))
            .and(body_string_contains("client_id=cid"))
            .and(body_string_contains("client_secret=shh"))
            .and(body_string_contains("grant_type=client_credentials"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"access_token":"AT-POST","token_type":"Bearer","expires_in":900}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/token", server.uri())).unwrap();
        let mut c = cfg(token_url);
        c.credentials_in_body = true;
        let client = reqwest::Client::new();
        let acquired = acquire_client_credentials(&client, &c, &RetryPolicy::default())
            .await
            .expect("CC flow succeeds");
        assert_eq!(acquired.access_token.reveal(), "AT-POST");
    }

    #[tokio::test]
    async fn idp_error_response_surfaces_idp_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_raw(
                r#"{"error":"invalid_client","error_description":"The client secret supplied for a confidential client is invalid."}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/token", server.uri())).unwrap();
        let client = reqwest::Client::new();
        let err = acquire_client_credentials(&client, &cfg(token_url), &RetryPolicy::default())
            .await
            .expect_err("must fail");
        match err {
            OAuthError::IdpError {
                error, description, ..
            } => {
                assert_eq!(error, "invalid_client");
                assert_eq!(
                    description,
                    "The client secret supplied for a confidential client is invalid."
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn missing_access_token_in_2xx_response_surfaces_specific_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"token_type":"Bearer","expires_in":900}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/token", server.uri())).unwrap();
        let client = reqwest::Client::new();
        let err = acquire_client_credentials(&client, &cfg(token_url), &RetryPolicy::default())
            .await
            .expect_err("must fail");
        // The oauth2 crate surfaces missing access_token as a parse error;
        // either MissingAccessToken or TokenResponseDecode is acceptable.
        assert!(
            matches!(
                err,
                OAuthError::MissingAccessToken { .. } | OAuthError::TokenResponseDecode { .. }
            ),
            "unexpected: {err:?}"
        );
    }

    #[tokio::test]
    async fn scope_is_added_to_body_when_provided() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("scope=session%3Arole%3ADEV"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"access_token":"AT-SCOPED","token_type":"Bearer"}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/token", server.uri())).unwrap();
        let mut c = cfg(token_url);
        c.scope = Some("session:role:DEV".to_string());
        let client = reqwest::Client::new();
        let acquired = acquire_client_credentials(&client, &c, &RetryPolicy::default())
            .await
            .expect("CC");
        assert_eq!(acquired.access_token.reveal(), "AT-SCOPED");
    }

    /// Wiremock matcher that asserts the absence of a substring in the
    /// request body. Used to lock body shapes across refactors without
    /// pulling in extra dependencies.
    struct BodyDoesNotContain(&'static str);
    impl wiremock::Match for BodyDoesNotContain {
        fn matches(&self, request: &wiremock::Request) -> bool {
            let body = std::str::from_utf8(&request.body).unwrap_or("");
            !body.contains(self.0)
        }
    }

    /// Wiremock matcher asserting a request header is absent. Used to lock
    /// the `client_secret_post` shape: no `Authorization: Basic` header.
    struct HeaderIsAbsent(&'static str);
    impl wiremock::Match for HeaderIsAbsent {
        fn matches(&self, request: &wiremock::Request) -> bool {
            !request.headers.contains_key(self.0)
        }
    }

    #[tokio::test]
    async fn scope_is_omitted_from_body_when_none() {
        // Negative-case sibling to `scope_is_added_to_body_when_provided`.
        // RFC 6749 §4.4.2 makes the `scope` parameter optional; when not
        // configured, our request body must NOT carry an empty
        // `scope=` field (which some IdPs reject).
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(BodyDoesNotContain("scope="))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"access_token":"AT-NOSCOPE","token_type":"Bearer"}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/token", server.uri())).unwrap();
        let c = cfg(token_url);
        assert!(c.scope.is_none(), "default config has scope=None");
        let client = reqwest::Client::new();
        let acquired = acquire_client_credentials(&client, &c, &RetryPolicy::default())
            .await
            .expect("CC");
        assert_eq!(acquired.access_token.reveal(), "AT-NOSCOPE");
    }

    fn token_url(server: &MockServer) -> Url {
        Url::parse(&format!("{}/token", server.uri())).expect("token url")
    }

    async fn mount_retryable_then_token(server: &MockServer, first: ResponseTemplate) {
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(first)
            .up_to_n_times(1)
            .with_priority(1)
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"access_token":"AT-RETRIED","token_type":"Bearer","expires_in":900}"#,
                "application/json",
            ))
            .with_priority(5)
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn oauth_idp_token_retry_after_retryable_status_succeeds() {
        for status in [503u16, 429, 408] {
            let server = MockServer::start().await;
            mount_retryable_then_token(
                &server,
                ResponseTemplate::new(status)
                    .set_body_raw(r#"{"error":"temporarily_unavailable"}"#, "application/json"),
            )
            .await;

            let client = reqwest::Client::new();
            let acquired = tokio::time::timeout(
                Duration::from_secs(15),
                acquire_client_credentials(
                    &client,
                    &cfg(token_url(&server)),
                    &RetryPolicy::default(),
                ),
            )
            .await
            .expect("retryable status must not hang")
            .expect("token endpoint recovers");
            assert_eq!(acquired.access_token.reveal(), "AT-RETRIED");
            let requests = server.received_requests().await.expect("recorded requests");
            assert_eq!(requests.len(), 2, "status {status} must be retried once");
        }
    }

    #[tokio::test]
    async fn oauth_idp_token_retry_after_timeout_succeeds() {
        let server = MockServer::start().await;
        mount_retryable_then_token(
            &server,
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(5))
                .set_body_raw(
                    r#"{"access_token":"AT-LATE","token_type":"Bearer"}"#,
                    "application/json",
                ),
        )
        .await;

        let mut config = cfg(token_url(&server));
        config.flow_options.authentication_timeout_secs = 2;
        let client = reqwest::Client::new();
        let acquired = tokio::time::timeout(
            Duration::from_secs(15),
            acquire_client_credentials(&client, &config, &policy_with_attempts(6)),
        )
        .await
        .expect("a stalled attempt must leave time for another try")
        .expect("token endpoint recovers");
        assert_eq!(acquired.access_token.reveal(), "AT-RETRIED");
        let requests = server.received_requests().await.expect("recorded requests");
        assert_eq!(requests.len(), 2);
    }

    #[tokio::test]
    async fn oauth_idp_token_retry_stall_stops_at_the_authentication_budget() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_secs(5))
                    .set_body_raw(
                        r#"{"access_token":"AT","token_type":"Bearer"}"#,
                        "application/json",
                    ),
            )
            .mount(&server)
            .await;

        let mut config = cfg(token_url(&server));
        config.flow_options.authentication_timeout_secs = 2;
        let client = reqwest::Client::new();
        let err = tokio::time::timeout(
            Duration::from_secs(15),
            acquire_client_credentials(&client, &config, &policy_with_attempts(6)),
        )
        .await
        .expect("a stall must stop when the authentication budget is used")
        .expect_err("every attempt stalls");
        assert!(
            matches!(err, OAuthError::AuthenticationTimeout { .. }),
            "unexpected error: {err:?}"
        );
        let requests = server.received_requests().await.expect("recorded requests");
        assert_eq!(requests.len(), 2, "two 1s attempts fill the 2s budget");
    }

    #[tokio::test]
    async fn oauth_idp_token_retry_skips_non_retryable_4xx() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_raw(
                r#"{"error":"invalid_grant","error_description":"nope"}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let err = tokio::time::timeout(
            Duration::from_secs(5),
            acquire_client_credentials(&client, &cfg(token_url(&server)), &RetryPolicy::default()),
        )
        .await
        .expect("a 400 must not be retried until the deadline")
        .expect_err("400 fails");
        match err {
            OAuthError::IdpError { error, .. } => assert_eq!(error, "invalid_grant"),
            other => panic!("unexpected error: {other:?}"),
        }
        let requests = server.received_requests().await.expect("recorded requests");
        assert_eq!(requests.len(), 1);
    }

    fn policy_with_attempts(attempts: u32) -> RetryPolicy {
        let mut params = ParamStore::new();
        params.insert(
            param_names::RETRY_MAX_ATTEMPTS.as_str().to_string(),
            Setting::Int(attempts as i64),
        );
        params.insert(
            param_names::RETRY_BACKOFF_BASE_MS.as_str().to_string(),
            Setting::Int(0),
        );
        params.insert(
            param_names::RETRY_BACKOFF_CAP_MS.as_str().to_string(),
            Setting::Int(0),
        );
        params.insert(
            param_names::RETRY_BACKOFF_JITTER.as_str().to_string(),
            Setting::String("none".into()),
        );
        RetryPolicy::login(&params)
    }

    #[tokio::test]
    async fn oauth_idp_token_retry_honors_retry_max_attempts() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(503).set_body_raw(
                r#"{"error":"temporarily_unavailable","error_description":"try later"}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let policy = policy_with_attempts(2);
        let client = reqwest::Client::new();
        let err = tokio::time::timeout(
            Duration::from_secs(5),
            acquire_client_credentials(&client, &cfg(token_url(&server)), &policy),
        )
        .await
        .expect("exhausted retries must return")
        .expect_err("503 persists");
        match err {
            OAuthError::IdpError {
                error, description, ..
            } => {
                assert_eq!(error, "temporarily_unavailable");
                assert_eq!(description, "try later");
            }
            other => panic!("unexpected error: {other:?}"),
        }
        let requests = server.received_requests().await.expect("recorded requests");
        assert_eq!(requests.len(), policy.max_attempts as usize);
    }

    #[tokio::test]
    async fn oauth_idp_token_retry_skips_redirects() {
        for status in [307u16, 308] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/token"))
                .respond_with(ResponseTemplate::new(status))
                .mount(&server)
                .await;

            let policy = policy_with_attempts(3);
            let client = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("reqwest client");
            let _err = tokio::time::timeout(
                Duration::from_secs(5),
                acquire_client_credentials(&client, &cfg(token_url(&server)), &policy),
            )
            .await
            .expect("a redirect must not be retried")
            .expect_err("redirect fails");
            let requests = server.received_requests().await.expect("recorded requests");
            assert_eq!(requests.len(), 1, "status {status} must not be retried");
        }
    }

    #[tokio::test]
    async fn oauth_idp_token_retry_skips_certificate_failure() {
        let (addr, accepts) = spawn_untrusted_tls().await;
        let token_url = Url::parse(&format!("https://127.0.0.1:{}/token", addr.port())).unwrap();
        let policy = policy_with_attempts(4);
        let client = reqwest::Client::new();
        let err = tokio::time::timeout(
            Duration::from_secs(10),
            acquire_client_credentials(&client, &cfg(token_url), &policy),
        )
        .await
        .expect("a certificate failure must not be retried")
        .expect_err("certificate failure");
        assert!(
            matches!(err, OAuthError::Transport { .. }),
            "unexpected error: {err:?}"
        );
        assert_eq!(
            accepts.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "certificate validation must not open another connection"
        );
    }

    #[tokio::test]
    async fn dpop_nonce_replay_does_not_consume_transient_attempt() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(
                ResponseTemplate::new(400)
                    .insert_header("DPoP-Nonce", "nonce-from-server")
                    .set_body_raw(r#"{"error":"use_dpop_nonce"}"#, "application/json"),
            )
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(
                ResponseTemplate::new(503)
                    .set_body_raw(r#"{"error":"temporarily_unavailable"}"#, "application/json"),
            )
            .up_to_n_times(1)
            .with_priority(2)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"access_token":"AT-AFTER-NONCE","token_type":"Bearer","expires_in":900}"#,
                "application/json",
            ))
            .with_priority(5)
            .mount(&server)
            .await;

        let mut config = cfg(token_url(&server));
        config.flow_options.enable_dpop = true;
        let policy = policy_with_attempts(2);
        let client = reqwest::Client::new();
        let acquired = tokio::time::timeout(
            Duration::from_secs(5),
            acquire_client_credentials(&client, &config, &policy),
        )
        .await
        .expect("nonce replay must not hang")
        .expect("nonce replay recovers");
        assert_eq!(acquired.access_token.reveal(), "AT-AFTER-NONCE");
        let requests = server.received_requests().await.expect("recorded requests");
        assert_eq!(requests.len(), 3);
        let proofs: std::collections::HashSet<_> = requests
            .iter()
            .map(|request| {
                request
                    .headers
                    .get("dpop")
                    .expect("dpop")
                    .to_str()
                    .expect("dpop ascii")
                    .to_string()
            })
            .collect();
        assert_eq!(proofs.len(), requests.len());
    }

    async fn spawn_untrusted_tls() -> (
        std::net::SocketAddr,
        std::sync::Arc<std::sync::atomic::AtomicU32>,
    ) {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let cert = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()])
            .expect("cert generation");
        let cert_der = rustls::pki_types::CertificateDer::from(cert.cert);
        let key_der = rustls::pki_types::PrivateKeyDer::Pkcs8(
            rustls::pki_types::PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der()),
        );
        let server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der)
            .expect("server config");
        let acceptor = tokio_rustls::TlsAcceptor::from(std::sync::Arc::new(server_config));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        let accepts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let accepts_bg = std::sync::Arc::clone(&accepts);
        tokio::spawn(async move {
            loop {
                let Ok((tcp, _)) = listener.accept().await else {
                    break;
                };
                accepts_bg.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let acceptor = acceptor.clone();
                tokio::spawn(async move {
                    let _ = acceptor.accept(tcp).await;
                });
            }
        });
        (addr, accepts)
    }
}
