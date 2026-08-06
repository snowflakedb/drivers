//! Azure Managed Identity attestation for Workload Identity Federation.
//!
//! Token acquisition priority:
//! 1. Azure Functions runtime (environment variables `IDENTITY_ENDPOINT` +
//!    `IDENTITY_HEADER` or legacy `MSI_ENDPOINT` + `MSI_SECRET`).
//! 2. Azure IMDS (`http://169.254.169.254/metadata/identity/oauth2/token`).
//!
//! The `resource` (Entra audience) defaults to the Snowflake-assigned
//! resource URI [`crate::config::rest_parameters::DEFAULT_AZURE_ENTRA_RESOURCE`].
//! Callers can override via `workload_identity_entra_resource`.
//!
//! When `workload_identity_impersonation_path` is set (exactly one SP client_id),
//! a two-step flow is performed:
//! 1. Acquire a MI token scoped to `AZURE_WIF_FEDERATION_AUDIENCE`.
//! 2. Exchange it for an SP access token via Entra ID's `oauth2/v2.0/token`
//!    endpoint (`client_credentials` grant with JWT-bearer client assertion).
//!    The resulting SP token is what is sent to Snowflake.

use crate::config::rest_parameters::{DEFAULT_AZURE_ENTRA_RESOURCE, WorkloadIdentityConfig};
use crate::sensitive::SensitiveString;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use snafu::{Location, OptionExt, ResultExt, Snafu};

use super::AttestationEndpoints;

const IMDS_TIMEOUT_SECS: u64 = 10;
/// Audience used when requesting a Managed Identity token for service-principal
/// impersonation. The MI token is exchanged (not sent to Snowflake directly).
const AZURE_WIF_FEDERATION_AUDIENCE: &str = "api://AzureADTokenExchange";

/// Errors raised while acquiring an Azure Managed Identity attestation.
///
/// HTTP-plumbing variants carry a `context` label identifying the endpoint
/// being called (IMDS, Azure Functions, or Entra ID token exchange).
#[derive(Debug, Snafu, error_trace::ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub enum AzureAttestationError {
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
    #[snafu(display("Managed Identity token is not a valid JWT (missing payload segment)"))]
    JwtMissingPayload {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to base64-decode JWT payload"))]
    JwtPayloadDecode {
        source: base64::DecodeError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse JWT payload JSON"))]
    JwtPayloadParse {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Managed Identity token is missing 'tid' claim; cannot determine tenant ID for impersonation"
    ))]
    JwtMissingTidClaim {
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Deserialize)]
struct ManagedIdentityTokenResponse {
    access_token: SensitiveString,
}

