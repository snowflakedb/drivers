//! OAuth DSN keys and helpers used by the ODBC wrapper.
//!
//! This module is the single source of truth for the OAuth-related
//! ODBC connection-string / DSN keys that the wrapper recognises.  The
//! actual OAuth flow lives in `sf_core::rest::snowflake::oauth`; this
//! module only declares the wire-level identifiers (the SCREAMING_SNAKE
//! ODBC keys) and the corresponding `sf_core` canonical names so the
//! wrapper can:
//!
//! * forward connection-string keys consistently
//!   ([`canonical_name`], consumed by `connection::normalize_connection_string_option`),
//! * never persist OAuth secrets to the DSN registry
//!   ([`should_persist_to_dsn`], consumed by `setup_common::write_dsn_values`),
//! * redact OAuth secrets from logs
//!   ([`redacted_param_map`], consumed by `connection::connect_with_params`).
//!
//! The wrapper composes its connection-string-level redaction list
//! ([`SENSITIVE_LOGGING_KEYS`]) from both the legacy PWD-style secrets
//! and the OAuth secret list so [`redacted_param_map`] is the single
//! place to update when a new sensitive key joins either family.
//!
//! The key list mirrors the cross-driver configuration matrix and
//! matches the `param_registry` aliases in
//! `sf_core::config::param_registry`. All ODBC keys here are the
//! `JDBC/ODBC` SCREAMING_SNAKE form and resolve via `sf_core` to the
//! lowercase canonical name shown in the doc comment for each constant.
//!
//! TODO(SNOW-3552555): consolidate the redaction / canonical-name
//! policy into `sf_core::config::param_registry`. The registry already
//! models every OAuth `ParamDef` with `sensitive: bool` and the ODBC
//! SCREAMING_SNAKE form as an alias; exposing
//! `param_registry::is_sensitive(key)` and
//! `param_registry::canonical_name(key)` lookups would let
//! [`OAUTH_CANONICAL_NAMES`] and the OAuth half of
//! [`SENSITIVE_LOGGING_KEYS`] disappear. The legacy PWD-family entries
//! must keep living here because they are aliases the wrapper redacts
//! BEFORE `sf_core` normalization happens on the raw connection-string
//! map. [`ALL_OAUTH_KEYS`] would stay too (it is consumed by the ODBC
//! SQLSTATE classifier in `api::error`).

/// `OAUTH_CLIENT_ID` (canonical: `oauth_client_id`).
///
/// External-IdP client ID. When absent and the IdP is Snowflake, the
/// flow substitutes the literal `LOCAL_APPLICATION` (Snowflake-as-IdP default).
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
/// not mint client-credentials tokens) and also used to derive the
/// cache-key host (prefers IdP token URL host, falls back to Snowflake host).
pub const OAUTH_TOKEN_REQUEST_URL: &str = "OAUTH_TOKEN_REQUEST_URL";

/// `OAUTH_REDIRECT_URI` (canonical: `oauth_redirect_uri`).
///
/// Loopback redirect URI for the AC flow. Defaults to
/// `http://127.0.0.1:<random>` when omitted (always binds loopback only).
pub const OAUTH_REDIRECT_URI: &str = "OAUTH_REDIRECT_URI";

/// `OAUTH_SCOPE` (canonical: `oauth_scope`).
///
/// Space-separated OAuth scope list. Defaults to
/// `session:role:<role>` when omitted.
pub const OAUTH_SCOPE: &str = "OAUTH_SCOPE";

/// `OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS` (canonical:
/// `oauth_enable_single_use_refresh_tokens`).
///
/// When `true` and Snowflake is the IdP, the AC flow asks for a
/// rotating single-use refresh token (Snowflake-IdP only).
pub const OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS: &str = "OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS";

/// `OAUTH_DISABLE_PKCE` (canonical: `oauth_disable_pkce`).
///
/// Python parity escape hatch. Defaults to `false`. When `true`, the
/// AC flow omits `code_challenge` / `code_verifier`.
pub const OAUTH_DISABLE_PKCE: &str = "OAUTH_DISABLE_PKCE";

/// `OAUTH_ENABLE_DPOP` (canonical: `oauth_enable_dpop`).
///
/// Opt-in to RFC 9449 DPoP (JDBC parity). Defaults to
/// `false`.
pub const OAUTH_ENABLE_DPOP: &str = "OAUTH_ENABLE_DPOP";

/// `OAUTH_DISABLE_CONSOLE_LOGIN` (canonical:
/// `oauth_disable_console_login`).
///
/// Carried for parity with JDBC `DISABLE_CONSOLE_LOGIN`; **does not**
/// affect OAuth flows — only the legacy SAML
/// EXTERNALBROWSER `console_login` form.
pub const OAUTH_DISABLE_CONSOLE_LOGIN: &str = "OAUTH_DISABLE_CONSOLE_LOGIN";

