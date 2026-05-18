//! OAuth 2.0 Client Credentials flow (external IdP only).
//!
//! Snowflake-as-IdP does not currently issue tokens for
//! `grant_type=client_credentials` (analysis_feature_oauth.md §4), so this
//! module always requires an explicit `token_url`. Tokens obtained here are
//! intentionally not persisted to the OS token cache (analysis §14 #12).
//!
//! HTTP authentication uses RFC 6749 §2.3.1 Basic auth, mirroring JDBC,
//! ODBC, .NET, Python (default), and Go. Node's `ClientSecretPost` shape
//! is intentionally not replicated. The token exchange is driven through
//! the `oauth2` crate via [`OAuthHttpClient`], which also injects the
//! optional DPoP proof header.

use std::time::Duration;

use oauth2::basic::BasicClient;
use oauth2::{ClientId, ClientSecret, Scope, TokenResponse, TokenUrl};

use super::authorization_code::{AcquiredOAuthToken, map_request_token_error};
use super::dpop::DPoPKey;
use super::error::{AuthenticationTimeoutSnafu, MissingAccessTokenSnafu, OAuthError};
use super::http_client::make_http_client;
use crate::config::rest_parameters::OAuthClientCredentialsConfig;
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
) -> Result<AcquiredOAuthToken, OAuthError> {
    tracing::info!("Starting OAuth client credentials flow");

    let dpop_key = if config.flow_options.enable_dpop {
        Some(DPoPKey::generate()?)
    } else {
        None
    };

    // Drift B.1: `LOCAL_APPLICATION` substitution is intentionally NOT
    // performed for CC. CC is external-IdP only (analysis §4: Snowflake's
    // GS does not issue tokens for `grant_type=client_credentials`), and
    // `LoginMethod::from_settings` enforces non-empty `client_id` /
    // `client_secret` before the config ever reaches this function — so
    // the Snowflake-IdP substitution path documented for AC has no
    // analogue here.
    //
    let oauth_client = BasicClient::new(ClientId::new(config.client_id.clone()))
        .set_client_secret(ClientSecret::new(config.client_secret.reveal().to_string()))
        .set_token_uri(TokenUrl::from_url(config.token_url.clone()));

    let mut request = oauth_client.exchange_client_credentials();
    if let Some(scope) = config.scope.as_deref() {
        request = request.add_scope(Scope::new(scope.to_string()));
    }

    let http = make_http_client(client, dpop_key.as_ref(), &config.token_url);

    // Drift B.3: enforce the configured `authentication_timeout` budget
    // around the token-endpoint round-trip. Previously the field was
    // parsed but never honored for CC.
    let budget = Duration::from_secs(config.flow_options.authentication_timeout_secs);
    let response = match tokio::time::timeout(budget, request.request_async(&http)).await {
        Ok(inner) => inner.map_err(map_request_token_error)?,
        Err(_) => {
            return AuthenticationTimeoutSnafu {
                elapsed_secs: config.flow_options.authentication_timeout_secs,
            }
            .fail();
        }
    };

    let access_token = SensitiveString::from(response.access_token().secret().clone());
    if access_token.reveal().is_empty() {
        return MissingAccessTokenSnafu.fail();
    }

    let dpop_jwk_json = match dpop_key.as_ref() {
        Some(k) => Some(k.to_jwk_json()?),
        None => None,
    };

    tracing::info!("OAuth client credentials flow completed");
    // CC tokens MUST NOT be persisted (analysis §14 #12); we hand them
    // straight back to the caller.
    Ok(AcquiredOAuthToken {
        access_token,
        refresh_token: None,
        dpop_jwk_json,
        expires_in: response.expires_in(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rest_parameters::{DEFAULT_AUTHENTICATION_TIMEOUT_SECS, OAuthFlowOptions};
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
        let acquired = acquire_client_credentials(&client, &cfg(token_url))
            .await
            .expect("CC flow succeeds");
        assert_eq!(acquired.access_token.reveal(), "AT-CC");
        assert!(acquired.refresh_token.is_none());
        assert_eq!(acquired.expires_in, Some(Duration::from_secs(900)));
        assert!(acquired.dpop_jwk_json.is_none());
    }

    #[tokio::test]
    async fn idp_error_response_surfaces_idp_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(401).set_body_raw(
                r#"{"error":"invalid_client","error_description":"bad secret"}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/token", server.uri())).unwrap();
        let client = reqwest::Client::new();
        let err = acquire_client_credentials(&client, &cfg(token_url))
            .await
            .expect_err("must fail");
        match err {
            OAuthError::IdpError {
                error, description, ..
            } => {
                assert_eq!(error, "invalid_client");
                assert_eq!(description, "bad secret");
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
        let err = acquire_client_credentials(&client, &cfg(token_url))
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
        let acquired = acquire_client_credentials(&client, &c).await.expect("CC");
        assert_eq!(acquired.access_token.reveal(), "AT-SCOPED");
    }
}
