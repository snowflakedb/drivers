//! Azure Managed Identity attestation for Workload Identity Federation.
//!
//! Token acquisition priority:
//! 1. AKS Workload Identity — the Azure Workload Identity webhook injects
//!    `AZURE_CLIENT_ID`, `AZURE_TENANT_ID` and `AZURE_FEDERATED_TOKEN_FILE`
//!    into the pod and projects a Kubernetes service-account token at that
//!    path. That federated JWT is exchanged directly with Entra ID for an
//!    access token, so no IMDS round-trip is needed.
//!    `workload_identity_impersonation_path` is **not supported** here and is
//!    rejected with [`AzureAttestationError::AksImpersonationNotSupported`].
//! 2. Azure Functions runtime (environment variables `IDENTITY_ENDPOINT` +
//!    `IDENTITY_HEADER` or legacy `MSI_ENDPOINT` + `MSI_SECRET`).
//! 3. Azure IMDS (`http://169.254.169.254/metadata/identity/oauth2/token`).
//!
//! The `resource` (Entra audience) defaults to the Snowflake-assigned
//! resource URI [`crate::config::rest_parameters::DEFAULT_AZURE_ENTRA_RESOURCE`].
//! Callers can override via `workload_identity_entra_resource`.
//!
//! When `workload_identity_impersonation_path` is set (exactly one SP client_id)
//! and the environment is *not* AKS, a two-step flow is performed:
//! 1. Acquire a MI token scoped to `AZURE_WIF_FEDERATION_AUDIENCE`.
//! 2. Exchange it for an SP access token via Entra ID's `oauth2/v2.0/token`
//!    endpoint (`client_credentials` grant with JWT-bearer client assertion).
//!    The resulting SP token is what is sent to Snowflake.