/// `TOKEN` (canonical: `token`).
///
/// Pre-acquired access token used by the legacy
/// `AUTHENTICATOR=OAUTH` mode (pre-acquired access token). **Sensitive** —
/// listed in [`SENSITIVE_LOGGING_KEYS`] so [`redacted_param_map`]
/// hides it from `tracing` sinks, and rejected by
/// [`should_persist_to_dsn`] so the Windows DSN-write path skips it.
pub const TOKEN: &str = "TOKEN";

/// All ODBC DSN/connection-string keys defined by the OAuth feature.
/// Consumed by [`api::error`](crate::api::error) to extend the
/// SQLSTATE-`28000` classifier with every OAuth parameter so missing
/// or invalid OAuth keys map to an auth-class SQLSTATE instead of the
/// generic `HY000`. Also serves as the iteration list for the
/// canonical-name guard tests.
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

// TODO(SNOW-3552555): replace this table with a call to
// `sf_core::config::param_registry::canonical_name(key)` once that
// lookup is exposed. Every entry below is already an ODBC alias on
// the matching `ParamDef` in `sf_core/src/config/param_registry.rs`,
// so the table is pure duplication kept here only because the
// registry does not yet expose an alias-keyed canonical-name API.
/// Lookup table mapping ODBC OAuth keys to their `sf_core` canonical
/// (lowercase) name used by `param_registry`. The table is intentionally
/// the identity map (lowercase form of the same key) for every entry —
/// the wrapper exposes this function so callers don't have to re-derive
/// it from each key, and so a future divergence between the ODBC key
/// and the `sf_core` canonical name has a single place to land.
const OAUTH_CANONICAL_NAMES: &[(&str, &str)] = &[
    (OAUTH_CLIENT_ID, "oauth_client_id"),
    (OAUTH_CLIENT_SECRET, "oauth_client_secret"),
    (OAUTH_AUTHORIZATION_URL, "oauth_authorization_url"),
    (OAUTH_TOKEN_REQUEST_URL, "oauth_token_request_url"),
    (OAUTH_REDIRECT_URI, "oauth_redirect_uri"),
    (OAUTH_SCOPE, "oauth_scope"),
    (
        OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS,
        "oauth_enable_single_use_refresh_tokens",
    ),
    (OAUTH_DISABLE_PKCE, "oauth_disable_pkce"),
    (OAUTH_ENABLE_DPOP, "oauth_enable_dpop"),
    (OAUTH_DISABLE_CONSOLE_LOGIN, "oauth_disable_console_login"),
];

/// Returns the `sf_core` canonical (lowercase) parameter name for a
/// known ODBC OAuth key, or `None` when `key` is not an OAuth key.
///
/// Case-insensitive in the input. Used by the connection-string
/// normalizer to forward OAuth keys with explicit canonical names —
/// avoiding any reliance on `sf_core`'s alias-resolution fallback for
/// the OAuth surface, which keeps this layer self-documenting.
pub fn canonical_name(key: &str) -> Option<&'static str> {
    OAUTH_CANONICAL_NAMES
        .iter()
        .find(|(odbc_key, _)| key.eq_ignore_ascii_case(odbc_key))
        .map(|(_, canonical)| *canonical)
}

/// Returns `false` when `key` is an OAuth secret that must never
/// reach the on-disk DSN registry (secrets must not be persisted). Returns
/// `true` for all non-secret OAuth keys and for any non-OAuth key
/// (callers compose this with their own DSN-skip rules for `PWD`,
/// `DSN`, etc.). Case-insensitive in `key`.
#[allow(dead_code)] // consumed by setup_common.rs (windows-only DSN write path)
pub fn should_persist_to_dsn(key: &str) -> bool {
    // TODO(SNOW-3552555): replace with
    // `!sf_core::config::param_registry::is_sensitive(key)` once that
    // predicate is exposed. The two OAuth secrets enumerated below
    // already carry `sensitive: true` in their `ParamDef`.
    !key.eq_ignore_ascii_case(OAUTH_CLIENT_SECRET) && !key.eq_ignore_ascii_case(TOKEN)
}

