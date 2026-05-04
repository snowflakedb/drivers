//! OAuth 2.0 Client Credentials flow (external IdP only).
//!
//! Snowflake-as-IdP does not currently issue tokens for
//! `grant_type=client_credentials` (analysis_feature_oauth.md §4), so this
//! module always requires an explicit `token_url`. Tokens obtained here are
//! intentionally not persisted to the OS token cache (analysis §14 #12).
//!
//! HTTP authentication uses RFC 6749 §2.3.1 Basic auth, mirroring JDBC,
//! ODBC, .NET, Python (default), and Go. Node's `ClientSecretPost` shape
//! is intentionally not replicated.

use std::time::Duration;

use serde::Deserialize;
use snafu::ResultExt;
use url::Url;

use super::authorization_code::{AcquiredOAuthToken, post_token_request};
use super::dpop::{self, DPoPKey};
use super::error::{IdpSnafu, MissingAccessTokenSnafu, OAuthError, TokenResponseDecodeSnafu};
use crate::config::rest_parameters::OAuthClientCredentialsConfig;
use crate::sensitive::SensitiveString;

#[derive(Debug, Deserialize)]
struct TokenResponseBody {
    access_token: Option<String>,
    expires_in: Option<u64>,
    error: Option<String>,
    error_description: Option<String>,
}

#[tracing::instrument(
    skip(client, config),
    fields(token_url = %config.token_url, username = %config.username),
)]
pub(crate) async fn acquire_client_credentials(
    client: &reqwest::Client,
    config: &OAuthClientCredentialsConfig,
) -> Result<AcquiredOAuthToken, OAuthError> {
    tracing::info!("Starting OAuth client credentials flow");

    let dpop_key = if config.enable_dpop {
        Some(DPoPKey::generate()?)
    } else {
        None
    };

    let mut params: Vec<(&str, String)> = vec![("grant_type", "client_credentials".to_string())];
    if let Some(scope) = config.scope.as_deref() {
        params.push(("scope", scope.to_string()));
    }

    let body = post_token_request(
        client,
        &config.token_url,
        &config.client_id,
        config.client_secret.reveal(),
        &params,
        dpop_key.as_ref(),
    )
    .await?;

    let token_response: TokenResponseBody =
        serde_json::from_str(&body).context(TokenResponseDecodeSnafu)?;

    if let Some(error) = token_response.error.as_deref() {
        return IdpSnafu {
            error: error.to_string(),
            description: token_response.error_description.unwrap_or_default(),
        }
        .fail();
    }

    let access_token = token_response
        .access_token
        .filter(|s| !s.is_empty())
        .ok_or_else(|| MissingAccessTokenSnafu.build())?;

    let dpop_jwk_json = match dpop_key.as_ref() {
        Some(k) => Some(k.to_jwk_json()?),
        None => None,
    };

    tracing::info!("OAuth client credentials flow completed");
    // CC tokens MUST NOT be persisted (analysis §14 #12); we hand them
    // straight back to the caller.
    Ok(AcquiredOAuthToken {
        access_token: SensitiveString::from(access_token),
        refresh_token: None,
        dpop_jwk_json,
        expires_in: token_response.expires_in.map(Duration::from_secs),
    })
}

// Keep `dpop` imported under a use-statement to avoid a "imported but
// unused" warning when DPoP is disabled at compile time. The actual usage
// is via the call to [`DPoPKey::generate`] above.
#[allow(dead_code)]
fn _ensure_dpop_used(_: &Url) {
    let _ = dpop::check_use_dpop_nonce;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rest_parameters::DEFAULT_AUTHENTICATION_TIMEOUT_SECS;
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn cfg(token_url: Url) -> OAuthClientCredentialsConfig {
        OAuthClientCredentialsConfig {
            username: "alice".to_string(),
            client_id: "cid".to_string(),
            client_secret: "shh".into(),
            token_url,
            scope: None,
            enable_dpop: false,
            authentication_timeout_secs: DEFAULT_AUTHENTICATION_TIMEOUT_SECS,
        }
    }

    #[tokio::test]
    async fn happy_path_returns_access_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(header("authorization", "Basic Y2lkOnNoaA=="))
            .and(body_string_contains("grant_type=client_credentials"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"access_token":"AT-CC","token_type":"Bearer","expires_in":900}"#,
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
            .respond_with(
                ResponseTemplate::new(401).set_body_string(
                    r#"{"error":"invalid_client","error_description":"bad secret"}"#,
                ),
            )
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
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"token_type":"Bearer","expires_in":900}"#),
            )
            .mount(&server)
            .await;

        let token_url = Url::parse(&format!("{}/token", server.uri())).unwrap();
        let client = reqwest::Client::new();
        let err = acquire_client_credentials(&client, &cfg(token_url))
            .await
            .expect_err("must fail");
        assert!(matches!(err, OAuthError::MissingAccessToken { .. }));
    }

    #[tokio::test]
    async fn scope_is_added_to_body_when_provided() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("scope=session%3Arole%3ADEV"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"AT-SCOPED","token_type":"Bearer"}"#),
            )
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
