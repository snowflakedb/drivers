//! URL-safe random tokens for the OAuth flows, drawn from the crypto module.
//!
//! `oauth2`'s own generators -- `CsrfToken::new_random_len` and
//! `PkceCodeChallenge::new_random_sha256` -- draw from `rand`'s thread RNG.
//! That is a reasonable default for a general-purpose client and the wrong
//! source here: under `fips-tls` the claim is that security-relevant randomness
//! comes from the validated module, and `rand` is not it (F8).
//!
//! The shape is deliberately identical to what `oauth2` produces -- N bytes of
//! randomness, base64url-encoded without padding -- so only the *source*
//! changes. Values on the wire stay indistinguishable from the previous
//! implementation, which is what keeps RFC 7636 compliance and IdP
//! interoperability from depending on this swap.

use aws_lc_rs::rand::fill;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use snafu::ResultExt as _;
use zeroize::Zeroize as _;

use super::error::{OAuthError, RandomGenerationSnafu};

/// `num_bytes` of DRBG output, base64url-encoded without padding.
///
/// `purpose` names the value for the error message and is a `&'static str`
/// rather than the value itself: these are secrets (the PKCE verifier) or
/// anti-forgery tokens (the CSRF state) and must not reach a log.
///
/// The raw buffer is zeroized before returning. The encoded string is the only
/// copy that leaves, and its owner decides how to protect it -- the PKCE
/// verifier goes straight into a `SensitiveString`.
pub(crate) fn token_b64url(num_bytes: usize, purpose: &'static str) -> Result<String, OAuthError> {
    let mut bytes = vec![0u8; num_bytes];
    fill(&mut bytes).context(RandomGenerationSnafu { purpose })?;
    let encoded = URL_SAFE_NO_PAD.encode(&bytes);
    bytes.zeroize();
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 32 bytes base64url-encode to 43 characters with no padding. This is the
    /// length RFC 7636 requires of a PKCE verifier, and the reason callers pass
    /// 32 rather than a rounder number.
    #[test]
    fn thirty_two_bytes_encodes_to_forty_three_chars() {
        let t = token_b64url(32, "test").expect("DRBG");
        assert_eq!(t.len(), 43, "got {t}");
        assert!(!t.contains('='), "unexpected padding: {t}");
    }

    #[test]
    fn uses_url_safe_alphabet_only() {
        let t = token_b64url(32, "test").expect("DRBG");
        for c in t.chars() {
            assert!(
                c.is_ascii_alphanumeric() || c == '-' || c == '_',
                "non-URL-safe character {c} in {t}"
            );
        }
    }

    /// Not a randomness quality test -- that belongs to AWS-LC. This only
    /// catches the wiring mistake of returning a constant or a reused buffer.
    #[test]
    fn distinct_calls_differ() {
        let a = token_b64url(32, "test").expect("DRBG");
        let b = token_b64url(32, "test").expect("DRBG");
        assert_ne!(a, b);
    }
}
