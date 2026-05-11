// Some of the constants and helpers in this module are wired into the
// connection-string and setup-dialog paths in subsequent commits of
// the OAuth ODBC stack. Allow them to live here without warnings while
// the stack is being built.
#![allow(dead_code)]
//! OAuth DSN keys and helpers used by the ODBC wrapper.
//!
//! This module is the single source of truth for the OAuth-related
//! ODBC connection-string / DSN keys that the wrapper recognises.  The
//! actual OAuth flow lives in `sf_core::rest::snowflake::oauth`; this
//! module only declares the wire-level identifiers (the SCREAMING_SNAKE
//! ODBC keys) and the corresponding `sf_core` canonical names so the
//! wrapper can:
//!
//! * forward connection-string keys consistently,
//! * never persist OAuth secrets to the DSN registry,
//! * redact OAuth secrets from logs.
//!
//! The key list mirrors `analysis_feature_oauth.md` §9 (configuration
//! matrix) and matches the `param_registry` aliases in
//! `sf_core::config::param_registry`. All ODBC keys here are the
//! `JDBC/ODBC` SCREAMING_SNAKE form and resolve via `sf_core` to the
//! lowercase canonical name shown in the doc comment for each constant.

/// `OAUTH_CLIENT_ID` (canonical: `oauth_client_id`).
///
/// External-IdP client ID. When absent and the IdP is Snowflake, the
/// flow substitutes the literal `LOCAL_APPLICATION` (analysis §1).
pub const OAUTH_CLIENT_ID: &str = "OAUTH_CLIENT_ID";

/// `OAUTH_CLIENT_SECRET` (canonical: `oauth_client_secret`).
///
/// External-IdP client secret. **Sensitive** — must never be logged or
/// written to the DSN registry; treated like `PWD` by the wrapper.
pub const OAUTH_CLIENT_SECRET: &str = "OAUTH_CLIENT_SECRET";

/// `OAUTH_AUTHORIZATION_URL` (canonical: `oauth_authorization_url`).
///
/// IdP `/authorize` endpoint. Optional for Snowflake-as-IdP; required
/// for external IdPs in the AC flow.
pub const OAUTH_AUTHORIZATION_URL: &str = "OAUTH_AUTHORIZATION_URL";

/// `OAUTH_TOKEN_REQUEST_URL` (canonical: `oauth_token_request_url`).
///
/// IdP token endpoint. Required for the CC flow (Snowflake's GS does
/// not mint client-credentials tokens — analysis §4) and also used to
/// derive the cache host (analysis §7.3).
pub const OAUTH_TOKEN_REQUEST_URL: &str = "OAUTH_TOKEN_REQUEST_URL";

/// `OAUTH_REDIRECT_URI` (canonical: `oauth_redirect_uri`).
///
/// Loopback redirect URI for the AC flow. Defaults to
/// `http://127.0.0.1:<random>` when omitted (analysis §3.5).
pub const OAUTH_REDIRECT_URI: &str = "OAUTH_REDIRECT_URI";

/// `OAUTH_SCOPE` (canonical: `oauth_scope`).
///
/// Space-separated OAuth scope list. Defaults to
/// `session:role:<role>` when omitted (analysis §9).
pub const OAUTH_SCOPE: &str = "OAUTH_SCOPE";

/// `OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS` (canonical:
/// `oauth_enable_single_use_refresh_tokens`).
///
/// When `true` and Snowflake is the IdP, the AC flow asks for a
/// rotating single-use refresh token (analysis §3 / §7.4).
pub const OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS: &str = "OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS";

/// `OAUTH_DISABLE_PKCE` (canonical: `oauth_disable_pkce`).
///
/// Python parity (analysis §9). Defaults to `false`. When `true`, the
/// AC flow omits `code_challenge` / `code_verifier`.
pub const OAUTH_DISABLE_PKCE: &str = "OAUTH_DISABLE_PKCE";

/// `OAUTH_ENABLE_DPOP` (canonical: `oauth_enable_dpop`).
///
/// Opt-in to RFC 9449 DPoP (JDBC parity — analysis §5). Defaults to
/// `false`.
pub const OAUTH_ENABLE_DPOP: &str = "OAUTH_ENABLE_DPOP";

/// `OAUTH_DISABLE_CONSOLE_LOGIN` (canonical:
/// `oauth_disable_console_login`).
///
/// Carried for parity with JDBC `DISABLE_CONSOLE_LOGIN`; **does not**
/// affect OAuth flows (analysis §3.6) — only the legacy SAML
/// EXTERNALBROWSER `console_login` form.
pub const OAUTH_DISABLE_CONSOLE_LOGIN: &str = "OAUTH_DISABLE_CONSOLE_LOGIN";

/// `TOKEN` (canonical: `token`).
///
/// Pre-acquired access token used by the legacy
/// `AUTHENTICATOR=OAUTH` mode (analysis §6 / §10.1). **Sensitive** —
/// already in `REDACTED_KEYS` in `connection::connect_with_params`.
pub const TOKEN: &str = "TOKEN";

/// All ODBC DSN/connection-string keys defined by the OAuth feature
/// (analysis §9). Used by the wrapper to:
///
/// * teach `setup_dialog::write_dsn_values` which OAuth keys are safe
///   to persist (everything except [`OAUTH_CLIENT_SECRET`] and
///   [`TOKEN`]),
/// * extend log-redaction with the OAuth secret keys,
/// * keep a single grep target for cross-driver feature parity.
pub const ALL_OAUTH_KEYS: &[&str] = &[
    OAUTH_CLIENT_ID,
    OAUTH_CLIENT_SECRET,
    OAUTH_AUTHORIZATION_URL,
    OAUTH_TOKEN_REQUEST_URL,
    OAUTH_REDIRECT_URI,
    OAUTH_SCOPE,
    OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS,
    OAUTH_DISABLE_PKCE,
    OAUTH_ENABLE_DPOP,
    OAUTH_DISABLE_CONSOLE_LOGIN,
];

