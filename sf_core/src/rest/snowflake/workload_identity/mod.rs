//! Workload Identity Federation (WIF) attestation.
//!
//! Entry point is [`create_attestation`], which dispatches to the
//! appropriate provider module based on the configured [`WifProvider`].
//! The resulting [`Attestation`] is then embedded into the Snowflake
//! login-request body by [`crate::rest::snowflake::auth_request_data`].

mod aws;
mod azure;
mod oidc;

use crate::config::rest_parameters::{WifProvider, WorkloadIdentityConfig};
use crate::sensitive::SensitiveString;
use snafu::{Location, ResultExt, Snafu};

/// Resolved identity token to be forwarded to Snowflake GS.
///
/// `provider` is the wire string (`AWS`, `AZURE`, `OIDC`) sent
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
// The shared `…AttestationFailed` postfix is intentional: each variant is a
// per-provider wrapper around that provider's domain error, named in the
// past-tense failure style the project convention asks for. The common suffix
// is meaningful, so opt out of the enum-variant-name lint.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Snafu, error_trace::ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub enum AttestationError {
    #[snafu(display("AWS attestation failed"))]
    AwsAttestationFailed {
        source: aws::AwsAttestationError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Azure attestation failed"))]
    AzureAttestationFailed {
        source: azure::AzureAttestationError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("OIDC attestation failed"))]
    OidcAttestationFailed {
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
                .context(AwsAttestationFailedSnafu)?;
            Ok(Attestation {
                provider: WifProvider::Aws.as_wire_str(),
                token: SensitiveString::from(token),
            })
        }
        WifProvider::Azure => {
            let token = azure::get_managed_identity_token(client, config)
                .await
                .context(AzureAttestationFailedSnafu)?;
            Ok(Attestation {
                provider: WifProvider::Azure.as_wire_str(),
                token: SensitiveString::from(token),
            })
        }
        WifProvider::Oidc => {
            let token = oidc::get_token(config).context(OidcAttestationFailedSnafu)?;
            Ok(Attestation {
                provider: WifProvider::Oidc.as_wire_str(),
                token,
            })
        }
    }
}
