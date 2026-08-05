//! Workload Identity Federation (WIF) attestation.
//!
//! Entry point is [`create_attestation`], which dispatches to the
//! appropriate provider module based on the configured [`WifProvider`].
//! The resulting [`Attestation`] is then embedded into the Snowflake
//! login-request body by [`crate::rest::snowflake::auth_request_data`].

mod aws;
mod azure;
mod gcp;
mod oidc;

use crate::config::rest_parameters::{WifProvider, WorkloadIdentityConfig};
use crate::sensitive::SensitiveString;
use snafu::{Location, ResultExt, Snafu};

/// Resolved identity token to be forwarded to Snowflake GS.
///
/// `provider` is the wire string (`AWS`, `AZURE`, `GCP`, `OIDC`) sent
/// in the `PROVIDER` field of the login-request body.
/// `token` is the raw JWT or attested credential string sent in `TOKEN`.
/// The field is `SensitiveString` to prevent the token from appearing in
/// debug logs.
#[derive(Debug)]
pub struct Attestation {
    pub provider: &'static str,
    pub token: SensitiveString,
}

/// Error type for attestation failures.
///
/// Each variant wraps the provider-specific error type so the underlying
/// failure (and its captured call-site location) is preserved in the
/// [`error_trace::ErrorTrace`] chain.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Snafu, error_trace::ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub enum AttestationError {
    #[snafu(display("AWS attestation failed"))]
    AwsAttestation {
        source: aws::AwsAttestationError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Azure attestation failed"))]
    AzureAttestation {
        source: azure::AzureAttestationError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCP attestation failed"))]
    GcpAttestation {
        source: gcp::GcpAttestationError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("OIDC attestation failed"))]
    OidcAttestation {
        source: oidc::OidcAttestationError,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Injectable base URLs for the cloud-metadata / IdP endpoints that the
/// provider modules call. Defaults are the real production endpoints;
/// tests override individual fields to point at a `wiremock::MockServer`.
#[derive(Debug, Clone)]
pub(crate) struct AttestationEndpoints {
    /// AWS EC2 instance-metadata service (IMDS) base URL, used only to
    /// resolve the AWS region when `AWS_REGION`/`AWS_DEFAULT_REGION` are
    /// unset. AWS STS itself is never called directly by the default
    /// (pre-signed `GetCallerIdentity`) path — that path only builds a URL
    /// string embedded in the attestation body for Snowflake GS to replay.
    pub(crate) aws_imds_base_url: String,
    /// Azure IMDS base URL for the Managed Identity token endpoint.
    pub(crate) azure_imds_base_url: String,
    /// Entra ID base URL used for the SP token exchange during Azure
    /// impersonation (`{base}/{tenant_id}/oauth2/v2.0/token`).
    pub(crate) azure_entra_base_url: String,
    /// GCE metadata server base URL.
    pub(crate) gcp_metadata_base_url: String,
    /// IAM Service Account Credentials API base URL (host only; callers
    /// append `/v1/projects/-/serviceAccounts/...`).
    pub(crate) gcp_iam_credentials_base_url: String,
}

impl Default for AttestationEndpoints {
    fn default() -> Self {
        Self {
            aws_imds_base_url: "http://169.254.169.254".to_string(),
            azure_imds_base_url: "http://169.254.169.254".to_string(),
            azure_entra_base_url: "https://login.microsoftonline.com".to_string(),
            gcp_metadata_base_url: "http://metadata.google.internal".to_string(),
            gcp_iam_credentials_base_url: "https://iamcredentials.googleapis.com".to_string(),
        }
    }
}

/// Acquire a Workload Identity Federation attestation token.
///
/// Dispatches to the provider-specific module and returns the raw token
/// together with the provider label expected by GS.
pub async fn create_attestation(
    client: &reqwest::Client,
    config: &WorkloadIdentityConfig,
) -> Result<Attestation, AttestationError> {
    let endpoints = AttestationEndpoints::default();
    match config.provider {
        WifProvider::Aws => {
            let token = aws::get_attestation_token(client, config, &endpoints)
                .await
                .context(AwsAttestationSnafu)?;
            Ok(Attestation {
                provider: WifProvider::Aws.as_wire_str(),
                token: SensitiveString::from(token),
            })
        }
        WifProvider::Azure => {
            let token = azure::get_managed_identity_token(client, config, &endpoints)
                .await
                .context(AzureAttestationSnafu)?;
            Ok(Attestation {
                provider: WifProvider::Azure.as_wire_str(),
                token: SensitiveString::from(token),
            })
        }
        WifProvider::Gcp => {
            let token = gcp::get_identity_token(client, config, &endpoints)
                .await
                .context(GcpAttestationSnafu)?;
            Ok(Attestation {
                provider: WifProvider::Gcp.as_wire_str(),
                token: SensitiveString::from(token),
            })
        }
        WifProvider::Oidc => {
            let token = oidc::get_token(config).context(OidcAttestationSnafu)?;
            Ok(Attestation {
                provider: WifProvider::Oidc.as_wire_str(),
                token,
            })
        }
    }
}