/// OAuth keys that contain secrets and must never be logged or
/// persisted to the DSN ini. The wrapper joins this list with
/// [`crate::api::oauth::TOKEN`] and the existing PWD-style redaction
/// list at the connection-string boundary.
pub const SENSITIVE_OAUTH_KEYS: &[&str] = &[OAUTH_CLIENT_SECRET, TOKEN];

/// Recognised values of the `AUTHENTICATOR` connection-string key
/// that select an OAuth flow. The wrapper does not parse these —
/// `sf_core::config::connection_config::build_auth_config` matches
/// them case-insensitively — but we re-declare them here so the
/// setup dialog and tests can branch on a single source of truth.
pub mod authenticator {
    /// `AUTHENTICATOR=OAUTH` — legacy pre-acquired access token mode
    /// (analysis §6).
    pub const OAUTH: &str = "OAUTH";

    /// `AUTHENTICATOR=OAUTH_AUTHORIZATION_CODE` — interactive
    /// authorization-code-with-PKCE flow (analysis §3).
    pub const OAUTH_AUTHORIZATION_CODE: &str = "OAUTH_AUTHORIZATION_CODE";

    /// `AUTHENTICATOR=OAUTH_CLIENT_CREDENTIALS` — non-interactive
    /// machine-to-machine flow against an external IdP (analysis §4).
    pub const OAUTH_CLIENT_CREDENTIALS: &str = "OAUTH_CLIENT_CREDENTIALS";
}

/// Returns `true` when `key` is a known ODBC OAuth DSN key
/// (case-insensitive).
pub fn is_oauth_key(key: &str) -> bool {
    ALL_OAUTH_KEYS.iter().any(|k| key.eq_ignore_ascii_case(k))
}

/// Returns `true` when `key` is an OAuth secret that must never be
/// persisted to the DSN registry or printed in logs (case-insensitive).
pub fn is_sensitive_oauth_key(key: &str) -> bool {
    SENSITIVE_OAUTH_KEYS
        .iter()
        .any(|k| key.eq_ignore_ascii_case(k))
}

/// Returns `true` when `auth` selects one of the OAuth flows
/// recognised by the wrapper (case-insensitive).
pub fn is_oauth_authenticator(auth: &str) -> bool {
    [
        authenticator::OAUTH,
        authenticator::OAUTH_AUTHORIZATION_CODE,
        authenticator::OAUTH_CLIENT_CREDENTIALS,
    ]
    .iter()
    .any(|v| auth.eq_ignore_ascii_case(v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_oauth_keys_includes_secret() {
        assert!(ALL_OAUTH_KEYS.contains(&OAUTH_CLIENT_SECRET));
    }

    #[test]
    fn is_oauth_key_is_case_insensitive() {
        assert!(is_oauth_key("oauth_client_id"));
        assert!(is_oauth_key("OAUTH_CLIENT_ID"));
        assert!(is_oauth_key("OAuth_Client_Id"));
        assert!(!is_oauth_key("UID"));
        assert!(!is_oauth_key(""));
    }

    #[test]
    fn is_sensitive_oauth_key_only_returns_true_for_secrets() {
        assert!(is_sensitive_oauth_key("OAUTH_CLIENT_SECRET"));
        assert!(is_sensitive_oauth_key("oauth_client_secret"));
        assert!(is_sensitive_oauth_key("TOKEN"));
        assert!(is_sensitive_oauth_key("token"));
        assert!(!is_sensitive_oauth_key("OAUTH_CLIENT_ID"));
        assert!(!is_sensitive_oauth_key("OAUTH_REDIRECT_URI"));
        assert!(!is_sensitive_oauth_key("OAUTH_SCOPE"));
    }

    #[test]
    fn is_oauth_authenticator_recognises_canonical_values() {
        assert!(is_oauth_authenticator("OAUTH"));
        assert!(is_oauth_authenticator("OAUTH_AUTHORIZATION_CODE"));
        assert!(is_oauth_authenticator("OAUTH_CLIENT_CREDENTIALS"));
        assert!(is_oauth_authenticator("oauth_authorization_code"));
        assert!(!is_oauth_authenticator("SNOWFLAKE_JWT"));
        assert!(!is_oauth_authenticator("PROGRAMMATIC_ACCESS_TOKEN"));
        assert!(!is_oauth_authenticator(""));
    }

    #[test]
    fn all_oauth_keys_and_sensitive_oauth_keys_are_disjoint_modulo_token() {
        // Every entry in SENSITIVE_OAUTH_KEYS that comes from the
        // OAuth feature must also appear in ALL_OAUTH_KEYS. TOKEN is
        // intentionally not part of ALL_OAUTH_KEYS because it is the
        // pre-existing legacy-OAuth key shared with the PAT/legacy
        // authenticator paths.
        for &k in SENSITIVE_OAUTH_KEYS {
            if k != TOKEN {
                assert!(
                    ALL_OAUTH_KEYS.contains(&k),
                    "sensitive OAuth key {k} should be in ALL_OAUTH_KEYS"
                );
            }
        }
    }
}
