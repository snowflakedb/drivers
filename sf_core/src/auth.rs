//! Authentication module for Snowflake login.
//!
//! This module handles credential creation for various authentication methods.

use snafu::{Location, Snafu};

use crate::config::rest_parameters::{LoginMethod, LoginParameters};
use crate::crypto::{CryptoError, DefaultJwtSigner, JwtSigner};

/// Extracts the account locator from a full account identifier.
///
/// Per Snowflake documentation, the JWT `iss` field must use just the account locator
/// without region or cloud provider information, in uppercase.
/// See: https://docs.snowflake.com/en/developer-guide/sql-api/authenticating#using-key-pair-authentication
///
/// # Examples
/// - `"sfctest0"` -> `"SFCTEST0"`
/// - `"driverspreprod6.preprod6.us-west-2.aws"` -> `"DRIVERSPREPROD6"`
/// - `"myaccount.us-east-1"` -> `"MYACCOUNT"`
pub fn extract_account_locator(account: &str) -> String {
    account.split('.').next().unwrap().to_uppercase()
}

pub enum Credentials {
    Password { username: String, password: String },
    Jwt { username: String, token: String },
    Pat { username: String, token: String },
}

pub fn create_credentials(login_parameters: &LoginParameters) -> Result<Credentials, AuthError> {
    match &login_parameters.login_method {
        LoginMethod::Password { username, password } => Ok(Credentials::Password {
            username: username.clone(),
            password: password.clone(),
        }),
        LoginMethod::PrivateKey {
            username,
            private_key,
            passphrase,
        } => {
            let signer = DefaultJwtSigner;
            let token = signer
                .sign_rs256(
                    private_key.as_bytes(),
                    passphrase.as_ref().map(|p| p.as_bytes()),
                    &login_parameters.account_name,
                    username,
                )
                .map_err(|e| AuthError::JwtGeneration {
                    source: e,
                    location: snafu::Location::new(file!(), line!(), column!()),
                })?;
            Ok(Credentials::Jwt {
                username: username.clone(),
                token,
            })
        }
        LoginMethod::Pat { username, token } => Ok(Credentials::Pat {
            username: username.clone(),
            token: token.clone(),
        }),
    }
}

#[derive(Debug, Snafu)]
pub enum AuthError {
    #[snafu(display("JWT generation failed"))]
    JwtGeneration {
        source: CryptoError,
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_account_locator_simple() {
        // Simple account name without region
        assert_eq!(extract_account_locator("sfctest0"), "SFCTEST0");
        assert_eq!(extract_account_locator("myaccount"), "MYACCOUNT");
    }

    #[test]
    fn test_extract_account_locator_with_region() {
        // Account name with region suffix (common format)
        assert_eq!(
            extract_account_locator("driverspreprod6.preprod6.us-west-2.aws"),
            "DRIVERSPREPROD6"
        );
        assert_eq!(extract_account_locator("myaccount.us-east-1"), "MYACCOUNT");
        assert_eq!(
            extract_account_locator("testaccount.eu-central-1.azure"),
            "TESTACCOUNT"
        );
    }

    #[test]
    fn test_extract_account_locator_already_uppercase() {
        // Already uppercase input
        assert_eq!(extract_account_locator("SFCTEST0"), "SFCTEST0");
        assert_eq!(extract_account_locator("MYACCOUNT.US-WEST-2"), "MYACCOUNT");
    }

    #[test]
    fn test_extract_account_locator_mixed_case() {
        // Mixed case input
        assert_eq!(extract_account_locator("SfcTest0"), "SFCTEST0");
        assert_eq!(
            extract_account_locator("MyAccount.Us-West-2.Aws"),
            "MYACCOUNT"
        );
    }

    #[test]
    fn test_extract_account_locator_empty() {
        // Edge case: empty string
        assert_eq!(extract_account_locator(""), "");
    }
}