/// Acquire an Azure token for Workload Identity Federation.
///
/// Without impersonation: fetches a Managed Identity access token scoped to
/// the Snowflake Entra resource (default or caller-supplied) and returns it.
///
/// With impersonation (`impersonation_path` has exactly one element — the SP
/// client_id): acquires an MI token scoped to `api://AzureADTokenExchange`,
/// then exchanges it for a Service Principal access token via Entra ID's
/// `oauth2/v2.0/token` endpoint. The SP token is returned.
pub(super) async fn get_managed_identity_token(
    client: &reqwest::Client,
    config: &WorkloadIdentityConfig,
    endpoints: &AttestationEndpoints,
) -> Result<String, AzureAttestationError> {
    let snowflake_resource = config
        .entra_resource
        .as_deref()
        .unwrap_or(DEFAULT_AZURE_ENTRA_RESOURCE);

    // When impersonating an SP, the MI token must be issued for the federation
    // audience so Entra ID will accept it as a client assertion.
    let mi_resource = if config.impersonation_path.is_empty() {
        snowflake_resource
    } else {
        AZURE_WIF_FEDERATION_AUDIENCE
    };

    let client_id = std::env::var("MANAGED_IDENTITY_CLIENT_ID").ok();

    if let (Ok(endpoint), Ok(header)) = (
        std::env::var("IDENTITY_ENDPOINT"),
        std::env::var("IDENTITY_HEADER"),
    ) {
        return get_from_azure_functions(
            &endpoint,
            &header,
            mi_resource,
            client_id.as_deref(),
            client,
            snowflake_resource,
            &config.impersonation_path,
            endpoints,
        )
        .await;
    }

    // Legacy Azure Functions (older runtimes)
    if let (Ok(endpoint), Ok(secret)) = (std::env::var("MSI_ENDPOINT"), std::env::var("MSI_SECRET"))
    {
        return get_from_azure_functions(
            &endpoint,
            &secret,
            mi_resource,
            client_id.as_deref(),
            client,
            snowflake_resource,
            &config.impersonation_path,
            endpoints,
        )
        .await;
    }

    get_from_imds(
        mi_resource,
        client_id.as_deref(),
        client,
        snowflake_resource,
        &config.impersonation_path,
        endpoints,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn get_from_azure_functions(
    endpoint: &str,
    identity_header: &str,
    mi_resource: &str,
    client_id: Option<&str>,
    client: &reqwest::Client,
    snowflake_resource: &str,
    impersonation_path: &[String],
    endpoints: &AttestationEndpoints,
) -> Result<String, AzureAttestationError> {
    const CTX: &str = "Azure Functions identity request";

    let mut url = format!("{endpoint}?resource={mi_resource}&api-version=2019-08-01");
    if let Some(id) = client_id {
        url.push_str(&format!("&client_id={id}"));
    }

    let parsed = reqwest::Url::parse(&url).ok();
    tracing::info!(
        method = "GET",
        host = parsed
            .as_ref()
            .and_then(|u| u.host_str())
            .unwrap_or("<none>"),
        path = parsed.as_ref().map_or("", |u| u.path()),
        "outbound HTTP call"
    );
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(IMDS_TIMEOUT_SECS),
        client
            .get(&url)
            .header("X-IDENTITY-HEADER", identity_header)
            .send(),
    )
    .await
    .context(RequestTimedOutSnafu { context: CTX })?
    .context(RequestSnafu { context: CTX })?;

    let status = response.status();
    tracing::info!(status = status.as_u16(), "HTTP response");
    let body = response
        .text()
        .await
        .context(ResponseBodyReadSnafu { context: CTX })?;

    if !status.is_success() {
        return UnexpectedHttpStatusSnafu {
            context: CTX,
            status,
            body,
        }
        .fail();
    }

    let parsed: ManagedIdentityTokenResponse =
        serde_json::from_str(&body).context(ResponseParseSnafu { context: CTX })?;
    maybe_impersonate_sp(
        parsed.access_token.reveal().to_string(),
        snowflake_resource,
        impersonation_path,
        client,
        endpoints,
    )
    .await
}

async fn get_from_imds(
    mi_resource: &str,
    client_id: Option<&str>,
    client: &reqwest::Client,
    snowflake_resource: &str,
    impersonation_path: &[String],
    endpoints: &AttestationEndpoints,
) -> Result<String, AzureAttestationError> {
    const CTX: &str = "Azure IMDS request";

    let mut url = format!(
        "{}/metadata/identity/oauth2/token?api-version=2018-02-01&resource={mi_resource}",
        endpoints.azure_imds_base_url
    );
    if let Some(id) = client_id {
        url.push_str(&format!("&client_id={id}"));
    }

    let parsed = reqwest::Url::parse(&url).ok();
    tracing::info!(
        method = "GET",
        host = parsed
            .as_ref()
            .and_then(|u| u.host_str())
            .unwrap_or("<none>"),
        path = parsed.as_ref().map_or("", |u| u.path()),
        "outbound HTTP call"
    );
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(IMDS_TIMEOUT_SECS),
        client.get(&url).header("Metadata", "true").send(),
    )
    .await
    .context(RequestTimedOutSnafu { context: CTX })?
    .context(RequestSnafu { context: CTX })?;

    let status = response.status();
    tracing::info!(status = status.as_u16(), "HTTP response");
    let body = response
        .text()
        .await
        .context(ResponseBodyReadSnafu { context: CTX })?;

    if !status.is_success() {
        return UnexpectedHttpStatusSnafu {
            context: CTX,
            status,
            body,
        }
        .fail();
    }

    let parsed: ManagedIdentityTokenResponse =
        serde_json::from_str(&body).context(ResponseParseSnafu { context: CTX })?;
    maybe_impersonate_sp(
        parsed.access_token.reveal().to_string(),
        snowflake_resource,
        impersonation_path,
        client,
        endpoints,
    )
    .await
}

/// If `impersonation_path` is non-empty, exchanges the MI token for a Service
/// Principal access token via the Entra ID `oauth2/v2.0/token` endpoint.
/// Otherwise, returns `mi_token` unchanged.
async fn maybe_impersonate_sp(
    mi_token: String,
    snowflake_resource: &str,
    impersonation_path: &[String],
    client: &reqwest::Client,
    endpoints: &AttestationEndpoints,
) -> Result<String, AzureAttestationError> {
    if impersonation_path.is_empty() {
        return Ok(mi_token);
    }
    // Validation in connection_config ensures exactly one element for Azure.
    let sp_client_id = &impersonation_path[0];
    get_sp_token_via_impersonation(
        &mi_token,
        sp_client_id,
        snowflake_resource,
        client,
        endpoints,
    )
    .await
}

