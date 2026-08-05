//! GCP metadata-server attestation for Workload Identity Federation.
//!
//! Token acquisition:
//! - **No impersonation**: fetches an OIDC identity token from the GCE
//!   metadata server for the VM's default service account, with audience
//!   `snowflakecomputing.com`.
//! - **With impersonation**: issues a single `generateIdToken` call against
//!   the IAM Service Account Credentials API for the target (last) account in
//!   `workload_identity_impersonation_path`, delegating through any
//!   intermediate accounts via the request's `delegates` field. The resulting
//!   identity token is bound to `snowflakecomputing.com`.
//!
//! The ambient GCE metadata server is always the entry point; the `reqwest`
//! client passed in is reused for both the metadata server and IAM Credentials
//! API calls (same TLS config as for Snowflake).

use crate::config::rest_parameters::WorkloadIdentityConfig;
use crate::sensitive::SensitiveString;
use serde::Deserialize;
use snafu::{Location, ResultExt, Snafu};

use super::AttestationEndpoints;

const METADATA_FLAVOR_HEADER: &str = "metadata-flavor";
const METADATA_FLAVOR_VALUE: &str = "Google";
const SNOWFLAKE_AUDIENCE: &str = "snowflakecomputing.com";
const METADATA_TIMEOUT_SECS: u64 = 10;

