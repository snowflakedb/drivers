use jwt::PKeyWithDigest;
use jwt::SignWithKey;
use openssl::hash::MessageDigest;
use serde::Serialize;
use snafu::{Location, ResultExt, Snafu};

use crate::config::rest_parameters::{LoginMethod, LoginParameters};

pub enum Credentials {
    Password { username: String, password: String },
    Jwt { username: String, token: String },
    Pat { username: String, token: String },
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

    let sub = format!("{}.{}", account.to_uppercase(), username.to_uppercase());
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
        LoginMethod::PrivateKey {
            username,
            private_key,
            passphrase,
        } => {
            let token = generate_jwt_token(
                &login_parameters.account_name,
                username,
                private_key,
                passphrase.as_deref(),
            )?;
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