/// Exchange a Managed Identity JWT for a Service Principal access token.
///
/// The Entra tenant is extracted from the `tid` claim of the MI token (no
/// signature verification — the real verification happens server-side).
async fn get_sp_token_via_impersonation(
    mi_token: &str,
    sp_client_id: &str,
    snowflake_resource: &str,
    client: &reqwest::Client,
    endpoints: &AttestationEndpoints,
) -> Result<String, AzureAttestationError> {
    const CTX: &str = "Entra ID SP token exchange";

    let tenant_id = extract_tid_from_jwt(mi_token)?;

    #[derive(Deserialize)]
    struct SpTokenResponse {
        access_token: String,
    }

    let url = format!(
        "{}/{tenant_id}/oauth2/v2.0/token",
        endpoints.azure_entra_base_url
    );
    let params = [
        ("grant_type", "client_credentials"),
        ("client_id", sp_client_id),
        (
            "client_assertion_type",
            "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
        ),
        ("client_assertion", mi_token),
        ("scope", &format!("{snowflake_resource}/.default")),
    ];

    let parsed = reqwest::Url::parse(&url).ok();
    tracing::info!(
        method = "POST",
        host = parsed
            .as_ref()
            .and_then(|u| u.host_str())
            .unwrap_or("<none>"),
        path = parsed.as_ref().map_or("", |u| u.path()),
        "outbound HTTP call"
    );
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(IMDS_TIMEOUT_SECS),
        client.post(&url).form(&params).send(),
    )
    .await
    .context(RequestTimedOutSnafu { context: CTX })?
    .context(RequestSnafu { context: CTX })?;

    let status = response.status();
    tracing::info!(status = status.as_u16(), "HTTP response");
    let body = response
        .text()
        .await
        .context(ResponseBodyReadSnafu { context: CTX })?;

    if !status.is_success() {
        return UnexpectedHttpStatusSnafu {
            context: CTX,
            status,
            body,
        }
        .fail();
    }

    let parsed: SpTokenResponse =
        serde_json::from_str(&body).context(ResponseParseSnafu { context: CTX })?;
    Ok(parsed.access_token)
}

