use jwt::PKeyWithDigest;
use jwt::SignWithKey;
use openssl::hash::MessageDigest;
use serde::Serialize;
use snafu::{Location, ResultExt, Snafu};

use crate::config::rest_parameters::{LoginMethod, LoginParameters};
use crate::sensitive::SensitiveString;

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
    Password {
        username: String,
        password: SensitiveString,
    },
    Jwt {
        username: String,
        token: SensitiveString,
    },
    Pat {
        username: String,
        token: SensitiveString,
    },
    UserPasswordMfa {
        username: String,
        password: SensitiveString,
        passcode_in_password: bool,
        passcode: Option<SensitiveString>,
    },
}

#[derive(Debug, Serialize)]
struct Claim {
    sub: String,
    iss: String,
    iat: i64,
    exp: i64,
}

fn generate_jwt_token(
    account: &str,
    username: &str,
    private_key: &str,
    passphrase: Option<&str>,
) -> Result<String, AuthError> {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    use jwt::{Header, Token};
    use openssl::{pkey::PKey, rsa::Rsa};
    use std::time::{SystemTime, UNIX_EPOCH};

    // Parse RSA private key
    let rsa = if let Some(passphrase) = passphrase {
        Rsa::private_key_from_pem_passphrase(private_key.as_bytes(), passphrase.as_bytes())
    } else {
        Rsa::private_key_from_pem(private_key.as_bytes())
    }
    .context(InvalidPrivateKeyFormatSnafu)?;
    let private_key = PKey::from_rsa(rsa).context(PrivateKeyCreationSnafu)?;

    // Extract public key and hash it
    let public_key_der = private_key
        .public_key_to_der()
        .context(PublicKeyExtractionSnafu)?;
    let mut hasher = openssl::sha::Sha256::new();
    hasher.update(&public_key_der);
    let public_key_hash = hasher.finish();
    let public_key_b64 = BASE64.encode(public_key_hash);

    let pkey_with_digest = PKeyWithDigest {
        digest: MessageDigest::sha256(),
        key: private_key,
    };

    // Create JWT header
    let header = Header {
        algorithm: jwt::AlgorithmType::Rs256,
        ..Default::default()
    };

    // Create claims
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context(SystemTimeSnafu)?
        .as_secs() as i64;

    // Normalize the account name to just the account locator (first segment before any dots).
    // Per Snowflake documentation, the JWT `iss` field must use the account locator without
    // region information, and both account and username must be uppercase.
    // See: https://docs.snowflake.com/en/developer-guide/sql-api/authenticating#using-key-pair-authentication
    // Format: <account_locator>.<user>.SHA256:<public_key_fingerprint>
    // Example: "driverspreprod6.preprod6.us-west-2.aws" -> "DRIVERSPREPROD6"
    let account_locator = extract_account_locator(account);

    let sub = format!("{}.{}", account_locator, username.to_uppercase());
    let iss = format!("{sub}.SHA256:{public_key_b64}");
    let claim: Claim = Claim {
        sub,
        iss,
        iat: now,
        exp: now + 120,
    };

    // Create and sign token
    let token = Token::new(header, claim)
        .sign_with_key(&pkey_with_digest)
        .context(JWTSigningSnafu)?;

    Ok(token.as_str().to_string())
}

pub fn create_credentials(login_parameters: &LoginParameters) -> Result<Credentials, AuthError> {
    match &login_parameters.login_method {
        LoginMethod::Password { username, password } => Ok(Credentials::Password {
            username: username.clone(),
            password: password.clone(),
        }),
        // NativeOkta performs its own multi-step SAML flow in auth_request_data()
        // and never reaches create_credentials(). Return an error rather than panicking
        // to avoid a footgun if a future caller invokes this function directly.
        LoginMethod::NativeOkta(_) => UnsupportedLoginMethodSnafu {
            method: "NativeOkta",
        }
        .fail(),
        LoginMethod::PrivateKey {
            username,
            private_key,
            passphrase,
        } => {
            let token = generate_jwt_token(
                &login_parameters.account_name,
                username,
                private_key.reveal(),
                passphrase.as_ref().map(|p| p.reveal().as_str()),
            )?;
            Ok(Credentials::Jwt {
                username: username.clone(),
                token: token.into(),
            })
        }
        LoginMethod::Pat { username, token } => Ok(Credentials::Pat {
            username: username.clone(),
            token: token.clone(),
        }),
        LoginMethod::UserPasswordMfa {
            username,
            password,
            passcode_in_password,
            passcode,
            ..
        } => Ok(Credentials::UserPasswordMfa {
            username: username.clone(),
            password: password.clone(),
            passcode: passcode.clone(),
            passcode_in_password: *passcode_in_password,
        }),
    }
}

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
pub enum AuthError {
    #[snafu(display(
        "Login method '{method}' does not use create_credentials — it has its own auth flow"
    ))]
    UnsupportedLoginMethod {
        method: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid private key format"))]
    InvalidPrivateKeyFormat {
        source: openssl::error::ErrorStack,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to create private key from RSA"))]
    PrivateKeyCreation {
        source: openssl::error::ErrorStack,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to extract public key from private key"))]
    PublicKeyExtraction {
        source: openssl::error::ErrorStack,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to get current system time"))]
    SystemTime {
        source: std::time::SystemTimeError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to sign JWT token"))]
    JWTSigning {
        source: jwt::Error,
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
