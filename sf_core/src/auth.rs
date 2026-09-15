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
    account
        .split('.')
        .next()
        .expect("str::split always yields at least one element")
        .to_uppercase()
}

pub enum Credentials {
    Password {
        username: String,
        password: SensitiveString,
        passcode_in_password: bool,
        passcode: Option<SensitiveString>,
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
    /// Pre-acquired OAuth access token forwarded to Snowflake unchanged
    /// (legacy `AUTHENTICATOR=OAUTH` with raw `token=`). Consumed by
    /// `auth_request_data` to populate the legacy login body.
    OAuth {
        username: String,
        access_token: SensitiveString,
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
    use aws_lc_rs::encoding::AsDer;
    use aws_lc_rs::signature::KeyPair;
    use base64::Engine as _;
    use base64::engine::general_purpose::{STANDARD as BASE64, URL_SAFE_NO_PAD};
    use std::time::{SystemTime, UNIX_EPOCH};

    let key = crate::crypto::private_key::load_rsa_key(private_key.as_bytes(), passphrase)
        .context(InvalidPrivateKeyFormatSnafu)?;

    // Snowflake matches the `iss` fingerprint against SHA-256 of the *SPKI*
    // (X.509 SubjectPublicKeyInfo) DER -- the same bytes
    // `openssl rsa -pubin -outform DER` emits. `PublicKeyX509Der` is that
    // encoding; the PKCS#1 `RSAPublicKey` form would hash to something the
    // server does not recognise.
    let spki = key
        .public_key()
        .as_der()
        .context(PublicKeyExtractionSnafu)?;
    let fingerprint = aws_lc_rs::digest::digest(&aws_lc_rs::digest::SHA256, spki.as_ref());
    let public_key_b64 = BASE64.encode(fingerprint.as_ref());

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
    let claim = Claim {
        sub,
        iss,
        iat: now,
        exp: now + 120,
    };

    // The JWS is assembled here rather than through the `jwt` crate, whose
    // signing traits are implemented only for OpenSSL keys (`PKeyWithDigest`).
    // Same shape as `rest::snowflake::oauth::dpop`, which hand-builds its proof
    // for a different reason.
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","typ":"JWT"}"#);
    let claims = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claim).context(JWTSerializeSnafu)?);
    let signing_input = format!("{header}.{claims}");

    let mut signature = vec![0u8; key.public_modulus_len()];
    key.sign(
        &aws_lc_rs::signature::RSA_PKCS1_SHA256,
        &aws_lc_rs::rand::SystemRandom::new(),
        signing_input.as_bytes(),
        &mut signature,
    )
    .context(JWTSignSnafu)?;

    Ok(format!(
        "{signing_input}.{}",
        URL_SAFE_NO_PAD.encode(&signature)
    ))
}

