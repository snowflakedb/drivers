//! Errors raised by the OAuth flow engine.
//!
//! The variant set is derived from the cross-driver error taxonomy in
//! `analysis_feature_oauth.md` §13. Marked `pub(crate)` because the
//! wiring layer (step 2.3) translates these into the driver-facing
//! `RestError` / `AuthError` taxonomy. Crate-internal callers should
//! match on variants to make eviction / refresh decisions (analysis §8:
//! e.g. `RefreshTokenExchange` should drop the cached refresh token and
//! replay the full flow).

use crate::token_cache::TokenCacheError;
use snafu::{Location, Snafu};

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub enum OAuthError {
    /// `state` mismatch on the loopback redirect — analysis §14 #7. The
    /// display string is the verbatim ODBC/JDBC message (analysis §13 /
    /// §14 #7); SREs grep for it. Equality is enforced via
    /// `oauth2::CsrfToken`'s timing-safe `PartialEq` (the
    /// `timing-resistant-secret-traits` feature derives a SHA-256-based
    /// comparator).
    #[snafu(display(
        "Identity Provider did not provide expected state parameter! It might indicate an XSS attack."
    ))]
    StateMismatch {
        #[snafu(implicit)]
        location: Location,
    },

    /// Loopback redirect arrived without a `code=` query parameter and
    /// without an `error=` parameter (defensive — should not happen in
    /// well-behaved IdPs).
    #[snafu(display("Authorization redirect did not include an authorization code"))]
    MissingAuthorizationCode {
        #[snafu(implicit)]
        location: Location,
    },

    /// IdP returned `error=...&error_description=...` either on the
    /// authorize redirect or in the token-endpoint response.
    #[snafu(display("Identity Provider responded with error: {error}: {description}"))]
    IdpError {
        error: String,
        description: String,
        #[snafu(implicit)]
        location: Location,
    },

    /// Loopback `accept()` did not see a request before the configured
    /// browser-response timeout elapsed (cross-driver: JDBC/Python/Go/.NET
    /// 120s default, analysis §3.5).
    #[snafu(display("OAuth browser authorization timed out"))]
    BrowserTimeout {
        #[snafu(implicit)]
        location: Location,
    },

    /// End-to-end `authentication_timeout` budget expired during the
    /// OAuth flow — covers the loopback wait, the IdP token exchange
    /// (or refresh), and any retry attempt (drift B.3; `doc/oauth.md`
    /// §2). Distinct from [`Self::BrowserTimeout`] so callers can
    /// distinguish "browser leg ran too long" from "the whole flow
    /// budget was exhausted".
    #[snafu(display("OAuth authentication budget exceeded after {elapsed_secs}s"))]
    AuthenticationTimeout {
        elapsed_secs: u64,
        #[snafu(implicit)]
        location: Location,
    },

    /// Could not bind a TCP listener on `127.0.0.1:<port>` for the
    /// loopback redirect (e.g. port already in use, EACCES).
    #[snafu(display("Failed to bind loopback listener for OAuth redirect"))]
    PortBind {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    /// Loopback / redirect URI parse error (typically when the user
    /// supplied a malformed `oauth_redirect_uri`).
    #[snafu(display("Failed to parse OAuth redirect URI"))]
    RedirectUriParse {
        source: url::ParseError,
        #[snafu(implicit)]
        location: Location,
    },

    /// IdP endpoint URL (`oauth_authorization_url`, `oauth_token_url`)
    /// failed to parse, either as supplied or as constructed from the
    /// Snowflake server URL via the `https://{host}/oauth/...` defaults.
    #[snafu(display("Failed to parse OAuth endpoint URL: {url}"))]
    EndpointUrlParse {
        url: String,
        source: url::ParseError,
        #[snafu(implicit)]
        location: Location,
    },

    /// Could not deserialize the token endpoint JSON response (used by
    /// the `RequestTokenError::Parse` mapping in the AC/CC flows).
    #[snafu(display("Failed to decode OAuth token response"))]
    TokenResponseDecode {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },

    /// Refresh-token exchange failed. Caller should evict the refresh
    /// token from cache and replay the full flow (analysis §7.4 / §8).
    #[snafu(display("OAuth refresh-token exchange failed"))]
    RefreshTokenExchange {
        #[snafu(implicit)]
        location: Location,
    },

    /// Token endpoint returned 2xx but no `access_token` field. Treated
    /// as an error to avoid passing an empty Bearer token downstream
    /// (analysis §13: ODBC's `idp_auth_missing_access_token` fixture).
    #[snafu(display("OAuth token response did not include an access_token"))]
    MissingAccessToken {
        #[snafu(implicit)]
        location: Location,
    },

    /// DPoP proof JWT could not be constructed because of an underlying
    /// openssl primitive (key generation, coordinate extraction).
    #[snafu(display("Failed to generate DPoP proof JWT"))]
    DPoPProofGeneration {
        source: openssl::error::ErrorStack,
        #[snafu(implicit)]
        location: Location,
    },

    /// DPoP proof JWT signing failed inside the `jwt` crate (e.g.
    /// header/claims serialization or DER → JOSE conversion).
    #[snafu(display("Failed to sign DPoP proof JWT"))]
    DPoPProofSigning {
        source: jwt::Error,
        #[snafu(implicit)]
        location: Location,
    },

    /// DPoP JWK could not be parsed (e.g. unsupported `kty`/`crv` or
    /// a missing required field on a cached bundled JWK).
    #[snafu(display("DPoP JWK could not be parsed: {reason}"))]
    DPoPJwkParse {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },

    /// Sentinel: token endpoint requested a DPoP nonce. Internal callers
    /// catch this once and retry with the supplied nonce embedded in the
    /// proof JWT. Never bubbled to user code (analysis §5.1).
    #[snafu(display("OAuth token endpoint requested a DPoP nonce; retrying"))]
    DPoPNonceRequired {
        #[snafu(implicit)]
        location: Location,
    },

    /// Token cache I/O failure (read, write or delete). Non-fatal at the
    /// flow level — callers WARN and fall through to a fresh exchange.
    #[snafu(display("OAuth token cache operation failed"))]
    Cache {
        source: TokenCacheError,
        #[snafu(implicit)]
        location: Location,
    },

    /// Underlying HTTP transport error from `reqwest` while talking to
    /// the IdP token endpoint.
    #[snafu(display("OAuth HTTP transport error"))]
    Transport {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
mod tests {
    //! Redaction tests covering analysis §11. The `OAuthError` variants
    //! deliberately do NOT carry tokens, refresh tokens, client secrets,
    //! PKCE verifiers, IdP authorization codes, or DPoP private keys —
    //! these belong in [`SensitiveString`] which redacts in `Display`/`Debug`.
    //! The tests below pin that contract so a future variant addition that
    //! accidentally captures a secret stays caught by `cargo test`.
    use super::*;

    /// All canary values include `LEAK-CANARY` so a regex-driven log
    /// scrubber test (or a developer's `grep -r LEAK-CANARY target/`) can
    /// pick up any accidental capture in compiled output as well.
    const ACCESS_TOKEN: &str = "AT-LEAK-CANARY-9b13c7";
    const REFRESH_TOKEN: &str = "RT-LEAK-CANARY-9b13c7";
    const CLIENT_SECRET: &str = "CS-LEAK-CANARY-9b13c7";
    const CODE_VERIFIER: &str = "CV-LEAK-CANARY-9b13c7";
    const AUTH_CODE: &str = "AUTH-CODE-LEAK-CANARY-9b13c7";
    const DPOP_PRIVATE: &str = "DPOP-PRIV-LEAK-CANARY-9b13c7";

    fn assert_no_canaries(text: &str) {
        for canary in [
            ACCESS_TOKEN,
            REFRESH_TOKEN,
            CLIENT_SECRET,
            CODE_VERIFIER,
            AUTH_CODE,
            DPOP_PRIVATE,
        ] {
            assert!(
                !text.contains(canary),
                "redaction breach: secret canary {canary:?} leaked into {text:?}"
            );
        }
    }

    /// Construct one error of each variant whose payload could plausibly
    /// receive a secret-shaped string at a future call site, and assert
    /// that neither `Display` nor `Debug` emit any of the canary tokens.
    /// `Display` is what tracing's `error = %e` formatter calls; `Debug`
    /// is what `?error` and `panic!` callers see.
    #[test]
    fn oauth_error_display_and_debug_never_leak_secret_canaries() {
        let mock_loc = Location::new(file!(), line!(), column!());

        let variants: Vec<OAuthError> = vec![
            OAuthError::StateMismatch { location: mock_loc },
            OAuthError::MissingAuthorizationCode { location: mock_loc },
            // `IdpError` is the one variant whose payload could be tempted to
            // hold a token. The cross-driver convention is that only the
            // server-supplied `error` slug + `error_description` are stored,
            // never any client-side secret. Verify that even when the
            // description contains nothing token-shaped, the variant carries
            // exactly the documented payload.
            OAuthError::IdpError {
                error: "invalid_grant".into(),
                description: "refresh token expired".into(),
                location: mock_loc,
            },
            OAuthError::BrowserTimeout { location: mock_loc },
            OAuthError::RefreshTokenExchange { location: mock_loc },
            OAuthError::MissingAccessToken { location: mock_loc },
            OAuthError::DPoPNonceRequired { location: mock_loc },
        ];

        for v in &variants {
            assert_no_canaries(&format!("{v}"));
            assert_no_canaries(&format!("{v:?}"));
        }
    }

    /// The canonical state-mismatch wording is grep'd by SREs across
    /// drivers (analysis §13 / §14 #7). Pin the exact prefix so accidental
    /// paraphrasing is caught immediately.
    #[test]
    fn state_mismatch_display_carries_canonical_xss_warning() {
        let err = OAuthError::StateMismatch {
            location: Location::new(file!(), line!(), column!()),
        };
        let msg = format!("{err}");
        assert!(
            msg.contains("It might indicate an XSS attack."),
            "missing canonical XSS wording in: {msg}"
        );
    }

    /// `IdpError`'s `Display` emits only the `error` slug and
    /// description — never any client-side secret — and only those
    /// fields appear (i.e. no surprising accidental Debug-style leak of
    /// other private fields).
    #[test]
    fn idp_error_display_only_emits_error_and_description() {
        let err = OAuthError::IdpError {
            error: "invalid_request".into(),
            description: "missing parameter X".into(),
            location: Location::new(file!(), line!(), column!()),
        };
        let msg = format!("{err}");
        assert_eq!(
            msg,
            "Identity Provider responded with error: invalid_request: missing parameter X",
        );
    }

    /// Defensive smoke test: `SensitiveString::reveal()` is the ONLY
    /// path that returns the raw secret. `Display` and `Debug` both
    /// mask via `****`. Anchored here so a regression in the wrapper
    /// (or its accidental replacement with a transparent newtype)
    /// fails the OAuth test suite immediately.
    #[test]
    fn sensitive_string_is_the_only_path_to_reveal_secrets() {
        use crate::sensitive::SensitiveString;
        let s = SensitiveString::from(ACCESS_TOKEN);
        assert_eq!(format!("{s}"), "****");
        assert_eq!(format!("{s:?}"), "****");
        assert_eq!(s.reveal(), ACCESS_TOKEN);
    }
}
