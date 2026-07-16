//! OAuth token cache I/O and refresh-token rotation.
//!
//! Cache keys are versioned, uniformly-hashed [`CacheKey`] values built from
//! `(idp_url, snowflake_url, username, role, token_type)`:
//! * `idp_url` — the IdP token-endpoint URL (e.g. the `token_url` config field).
//! * `snowflake_url` — the Snowflake server URL (always the account endpoint).
//!
//! Eviction on Snowflake error codes `390303` / `390318` is required across all
//! drivers.
//!
//! These helpers mirror the MFA-token helpers in
//! `sf_core::rest::snowflake::mod` so the call sites in the OAuth flow
//! orchestrators read symmetrically.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STD;
#[cfg(any(test, feature = "test-utils"))]
use url::Url;

use crate::sensitive::SensitiveString;
use crate::token_cache::{CacheKey, TokenCache, TokenType, normalize_identifier, normalize_url};

/// Resolve the cache-key host for OAuth tokens.
///
/// Python convention (`urllib.parse.urlparse(token_request_url).hostname`):
/// use the IdP token endpoint host when present,
/// otherwise fall back to the Snowflake server host.
///
/// Kept for test-utility use (re-exported at
/// `sf_core::rest::snowflake::host_from_token_url`). Production cache-key
/// construction uses [`CacheKey`] directly via the helpers in this module,
/// which accept explicit `idp_url` and `snowflake_url` parameters.
#[cfg(any(test, feature = "test-utils"))]
pub fn host_from_token_url(token_request_url: &str, fallback_server_url: &str) -> Option<String> {
    host_from_token_url_inner(token_request_url, fallback_server_url)
}

#[cfg(any(test, feature = "test-utils"))]
fn host_from_token_url_inner(token_request_url: &str, fallback_server_url: &str) -> Option<String> {
    if let Some(host) = Url::parse(token_request_url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        && !host.is_empty()
    {
        return Some(host);
    }
    Url::parse(fallback_server_url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
}

// ─── Access token ────────────────────────────────────────────────────────

pub(crate) fn try_get_cached_oauth_access_token(
    idp_url: &str,
    snowflake_url: &str,
    username: &str,
    role: &str,
    token_cache: Option<&dyn TokenCache>,
) -> Option<SensitiveString> {
    let cache = token_cache?;
    let key = CacheKey {
        token_type: TokenType::OAuthAccessToken,
        idp: normalize_url(idp_url),
        snowflake: normalize_url(snowflake_url),
        username: normalize_identifier(username),
        role: normalize_identifier(role),
    };
    match cache.get_token(&key) {
        Ok(Some(token)) if !token.is_empty() => {
            tracing::info!("Found cached OAuth access token");
            Some(token.into())
        }
        Ok(_) => None,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to retrieve cached OAuth access token");
            None
        }
    }
}

pub(crate) fn store_oauth_access_token(
    idp_url: &str,
    snowflake_url: &str,
    username: &str,
    role: &str,
    access_token: &str,
    token_cache: Option<&dyn TokenCache>,
) {
    let Some(cache) = token_cache else {
        tracing::debug!("No token cache available for OAuth access token storage");
        return;
    };
    let key = CacheKey {
        token_type: TokenType::OAuthAccessToken,
        idp: normalize_url(idp_url),
        snowflake: normalize_url(snowflake_url),
        username: normalize_identifier(username),
        role: normalize_identifier(role),
    };
    if let Err(e) = cache.add_token(&key, access_token) {
        tracing::warn!(error = %e, "Failed to cache OAuth access token");
    } else {
        tracing::info!("Cached OAuth access token for future use");
    }
}

pub(crate) fn remove_oauth_access_token(
    idp_url: &str,
    snowflake_url: &str,
    username: &str,
    role: &str,
    token_cache: Option<&dyn TokenCache>,
) {
    let Some(cache) = token_cache else {
        return;
    };
    let key = CacheKey {
        token_type: TokenType::OAuthAccessToken,
        idp: normalize_url(idp_url),
        snowflake: normalize_url(snowflake_url),
        username: normalize_identifier(username),
        role: normalize_identifier(role),
    };
    if let Err(e) = cache.remove_token(&key) {
        tracing::warn!(error = %e, "Failed to remove cached OAuth access token");
    } else {
        tracing::info!("Removed cached OAuth access token");
    }
}

// ─── Refresh token ───────────────────────────────────────────────────────

pub(crate) fn try_get_cached_oauth_refresh_token(
    idp_url: &str,
    snowflake_url: &str,
    username: &str,
    role: &str,
    token_cache: Option<&dyn TokenCache>,
) -> Option<SensitiveString> {
    let cache = token_cache?;
    let key = CacheKey {
        token_type: TokenType::OAuthRefreshToken,
        idp: normalize_url(idp_url),
        snowflake: normalize_url(snowflake_url),
        username: normalize_identifier(username),
        role: normalize_identifier(role),
    };
    match cache.get_token(&key) {
        Ok(Some(token)) if !token.is_empty() => {
            tracing::info!("Found cached OAuth refresh token");
            Some(token.into())
        }
        Ok(_) => None,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to retrieve cached OAuth refresh token");
            None
        }
    }
}