pub async fn create_credentials(
    login_parameters: &LoginParameters,
) -> Result<Credentials, AuthError> {
    match &login_parameters.login_method {
        LoginMethod::Password {
            username,
            password,
            passcode_in_password,
            passcode,
        } => Ok(Credentials::Password {
            username: username.clone(),
            password: password.clone(),
            passcode_in_password: *passcode_in_password,
            passcode: passcode.clone(),
        }),
        // NativeOkta and ExternalBrowser perform their own multi-step flows in
        // auth_request_data() and never reach create_credentials(). Return an error
        // rather than panicking to avoid a footgun if a future caller invokes this
        // function directly.
        LoginMethod::NativeOkta(_) => UnsupportedLoginMethodSnafu {
            method: "NativeOkta",
        }
        .fail(),
        LoginMethod::ExternalBrowser { .. } => UnsupportedLoginMethodSnafu {
            method: "ExternalBrowser",
        }
        .fail(),
        LoginMethod::PrivateKey {
            username,
            private_key,
            passphrase,
        } => {
            // RSA key parsing + RS256 signing is CPU-bound crypto; run it on
            // the blocking pool so it doesn't stall this runtime worker. Move
            // the secrets in as owned `SensitiveString`s so they stay zeroizing.
            let account = login_parameters.account_name.clone();
            let username_owned = username.clone();
            let private_key = private_key.clone();
            let passphrase = passphrase.clone();
            let token = tokio::task::spawn_blocking(move || {
                generate_jwt_token(
                    &account,
                    &username_owned,
                    private_key.reveal(),
                    passphrase.as_ref().map(|p| p.reveal().as_str()),
                )
            })
            .await
            .context(BlockingTaskJoinSnafu)??;
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
        LoginMethod::OAuthAccessToken { username, token } => Ok(Credentials::OAuth {
            username: username.clone(),
            access_token: token.clone(),
        }),
        // OAuth AC and CC run their own multi-step flow (PKCE,
        // browser/loopback, token exchange, refresh) outside of
        // create_credentials. Mirror the NativeOkta arm above and surface
        // a typed error rather than panicking.
        LoginMethod::OAuthAuthorizationCode(_) => UnsupportedLoginMethodSnafu {
            method: "OAuthAuthorizationCode",
        }
        .fail(),
        LoginMethod::OAuthClientCredentials(_) => UnsupportedLoginMethodSnafu {
            method: "OAuthClientCredentials",
        }
        .fail(),
        // Session token auth bypasses create_credentials entirely — it validates
        // via /session/token-request before this function is ever reached.
        LoginMethod::SessionToken { .. } => UnsupportedLoginMethodSnafu {
            method: "SessionToken",
        }
        .fail(),
        LoginMethod::WorkloadIdentity(_) => UnsupportedLoginMethodSnafu {
            method: "WorkloadIdentity",
        }
        .fail(),
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
        source: crate::crypto::private_key::PrivateKeyError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to extract public key from private key"))]
    PublicKeyExtraction {
        source: aws_lc_rs::error::Unspecified,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to get current system time"))]
    SystemTime {
        source: std::time::SystemTimeError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to serialize JWT claims"))]
    JWTSerialize {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to sign JWT token"))]
    JWTSign {
        source: aws_lc_rs::error::Unspecified,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Background JWT-signing task failed to join"))]
    BlockingTaskJoin {
        source: tokio::task::JoinError,
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    use base64::Engine as _;
    use base64::engine::general_purpose::{STANDARD as BASE64, URL_SAFE_NO_PAD};

    const PASSPHRASE: &str = "correct horse battery staple";

    /// The `iss` fingerprint must be SHA-256 of the SPKI DER, Base64.
    ///
    /// This is the one value the server independently recomputes: Snowflake
    /// compares it against `RSA_PUBLIC_KEY_FP`, and the documented way for a
    /// user to check their own key is
    /// `openssl rsa -pubin -outform DER | openssl dgst -sha256 -binary | openssl enc -base64`.
    /// So the assertion is against OpenSSL's `public_key_to_der()`, not against
    /// another AWS-LC call -- an encoding mistake here (PKCS#1 `RSAPublicKey`
    /// instead of SPKI) would produce a well-formed JWT that every login
    /// rejects.
    #[test]
    fn jwt_iss_fingerprint_matches_openssl_spki_sha256() {
        let rsa = openssl::rsa::Rsa::generate(2048).unwrap();
        let pem = String::from_utf8(rsa.private_key_to_pem().unwrap()).unwrap();
        let pkey = openssl::pkey::PKey::from_rsa(rsa).unwrap();

        let expected = BASE64.encode(openssl::sha::sha256(&pkey.public_key_to_der().unwrap()));

        let token = generate_jwt_token("myaccount.us-east-1", "myuser", &pem, None).unwrap();
        let claims: serde_json::Value = serde_json::from_slice(
            &URL_SAFE_NO_PAD
                .decode(token.split('.').nth(1).unwrap())
                .unwrap(),
        )
        .unwrap();

        assert_eq!(
            claims["iss"].as_str().unwrap(),
            format!("MYACCOUNT.MYUSER.SHA256:{expected}"),
            "iss must carry the SPKI SHA-256 fingerprint the server recomputes"
        );
        assert_eq!(claims["sub"].as_str().unwrap(), "MYACCOUNT.MYUSER");
    }

    /// Sign with our code, verify with OpenSSL. Proves the hand-built JWS is a
    /// valid RS256 signature over `header.claims` and not merely well-shaped.
    #[test]
    fn jwt_signature_verifies_under_openssl() {
        let rsa = openssl::rsa::Rsa::generate(2048).unwrap();
        let pem = String::from_utf8(rsa.private_key_to_pem().unwrap()).unwrap();
        let pkey = openssl::pkey::PKey::from_rsa(rsa).unwrap();

        let token = generate_jwt_token("acct", "user", &pem, None).unwrap();
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT must have three segments");

        let header: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0]).unwrap()).unwrap();
        assert_eq!(header["alg"], "RS256");
        assert_eq!(header["typ"], "JWT");

        let mut verifier =
            openssl::sign::Verifier::new(openssl::hash::MessageDigest::sha256(), &pkey).unwrap();
        verifier
            .update(format!("{}.{}", parts[0], parts[1]).as_bytes())
            .unwrap();
        assert!(
            verifier
                .verify(&URL_SAFE_NO_PAD.decode(parts[2]).unwrap())
                .unwrap(),
            "RS256 signature must verify under OpenSSL"
        );
    }

    /// An encrypted key in the format Snowflake documents (`-v2 des3`) must
    /// produce the same JWT as its unencrypted equivalent.
    #[test]
    fn encrypted_key_produces_the_same_issuer_as_unencrypted() {
        let rsa = openssl::rsa::Rsa::generate(2048).unwrap();
        let pkey = openssl::pkey::PKey::from_rsa(rsa).unwrap();
        let plain = String::from_utf8(pkey.private_key_to_pem_pkcs8().unwrap()).unwrap();
        let encrypted = String::from_utf8(
            pkey.private_key_to_pem_pkcs8_passphrase(
                openssl::symm::Cipher::des_ede3_cbc(),
                PASSPHRASE.as_bytes(),
            )
            .unwrap(),
        )
        .unwrap();

        let iss_of = |pem: &str, pass: Option<&str>| -> String {
            let token = generate_jwt_token("acct", "user", pem, pass).unwrap();
            let claims: serde_json::Value = serde_json::from_slice(
                &URL_SAFE_NO_PAD
                    .decode(token.split('.').nth(1).unwrap())
                    .unwrap(),
            )
            .unwrap();
            claims["iss"].as_str().unwrap().to_string()
        };

        assert_eq!(
            iss_of(&plain, None),
            iss_of(&encrypted, Some(PASSPHRASE)),
            "unwrapping must recover the identical key"
        );
    }

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

    /// generate_jwt_token decrypts an encrypted PEM with
    /// Rsa::private_key_from_pem_passphrase. Wrong and empty passphrases must
    /// be rejected.
    ///
    /// Deliberately not testing a missing passphrase (None): that routes to
    /// Rsa::private_key_from_pem, which for an encrypted key falls back to
    /// OpenSSL's interactive "Enter PEM pass phrase:" UI. That is
    /// environment-dependent and not safe in an automated test.
    #[test]
    fn generate_jwt_token_rejects_wrong_password_for_encrypted_key() {
        use openssl::rsa::Rsa;
        use openssl::symm::Cipher;

        let rsa = Rsa::generate(2048).expect("generate rsa key");
        let encrypted_pem = rsa
            .private_key_to_pem_passphrase(Cipher::aes_256_cbc(), b"correct_password")
            .expect("encrypt key");
        let encrypted_pem = String::from_utf8(encrypted_pem).expect("pem is utf8");

        let wrong = generate_jwt_token("acct", "user", &encrypted_pem, Some("wrong_password"));
        assert!(
            matches!(wrong, Err(AuthError::InvalidPrivateKeyFormat { .. })),
            "wrong password should be rejected, got: {wrong:?}"
        );

        let empty = generate_jwt_token("acct", "user", &encrypted_pem, Some(""));
        assert!(
            matches!(empty, Err(AuthError::InvalidPrivateKeyFormat { .. })),
            "empty password should be rejected, got: {empty:?}"
        );

        // Sanity: correct password succeeds, proving the encrypted fixture
        // above is genuinely encrypted and genuinely decryptable.
        assert!(
            generate_jwt_token("acct", "user", &encrypted_pem, Some("correct_password")).is_ok()
        );
    }
}
