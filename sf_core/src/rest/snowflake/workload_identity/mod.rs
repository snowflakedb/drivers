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

/// Acquire a Workload Identity Federation attestation token.
///
/// Dispatches to the provider-specific module and returns the raw token
/// together with the provider label expected by GS.
pub async fn create_attestation(
    client: &reqwest::Client,
    config: &WorkloadIdentityConfig,
) -> Result<Attestation, AttestationError> {
    match config.provider {
        WifProvider::Aws => {
            let token = aws::get_attestation_token(client, config)
                .await
                .context(AwsAttestationSnafu)?;
            Ok(Attestation {
                provider: WifProvider::Aws.as_wire_str(),
                token: SensitiveString::from(token),
            })
        }
        WifProvider::Azure => {
            let token = azure::get_managed_identity_token(client, config)
                .await
                .context(AzureAttestationSnafu)?;
            Ok(Attestation {
                provider: WifProvider::Azure.as_wire_str(),
                token: SensitiveString::from(token),
            })
        }
        WifProvider::Gcp => {
            let token = gcp::get_identity_token(client, config)
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