// TODO(SNOW-3552555): collapse the OAuth half of this list into a
// `sf_core::config::param_registry::sensitive_aliases()` iterator
// once the registry exposes sensitivity by alias. The legacy
// PWD-family entries must keep living here because they are matched
// against the raw ODBC connection-string map BEFORE `sf_core`
// normalization — at that boundary the keys are still the
// SCREAMING_SNAKE wrapper aliases (`PWD`, `PRIV_KEY_FILE_PWD`, …),
// not the lowercase canonical names that `sf_core` would key on.
/// Combined list of connection-string keys that must be redacted at
/// the wrapper's logging boundary (`connect_with_params`). It joins
/// the legacy PWD-style secrets recognised by the ODBC layer with the
/// OAuth secret list, so adding a new sensitive OAuth key here
/// automatically flows through to log redaction without touching
/// `connection.rs`.
pub const SENSITIVE_LOGGING_KEYS: &[&str] = &[
    // Pre-OAuth keys: kept here verbatim so the wrapper has a single
    // grep target for "things that must never appear in logs".
    "PWD",
    "PRIV_KEY_FILE_PWD",
    "PRIV_KEY_PWD",
    "PRIV_KEY_BASE64",
    "PASSCODE",
    // Proxy URL may contain credentials (user:pass@host:port).
    "PROXY",
    // OAuth keys (OAUTH_CLIENT_SECRET + TOKEN). TOKEN was already in
    // the legacy redaction list; routing it through the OAuth list
    // keeps the source of truth in one place.
    OAUTH_CLIENT_SECRET,
    TOKEN,
];

