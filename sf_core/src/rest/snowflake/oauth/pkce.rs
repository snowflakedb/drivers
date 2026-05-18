//! PKCE (RFC 7636) verifier and S256 challenge generation.
//!
//! Cross-driver verifier sizes converge on ≥43 URL-safe characters; see
//! `analysis_feature_oauth.md` §3.3 for the per-driver matrix. We use the
//! `oauth2` crate's RFC-compliant generator (32 random bytes → 43-char
//! URL-safe verifier), matching ODBC/Go/Node and well within the
//! 43..=128 character window required by the RFC.

use oauth2::PkceCodeChallenge;

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
/// Delegates to `oauth2::PkceCodeChallenge::new_random_sha256()` so that
/// (a) the verifier is RFC 7636 compliant (43 URL-safe chars from 32 random
/// bytes) and (b) the SHA-256 derivation of the challenge matches what the
/// IdP expects to verify against.
pub(crate) fn generate() -> PkceMaterial {
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    PkceMaterial {
        verifier: SensitiveString::from(verifier.into_secret()),
        challenge: challenge.as_str().to_string(),
        method: "S256",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use sha2::{Digest, Sha256};

    #[test]
    fn generated_verifier_is_at_least_43_chars() {
        let m = generate();
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
        let m = generate();
        for c in m.verifier.reveal().chars() {
            assert!(
                c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~',
                "verifier contains non-URL-safe character: {c}"
            );
        }
    }

    #[test]
    fn challenge_is_base64url_no_padding() {
        let m = generate();
        assert!(
            !m.challenge.contains('='),
            "challenge has padding: {}",
            m.challenge
        );
        assert!(URL_SAFE_NO_PAD.decode(m.challenge.as_bytes()).is_ok());
    }

    #[test]
    fn method_is_s256() {
        let m = generate();
        assert_eq!(m.method, "S256");
    }

    #[test]
    fn distinct_calls_produce_distinct_verifiers() {
        let a = generate();
        let b = generate();
        assert_ne!(a.verifier.reveal(), b.verifier.reveal());
        assert_ne!(a.challenge, b.challenge);
    }

    #[test]
    fn challenge_equals_b64url_sha256_of_verifier() {
        let m = generate();
        let mut hasher = Sha256::new();
        hasher.update(m.verifier.reveal().as_bytes());
        let digest = hasher.finalize();
        let expected = URL_SAFE_NO_PAD.encode(digest);
        assert_eq!(m.challenge, expected);
    }
}