use crate::config::rest_parameters::{DEFAULT_AZURE_ENTRA_RESOURCE, WorkloadIdentityConfig};
use crate::env_vars;
use crate::sensitive::SensitiveString;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use snafu::{Location, OptionExt, ResultExt, Snafu, ensure};
use std::path::PathBuf;

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
    #[snafu(display("workload_identity_impersonation_path is not supported on AKS."))]
    AksImpersonationNotSupported {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read AKS federated token file '{path}'"))]
    AksFederatedTokenFileRead {
        path: String,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Response shape shared by the Azure Managed Identity endpoints (IMDS and the
/// Azure Functions identity endpoint) and the Entra ID `oauth2/v2.0/token`
/// endpoint — all three return the token under `access_token`.
#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    access_token: SensitiveString,
}

/// Workload identity injected into an AKS pod by the Azure Workload Identity
/// mutating webhook.
#[derive(Debug)]
struct AksWorkloadIdentity {
    /// `AZURE_CLIENT_ID` — Entra application registration the pod's federated
    /// identity credential is bound to.
    client_id: String,
    /// `AZURE_TENANT_ID` — Entra tenant that issues the access token.
    tenant_id: String,
    /// Contents of the file at `AZURE_FEDERATED_TOKEN_FILE` — the projected
    /// Kubernetes service-account token — read once at detection time and
    /// held as `SensitiveString` so it is zeroized on drop and cannot be
    /// printed by accident.
    federated_token: SensitiveString,
}

/// Inputs for an Entra ID `oauth2/v2.0/token` request using the
/// `client_credentials` grant with a JWT-bearer client assertion.
struct EntraTokenExchange<'a> {
    /// Label identifying which flow failed, used in error messages.
    context: &'static str,
    /// Entra tenant whose token endpoint is called.
    tenant_id: &'a str,
    /// Application the assertion authenticates as.
    client_id: &'a str,
    /// JWT presented as proof of identity — a Managed Identity token when
    /// impersonating a service principal, or the projected Kubernetes
    /// service-account token on AKS.
    client_assertion: &'a str,
    /// Entra resource the issued access token is scoped to; `/.default` is
    /// appended to form the OAuth2 `scope`.
    resource: &'a str,
}

/// Acquire an Azure token for Workload Identity Federation.
///
/// On AKS (see [`detect_aks_workload_identity`]): exchanges the projected
/// Kubernetes service-account token for an Entra ID access token scoped to the
/// Snowflake Entra resource. Impersonation is rejected in this environment.
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

    if let Some(aks) = detect_aks_workload_identity().await? {
        // The federated credential authenticates as exactly one Entra
        // application, and there is no Managed Identity token to present as the
        // client assertion for a second hop.
        ensure!(
            config.impersonation_path.is_empty(),
            AksImpersonationNotSupportedSnafu
        );
        return get_token_via_aks(client, &aks, snowflake_resource, endpoints).await;
    }

    // When impersonating an SP, the MI token must be issued for the federation
    // audience so Entra ID will accept it as a client assertion.
    let mi_resource = if config.impersonation_path.is_empty() {
        snowflake_resource
    } else {
        AZURE_WIF_FEDERATION_AUDIENCE
    };

    let client_id = std::env::var(env_vars::MANAGED_IDENTITY_CLIENT_ID).ok();

    if let (Ok(endpoint), Ok(header)) = (
        std::env::var(env_vars::IDENTITY_ENDPOINT),
        std::env::var(env_vars::IDENTITY_HEADER),
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
    if let (Ok(endpoint), Ok(secret)) = (
        std::env::var(env_vars::MSI_ENDPOINT),
        std::env::var(env_vars::MSI_SECRET),
    ) {
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

/// Detect an AKS Workload Identity environment.
///
/// Returns `Some` only when all three variables injected by the Azure Workload
/// Identity mutating webhook are set to a non-empty value *and* the projected
/// service-account token file can be read.
///
/// Probing for the token file — rather than for `KUBERNETES_SERVICE_HOST` — is
/// deliberate: that variable is absent in pods running with
/// `enableServiceLinks: false`, and present on *any* Kubernetes cluster,
/// including non-AKS ones that have no Azure federated identity to exchange.
///
/// The file is opened once here rather than probed for existence and reopened
/// later: a missing file is not AKS (`Ok(None)`, so the pre-existing Azure
/// Functions / IMDS flows still get their chance), while any other read
/// failure — permission denied, a directory in its place — means AKS *is*
/// detected but broken, surfaced as [`AzureAttestationError::AksFederatedTokenFileRead`].
async fn detect_aks_workload_identity() -> Result<Option<AksWorkloadIdentity>, AzureAttestationError>
{
    let Some(client_id) = non_empty_env(env_vars::AZURE_CLIENT_ID) else {
        return Ok(None);
    };
    let Some(tenant_id) = non_empty_env(env_vars::AZURE_TENANT_ID) else {
        return Ok(None);
    };
    let Some(federated_token_file) = non_empty_env(env_vars::AZURE_FEDERATED_TOKEN_FILE) else {
        return Ok(None);
    };
    let federated_token_file = PathBuf::from(federated_token_file);

    let federated_token = match tokio::fs::read_to_string(&federated_token_file).await {
        Ok(content) => content,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(source).context(AksFederatedTokenFileReadSnafu {
                path: federated_token_file.display().to_string(),
            });
        }
    };

    Ok(Some(AksWorkloadIdentity {
        client_id,
        tenant_id,
        // Projected token files are commonly newline-terminated.
        federated_token: federated_token.trim().to_string().into(),
    }))
}

/// Read an environment variable, treating unset and empty as equivalent.
fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// Exchange the projected Kubernetes service-account token for an Entra ID
/// access token scoped to the Snowflake resource.
///
/// This is the same `client_credentials` + JWT-bearer grant that
/// `azure-identity`'s `WorkloadIdentityCredential` performs internally, which is
/// why AKS needs no IMDS round-trip and no projected-volume workaround.
async fn get_token_via_aks(
    client: &reqwest::Client,
    aks: &AksWorkloadIdentity,
    snowflake_resource: &str,
    endpoints: &AttestationEndpoints,
) -> Result<String, AzureAttestationError> {
    exchange_client_assertion(
        client,
        endpoints,
        EntraTokenExchange {
            context: "Entra ID AKS federated token exchange",
            tenant_id: &aks.tenant_id,
            client_id: &aks.client_id,
            client_assertion: aks.federated_token.reveal(),
            resource: snowflake_resource,
        },
    )
    .await
}

/// Perform the Entra ID `client_credentials` + JWT-bearer token exchange shared
/// by the AKS federated-token flow and service-principal impersonation.
async fn exchange_client_assertion(
    client: &reqwest::Client,
    endpoints: &AttestationEndpoints,
    exchange: EntraTokenExchange<'_>,
) -> Result<String, AzureAttestationError> {
    let EntraTokenExchange {
        context,
        tenant_id,
        client_id,
        client_assertion,
        resource,
    } = exchange;

    let url = format!(
        "{}/{tenant_id}/oauth2/v2.0/token",
        endpoints.azure_entra_base_url
    );
    let params = [
        ("grant_type", "client_credentials"),
        ("client_id", client_id),
        (
            "client_assertion_type",
            "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
        ),
        ("client_assertion", client_assertion),
        ("scope", &format!("{resource}/.default")),
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
    .context(RequestTimedOutSnafu { context })?
    .context(RequestSnafu { context })?;

    let status = response.status();
    tracing::info!(status = status.as_u16(), "HTTP response");
    let body = response
        .text()
        .await
        .context(ResponseBodyReadSnafu { context })?;

    if !status.is_success() {
        return UnexpectedHttpStatusSnafu {
            context,
            status,
            body,
        }
        .fail();
    }

    let parsed: AccessTokenResponse =
        serde_json::from_str(&body).context(ResponseParseSnafu { context })?;
    Ok(parsed.access_token.reveal().to_string())
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

    let parsed: AccessTokenResponse =
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

    let parsed: AccessTokenResponse =
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
    let tenant_id = extract_tid_from_jwt(mi_token)?;

    exchange_client_assertion(
        client,
        endpoints,
        EntraTokenExchange {
            context: "Entra ID SP token exchange",
            tenant_id: &tenant_id,
            client_id: sp_client_id,
            client_assertion: mi_token,
            resource: snowflake_resource,
        },
    )
    .await
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

    /// Clears the AKS Workload Identity env vars, so a test's outcome cannot
    /// depend on the host environment — a CI runner that is itself an AKS pod
    /// would otherwise divert every test below into the AKS branch. Spread into
    /// each `temp_env` list by the helpers that follow.
    const NO_AKS_ENV: [(&str, Option<&str>); 3] = [
        (env_vars::AZURE_CLIENT_ID, None),
        (env_vars::AZURE_TENANT_ID, None),
        (env_vars::AZURE_FEDERATED_TOKEN_FILE, None),
    ];

    /// Clears the Azure Functions and AKS env vars so `get_managed_identity_token`
    /// always takes the IMDS path, optionally setting
    /// `MANAGED_IDENTITY_CLIENT_ID` for the duration of `f`.
    async fn without_azure_functions_env<F: Future<Output = ()>>(client_id: Option<&str>, f: F) {
        temp_env::async_with_vars(
            [
                (env_vars::IDENTITY_ENDPOINT, None::<&str>),
                (env_vars::IDENTITY_HEADER, None::<&str>),
                (env_vars::MSI_ENDPOINT, None::<&str>),
                (env_vars::MSI_SECRET, None::<&str>),
                (env_vars::MANAGED_IDENTITY_CLIENT_ID, client_id),
            ]
            .into_iter()
            .chain(NO_AKS_ENV)
            .collect::<Vec<_>>(),
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
                (env_vars::IDENTITY_ENDPOINT, Some(endpoint)),
                (env_vars::IDENTITY_HEADER, Some(identity_header)),
                (env_vars::MSI_ENDPOINT, None::<&str>),
                (env_vars::MSI_SECRET, None::<&str>),
                (env_vars::MANAGED_IDENTITY_CLIENT_ID, client_id),
            ]
            .into_iter()
            .chain(NO_AKS_ENV)
            .collect::<Vec<_>>(),
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
                (env_vars::IDENTITY_ENDPOINT, None::<&str>),
                (env_vars::IDENTITY_HEADER, None::<&str>),
                (env_vars::MSI_ENDPOINT, Some(endpoint)),
                (env_vars::MSI_SECRET, Some(secret)),
                (env_vars::MANAGED_IDENTITY_CLIENT_ID, None::<&str>),
            ]
            .into_iter()
            .chain(NO_AKS_ENV)
            .collect::<Vec<_>>(),
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
                (env_vars::IDENTITY_ENDPOINT, None::<&str>),
                (env_vars::IDENTITY_HEADER, None::<&str>),
                (env_vars::MSI_ENDPOINT, None::<&str>),
                (env_vars::MSI_SECRET, None::<&str>),
                (env_vars::MANAGED_IDENTITY_CLIENT_ID, Some(CLIENT_ID_CANARY)),
            ]
            .into_iter()
            .chain(NO_AKS_ENV)
            .collect::<Vec<_>>(),
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

    // -- AKS Workload Identity --
    //
    // Backport of snowflake-connector-python#2903 (SNOW-3533720). The AKS
    // branch is checked before the Azure Functions / IMDS branches, so these
    // tests both prove the new flow and pin down when it must *not* engage.

    const AKS_CLIENT_ID: &str = "66666666-7777-8888-9999-000000000000";
    const AKS_TENANT_ID: &str = "11111111-2222-3333-4444-555555555555";

    /// Sets the three variables the Azure Workload Identity webhook injects
    /// (clearing the Azure Functions / IMDS pair so only the AKS branch can be
    /// taken), plus any extra overrides the caller needs.
    async fn with_aks_env<F: Future<Output = ()>>(
        federated_token_file: &str,
        extra: &[(&str, Option<&str>)],
        f: F,
    ) {
        temp_env::async_with_vars(
            [
                (env_vars::AZURE_CLIENT_ID, Some(AKS_CLIENT_ID)),
                (env_vars::AZURE_TENANT_ID, Some(AKS_TENANT_ID)),
                (
                    env_vars::AZURE_FEDERATED_TOKEN_FILE,
                    Some(federated_token_file),
                ),
                (env_vars::IDENTITY_ENDPOINT, None),
                (env_vars::IDENTITY_HEADER, None),
                (env_vars::MSI_ENDPOINT, None),
                (env_vars::MSI_SECRET, None),
                (env_vars::MANAGED_IDENTITY_CLIENT_ID, None),
            ]
            .into_iter()
            .chain(extra.iter().copied())
            .collect::<Vec<_>>(),
            f,
        )
        .await;
    }

    /// Returns a path that is guaranteed not to exist on disk, by creating a
    /// temp file and immediately deleting it.
    fn nonexistent_path() -> String {
        let file = tempfile::NamedTempFile::new().expect("temp file can be created");
        let path = file.path().display().to_string();
        drop(file);
        path
    }

    /// The projected Kubernetes service-account token is exchanged with Entra
    /// ID for an access token: the request goes to the tenant named by
    /// `AZURE_TENANT_ID`, authenticates as `AZURE_CLIENT_ID` with a JWT-bearer
    /// client assertion carrying the token file's contents, and the returned
    /// `access_token` is what the caller receives. No IMDS call is involved —
    /// `azure_imds_base_url` is left at its real link-local default, so an IMDS
    /// round-trip would fail rather than silently succeed.
    #[tokio::test]
    async fn get_managed_identity_token_exchanges_aks_federated_token_for_entra_access_token() {
        let token_file = tempfile::NamedTempFile::new().expect("temp file can be created");
        // Trailing newline is how the projected service-account token file is
        // actually written; it must not end up inside the client assertion.
        std::fs::write(token_file.path(), "k8s-service-account-jwt\n")
            .expect("temp file can be written");

        let entra_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!("/{AKS_TENANT_ID}/oauth2/v2.0/token")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"entra-access-token"}"#),
            )
            .expect(1)
            .mount(&entra_server)
            .await;

        let endpoints = AttestationEndpoints {
            azure_entra_base_url: entra_server.uri(),
            ..Default::default()
        };
        let config = azure_config(Some("api://test-resource"), Vec::new());
        let client = reqwest::Client::new();

        with_aks_env(&token_file.path().display().to_string(), &[], async {
            let token = get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect("expected the AKS federated token exchange to succeed");
            assert_eq!(token, "entra-access-token");
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
            Some(AKS_CLIENT_ID)
        );
        assert_eq!(
            form.get("client_assertion_type").map(String::as_str),
            Some("urn:ietf:params:oauth:client-assertion-type:jwt-bearer")
        );
        assert_eq!(
            form.get("client_assertion").map(String::as_str),
            Some("k8s-service-account-jwt"),
            "the projected token's trailing newline must be trimmed off the assertion"
        );
        assert_eq!(
            form.get("scope").map(String::as_str),
            Some("api://test-resource/.default")
        );
    }

    /// All three webhook variables set but no token file on disk is not an AKS
    /// environment: the variables alone can be inherited by any process (e.g. a
    /// local shell that once sourced a pod env dump), and without the projected
    /// token there is nothing to exchange.
    #[tokio::test]
    async fn detect_aks_workload_identity_returns_none_when_federated_token_file_absent() {
        let missing_path = nonexistent_path();

        let mut detected = None;
        with_aks_env(&missing_path, &[], async {
            detected = detect_aks_workload_identity()
                .await
                .expect("a missing token file must not surface as an error");
        })
        .await;

        assert!(
            detected.is_none(),
            "all three env vars set but no token file on disk must not be treated as AKS, \
             got {detected:?}"
        );
    }

    /// An empty `AZURE_CLIENT_ID` counts as unset — some tooling exports the
    /// webhook variables with empty values, which would otherwise send an
    /// assertion with a blank `client_id` to Entra.
    #[tokio::test]
    async fn detect_aks_workload_identity_returns_none_when_client_id_is_empty() {
        let token_file = tempfile::NamedTempFile::new().expect("temp file can be created");
        std::fs::write(token_file.path(), "k8s-service-account-jwt")
            .expect("temp file can be written");

        let mut detected = None;
        with_aks_env(
            &token_file.path().display().to_string(),
            &[(env_vars::AZURE_CLIENT_ID, Some(""))],
            async {
                detected = detect_aks_workload_identity()
                    .await
                    .expect("an empty AZURE_CLIENT_ID must not surface as an error");
            },
        )
        .await;

        assert!(
            detected.is_none(),
            "an empty AZURE_CLIENT_ID must not be treated as AKS, got {detected:?}"
        );
    }

    /// Failing AKS detection must not break the pre-existing flows: with the
    /// webhook variables set but no token file, the Azure Functions branch
    /// still runs. The Entra endpoint is pointed at a refused port, so any
    /// attempt to take the AKS exchange after all would fail loudly.
    #[tokio::test]
    async fn get_managed_identity_token_falls_through_to_azure_functions_when_aks_token_file_absent()
     {
        let identity_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/MSI/token"))
            .and(header("X-IDENTITY-HEADER", "test-identity-header"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"managed-identity-token"}"#),
            )
            .expect(1)
            .mount(&identity_server)
            .await;

        let endpoints = AttestationEndpoints {
            azure_entra_base_url: "http://127.0.0.1:1".to_string(),
            ..Default::default()
        };
        let config = azure_config(None, Vec::new());
        let client = reqwest::Client::new();
        let identity_endpoint = format!("{}/MSI/token", identity_server.uri());

        with_aks_env(
            &nonexistent_path(),
            &[
                (env_vars::IDENTITY_ENDPOINT, Some(&identity_endpoint)),
                (env_vars::IDENTITY_HEADER, Some("test-identity-header")),
            ],
            async {
                let token = get_managed_identity_token(&client, &config, &endpoints)
                    .await
                    .expect("expected the Azure Functions branch to still run");
                assert_eq!(token, "managed-identity-token");
            },
        )
        .await;
    }

    /// `workload_identity_impersonation_path` has no meaning on AKS: the
    /// federated credential authenticates as exactly one Entra application and
    /// there is no Managed Identity token to present as the assertion for a
    /// second hop. It is rejected with a dedicated error before any HTTP call
    /// is made, rather than silently ignored — the Entra base URL is a refused
    /// port so an attempted exchange would surface as a `Request` error.
    #[tokio::test]
    async fn get_managed_identity_token_rejects_impersonation_on_aks() {
        let token_file = tempfile::NamedTempFile::new().expect("temp file can be created");
        std::fs::write(token_file.path(), "k8s-service-account-jwt")
            .expect("temp file can be written");

        let endpoints = AttestationEndpoints {
            azure_entra_base_url: "http://127.0.0.1:1".to_string(),
            ..Default::default()
        };
        let config = azure_config(None, vec!["some-sp-client-id".to_string()]);
        let client = reqwest::Client::new();

        with_aks_env(&token_file.path().display().to_string(), &[], async {
            let err = get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect_err("expected impersonation on AKS to be rejected");
            assert!(
                matches!(
                    err,
                    AzureAttestationError::AksImpersonationNotSupported { .. }
                ),
                "expected AksImpersonationNotSupported, got: {err:?}"
            );
            assert_eq!(
                err.to_string(),
                "workload_identity_impersonation_path is not supported on AKS."
            );
        })
        .await;
    }

    /// A token file that exists at detection time but cannot be read surfaces a
    /// dedicated error naming the path, rather than falling through to IMDS and
    /// failing with an unrelated message. Exercised with a directory in place of
    /// the token file, which `read_to_string` rejects with an error other than
    /// `NotFound`.
    #[tokio::test]
    async fn get_managed_identity_token_surfaces_error_when_aks_token_file_unreadable() {
        let token_dir = tempfile::tempdir().expect("temp dir can be created");
        let token_path = token_dir.path().display().to_string();

        let endpoints = AttestationEndpoints::default();
        let config = azure_config(None, Vec::new());
        let client = reqwest::Client::new();

        with_aks_env(&token_path, &[], async {
            let err = get_managed_identity_token(&client, &config, &endpoints)
                .await
                .expect_err("expected an unreadable token file to surface an error");
            match err {
                AzureAttestationError::AksFederatedTokenFileRead { ref path, .. } => {
                    assert_eq!(path, &token_path);
                }
                other => panic!("expected AksFederatedTokenFileRead, got: {other:?}"),
            }
        })
        .await;
    }
}
