//! OIDC provider for Workload Identity Federation.
//!
//! The OIDC path forwards a pre-acquired JWT supplied via the `token`
//! connection parameter.  Before forwarding, the driver performs minimal
//! structural validation (three-part JWT, base64-decodable payload) and
//! extracts `iss`/`sub` claims for diagnostic logging.  Signature
//! verification is performed server-side by Snowflake.

use crate::config::rest_parameters::WorkloadIdentityConfig;
use crate::sensitive::SensitiveString;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use snafu::{Location, OptionExt, ResultExt, Snafu};

/// Errors raised while resolving the passthrough OIDC token.
#[derive(Debug, Snafu, error_trace::ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub enum OidcAttestationError {
    #[snafu(display(
        "OIDC provider requires a pre-acquired token set via the 'token' connection parameter"
    ))]
    MissingToken {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("OIDC token is not a valid JWT (expected header.payload.signature)"))]
    MalformedToken {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to base64-decode JWT payload"))]
    PayloadDecodeFailed {
        source: base64::DecodeError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("JWT payload is not valid JSON"))]
    PayloadNotJson {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Return the pre-acquired OIDC token from the connection configuration.
///
/// Performs structural validation (three-segment JWT, decodable payload)
/// and logs `iss`/`sub` claims at debug level for diagnostics.  Returns
/// an error if the token is absent or structurally malformed.
pub(super) fn get_token(
    config: &WorkloadIdentityConfig,
) -> Result<SensitiveString, OidcAttestationError> {
    let token = config
        .oidc_token
        .as_ref()
        .map(|s| s.reveal().to_string())
        .context(MissingTokenSnafu)?;

    validate_and_log_claims(&token)?;

    Ok(SensitiveString::from(token))
}

/// Validate the JWT structure and extract `iss`/`sub` for diagnostics.
fn validate_and_log_claims(token: &str) -> Result<(), OidcAttestationError> {
    let mut parts = token.splitn(3, '.');
    let _header = parts.next().context(MalformedTokenSnafu)?;
    let payload_b64 = parts.next().context(MalformedTokenSnafu)?;
    let _signature = parts.next().context(MalformedTokenSnafu)?;

    if payload_b64.is_empty() {
        return MalformedTokenSnafu.fail();
    }

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .context(PayloadDecodeFailedSnafu)?;

    let payload: serde_json::Value =
        serde_json::from_slice(&payload_bytes).context(PayloadNotJsonSnafu)?;

    let iss = payload["iss"].as_str().unwrap_or("<missing>");
    let sub = payload["sub"].as_str().unwrap_or("<missing>");
    tracing::debug!(iss, sub, "OIDC WIF token claims");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_jwt(header: &str, payload: &str, signature: &str) -> String {
        format!(
            "{}.{}.{}",
            URL_SAFE_NO_PAD.encode(header.as_bytes()),
            URL_SAFE_NO_PAD.encode(payload.as_bytes()),
            URL_SAFE_NO_PAD.encode(signature.as_bytes()),
        )
    }

    #[test]
    fn valid_token_with_iss_sub() {
        let token = make_jwt(
            r#"{"alg":"RS256"}"#,
            r#"{"iss":"https://accounts.google.com","sub":"user@example.com"}"#,
            "sig",
        );
        assert!(validate_and_log_claims(&token).is_ok());
    }

    #[test]
    fn valid_token_without_iss_sub() {
        let token = make_jwt(r#"{"alg":"RS256"}"#, r#"{"aud":"snowflake"}"#, "sig");
        assert!(validate_and_log_claims(&token).is_ok());
    }

    #[test]
    fn malformed_no_dots() {
        let err = validate_and_log_claims("nodots").unwrap_err();
        assert!(err.to_string().contains("not a valid JWT"));
    }

    #[test]
    fn malformed_one_dot() {
        let err = validate_and_log_claims("one.dot").unwrap_err();
        assert!(err.to_string().contains("not a valid JWT"));
    }

    #[test]
    fn malformed_empty_payload() {
        let err = validate_and_log_claims("header..signature").unwrap_err();
        assert!(err.to_string().contains("not a valid JWT"));
    }

    #[test]
    fn invalid_base64_payload() {
        let err = validate_and_log_claims("header.!!!invalid.signature").unwrap_err();
        assert!(err.to_string().contains("base64-decode"));
    }

    #[test]
    fn invalid_json_payload() {
        let payload = URL_SAFE_NO_PAD.encode(b"not json");
        let token = format!("header.{payload}.signature");
        let err = validate_and_log_claims(&token).unwrap_err();
        assert!(err.to_string().contains("not valid JSON"));
    }
}
