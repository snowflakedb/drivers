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

const GCE_METADATA_HOST: &str = "http://metadata.google.internal";
const METADATA_FLAVOR_HEADER: &str = "metadata-flavor";
const METADATA_FLAVOR_VALUE: &str = "Google";
const SNOWFLAKE_AUDIENCE: &str = "snowflakecomputing.com";
const METADATA_TIMEOUT_SECS: u64 = 10;
/// Base URL of the IAM Service Account Credentials API's `serviceAccounts`
/// collection (the `iamcredentials.googleapis.com` API root plus the
/// `v1/projects/-/serviceAccounts` path). Per-account methods such as
/// `:generateIdToken` are appended to `{base}/{service_account}`.
const IAM_CREDENTIALS_SERVICE_ACCOUNTS_BASE_URL: &str =
    "https://iamcredentials.googleapis.com/v1/projects/-/serviceAccounts";

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
) -> Result<String, GcpAttestationError> {
    match config.impersonation_path.split_last() {
        // No impersonation — fetch the identity token directly.
        None => get_identity_token_from_metadata(client).await,
        // Impersonation — `target_sa` is the final account; `delegates` is the
        // (possibly empty) intermediate delegation chain.
        Some((target_sa, delegates)) => {
            let access_token = get_access_token_from_metadata(client).await?;
            generate_identity_token(
                client,
                access_token.reveal(),
                target_sa,
                delegates,
                SNOWFLAKE_AUDIENCE,
            )
            .await
        }
    }
}

/// Fetch an OAuth access token for the VM's default service account.
async fn get_access_token_from_metadata(
    client: &reqwest::Client,
) -> Result<SensitiveString, GcpAttestationError> {
    const CTX: &str = "GCE metadata access token";

    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: SensitiveString,
    }

    let url =
        format!("{GCE_METADATA_HOST}/computeMetadata/v1/instance/service-accounts/default/token");
    let body = metadata_get(client, &url, CTX).await?;
    let parsed: TokenResponse =
        serde_json::from_str(&body).context(ResponseParseSnafu { context: CTX })?;
    Ok(parsed.access_token)
}

/// Fetch an OIDC identity token for the VM's default service account.
async fn get_identity_token_from_metadata(
    client: &reqwest::Client,
) -> Result<String, GcpAttestationError> {
    let url = format!(
        "{GCE_METADATA_HOST}/computeMetadata/v1/instance/service-accounts/default/identity?audience={SNOWFLAKE_AUDIENCE}&format=full"
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
) -> Result<String, GcpAttestationError> {
    const CTX: &str = "GCP IAM generateIdToken";

    #[derive(Deserialize)]
    struct IdTokenResponse {
        token: String,
    }

    let url = format!(
        "{IAM_CREDENTIALS_SERVICE_ACCOUNTS_BASE_URL}/{target_service_account}:generateIdToken"
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