/// Returns a borrowed view of `params` with the value of every
/// sensitive key (OAuth secrets + legacy PWD-style secrets) replaced
/// by `"****"`. Non-sensitive entries borrow their value from
/// `params`; redacted entries use the static `"****"` placeholder, so
/// no allocation is performed in either branch.
///
/// Use this at every connection-string logging boundary so OAuth
/// client secrets and access tokens never reach `tracing` sinks.
/// Centralising the policy here means future additions to
/// [`SENSITIVE_LOGGING_KEYS`] update every call site automatically.
pub fn redacted_param_map(
    params: &std::collections::HashMap<String, String>,
) -> std::collections::HashMap<&String, std::borrow::Cow<'_, str>> {
    params
        .iter()
        .map(|(k, v)| {
            let is_sensitive = SENSITIVE_LOGGING_KEYS
                .iter()
                .any(|r| k.eq_ignore_ascii_case(r));
            let value: std::borrow::Cow<'_, str> = if is_sensitive {
                std::borrow::Cow::Borrowed("****")
            } else {
                std::borrow::Cow::Borrowed(v.as_str())
            };
            (k, value)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_oauth_keys_includes_secret() {
        assert!(ALL_OAUTH_KEYS.contains(&OAUTH_CLIENT_SECRET));
    }

    #[test]
    fn canonical_name_maps_known_keys_case_insensitively() {
        assert_eq!(canonical_name("OAUTH_CLIENT_ID"), Some("oauth_client_id"));
        assert_eq!(canonical_name("oauth_client_id"), Some("oauth_client_id"));
        assert_eq!(
            canonical_name("OAuth_Client_Secret"),
            Some("oauth_client_secret")
        );
        assert_eq!(
            canonical_name("OAUTH_REDIRECT_URI"),
            Some("oauth_redirect_uri")
        );
        assert_eq!(
            canonical_name("OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS"),
            Some("oauth_enable_single_use_refresh_tokens")
        );
        assert_eq!(canonical_name("UID"), None);
        assert_eq!(canonical_name(""), None);
    }

    #[test]
    fn canonical_name_covers_every_all_oauth_keys_entry() {
        // Belt-and-braces: any OAuth key in ALL_OAUTH_KEYS must have
        // a canonical name; otherwise the wrapper would silently drop
        // it through the connection-string passthrough fallback.
        for &k in ALL_OAUTH_KEYS {
            assert!(
                canonical_name(k).is_some(),
                "missing canonical name for {k}"
            );
        }
    }

    #[test]
    fn should_persist_to_dsn_returns_false_only_for_secrets() {
        assert!(!should_persist_to_dsn("OAUTH_CLIENT_SECRET"));
        assert!(!should_persist_to_dsn("oauth_client_secret"));
        assert!(!should_persist_to_dsn("TOKEN"));
        assert!(should_persist_to_dsn("OAUTH_CLIENT_ID"));
        assert!(should_persist_to_dsn("OAUTH_REDIRECT_URI"));
        assert!(should_persist_to_dsn("OAUTH_SCOPE"));
        assert!(should_persist_to_dsn("UID"));
    }

    #[test]
    fn redacted_param_map_redacts_legacy_and_oauth_secrets() {
        use std::collections::HashMap;
        let params: HashMap<String, String> = [
            ("UID", "joe"),
            ("PWD", "hunter2"),
            ("PRIV_KEY_FILE_PWD", "kpwd"),
            ("PRIV_KEY_PWD", "kpwd2"),
            ("PRIV_KEY_BASE64", "AAA="),
            ("PASSCODE", "123456"),
            ("OAUTH_CLIENT_ID", "abc"),
            ("OAUTH_CLIENT_SECRET", "shhh"),
            ("TOKEN", "jwt.value"),
            ("PROXY", "user:pass@proxy:8080"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();

        let redacted = redacted_param_map(&params);

        assert_eq!(
            redacted.get(&"UID".to_owned()).map(|v| v.as_ref()),
            Some("joe")
        );
        assert_eq!(
            redacted
                .get(&"OAUTH_CLIENT_ID".to_owned())
                .map(|v| v.as_ref()),
            Some("abc")
        );
        for sensitive in SENSITIVE_LOGGING_KEYS {
            let key = sensitive.to_string();
            assert_eq!(
                redacted.get(&key).map(|v| v.as_ref()),
                Some("****"),
                "expected key {sensitive} to be redacted"
            );
        }
    }

    #[test]
    fn redacted_param_map_redaction_is_case_insensitive() {
        use std::collections::HashMap;
        let params: HashMap<String, String> = [
            ("oauth_client_secret", "shhh"),
            ("Pwd", "hunter2"),
            ("token", "jwt.value"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();

        let redacted = redacted_param_map(&params);
        for k in params.keys() {
            assert_eq!(
                redacted.get(k).map(|v| v.as_ref()),
                Some("****"),
                "expected key {k} to be redacted regardless of case"
            );
        }
    }

    /// Documentation guard: every key in `ALL_OAUTH_KEYS` has a
    /// canonical name AND its [`should_persist_to_dsn`] verdict
    /// agrees with its `OAUTH_CLIENT_SECRET`/`TOKEN` sensitivity.
    /// This is the single invariant the wrapper relies on to safely
    /// round-trip a DSN.
    #[test]
    fn all_oauth_keys_round_trip_through_every_helper() {
        for &k in ALL_OAUTH_KEYS {
            assert!(canonical_name(k).is_some(), "{k} has no canonical name");
            let sensitive = k.eq_ignore_ascii_case(OAUTH_CLIENT_SECRET);
            assert_eq!(
                should_persist_to_dsn(k),
                !sensitive,
                "{k}: DSN persistence policy disagrees with sensitivity"
            );
        }
    }

    /// Documents the DSN-write contract enforced by
    /// `setup_common::write_dsn_values`: every OAuth secret
    /// (`OAUTH_CLIENT_SECRET`, `TOKEN`) is rejected, every other
    /// OAuth key is persisted, and the result is independent of the
    /// caller's letter-casing.
    #[test]
    fn dsn_persistence_policy_for_oauth_keys_is_case_insensitive() {
        for &k in ALL_OAUTH_KEYS {
            for variant in [
                k.to_owned(),
                k.to_lowercase(),
                k.chars()
                    .enumerate()
                    .map(|(i, c)| {
                        if i.is_multiple_of(2) {
                            c.to_ascii_lowercase()
                        } else {
                            c.to_ascii_uppercase()
                        }
                    })
                    .collect::<String>(),
            ] {
                let sensitive = k.eq_ignore_ascii_case(OAUTH_CLIENT_SECRET);
                assert_eq!(
                    should_persist_to_dsn(&variant),
                    !sensitive,
                    "{variant:?} (variant of {k}) DSN persistence drifted"
                );
            }
        }
        // `TOKEN` is not in `ALL_OAUTH_KEYS` (it predates the OAuth
        // feature; see `TOKEN` const docs), so cover it explicitly.
        for variant in ["TOKEN", "token", "Token"] {
            assert!(
                !should_persist_to_dsn(variant),
                "{variant} must never be persisted to the DSN registry"
            );
        }
    }

    /// `SENSITIVE_LOGGING_KEYS` is the single authoritative list of
    /// connection-string keys that must never reach `tracing` sinks.
    /// Verify both that every OAuth secret is in the list AND that
    /// the legacy PWD-style keys are still covered after the
    /// refactor — guards against accidental policy regression when a
    /// future contributor edits `SENSITIVE_LOGGING_KEYS`.
    #[test]
    fn sensitive_logging_keys_covers_oauth_secrets_and_legacy_pwd_family() {
        for legacy in [
            "PWD",
            "PRIV_KEY_FILE_PWD",
            "PRIV_KEY_PWD",
            "PRIV_KEY_BASE64",
            "PASSCODE",
        ] {
            assert!(
                SENSITIVE_LOGGING_KEYS
                    .iter()
                    .any(|k| k.eq_ignore_ascii_case(legacy)),
                "legacy sensitive key {legacy} dropped from SENSITIVE_LOGGING_KEYS"
            );
        }
        for oauth_secret in [OAUTH_CLIENT_SECRET, TOKEN] {
            assert!(
                SENSITIVE_LOGGING_KEYS.contains(&oauth_secret),
                "OAuth secret {oauth_secret} not in SENSITIVE_LOGGING_KEYS"
            );
        }
    }
}