pub(crate) fn store_oauth_refresh_token(
    idp_url: &str,
    snowflake_url: &str,
    username: &str,
    role: &str,
    refresh_token: &str,
    token_cache: Option<&dyn TokenCache>,
) {
    let Some(cache) = token_cache else {
        tracing::debug!("No token cache available for OAuth refresh token storage");
        return;
    };
    let key = CacheKey {
        token_type: TokenType::OAuthRefreshToken,
        idp: normalize_url(idp_url),
        snowflake: normalize_url(snowflake_url),
        username: normalize_identifier(username),
        role: normalize_identifier(role),
    };
    if let Err(e) = cache.add_token(&key, refresh_token) {
        tracing::warn!(error = %e, "Failed to cache OAuth refresh token");
    } else {
        tracing::info!("Cached OAuth refresh token for future use");
    }
}

pub(crate) fn remove_oauth_refresh_token(
    idp_url: &str,
    snowflake_url: &str,
    username: &str,
    role: &str,
    token_cache: Option<&dyn TokenCache>,
) {
    let Some(cache) = token_cache else {
        return;
    };
    let key = CacheKey {
        token_type: TokenType::OAuthRefreshToken,
        idp: normalize_url(idp_url),
        snowflake: normalize_url(snowflake_url),
        username: normalize_identifier(username),
        role: normalize_identifier(role),
    };
    if let Err(e) = cache.remove_token(&key) {
        tracing::warn!(error = %e, "Failed to remove cached OAuth refresh token");
    } else {
        tracing::info!("Removed cached OAuth refresh token");
    }
}

// ─── DPoP-bundled access token ───────────────────────────────────────────

/// Pack `(access_token, jwk_json)` into the JDBC bundled-cache format
/// `<base64(access_token)>.<base64(jwk_json)>`. Standard base64 (with
/// `+`/`/`/`=`) is used to match `CredentialManager.java:294-310`.
pub(crate) fn pack_dpop_bundle(access_token: &str, jwk_json: &str) -> String {
    let at_b64 = BASE64_STD.encode(access_token.as_bytes());
    let jwk_b64 = BASE64_STD.encode(jwk_json.as_bytes());
    format!("{at_b64}.{jwk_b64}")
}

/// Inverse of [`pack_dpop_bundle`]. Returns `None` if the format is not
/// recognized so callers can evict the corrupt entry and start over
/// (mirrors JDBC's "legacy non-base64 encoded cache values" branch).
pub(crate) fn unpack_dpop_bundle(packed: &str) -> Option<(String, String)> {
    let (at_b64, jwk_b64) = packed.split_once('.')?;
    let at = BASE64_STD.decode(at_b64.as_bytes()).ok()?;
    let jwk = BASE64_STD.decode(jwk_b64.as_bytes()).ok()?;
    Some((String::from_utf8(at).ok()?, String::from_utf8(jwk).ok()?))
}

pub(crate) fn try_get_cached_oauth_dpop_bundled(
    idp_url: &str,
    snowflake_url: &str,
    username: &str,
    role: &str,
    token_cache: Option<&dyn TokenCache>,
) -> Option<(SensitiveString, String)> {
    let cache = token_cache?;
    let key = CacheKey {
        token_type: TokenType::DpopBundledAccessToken,
        idp: normalize_url(idp_url),
        snowflake: normalize_url(snowflake_url),
        username: normalize_identifier(username),
        role: normalize_identifier(role),
    };
    match cache.get_token(&key) {
        Ok(Some(packed)) if !packed.is_empty() => match unpack_dpop_bundle(&packed) {
            Some((access_token, jwk_json)) => {
                tracing::info!("Found cached DPoP-bundled OAuth access token");
                Some((SensitiveString::from(access_token), jwk_json))
            }
            None => {
                tracing::warn!("Cached DPoP-bundled access token has unexpected format; evicting");
                remove_oauth_dpop_bundled(idp_url, snowflake_url, username, role, token_cache);
                None
            }
        },
        Ok(_) => None,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to retrieve cached DPoP-bundled access token");
            None
        }
    }
}

