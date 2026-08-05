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

    let response = tokio::time::timeout(
        std::time::Duration::from_secs(IMDS_TIMEOUT_SECS),
        client.get(&url).header("Metadata", "true").send(),
    )
    .await
    .context(RequestTimedOutSnafu { context: CTX })?
    .context(RequestSnafu { context: CTX })?;

    let status = response.status();
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

    let response = tokio::time::timeout(
        std::time::Duration::from_secs(IMDS_TIMEOUT_SECS),
        client.post(&url).form(&params).send(),
    )
    .await
    .context(RequestTimedOutSnafu { context: CTX })?
    .context(RequestSnafu { context: CTX })?;

    let status = response.status();
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
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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
                ("MANAGED_IDENTITY_CLIENT_ID", None::<&str>),
            ],
            async {
                let token = get_managed_identity_token(&client, &config, &endpoints)
                    .await
                    .expect("expected managed identity token");
                assert_eq!(token, "mocked-mi-token");
            },
        )
        .await;
    }
}
