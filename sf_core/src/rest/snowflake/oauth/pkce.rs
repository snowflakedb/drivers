//! PKCE (RFC 7636) verifier and S256 challenge generation.
//!
//! Cross-driver verifier sizes converge on ≥43 URL-safe characters (JDBC
//! 256-bit Nimbus, ODBC/Go/Node 32-byte, Python 43-byte). We generate 32
//! random bytes → a 43-char URL-safe verifier, matching ODBC/Go/Node and well
//! within the 43..=128 character window required by the RFC.
//!
//! Both halves run on the crypto module rather than on `oauth2`'s defaults:
//! the verifier comes from the DRBG via [`super::random`], and the challenge
//! digest is computed by AWS-LC rather than RustCrypto (F8). `oauth2` exposes
//! no way to build a `PkceCodeChallenge` around a digest it did not compute
//! itself, so the caller puts `code_challenge`/`code_challenge_method` on the
//! authorize URL directly -- the same two parameters, with the same values,
//! that `set_pkce_challenge` would have added.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

use super::error::OAuthError;
use super::random;
use crate::sensitive::SensitiveString;

/// PKCE material for the authorization code flow.
///
/// `verifier` is sensitive (must never appear in logs); `challenge` and
/// `method` are public values that travel on the `/authorize` URL.
#[derive(Debug)]
pub(crate) struct PkceMaterial {
    pub(crate) verifier: SensitiveString,
    pub(crate) challenge: String,
    pub(crate) method: &'static str,
}

/// Generate a fresh PKCE verifier + S256 challenge pair.
///
/// RFC 7636 hashes the *encoded* verifier, not the random bytes behind it, so
/// the digest is taken over `verifier.as_bytes()`. Hashing the raw DRBG output
/// instead would still look plausible and would fail verification at the IdP.
pub(crate) fn generate() -> Result<PkceMaterial, OAuthError> {
    let verifier = random::token_b64url(32, "PKCE code verifier")?;
    let digest = aws_lc_rs::digest::digest(&aws_lc_rs::digest::SHA256, verifier.as_bytes());
    Ok(PkceMaterial {
        verifier: SensitiveString::from(verifier),
        challenge: URL_SAFE_NO_PAD.encode(digest.as_ref()),
        method: "S256",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use sha2::{Digest, Sha256};

    #[test]
    fn generated_verifier_is_at_least_43_chars() {
        let m = generate().expect("DRBG");
        assert!(
            m.verifier.reveal().len() >= 43,
            "verifier length is {}, expected >= 43",
            m.verifier.reveal().len()
        );
        assert!(
            m.verifier.reveal().len() <= 128,
            "verifier length is {}, expected <= 128",
            m.verifier.reveal().len()
        );
    }

    #[test]
    fn generated_verifier_is_url_safe_alphabet() {
        let m = generate().expect("DRBG");
        for c in m.verifier.reveal().chars() {
            assert!(
                c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~',
                "verifier contains non-URL-safe character: {c}"
            );
        }
    }

    #[test]
    fn challenge_is_base64url_no_padding() {
        let m = generate().expect("DRBG");
        assert!(
            !m.challenge.contains('='),
            "challenge has padding: {}",
            m.challenge
        );
        assert!(URL_SAFE_NO_PAD.decode(m.challenge.as_bytes()).is_ok());
    }

    #[test]
    fn method_is_s256() {
        let m = generate().expect("DRBG");
        assert_eq!(m.method, "S256");
    }

    #[test]
    fn distinct_calls_produce_distinct_verifiers() {
        let a = generate().expect("DRBG");
        let b = generate().expect("DRBG");
        assert_ne!(a.verifier.reveal(), b.verifier.reveal());
        assert_ne!(a.challenge, b.challenge);
    }

    #[test]
    fn challenge_equals_b64url_sha256_of_verifier() {
        let m = generate().expect("DRBG");
        let mut hasher = Sha256::new();
        hasher.update(m.verifier.reveal().as_bytes());
        let digest = hasher.finalize();
        let expected = URL_SAFE_NO_PAD.encode(digest);
        assert_eq!(m.challenge, expected);
    }
}
