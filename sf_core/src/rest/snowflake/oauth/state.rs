//! `state` parameter generation and CSRF validation for the AC flow.
//!
//! Mismatch must surface the canonical "It might indicate an XSS attack"
//! message — multiple drivers ship that exact wording and SREs grep for
//! it (analysis_feature_oauth.md §3.4 and §14 #7).

use oauth2::CsrfToken;

use super::error::{OAuthError, StateMismatchSnafu};
use crate::sensitive::SensitiveString;
use snafu::ensure;

/// Opaque CSRF state token.
///
/// Sensitive because state is correlated with a user session and reusing
/// or leaking it materially weakens CSRF protection (analysis §3.4).
#[derive(Debug, Clone)]
pub(crate) struct StateToken(SensitiveString);

impl StateToken {
    /// Borrow the inner state string for emission on the `/authorize` URL.
    /// Callers must not log this value.
    pub(crate) fn expose(&self) -> &str {
        self.0.reveal().as_str()
    }
}

/// Generate a fresh, base64url-encoded CSRF state token.
///
/// Uses `oauth2::CsrfToken::new_random()` which yields 16 random bytes
/// (~22 base64url chars). That's well above the OWASP recommendation of
/// ≥64 bits and comparable to ODBC's GUID + base64url.
pub(crate) fn generate() -> StateToken {
    StateToken(SensitiveString::from(CsrfToken::new_random().into_secret()))
}

/// Validate a received `state` against the expected token.
///
/// Constant-time byte comparison guards against timing oracles even though
/// state is not strictly secret — leaking the comparator's prefix would
/// undermine the CSRF guarantee. On mismatch (or empty `received`) we
/// surface the canonical XSS-suspicion message — gotcha §14 #7.
pub(crate) fn validate(expected: &StateToken, received: &str) -> Result<(), OAuthError> {
    ensure!(!received.is_empty(), StateMismatchSnafu);
    let expected_bytes = expected.0.reveal().as_bytes();
    let received_bytes = received.as_bytes();
    ensure!(
        expected_bytes.len() == received_bytes.len(),
        StateMismatchSnafu
    );
    let mut diff: u8 = 0;
    for (a, b) in expected_bytes.iter().zip(received_bytes.iter()) {
        diff |= a ^ b;
    }
    ensure!(diff == 0, StateMismatchSnafu);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_validates() {
        let token = generate();
        let copy = token.expose().to_string();
        assert!(validate(&token, &copy).is_ok());
    }

    #[test]
    fn mismatched_state_is_rejected_with_canonical_message() {
        let token = generate();
        let err =
            validate(&token, "tampered-or-attacker-supplied").expect_err("mismatch should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("It might indicate an XSS attack."),
            "unexpected message: {msg}"
        );
        assert!(matches!(err, OAuthError::StateMismatch { .. }));
    }

    #[test]
    fn empty_state_is_rejected() {
        let token = generate();
        let err = validate(&token, "").expect_err("empty must fail");
        assert!(matches!(err, OAuthError::StateMismatch { .. }));
    }

    #[test]
    fn distinct_generations_produce_distinct_tokens() {
        let a = generate();
        let b = generate();
        assert_ne!(a.expose(), b.expose());
    }

    #[test]
    fn close_but_not_equal_state_is_rejected() {
        // Same prefix, one extra trailing char; would slip past a naive
        // `.starts_with` check but must be rejected.
        let token = generate();
        let mut tampered = token.expose().to_string();
        tampered.push('A');
        let err = validate(&token, &tampered).expect_err("extra char must fail");
        assert!(matches!(err, OAuthError::StateMismatch { .. }));
    }
}