pub(crate) fn store_oauth_dpop_bundled(
    idp_url: &str,
    snowflake_url: &str,
    username: &str,
    role: &str,
    access_token: &str,
    jwk_json: &str,
    token_cache: Option<&dyn TokenCache>,
) {
    let Some(cache) = token_cache else {
        tracing::debug!("No token cache available for DPoP-bundled access token storage");
        return;
    };
    let packed = pack_dpop_bundle(access_token, jwk_json);
    let key = CacheKey {
        token_type: TokenType::DpopBundledAccessToken,
        idp: normalize_url(idp_url),
        snowflake: normalize_url(snowflake_url),
        username: normalize_identifier(username),
        role: normalize_identifier(role),
    };
    if let Err(e) = cache.add_token(&key, &packed) {
        tracing::warn!(error = %e, "Failed to cache DPoP-bundled access token");
    } else {
        tracing::info!("Cached DPoP-bundled OAuth access token for future use");
    }
}

pub(crate) fn remove_oauth_dpop_bundled(
    idp_url: &str,
    snowflake_url: &str,
    username: &str,
    role: &str,
    token_cache: Option<&dyn TokenCache>,
) {
    let Some(cache) = token_cache else {
        return;
    };
    let key = CacheKey {
        token_type: TokenType::DpopBundledAccessToken,
        idp: normalize_url(idp_url),
        snowflake: normalize_url(snowflake_url),
        username: normalize_identifier(username),
        role: normalize_identifier(role),
    };
    if let Err(e) = cache.remove_token(&key) {
        tracing::warn!(error = %e, "Failed to remove cached DPoP-bundled access token");
    } else {
        tracing::info!("Removed cached DPoP-bundled OAuth access token");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_cache::{CacheKey, TokenCacheError, build_cache_key};
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct StubTokenCache {
        store: Mutex<HashMap<String, String>>,
    }

    impl StubTokenCache {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
            }
        }
    }

    impl TokenCache for StubTokenCache {
        fn add_token(&self, key: &CacheKey, token_value: &str) -> Result<(), TokenCacheError> {
            self.store
                .lock()
                .unwrap()
                .insert(build_cache_key(key), token_value.to_string());
            Ok(())
        }

        fn remove_token(&self, key: &CacheKey) -> Result<(), TokenCacheError> {
            self.store.lock().unwrap().remove(&build_cache_key(key));
            Ok(())
        }

        fn get_token(&self, key: &CacheKey) -> Result<Option<String>, TokenCacheError> {
            Ok(self
                .store
                .lock()
                .unwrap()
                .get(&build_cache_key(key))
                .cloned())
        }
    }

    const IDP_URL: &str = "https://idp.example.com/token";
    const SNOWFLAKE_URL: &str = "https://acct.snowflakecomputing.com";

    #[test]
    fn host_from_token_url_prefers_token_url() {
        let host = host_from_token_url(
            "https://idp.example.com/oauth/token",
            "https://acct.snowflakecomputing.com",
        );
        assert_eq!(host.as_deref(), Some("idp.example.com"));
    }

    #[test]
    fn host_from_token_url_falls_back_to_server_url() {
        let host = host_from_token_url("not-a-url", "https://acct.snowflakecomputing.com");
        assert_eq!(host.as_deref(), Some("acct.snowflakecomputing.com"));
    }

    #[test]
    fn host_from_token_url_returns_none_when_neither_parses() {
        let host = host_from_token_url("not-a-url", "also-not-a-url");
        assert!(host.is_none());
    }

    #[test]
    fn access_token_round_trip() {
        let cache = StubTokenCache::new();
        store_oauth_access_token(IDP_URL, SNOWFLAKE_URL, "alice", "", "AAA", Some(&cache));
        let got =
            try_get_cached_oauth_access_token(IDP_URL, SNOWFLAKE_URL, "alice", "", Some(&cache));
        assert_eq!(got.as_ref().map(|s| s.reveal().as_str()), Some("AAA"));

        remove_oauth_access_token(IDP_URL, SNOWFLAKE_URL, "alice", "", Some(&cache));
        assert!(
            try_get_cached_oauth_access_token(IDP_URL, SNOWFLAKE_URL, "alice", "", Some(&cache))
                .is_none()
        );
    }

    #[test]
    fn refresh_token_round_trip() {
        let cache = StubTokenCache::new();
        store_oauth_refresh_token(IDP_URL, SNOWFLAKE_URL, "alice", "", "RRR", Some(&cache));
        let got =
            try_get_cached_oauth_refresh_token(IDP_URL, SNOWFLAKE_URL, "alice", "", Some(&cache));
        assert_eq!(got.as_ref().map(|s| s.reveal().as_str()), Some("RRR"));

        remove_oauth_refresh_token(IDP_URL, SNOWFLAKE_URL, "alice", "", Some(&cache));
        assert!(
            try_get_cached_oauth_refresh_token(IDP_URL, SNOWFLAKE_URL, "alice", "", Some(&cache))
                .is_none()
        );
    }

    #[test]
    fn pack_unpack_dpop_bundle_round_trips() {
        let packed = pack_dpop_bundle("access.tok.123", r#"{"kty":"EC","crv":"P-256"}"#);
        let (at, jwk) = unpack_dpop_bundle(&packed).expect("unpack");
        assert_eq!(at, "access.tok.123");
        assert_eq!(jwk, r#"{"kty":"EC","crv":"P-256"}"#);
    }

    #[test]
    fn unpack_dpop_bundle_rejects_corrupt_input() {
        assert!(unpack_dpop_bundle("not-a-bundle").is_none());
        assert!(unpack_dpop_bundle("zz!.zz!").is_none());
    }

    #[test]
    fn dpop_bundle_round_trip_through_cache() {
        let cache = StubTokenCache::new();
        store_oauth_dpop_bundled(
            IDP_URL,
            SNOWFLAKE_URL,
            "alice",
            "",
            "ACCESS-TOK",
            r#"{"crv":"P-256","kty":"EC"}"#,
            Some(&cache),
        );
        let got =
            try_get_cached_oauth_dpop_bundled(IDP_URL, SNOWFLAKE_URL, "alice", "", Some(&cache))
                .expect("hit");
        assert_eq!(got.0.reveal().as_str(), "ACCESS-TOK");
        assert_eq!(got.1, r#"{"crv":"P-256","kty":"EC"}"#);
    }

    #[test]
    fn dpop_bundle_corrupt_entry_is_evicted() {
        let cache = StubTokenCache::new();
        // Insert a corrupt entry using the same key that try_get_cached_oauth_dpop_bundled
        // will look up (normalize_url + normalize_identifier applied to the same inputs).
        let key = CacheKey {
            token_type: TokenType::DpopBundledAccessToken,
            idp: normalize_url(IDP_URL),
            snowflake: normalize_url(SNOWFLAKE_URL),
            username: normalize_identifier("alice"),
            role: String::new(),
        };
        cache.add_token(&key, "totally-not-a-bundle").unwrap();

        let got =
            try_get_cached_oauth_dpop_bundled(IDP_URL, SNOWFLAKE_URL, "alice", "", Some(&cache));
        assert!(got.is_none(), "corrupt bundle should not be returned");

        // …and should have been evicted as a side effect.
        let stored = cache.get_token(&key).unwrap();
        assert!(stored.is_none(), "corrupt entry should have been evicted");
    }

    #[test]
    fn empty_cache_value_returns_none_not_empty_string() {
        let cache = StubTokenCache::new();
        let key = CacheKey {
            token_type: TokenType::OAuthAccessToken,
            idp: normalize_url(IDP_URL),
            snowflake: normalize_url(SNOWFLAKE_URL),
            username: normalize_identifier("alice"),
            role: String::new(),
        };
        cache.add_token(&key, "").unwrap();

        let got =
            try_get_cached_oauth_access_token(IDP_URL, SNOWFLAKE_URL, "alice", "", Some(&cache));
        assert!(got.is_none());
    }

    #[test]
    fn different_snowflake_accounts_sharing_one_idp_do_not_collide() {
        let cache = StubTokenCache::new();
        let idp = "https://idp.shared.com/oauth/token";
        let sf1 = "https://org-account1.snowflakecomputing.com";
        let sf2 = "https://org-account2.snowflakecomputing.com";

        store_oauth_access_token(idp, sf1, "alice", "", "AT-FOR-SF1", Some(&cache));
        store_oauth_access_token(idp, sf2, "alice", "", "AT-FOR-SF2", Some(&cache));

        let got1 = try_get_cached_oauth_access_token(idp, sf1, "alice", "", Some(&cache));
        let got2 = try_get_cached_oauth_access_token(idp, sf2, "alice", "", Some(&cache));
        assert_eq!(
            got1.as_ref().map(|s| s.reveal().as_str()),
            Some("AT-FOR-SF1")
        );
        assert_eq!(
            got2.as_ref().map(|s| s.reveal().as_str()),
            Some("AT-FOR-SF2")
        );
    }

    #[test]
    fn distinct_roles_get_distinct_entries() {
        let cache = StubTokenCache::new();

        store_oauth_access_token(
            IDP_URL,
            SNOWFLAKE_URL,
            "alice",
            "ANALYST",
            "AT-ANALYST",
            Some(&cache),
        );
        store_oauth_access_token(
            IDP_URL,
            SNOWFLAKE_URL,
            "alice",
            "ADMIN",
            "AT-ADMIN",
            Some(&cache),
        );

        let got_analyst = try_get_cached_oauth_access_token(
            IDP_URL,
            SNOWFLAKE_URL,
            "alice",
            "ANALYST",
            Some(&cache),
        );
        let got_admin = try_get_cached_oauth_access_token(
            IDP_URL,
            SNOWFLAKE_URL,
            "alice",
            "ADMIN",
            Some(&cache),
        );
        assert_eq!(
            got_analyst.as_ref().map(|s| s.reveal().as_str()),
            Some("AT-ANALYST")
        );
        assert_eq!(
            got_admin.as_ref().map(|s| s.reveal().as_str()),
            Some("AT-ADMIN")
        );
    }

    // ─── host_from_token_url edge cases ──────────────────────────────────
    // `host_from_token_url` is a test-utility helper kept for e2e tests that
    // need to derive a cache-key host from a token URL without reimplementing
    // the Python-style `urlparse(token_request_url).hostname` fallback chain.
    // Production cache-key construction uses `normalize_url` + `CacheKey`
    // directly and no longer calls this function.
    //
    // The parameter set below targets URL shapes that have historically tripped
    // up token-cache key derivation in other drivers (JDBC/Python/.NET use the
    // parsed hostname; Go diverges with the full URL string).

    #[test]
    fn host_from_token_url_treats_url_with_empty_host_as_no_host() {
        // The url crate normalizes `https:///path` to `https://path/`,
        // so to truly exercise the empty-host fallback branch we lean
        // on `data:` URLs (which have no host segment at all). The
        // cross-driver `host_from_token_url` contract: fall back to the
        // Snowflake server URL host whenever the primary URL exposes no
        // usable host.
        let host = host_from_token_url("data:,", "https://acct.example.com");
        assert_eq!(host.as_deref(), Some("acct.example.com"));
    }

    #[test]
    fn host_from_token_url_falls_back_for_file_scheme_url() {
        // `file:///etc/passwd` is a valid URL but has no network host.
        // `Url::host_str` returns `None`, so we should fall back.
        let host = host_from_token_url("file:///etc/passwd", "https://acct.example.com");
        assert_eq!(host.as_deref(), Some("acct.example.com"));
    }

    #[test]
    fn host_from_token_url_falls_back_for_opaque_scheme() {
        // Opaque URLs like `mailto:` and `data:` parse but expose no
        // host either. Same fallback expectation.
        let host = host_from_token_url("mailto:ops@example.com", "https://acct.example.com");
        assert_eq!(host.as_deref(), Some("acct.example.com"));
        let host = host_from_token_url("data:text/plain,hello%20world", "https://acct.example.com");
        assert_eq!(host.as_deref(), Some("acct.example.com"));
    }

    #[test]
    fn host_from_token_url_returns_none_when_both_lack_a_host() {
        // No usable host on either input → None. Caller is expected to
        // skip caching entirely in this case.
        assert!(host_from_token_url("file:///x", "data:,").is_none());
    }
}