/// Extract the `tid` (tenant ID) claim from the JWT payload without verifying
/// the signature. Used only to determine the correct Entra tenant endpoint;
/// real token verification is performed server-side by Snowflake.
fn extract_tid_from_jwt(jwt: &str) -> Result<String, AzureAttestationError> {
    let payload_b64 = jwt.split('.').nth(1).context(JwtMissingPayloadSnafu)?;

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .context(JwtPayloadDecodeSnafu)?;

    let payload: serde_json::Value =
        serde_json::from_slice(&payload_bytes).context(JwtPayloadParseSnafu)?;

    payload["tid"]
        .as_str()
        .map(str::to_string)
        .context(JwtMissingTidClaimSnafu)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rest_parameters::WifProvider;
    use std::collections::HashMap;
    use std::future::Future;
    use std::time::Duration;
    use wiremock::matchers::{header, method, path, query_param, query_param_is_missing};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Builds a minimal Azure `WorkloadIdentityConfig` for the tests below.
    fn azure_config(
        entra_resource: Option<&str>,
        impersonation_path: Vec<String>,
    ) -> WorkloadIdentityConfig {
        WorkloadIdentityConfig {
            provider: WifProvider::Azure,
            entra_resource: entra_resource.map(str::to_string),
            impersonation_path,
            oidc_token: None,
        }
    }

    /// Builds a 3-segment JWT carrying the given payload JSON. The header
    /// and signature segments are never inspected by `extract_tid_from_jwt`
    /// or by any production code exercised here, so their content is
    /// arbitrary.
    fn make_jwt(payload_json: &str) -> String {
        format!(
            "{}.{}.{}",
            URL_SAFE_NO_PAD.encode(b"{}"),
            URL_SAFE_NO_PAD.encode(payload_json.as_bytes()),
            URL_SAFE_NO_PAD.encode(b"sig"),
        )
    }

    /// Clears the Azure Functions env vars so `get_managed_identity_token`
    /// always takes the IMDS path, optionally setting
    /// `MANAGED_IDENTITY_CLIENT_ID` for the duration of `f`.
    async fn without_azure_functions_env<F: Future<Output = ()>>(client_id: Option<&str>, f: F) {
        temp_env::async_with_vars(
            [
                ("IDENTITY_ENDPOINT", None::<&str>),
                ("IDENTITY_HEADER", None::<&str>),
                ("MSI_ENDPOINT", None::<&str>),
                ("MSI_SECRET", None::<&str>),
                ("MANAGED_IDENTITY_CLIENT_ID", client_id),
            ],
            f,
        )
        .await;
    }

    /// `get_managed_identity_token` returns the token from the Azure IMDS
    /// response.
    #[tokio::test]
    async fn get_managed_identity_token_reads_token_from_imds() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/metadata/identity/oauth2/token"))
            .and(header("Metadata", "true"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"access_token":"mocked-mi-token"}"#),
            )
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            azure_imds_base_url: server.uri(),
            ..Default::default()
        };
        let config = azure_config(None, Vec::new());
        let client = reqwest::Client::new();

        without_azure_functions_env(None, async {
            let token = get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect("expected managed identity token");
            assert_eq!(token, "mocked-mi-token");
        })
        .await;
    }

    // -- Azure Functions dispatch --
    //
    // `get_managed_identity_token` dispatches to `get_from_azure_functions`
    // instead of IMDS when either `IDENTITY_ENDPOINT`/`IDENTITY_HEADER`
    // (Azure Functions) or `MSI_ENDPOINT`/`MSI_SECRET` (legacy App Service
    // MSI) are set. Neither branch is exercised by any test above, which all
    // force the IMDS path via `without_azure_functions_env`.

    /// Sets `IDENTITY_ENDPOINT`/`IDENTITY_HEADER` (clearing the legacy
    /// `MSI_ENDPOINT`/`MSI_SECRET` pair) so `get_managed_identity_token`
    /// takes the Azure Functions branch, optionally setting
    /// `MANAGED_IDENTITY_CLIENT_ID` for the duration of `f`. Inverse of
    /// `without_azure_functions_env`.
    async fn with_azure_functions_env<F: Future<Output = ()>>(
        endpoint: &str,
        identity_header: &str,
        client_id: Option<&str>,
        f: F,
    ) {
        temp_env::async_with_vars(
            [
                ("IDENTITY_ENDPOINT", Some(endpoint)),
                ("IDENTITY_HEADER", Some(identity_header)),
                ("MSI_ENDPOINT", None::<&str>),
                ("MSI_SECRET", None::<&str>),
                ("MANAGED_IDENTITY_CLIENT_ID", client_id),
            ],
            f,
        )
        .await;
    }

    /// Sets the legacy `MSI_ENDPOINT`/`MSI_SECRET` pair (clearing
    /// `IDENTITY_ENDPOINT`/`IDENTITY_HEADER`) so `get_managed_identity_token`
    /// takes the Azure Functions branch via the legacy App Service MSI env
    /// vars, which dispatch to the same `get_from_azure_functions` code path.
    async fn with_legacy_msi_env<F: Future<Output = ()>>(endpoint: &str, secret: &str, f: F) {
        temp_env::async_with_vars(
            [
                ("IDENTITY_ENDPOINT", None::<&str>),
                ("IDENTITY_HEADER", None::<&str>),
                ("MSI_ENDPOINT", Some(endpoint)),
                ("MSI_SECRET", Some(secret)),
                ("MANAGED_IDENTITY_CLIENT_ID", None::<&str>),
            ],
            f,
        )
        .await;
    }

    /// Proves the `IDENTITY_ENDPOINT`/`IDENTITY_HEADER` dispatch branch
    /// reaches `get_from_azure_functions` and correctly extracts the token:
    /// the request goes to the literal `IDENTITY_ENDPOINT` URL (not IMDS),
    /// carries the identity header as `X-IDENTITY-HEADER`, and uses the
    /// default Entra resource plus Azure Functions' `api-version=2019-08-01`
    /// with no `client_id` param (since `MANAGED_IDENTITY_CLIENT_ID` is
    /// unset).
    #[tokio::test]
    async fn get_managed_identity_token_reads_token_from_mocked_azure_functions_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/MSI/token"))
            .and(header("X-IDENTITY-HEADER", "test-identity-header"))
            .and(query_param("resource", DEFAULT_AZURE_ENTRA_RESOURCE))
            .and(query_param("api-version", "2019-08-01"))
            .and(query_param_is_missing("client_id"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"mocked-functions-token"}"#),
            )
            .expect(1)
            .mount(&server)
            .await;

        let config = azure_config(None, Vec::new());
        let client = reqwest::Client::new();
        let endpoints = AttestationEndpoints::default();
        let endpoint = format!("{}/MSI/token", server.uri());

        with_azure_functions_env(&endpoint, "test-identity-header", None, async {
            let token = get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect("expected managed identity token");
            assert_eq!(token, "mocked-functions-token");
        })
        .await;
    }

    /// Proves the legacy `MSI_ENDPOINT`/`MSI_SECRET` dispatch branch also
    /// reaches `get_from_azure_functions` — the same code path exercised
    /// above via `IDENTITY_ENDPOINT`/`IDENTITY_HEADER`, so this only needs
    /// to confirm the *other* `if` branch in `get_managed_identity_token`'s
    /// dispatch is wired correctly, not re-prove the request shape.
    #[tokio::test]
    async fn get_managed_identity_token_reads_token_from_mocked_legacy_msi_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/MSI/token"))
            .and(header("X-IDENTITY-HEADER", "test-msi-secret"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"mocked-legacy-msi-token"}"#),
            )
            .expect(1)
            .mount(&server)
            .await;

        let config = azure_config(None, Vec::new());
        let client = reqwest::Client::new();
        let endpoints = AttestationEndpoints::default();
        let endpoint = format!("{}/MSI/token", server.uri());

        with_legacy_msi_env(&endpoint, "test-msi-secret", async {
            let token = get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect("expected managed identity token");
            assert_eq!(token, "mocked-legacy-msi-token");
        })
        .await;
    }

    /// A non-2xx response from the Azure Functions endpoint surfaces
    /// `UnexpectedHttpStatus` whose context names the Azure Functions
    /// request specifically (not "Azure IMDS request"), confirming the
    /// dispatch itself routed here rather than to IMDS. The underlying
    /// status/body handling is shared with the IMDS error tests below via
    /// the same helper code, so it is not re-proven per error variant here.
    #[tokio::test]
    async fn get_managed_identity_token_surfaces_clear_error_on_azure_functions_non_2xx_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&server)
            .await;

        let config = azure_config(None, Vec::new());
        let client = reqwest::Client::new();
        let endpoints = AttestationEndpoints::default();
        let endpoint = format!("{}/MSI/token", server.uri());

        with_azure_functions_env(&endpoint, "test-identity-header", None, async {
            let err = get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect_err("expected an error for a non-2xx Azure Functions response");
            match err {
                AzureAttestationError::UnexpectedHttpStatus {
                    context, status, ..
                } => {
                    assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
                    assert!(
                        context.contains("Azure Functions"),
                        "error should name the Azure Functions endpoint, got: {context}"
                    );
                }
                other => panic!("expected UnexpectedHttpStatus, got: {other:?}"),
            }
        })
        .await;
    }

    // -- Metadata-server error handling --

    /// A non-2xx IMDS response surfaces `UnexpectedHttpStatus` whose context
    /// names the Azure IMDS endpoint — enough for a user debugging outside
    /// Azure to recognize which call failed, not a bare/opaque HTTP error.
    /// Mirrors legacy's `test_explicit_azure_metadata_server_error_bubbles_up`.
    #[tokio::test]
    async fn get_managed_identity_token_surfaces_clear_error_on_imds_non_2xx_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/metadata/identity/oauth2/token"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            azure_imds_base_url: server.uri(),
            ..Default::default()
        };
        let config = azure_config(None, Vec::new());
        let client = reqwest::Client::new();

        without_azure_functions_env(None, async {
            let err = get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect_err("expected an error for a non-2xx IMDS response");
            match err {
                AzureAttestationError::UnexpectedHttpStatus {
                    context, status, ..
                } => {
                    assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
                    assert_eq!(context, "Azure IMDS request");
                }
                other => panic!("expected UnexpectedHttpStatus, got: {other:?}"),
            }
        })
        .await;
    }

    /// A malformed (non-JSON) IMDS response body surfaces `ResponseParse`
    /// whose context still names the Azure IMDS endpoint, not a generic
    /// parse error. Mirrors legacy's
    /// `test_explicit_azure_metadata_server_error_bubbles_up`.
    #[tokio::test]
    async fn get_managed_identity_token_surfaces_clear_error_on_imds_malformed_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/metadata/identity/oauth2/token"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            azure_imds_base_url: server.uri(),
            ..Default::default()
        };
        let config = azure_config(None, Vec::new());
        let client = reqwest::Client::new();

        without_azure_functions_env(None, async {
            let err = get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect_err("expected an error for a malformed IMDS response body");
            match err {
                AzureAttestationError::ResponseParse { context, .. } => {
                    assert_eq!(context, "Azure IMDS request");
                }
                other => panic!("expected ResponseParse, got: {other:?}"),
            }
        })
        .await;
    }

    /// A connection failure while calling IMDS (e.g. not running on Azure at
    /// all) surfaces `AzureAttestationError::Request`, not a panic or an
    /// opaque error. Pointing at a port that reliably refuses connections
    /// mirrors GCP's
    /// `get_identity_token_surfaces_connection_error_when_metadata_server_unreachable`.
    #[tokio::test]
    async fn get_managed_identity_token_surfaces_connection_error_when_imds_unreachable() {
        let endpoints = AttestationEndpoints {
            azure_imds_base_url: "http://127.0.0.1:1".to_string(),
            ..Default::default()
        };
        let config = azure_config(None, Vec::new());
        let client = reqwest::Client::new();

        without_azure_functions_env(None, async {
            let err = get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect_err("expected a connection error");
            match err {
                AzureAttestationError::Request { context, .. } => {
                    assert_eq!(context, "Azure IMDS request");
                }
                other => panic!("expected Request, got: {other:?}"),
            }
        })
        .await;
    }

    /// A slow-to-respond IMDS surfaces `AzureAttestationError::RequestTimedOut`
    /// once `IMDS_TIMEOUT_SECS` elapses, rather than hanging indefinitely.
    /// Uses paused tokio time (same pattern as GCP's
    /// `get_identity_token_times_out_when_metadata_server_is_slow`) so the
    /// test doesn't actually wait `IMDS_TIMEOUT_SECS` in real time.
    #[tokio::test(start_paused = true)]
    async fn get_managed_identity_token_times_out_when_imds_is_slow() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/metadata/identity/oauth2/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("too-slow")
                    .set_delay(Duration::from_secs(IMDS_TIMEOUT_SECS) + Duration::from_secs(1)),
            )
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            azure_imds_base_url: server.uri(),
            ..Default::default()
        };
        let config = azure_config(None, Vec::new());
        let client = reqwest::Client::new();

        without_azure_functions_env(None, async {
            let call = tokio::spawn(async move {
                get_managed_identity_token(&client, &config, &endpoints).await
            });
            tokio::time::advance(Duration::from_secs(IMDS_TIMEOUT_SECS) + Duration::from_secs(1))
                .await;
            let err = call
                .await
                .expect("task panicked")
                .expect_err("expected a timeout error");
            match err {
                AzureAttestationError::RequestTimedOut { context, .. } => {
                    assert_eq!(context, "Azure IMDS request");
                }
                other => panic!("expected RequestTimedOut, got: {other:?}"),
            }
        })
        .await;
    }

    // -- Entra resource / client_id query params --

    /// The default Entra resource is used when `entra_resource` is unset,
    /// and reaches the IMDS request's `resource=` query param. Previously
    /// only verified at the config-parsing layer
    /// (`connection_config::tests::build_wif_azure_with_entra_resource`
    /// covers the explicit case only, not that it reaches the request).
    /// Mirrors legacy's `test_explicit_azure_uses_default_entra_resource_if_unspecified`.
    #[tokio::test]
    async fn get_managed_identity_token_sends_default_entra_resource_to_imds() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/metadata/identity/oauth2/token"))
            .and(query_param("resource", DEFAULT_AZURE_ENTRA_RESOURCE))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"access_token":"mi-token"}"#),
            )
            .expect(1)
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            azure_imds_base_url: server.uri(),
            ..Default::default()
        };
        let config = azure_config(None, Vec::new());
        let client = reqwest::Client::new();

        without_azure_functions_env(None, async {
            let token = get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect("expected managed identity token");
            assert_eq!(token, "mi-token");
        })
        .await;
    }

    /// An explicit `workload_identity_entra_resource` is honored and reaches
    /// the same `resource=` query param, not the default. Mirrors legacy's
    /// `test_explicit_azure_uses_explicit_entra_resource`.
    #[tokio::test]
    async fn get_managed_identity_token_sends_explicit_entra_resource_to_imds() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/metadata/identity/oauth2/token"))
            .and(query_param("resource", "api://my-custom-app"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"access_token":"mi-token"}"#),
            )
            .expect(1)
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            azure_imds_base_url: server.uri(),
            ..Default::default()
        };
        let config = azure_config(Some("api://my-custom-app"), Vec::new());
        let client = reqwest::Client::new();

        without_azure_functions_env(None, async {
            let token = get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect("expected managed identity token");
            assert_eq!(token, "mi-token");
        })
        .await;
    }

    /// `MANAGED_IDENTITY_CLIENT_ID`, when set, is appended to the IMDS
    /// request as `client_id=`. Mirrors legacy's
    /// `test_explicit_azure_uses_explicit_client_id_if_set`.
    #[tokio::test]
    async fn get_managed_identity_token_appends_client_id_when_env_var_set() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/metadata/identity/oauth2/token"))
            .and(query_param("client_id", "custom-client-id"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"access_token":"mi-token"}"#),
            )
            .expect(1)
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            azure_imds_base_url: server.uri(),
            ..Default::default()
        };
        let config = azure_config(None, Vec::new());
        let client = reqwest::Client::new();

        without_azure_functions_env(Some("custom-client-id"), async {
            get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect("expected managed identity token");
        })
        .await;
    }

    /// `MANAGED_IDENTITY_CLIENT_ID`, when unset, is omitted from the IMDS
    /// request entirely (no `client_id=` param at all). Mirrors legacy's
    /// `test_explicit_azure_omits_client_id_if_not_set`.
    #[tokio::test]
    async fn get_managed_identity_token_omits_client_id_when_env_var_unset() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/metadata/identity/oauth2/token"))
            .and(query_param_is_missing("client_id"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"access_token":"mi-token"}"#),
            )
            .expect(1)
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            azure_imds_base_url: server.uri(),
            ..Default::default()
        };
        let config = azure_config(None, Vec::new());
        let client = reqwest::Client::new();

        without_azure_functions_env(None, async {
            get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect("expected managed identity token");
        })
        .await;
    }

    // -- `extract_tid_from_jwt` failure modes --
    //
    // Pure function, no HTTP mocking needed. Mirrors legacy's
    // `test_azure_impersonation_raises_error_if_mi_token_missing_tid` (the
    // missing-tid case) plus the structural-JWT failure modes legacy's
    // fake metadata service can't easily produce.

    #[test]
    fn extract_tid_from_jwt_returns_tid_when_present() {
        let jwt = make_jwt(r#"{"tid":"tenant-123"}"#);
        assert_eq!(extract_tid_from_jwt(&jwt).unwrap(), "tenant-123");
    }

    #[test]
    fn extract_tid_from_jwt_missing_payload_segment() {
        let err = extract_tid_from_jwt("not-a-jwt").unwrap_err();
        assert!(
            matches!(err, AzureAttestationError::JwtMissingPayload { .. }),
            "expected JwtMissingPayload, got: {err:?}"
        );
    }

    #[test]
    fn extract_tid_from_jwt_payload_decode_failure() {
        let err = extract_tid_from_jwt("header.!!!invalid.sig").unwrap_err();
        assert!(
            matches!(err, AzureAttestationError::JwtPayloadDecode { .. }),
            "expected JwtPayloadDecode, got: {err:?}"
        );
    }

    #[test]
    fn extract_tid_from_jwt_payload_parse_failure() {
        let payload = URL_SAFE_NO_PAD.encode(b"not json");
        let jwt = format!("header.{payload}.sig");
        let err = extract_tid_from_jwt(&jwt).unwrap_err();
        assert!(
            matches!(err, AzureAttestationError::JwtPayloadParse { .. }),
            "expected JwtPayloadParse, got: {err:?}"
        );
    }

    #[test]
    fn extract_tid_from_jwt_missing_tid_claim() {
        let jwt = make_jwt(r#"{"aud":"api://AzureADTokenExchange"}"#);
        let err = extract_tid_from_jwt(&jwt).unwrap_err();
        assert!(
            matches!(err, AzureAttestationError::JwtMissingTidClaim { .. }),
            "expected JwtMissingTidClaim, got: {err:?}"
        );
    }

    // -- SP token exchange for impersonation --

    /// When the MI token is missing `tid`, tenant extraction fails before
    /// any network call to Entra is attempted (the mounted Entra mock has
    /// an explicit expectation count of 0, so wiremock panics on server
    /// drop if it's ever hit). Mirrors legacy's
    /// `test_azure_impersonation_raises_error_if_mi_token_missing_tid`.
    #[tokio::test]
    async fn get_managed_identity_token_impersonation_makes_no_sp_call_when_tid_missing() {
        let mi_token = make_jwt(r#"{"aud":"api://AzureADTokenExchange"}"#);

        let imds_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/metadata/identity/oauth2/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!(r#"{{"access_token":"{mi_token}"}}"#)),
            )
            .mount(&imds_server)
            .await;

        let entra_server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&entra_server)
            .await;

        let endpoints = AttestationEndpoints {
            azure_imds_base_url: imds_server.uri(),
            azure_entra_base_url: entra_server.uri(),
            ..Default::default()
        };
        let config = azure_config(None, vec!["some-sp-client-id".to_string()]);
        let client = reqwest::Client::new();

        without_azure_functions_env(None, async {
            let err = get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect_err("expected tid-extraction failure before any SP exchange call");
            assert!(
                matches!(err, AzureAttestationError::JwtMissingTidClaim { .. }),
                "expected JwtMissingTidClaim, got: {err:?}"
            );
        })
        .await;

        assert!(
            entra_server.received_requests().await.unwrap().is_empty(),
            "no request should have reached the Entra endpoint"
        );
    }

    /// Full impersonation flow: the MI token is requested with the
    /// federation audience (`api://AzureADTokenExchange`), its `tid` claim
    /// selects the Entra tenant endpoint, and the SP exchange request is a
    /// `client_credentials` grant with a JWT-bearer client assertion and the
    /// correct scope. The mocked `access_token` becomes the returned token.
    /// Mirrors legacy's
    /// `test_azure_impersonation_calls_correct_api_and_populates_auth_data`.
    #[tokio::test]
    async fn get_managed_identity_token_impersonation_sends_correct_sp_exchange_request() {
        let tenant_id = "2c0183ed-cf17-480d-b3f7-df91bc0a97cd";
        let sp_client_id = "some-sp-client-id";
        let mi_token = make_jwt(&format!(r#"{{"tid":"{tenant_id}"}}"#));

        let imds_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/metadata/identity/oauth2/token"))
            .and(query_param("resource", AZURE_WIF_FEDERATION_AUDIENCE))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!(r#"{{"access_token":"{mi_token}"}}"#)),
            )
            .mount(&imds_server)
            .await;

        let entra_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!("/{tenant_id}/oauth2/v2.0/token")))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"access_token":"sp-access-token"}"#),
            )
            .expect(1)
            .mount(&entra_server)
            .await;

        let endpoints = AttestationEndpoints {
            azure_imds_base_url: imds_server.uri(),
            azure_entra_base_url: entra_server.uri(),
            ..Default::default()
        };
        let config = azure_config(None, vec![sp_client_id.to_string()]);
        let client = reqwest::Client::new();

        without_azure_functions_env(None, async {
            let token = get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect("expected an SP access token");
            assert_eq!(token, "sp-access-token");
        })
        .await;

        let requests = entra_server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        let form: HashMap<String, String> = url::form_urlencoded::parse(&requests[0].body)
            .into_owned()
            .collect();
        assert_eq!(
            form.get("grant_type").map(String::as_str),
            Some("client_credentials")
        );
        assert_eq!(
            form.get("client_id").map(String::as_str),
            Some(sp_client_id)
        );
        assert_eq!(
            form.get("client_assertion_type").map(String::as_str),
            Some("urn:ietf:params:oauth:client-assertion-type:jwt-bearer")
        );
        assert_eq!(
            form.get("client_assertion").map(String::as_str),
            Some(mi_token.as_str())
        );
        assert_eq!(
            form.get("scope").map(String::as_str),
            Some(format!("{DEFAULT_AZURE_ENTRA_RESOURCE}/.default").as_str())
        );
    }

    /// A non-2xx Entra response during the SP token exchange raises an
    /// error whose context clearly names "SP token exchange", not a
    /// generic/opaque error. Mirrors legacy's
    /// `test_azure_impersonation_raises_error_if_entra_api_fails`.
    #[tokio::test]
    async fn get_managed_identity_token_impersonation_surfaces_clear_error_on_entra_failure() {
        let tenant_id = "2c0183ed-cf17-480d-b3f7-df91bc0a97cd";
        let mi_token = make_jwt(&format!(r#"{{"tid":"{tenant_id}"}}"#));

        let imds_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/metadata/identity/oauth2/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!(r#"{{"access_token":"{mi_token}"}}"#)),
            )
            .mount(&imds_server)
            .await;

        let entra_server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&entra_server)
            .await;

        let endpoints = AttestationEndpoints {
            azure_imds_base_url: imds_server.uri(),
            azure_entra_base_url: entra_server.uri(),
            ..Default::default()
        };
        let config = azure_config(None, vec!["some-sp-client-id".to_string()]);
        let client = reqwest::Client::new();

        without_azure_functions_env(None, async {
            let err = get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect_err("expected the Entra API failure to surface");
            match err {
                AzureAttestationError::UnexpectedHttpStatus {
                    context, status, ..
                } => {
                    assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED);
                    assert!(
                        context.contains("SP token exchange"),
                        "error should clearly name the SP token exchange step, got: {context}"
                    );
                }
                other => panic!("expected UnexpectedHttpStatus, got: {other:?}"),
            }
        })
        .await;
    }

    /// A 2xx Entra response missing `access_token` fails to parse rather
    /// than panicking or silently returning an empty/`None` token. Mirrors
    /// legacy's
    /// `test_azure_impersonation_raises_error_if_access_token_missing_in_response`.
    #[tokio::test]
    async fn get_managed_identity_token_impersonation_errors_when_access_token_missing() {
        let tenant_id = "2c0183ed-cf17-480d-b3f7-df91bc0a97cd";
        let mi_token = make_jwt(&format!(r#"{{"tid":"{tenant_id}"}}"#));

        let imds_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/metadata/identity/oauth2/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!(r#"{{"access_token":"{mi_token}"}}"#)),
            )
            .mount(&imds_server)
            .await;

        let entra_server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .mount(&entra_server)
            .await;

        let endpoints = AttestationEndpoints {
            azure_imds_base_url: imds_server.uri(),
            azure_entra_base_url: entra_server.uri(),
            ..Default::default()
        };
        let config = azure_config(None, vec!["some-sp-client-id".to_string()]);
        let client = reqwest::Client::new();

        without_azure_functions_env(None, async {
            let err = get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect_err("expected a parse failure when access_token is missing");
            assert!(
                matches!(err, AzureAttestationError::ResponseParse { .. }),
                "expected ResponseParse, got: {err:?}"
            );
        })
        .await;
    }

    /// Proves the IMDS token call is logged at INFO per
    /// `ud-log-every-http-call-at-info`, and that the URL query string — which
    /// carries the managed-identity `client_id` — is stripped so only host and
    /// path appear in the log.
    #[tokio::test]
    #[tracing_test::traced_test]
    async fn imds_call_is_logged_at_info_without_query_string() {
        const CLIENT_ID_CANARY: &str = "client-id-canary-DEADBEEF";
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/metadata/identity/oauth2/token"))
            .and(header("Metadata", "true"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"access_token":"mocked-mi-token"}"#),
            )
            .mount(&server)
            .await;

        let endpoints = AttestationEndpoints {
            azure_imds_base_url: server.uri(),
            ..Default::default()
        };
        let config = WorkloadIdentityConfig {
            provider: WifProvider::Azure,
            entra_resource: None,
            impersonation_path: Vec::new(),
            oidc_token: None,
        };
        let client = reqwest::Client::new();

        temp_env::async_with_vars(
            [
                ("IDENTITY_ENDPOINT", None::<&str>),
                ("IDENTITY_HEADER", None::<&str>),
                ("MSI_ENDPOINT", None::<&str>),
                ("MSI_SECRET", None::<&str>),
                ("MANAGED_IDENTITY_CLIENT_ID", Some(CLIENT_ID_CANARY)),
            ],
            async {
                let token = get_managed_identity_token(&client, &config, &endpoints)
                    .await
                    .expect("expected managed identity token");
                assert_eq!(token, "mocked-mi-token");
            },
        )
        .await;

        assert!(logs_contain("outbound HTTP call"), "dispatch log missing");
        let expected_host = reqwest::Url::parse(&server.uri())
            .unwrap()
            .host_str()
            .unwrap()
            .to_owned();
        assert!(
            logs_contain(&expected_host),
            "host not logged on dispatch line"
        );
        assert!(
            logs_contain("/metadata/identity/oauth2/token"),
            "host/path not logged"
        );
        assert!(logs_contain("HTTP response"), "response log missing");
        assert!(logs_contain("status=200"), "response status not logged");
        assert!(
            !logs_contain(CLIENT_ID_CANARY),
            "client_id query param leaked into logs"
        );
        assert!(
            !logs_contain("api-version"),
            "query string leaked into logs"
        );
    }
}