/// Errors raised while acquiring a GCP identity-token attestation.
///
/// HTTP-plumbing variants carry a `context` label identifying the endpoint
/// being called (GCE metadata server or IAM Credentials API).
#[derive(Debug, Snafu, error_trace::ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub enum GcpAttestationError {
    #[snafu(display("{context} timed out"))]
    RequestTimedOut {
        context: &'static str,
        source: tokio::time::error::Elapsed,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("{context} failed"))]
    Request {
        context: &'static str,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("{context} returned HTTP {status}: {body}"))]
    UnexpectedHttpStatus {
        context: &'static str,
        status: reqwest::StatusCode,
        body: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read {context} response body"))]
    ResponseBodyRead {
        context: &'static str,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse {context} response"))]
    ResponseParse {
        context: &'static str,
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Acquire a GCP identity token bound to `snowflakecomputing.com`.
///
/// Without impersonation the token comes straight from the GCE metadata
/// server. With impersonation a single IAM Credentials `generateIdToken` call
/// is made against the target service account, delegating through any
/// intermediate accounts via the `delegates` field.
pub(super) async fn get_identity_token(
    client: &reqwest::Client,
    config: &WorkloadIdentityConfig,
    endpoints: &AttestationEndpoints,
) -> Result<String, GcpAttestationError> {
    match config.impersonation_path.split_last() {
        // No impersonation — fetch the identity token directly.
        None => get_identity_token_from_metadata(client, endpoints).await,
        // Impersonation — `target_sa` is the final account; `delegates` is the
        // (possibly empty) intermediate delegation chain.
        Some((target_sa, delegates)) => {
            let access_token = get_access_token_from_metadata(client, endpoints).await?;
            generate_identity_token(
                client,
                access_token.reveal(),
                target_sa,
                delegates,
                SNOWFLAKE_AUDIENCE,
                endpoints,
            )
            .await
        }
    }
}

/// Fetch an OAuth access token for the VM's default service account.
async fn get_access_token_from_metadata(
    client: &reqwest::Client,
    endpoints: &AttestationEndpoints,
) -> Result<SensitiveString, GcpAttestationError> {
    const CTX: &str = "GCE metadata access token";

    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: SensitiveString,
    }

    let url = format!(
        "{}/computeMetadata/v1/instance/service-accounts/default/token",
        endpoints.gcp_metadata_base_url
    );
    let body = metadata_get(client, &url, CTX).await?;
    let parsed: TokenResponse =
        serde_json::from_str(&body).context(ResponseParseSnafu { context: CTX })?;
    Ok(parsed.access_token)
}

/// Fetch an OIDC identity token for the VM's default service account.
async fn get_identity_token_from_metadata(
    client: &reqwest::Client,
    endpoints: &AttestationEndpoints,
) -> Result<String, GcpAttestationError> {
    let url = format!(
        "{}/computeMetadata/v1/instance/service-accounts/default/identity?audience={SNOWFLAKE_AUDIENCE}&format=full",
        endpoints.gcp_metadata_base_url
    );
    // The metadata server returns the JWT directly as plain text.
    metadata_get(client, &url, "GCE metadata identity token").await
}

/// Call `generateIdToken` on the IAM Credentials API for `target_service_account`.
///
/// `delegates` is the intermediate delegation chain (every account in the
/// impersonation path except the target), expressed as full IAM resource
/// names. It is empty for a single-hop impersonation.
async fn generate_identity_token(
    client: &reqwest::Client,
    access_token: &str,
    target_service_account: &str,
    delegates: &[String],
    audience: &str,
    endpoints: &AttestationEndpoints,
) -> Result<String, GcpAttestationError> {
    const CTX: &str = "GCP IAM generateIdToken";

    #[derive(Deserialize)]
    struct IdTokenResponse {
        token: String,
    }

    let url = format!(
        "{}/v1/projects/-/serviceAccounts/{target_service_account}:generateIdToken",
        endpoints.gcp_iam_credentials_base_url
    );
    let delegate_names: Vec<String> = delegates
        .iter()
        .map(|sa| format!("projects/-/serviceAccounts/{sa}"))
        .collect();
    let body = serde_json::json!({
        "audience": audience,
        "includeEmail": true,
        "delegates": delegate_names,
    });
    let resp = tokio::time::timeout(
        std::time::Duration::from_secs(METADATA_TIMEOUT_SECS),
        client
            .post(&url)
            .bearer_auth(access_token)
            .json(&body)
            .send(),
    )
    .await
    .context(RequestTimedOutSnafu { context: CTX })?
    .context(RequestSnafu { context: CTX })?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .context(ResponseBodyReadSnafu { context: CTX })?;
    if !status.is_success() {
        return UnexpectedHttpStatusSnafu {
            context: CTX,
            status,
            body: text,
        }
        .fail();
    }
    let parsed: IdTokenResponse =
        serde_json::from_str(&text).context(ResponseParseSnafu { context: CTX })?;
    Ok(parsed.token)
}

/// Shared helper: GET a GCE metadata server URL with the required header.
async fn metadata_get(
    client: &reqwest::Client,
    url: &str,
    context: &'static str,
) -> Result<String, GcpAttestationError> {
    let resp = tokio::time::timeout(
        std::time::Duration::from_secs(METADATA_TIMEOUT_SECS),
        client
            .get(url)
            .header(METADATA_FLAVOR_HEADER, METADATA_FLAVOR_VALUE)
            .send(),
    )
    .await
    .context(RequestTimedOutSnafu { context })?
    .context(RequestSnafu { context })?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .context(ResponseBodyReadSnafu { context })?;
    if !status.is_success() {
        return UnexpectedHttpStatusSnafu {
            context,
            status,
            body: text,
        }
        .fail();
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rest_parameters::WifProvider;
    use std::time::Duration;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// A config with no impersonation, ready for field overrides.
    fn no_impersonation_config() -> WorkloadIdentityConfig {
        WorkloadIdentityConfig {
            provider: WifProvider::Gcp,
            entra_resource: None,
            impersonation_path: Vec::new(),
            oidc_token: None,
        }
    }

    /// `get_identity_token` (no impersonation) returns the token from the
    /// GCE metadata server's response.
    #[tokio::test]
    async fn get_identity_token_reads_token_from_metadata_server() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(
                "/computeMetadata/v1/instance/service-accounts/default/identity",
            ))
            .and(header(METADATA_FLAVOR_HEADER, METADATA_FLAVOR_VALUE))
            .respond_with(ResponseTemplate::new(200).set_body_string("mocked-gcp-identity-token"))
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            gcp_metadata_base_url: server.uri(),
            ..Default::default()
        };
        let config = no_impersonation_config();
        let client = reqwest::Client::new();

        let token = get_identity_token(&client, &config, &endpoints)
            .await
            .expect("expected identity token");
        assert_eq!(token, "mocked-gcp-identity-token");
    }

    /// Legacy: `test_explicit_gcp_metadata_server_error_bubbles_up` (non-2xx
    /// case). A non-2xx metadata-server response must surface
    /// `GcpAttestationError::UnexpectedHttpStatus`, carrying the status and
    /// body, not a generic/opaque failure.
    #[tokio::test]
    async fn get_identity_token_surfaces_unexpected_http_status_from_metadata_server() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(
                "/computeMetadata/v1/instance/service-accounts/default/identity",
            ))
            .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            gcp_metadata_base_url: server.uri(),
            ..Default::default()
        };
        let config = no_impersonation_config();
        let client = reqwest::Client::new();

        let err = get_identity_token(&client, &config, &endpoints)
            .await
            .expect_err("expected an unexpected-status error");
        match err {
            GcpAttestationError::UnexpectedHttpStatus {
                status,
                body,
                context,
                ..
            } => {
                assert_eq!(status.as_u16(), 500);
                assert_eq!(body, "boom");
                assert_eq!(context, "GCE metadata identity token");
            }
            other => panic!("expected UnexpectedHttpStatus, got {other:?}"),
        }
    }

    /// Legacy: `test_explicit_gcp_metadata_server_error_bubbles_up`
    /// (connection-error case, e.g. `ConnectTimeout`/`HTTPError`). Pointing at
    /// a port that reliably refuses connections (same trick as
    /// `platform_detection::tests::test_detection_config`) must surface
    /// `GcpAttestationError::Request`, not a panic.
    #[tokio::test]
    async fn get_identity_token_surfaces_connection_error_when_metadata_server_unreachable() {
        let endpoints = AttestationEndpoints {
            gcp_metadata_base_url: "http://127.0.0.1:1".to_string(),
            ..Default::default()
        };
        let config = no_impersonation_config();
        let client = reqwest::Client::new();

        let err = get_identity_token(&client, &config, &endpoints)
            .await
            .expect_err("expected a connection error");
        match err {
            GcpAttestationError::Request { context, .. } => {
                assert_eq!(context, "GCE metadata identity token");
            }
            other => panic!("expected Request, got {other:?}"),
        }
    }

    /// No direct legacy analog (legacy's fake metadata server never returns a
    /// malformed body for the access-token endpoint), but this is the same
    /// error family as `test_explicit_gcp_metadata_server_error_bubbles_up`.
    /// A malformed JSON body from the metadata token endpoint (reached via
    /// the impersonation path's OAuth-token step) must surface
    /// `GcpAttestationError::ResponseParse`.
    #[tokio::test]
    async fn get_identity_token_surfaces_response_parse_error_for_malformed_access_token_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(
                "/computeMetadata/v1/instance/service-accounts/default/token",
            ))
            .and(header(METADATA_FLAVOR_HEADER, METADATA_FLAVOR_VALUE))
            .respond_with(ResponseTemplate::new(200).set_body_string("not-json"))
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            gcp_metadata_base_url: server.uri(),
            ..Default::default()
        };
        let config = WorkloadIdentityConfig {
            impersonation_path: vec!["sa1".to_string()],
            ..no_impersonation_config()
        };
        let client = reqwest::Client::new();

        let err = get_identity_token(&client, &config, &endpoints)
            .await
            .expect_err("expected a response-parse error");
        match err {
            GcpAttestationError::ResponseParse { context, .. } => {
                assert_eq!(context, "GCE metadata access token");
            }
            other => panic!("expected ResponseParse, got {other:?}"),
        }
    }

    /// Legacy: `test_explicit_gcp_metadata_server_error_bubbles_up` (timeout
    /// case, `Timeout`/`ConnectTimeout`). Uses paused tokio time (same
    /// pattern as `platform_detection::tests::drops_detectors_when_timeout_reached`)
    /// to force `metadata_get`'s internal `tokio::time::timeout` to fire
    /// without the test actually waiting `METADATA_TIMEOUT_SECS` in real time.
    #[tokio::test(start_paused = true)]
    async fn get_identity_token_times_out_when_metadata_server_is_slow() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(
                "/computeMetadata/v1/instance/service-accounts/default/identity",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("too-slow")
                    .set_delay(Duration::from_secs(METADATA_TIMEOUT_SECS) + Duration::from_secs(1)),
            )
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            gcp_metadata_base_url: server.uri(),
            ..Default::default()
        };
        let config = no_impersonation_config();
        let client = reqwest::Client::new();

        let call =
            tokio::spawn(async move { get_identity_token(&client, &config, &endpoints).await });
        tokio::time::advance(Duration::from_secs(METADATA_TIMEOUT_SECS) + Duration::from_secs(1))
            .await;
        let err = call
            .await
            .expect("task panicked")
            .expect_err("expected a timeout error");
        match err {
            GcpAttestationError::RequestTimedOut { context, .. } => {
                assert_eq!(context, "GCE metadata identity token");
            }
            other => panic!("expected RequestTimedOut, got {other:?}"),
        }
    }

    /// Legacy: `test_gcp_calls_correct_apis_and_populates_auth_data_for_final_sa`.
    /// The most important impersonation scenario: for a multi-hop
    /// `impersonation_path`, the `generateIdToken` call must go to the
    /// *last* account in the path, `delegates` must be every other account
    /// (in order) formatted as full IAM resource names, `audience` must be
    /// `SNOWFLAKE_AUDIENCE`, and the OAuth access token fetched from the
    /// metadata server's `/token` endpoint (the `get_access_token_from_metadata`
    /// step) must be forwarded as the bearer credential.
    #[tokio::test]
    async fn generate_identity_token_posts_correct_request_shape_for_multi_hop_impersonation() {
        let server = MockServer::start().await;
        let access_token = "sa1-access-token";
        let final_token = "sa3-identity-token";

        Mock::given(method("GET"))
            .and(path(
                "/computeMetadata/v1/instance/service-accounts/default/token",
            ))
            .and(header(METADATA_FLAVOR_HEADER, METADATA_FLAVOR_VALUE))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "access_token": access_token })),
            )
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/projects/-/serviceAccounts/sa3:generateIdToken"))
            .and(header("Authorization", format!("Bearer {access_token}")))
            .and(body_json(serde_json::json!({
                "audience": SNOWFLAKE_AUDIENCE,
                "includeEmail": true,
                "delegates": ["projects/-/serviceAccounts/sa2"],
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "token": final_token })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            gcp_metadata_base_url: server.uri(),
            gcp_iam_credentials_base_url: server.uri(),
            ..Default::default()
        };
        let config = WorkloadIdentityConfig {
            impersonation_path: vec!["sa2".to_string(), "sa3".to_string()],
            ..no_impersonation_config()
        };
        let client = reqwest::Client::new();

        let token = get_identity_token(&client, &config, &endpoints)
            .await
            .expect("expected identity token");
        assert_eq!(token, final_token);
    }

    /// Legacy: `test_gcp_calls_correct_apis_and_populates_auth_data_for_final_sa`
    /// covers only a 2-entry path; this covers the single-hop case (target
    /// account only, no intermediates), where `delegates` must be empty.
    #[tokio::test]
    async fn generate_identity_token_sends_empty_delegates_for_single_hop_impersonation() {
        let server = MockServer::start().await;
        let access_token = "sa0-access-token";
        let final_token = "sa1-identity-token";

        Mock::given(method("GET"))
            .and(path(
                "/computeMetadata/v1/instance/service-accounts/default/token",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "access_token": access_token })),
            )
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/projects/-/serviceAccounts/sa1:generateIdToken"))
            .and(body_json(serde_json::json!({
                "audience": SNOWFLAKE_AUDIENCE,
                "includeEmail": true,
                "delegates": Vec::<String>::new(),
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "token": final_token })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            gcp_metadata_base_url: server.uri(),
            gcp_iam_credentials_base_url: server.uri(),
            ..Default::default()
        };
        let config = WorkloadIdentityConfig {
            impersonation_path: vec!["sa1".to_string()],
            ..no_impersonation_config()
        };
        let client = reqwest::Client::new();

        let token = get_identity_token(&client, &config, &endpoints)
            .await
            .expect("expected identity token");
        assert_eq!(token, final_token);
    }

    /// A non-2xx `generateIdToken` response must surface
    /// `GcpAttestationError::UnexpectedHttpStatus` labeled with the IAM
    /// Credentials API's own context, distinct from the metadata-server
    /// context asserted above.
    #[tokio::test]
    async fn generate_identity_token_surfaces_unexpected_http_status() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "/computeMetadata/v1/instance/service-accounts/default/token",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "access_token": "sa0-access-token" })),
            )
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/projects/-/serviceAccounts/sa1:generateIdToken"))
            .respond_with(ResponseTemplate::new(403).set_body_string("permission denied"))
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            gcp_metadata_base_url: server.uri(),
            gcp_iam_credentials_base_url: server.uri(),
            ..Default::default()
        };
        let config = WorkloadIdentityConfig {
            impersonation_path: vec!["sa1".to_string()],
            ..no_impersonation_config()
        };
        let client = reqwest::Client::new();

        let err = get_identity_token(&client, &config, &endpoints)
            .await
            .expect_err("expected an unexpected-status error");
        match err {
            GcpAttestationError::UnexpectedHttpStatus {
                status,
                body,
                context,
                ..
            } => {
                assert_eq!(status.as_u16(), 403);
                assert_eq!(body, "permission denied");
                assert_eq!(context, "GCP IAM generateIdToken");
            }
            other => panic!("expected UnexpectedHttpStatus, got {other:?}"),
        }
    }
}
