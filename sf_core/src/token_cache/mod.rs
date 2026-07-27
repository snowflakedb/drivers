pub(crate) mod file_cache;
mod keyring_cache;

use std::collections::BTreeMap;
use std::path::PathBuf;

pub use keyring_cache::KeyringTokenCache;
use sha2::{Digest, Sha256};
use snafu::{Location, Snafu};

const KEY_VERSION: u32 = 2;
const KEY_PREFIX: &str = "SnowflakeTokenCache";

/// Represents the type of token stored in the keystore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenType {
    IdToken,
    MfaToken,
    OAuthAccessToken,
    OAuthRefreshToken,
    DpopBundledAccessToken,
}

impl TokenType {
    /// Returns the string representation of the token type.
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenType::IdToken => "ID_TOKEN",
            TokenType::MfaToken => "MFA_TOKEN",
            TokenType::OAuthAccessToken => "OAUTH_ACCESS_TOKEN",
            TokenType::OAuthRefreshToken => "OAUTH_REFRESH_TOKEN",
            TokenType::DpopBundledAccessToken => "DPOP_BUNDLED_ACCESS_TOKEN",
        }
    }

    /// Returns all token types.
    pub fn all() -> &'static [TokenType] {
        &[
            TokenType::IdToken,
            TokenType::MfaToken,
            TokenType::OAuthAccessToken,
            TokenType::OAuthRefreshToken,
            TokenType::DpopBundledAccessToken,
        ]
    }
}

impl std::fmt::Display for TokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// The key fields that uniquely identify a cached token across drivers.
///
/// Fields must be normalized before construction. Use [`normalize_url`] for
/// URL fields (`idp`, `snowflake`) and [`normalize_identifier`] for identifier
/// fields (`username`, `role`).
///
/// For non-OAuth flows (MFA, external-browser ID token), set both `idp` and
/// `snowflake` to the normalized Snowflake server URL; leave `role` empty for
/// MFA or populate it from `LoginParameters.role` for ID-token flows.
#[derive(Debug, Clone)]
pub struct CacheKey {
    pub token_type: TokenType,
    /// Normalized IdP token-endpoint URL (scheme stripped, uppercased).
    pub idp: String,
    /// Normalized Snowflake server URL (scheme stripped, uppercased).
    pub snowflake: String,
    /// Normalized Snowflake username (unquoted segments uppercased, quoted preserved).
    pub username: String,
    /// Normalized role name (unquoted segments uppercased, quoted preserved);
    /// empty for MFA flows.
    pub role: String,
}

/// Normalizes a URL for use as a cache key component.
///
/// Strips the URL scheme and any userinfo prefix, then uppercases the remaining
/// authority (host and any explicitly-stated port) and path. A trailing slash
/// on a root-only URL is omitted so bare host-only URLs produce a key without
/// a trailing slash.
///
/// The raw URL string is used rather than the parsed authority so that an
/// explicitly-stated default port (e.g., `:443` on an HTTPS URL) is preserved
/// verbatim — the `url` crate normalizes such ports away. Callers must supply
/// the raw connection-string URL, never a value produced by `url::Url::as_str()`.
///
/// Cross-driver spec (§2.3): strip scheme, uppercase remainder, keep port and path.
///
/// # Examples
/// ```text
/// "https://login.microsoftonline.com:443/tenant/oauth2/v2.0"
///   → "LOGIN.MICROSOFTONLINE.COM:443/TENANT/OAUTH2/V2.0"
/// "https://myorg-myaccount.snowflakecomputing.com"
///   → "MYORG-MYACCOUNT.SNOWFLAKECOMPUTING.COM"
/// ```
pub fn normalize_url(url: &str) -> String {
    // Strip the scheme prefix ("scheme://") from the raw string to preserve
    // any explicitly-stated default-scheme port (e.g., ":443" on HTTPS).
    let after_scheme = url.find("://").map(|i| &url[i + 3..]).unwrap_or(url);

    // Strip query string and fragment, which never appear in cache keys.
    let without_query = after_scheme
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(after_scheme);
    let without_fragment = without_query
        .split_once('#')
        .map(|(path, _)| path)
        .unwrap_or(without_query);

    // Strip optional userinfo ("user:pass@") from the authority only. The
    // authority ends at the first '/', so an '@' is a userinfo delimiter only
    // when it precedes that first slash; an '@' inside the path is preserved.
    let authority_end = without_fragment.find('/').unwrap_or(without_fragment.len());
    let (authority, path) = without_fragment.split_at(authority_end);
    let authority = authority
        .split_once('@')
        .map(|(_, rest)| rest)
        .unwrap_or(authority);

    // Trim a root-only trailing slash so bare-host URLs have no slash suffix.
    format!("{authority}{path}")
        .trim_end_matches('/')
        .to_uppercase()
}

/// Normalizes a Snowflake identifier for use as a cache key component.
///
/// Segments not enclosed in double quotes are uppercased; double-quoted
/// segments (including their surrounding `"` delimiters) are preserved
/// verbatim. This matches Snowflake's identifier case-folding rules.
///
/// # Examples
/// ```text
/// "first.last@domain.com"     → "FIRST.LAST@DOMAIN.COM"
/// "\"First Last\"@domain.com" → "\"First Last\"@DOMAIN.COM"
/// ```
pub fn normalize_identifier(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_quotes = false;
    for ch in s.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                result.push(ch);
            }
            _ if in_quotes => result.push(ch),
            _ => result.push(ch.to_ascii_uppercase()),
        }
    }
    result
}

/// A trait for implementing token caching functionality.
///
/// Implementations provide secure storage for authentication tokens, keyed
/// by a versioned, uniformly-hashed [`CacheKey`] that encodes the IdP URL,
/// Snowflake URL, username, role, and token type. The hash is computed by
/// [`build_cache_key`].
pub trait TokenCache: Send + Sync {
    /// Adds a token to the keystore.
    ///
    /// # Returns
    /// * `Ok(())` if the token was successfully stored
    /// * `Err(TokenCacheError)` if the operation failed
    fn add_token(&self, key: &CacheKey, token_value: &str) -> Result<(), TokenCacheError>;

    /// Removes a token from the keystore.
    ///
    /// # Returns
    /// * `Ok(())` if the token was successfully removed or did not exist
    /// * `Err(TokenCacheError)` if the operation failed
    fn remove_token(&self, key: &CacheKey) -> Result<(), TokenCacheError>;

    /// Retrieves a token from the keystore.
    ///
    /// # Returns
    /// * `Ok(Some(token))` if the token was found
    /// * `Ok(None)` if the token does not exist
    /// * `Err(TokenCacheError)` if the operation failed
    fn get_token(&self, key: &CacheKey) -> Result<Option<String>, TokenCacheError>;
}

/// Constructs a versioned, uniformly-hashed cache key from a [`CacheKey`].
///
/// The format is:
/// ```text
/// SnowflakeTokenCache.v<VERSION>.<sha256hex(compact_json_sorted_fields)>
/// ```
///
/// Fields are serialized into compact JSON in lexicographic key order
/// (`idp → role → snowflake → token_type → username`) to guarantee
/// byte-exact cross-driver parity regardless of field insertion order.
pub fn build_cache_key(key: &CacheKey) -> String {
    let serialized = serialize_cache_key(key);
    let hash = hex::encode(Sha256::digest(serialized.as_bytes()));
    format!("{KEY_PREFIX}.v{KEY_VERSION}.{hash}")
}

/// Serializes `key` into compact JSON with lexicographically sorted field names.
///
/// Uses [`BTreeMap`] so that `serde_json::to_string` emits fields in sorted
/// order without a custom serializer.
fn serialize_cache_key(key: &CacheKey) -> String {
    let mut map = BTreeMap::new();
    map.insert("idp", key.idp.as_str());
    map.insert("role", key.role.as_str());
    map.insert("snowflake", key.snowflake.as_str());
    map.insert("token_type", key.token_type.as_str());
    map.insert("username", key.username.as_str());
    serde_json::to_string(&map).expect("BTreeMap<&str, &str> serialization is infallible")
}

/// Validates that `idp`, `snowflake`, and `username` are non-empty.
///
/// `role` is allowed to be empty (e.g., MFA flows). `idp` is validated as
/// defense-in-depth: it is always populated in practice (the IdP URL for OAuth,
/// the server URL for non-OAuth flows), but an empty `idp` would silently
/// collapse the cross-IdP dimension of the key and reintroduce the collision
/// class this module guards against, so it is rejected rather than hashed.
pub(super) fn validate_key_components(key: &CacheKey) -> Result<(), TokenCacheError> {
    if key.snowflake.is_empty() {
        return InvalidKeyFormatSnafu {
            key: format!("snowflake=<empty>, username={}", key.username),
        }
        .fail();
    }
    if key.username.is_empty() {
        return InvalidKeyFormatSnafu {
            key: format!("snowflake={}, username=<empty>", key.snowflake),
        }
        .fail();
    }
    if key.idp.is_empty() {
        return InvalidKeyFormatSnafu {
            key: format!("idp=<empty>, snowflake={}", key.snowflake),
        }
        .fail();
    }
    Ok(())
}

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
#[snafu(visibility(pub))]
pub enum TokenCacheError {
    #[snafu(display("Failed to access keystore"))]
    KeystoreAccess {
        source: Box<dyn std::error::Error + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to store token in keystore"))]
    TokenStorage {
        source: Box<dyn std::error::Error + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to retrieve token from keystore"))]
    TokenRetrieval {
        source: Box<dyn std::error::Error + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to remove token from keystore"))]
    TokenRemoval {
        source: Box<dyn std::error::Error + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid token key format: {key}"))]
    InvalidKeyFormat {
        key: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Keystore is not available on this platform"))]
    UnsupportedPlatform {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to resolve cache directory from environment"))]
    CacheDirectoryResolution {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to acquire file lock for cache file"))]
    LockAcquisition {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to acquire file lock after maximum retries"))]
    LockExhausted {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Insufficient permissions on cache file: {}", path.display()))]
    InsufficientPermissions {
        path: PathBuf,
        source: Box<dyn std::error::Error + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "Cache file is not owned by the current user: {} (file uid: {file_uid}, current uid: {current_uid})",
        path.display()
    ))]
    FileNotOwnedByCurrentUser {
        path: PathBuf,
        file_uid: u32,
        current_uid: u32,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Cache file is not a regular file: {}", path.display()))]
    IrregularFileType {
        path: PathBuf,
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    mod build_cache_key_tests {
        use super::*;

        fn golden_key() -> CacheKey {
            CacheKey {
                token_type: TokenType::DpopBundledAccessToken,
                idp: "LOGIN.MICROSOFTONLINE.COM:443/TENANT-ID/OAUTH2/V2.0".to_string(),
                snowflake: "MYORG-MYACCOUNT.PRIVATELINK.SNOWFLAKECOMPUTING.COM".to_string(),
                username: "\"FIRST LAST\"@LONG-CORPORATE-DOMAIN.EXAMPLE.COM".to_string(),
                role: "\"ANALYST ROLE WITH SPACES\":NORTH_AMERICA:PROD:READONLY".to_string(),
            }
        }

        #[test]
        fn golden_hash_matches_spec() {
            assert_eq!(
                build_cache_key(&golden_key()),
                "SnowflakeTokenCache.v2.75ff2ad65a68afb402f125f62894697673c5ef3d863aba466d16b7a81053d1f4"
            );
        }

        #[test]
        fn key_starts_with_versioned_prefix() {
            assert!(build_cache_key(&golden_key()).starts_with("SnowflakeTokenCache.v2."));
        }

        #[test]
        fn different_snowflake_hosts_produce_different_keys() {
            let key1 = CacheKey {
                token_type: TokenType::OAuthAccessToken,
                idp: "IDP.EXAMPLE.COM".to_string(),
                snowflake: "ACCOUNT1.SNOWFLAKECOMPUTING.COM".to_string(),
                username: "USER".to_string(),
                role: String::new(),
            };
            let key2 = CacheKey {
                snowflake: "ACCOUNT2.SNOWFLAKECOMPUTING.COM".to_string(),
                ..key1.clone()
            };

            assert_ne!(build_cache_key(&key1), build_cache_key(&key2));
        }

        #[test]
        fn different_roles_produce_different_keys() {
            let key1 = CacheKey {
                token_type: TokenType::OAuthAccessToken,
                idp: "IDP.EXAMPLE.COM".to_string(),
                snowflake: "ACCOUNT.SNOWFLAKECOMPUTING.COM".to_string(),
                username: "USER".to_string(),
                role: "ROLE_A".to_string(),
            };
            let key2 = CacheKey {
                role: "ROLE_B".to_string(),
                ..key1.clone()
            };

            assert_ne!(build_cache_key(&key1), build_cache_key(&key2));
        }

        #[test]
        fn empty_role_for_mfa_produces_valid_key() {
            let key = CacheKey {
                token_type: TokenType::MfaToken,
                idp: "ACCOUNT.SNOWFLAKECOMPUTING.COM".to_string(),
                snowflake: "ACCOUNT.SNOWFLAKECOMPUTING.COM".to_string(),
                username: "USER".to_string(),
                role: String::new(),
            };

            let result = build_cache_key(&key);
            assert!(result.starts_with("SnowflakeTokenCache.v2."));
        }

        #[test]
        fn same_idp_different_snowflake_produce_different_keys() {
            let key1 = CacheKey {
                token_type: TokenType::OAuthAccessToken,
                idp: "IDP.SHARED.COM".to_string(),
                snowflake: "ORG-ACCOUNT1.SNOWFLAKECOMPUTING.COM".to_string(),
                username: "USER".to_string(),
                role: String::new(),
            };
            let key2 = CacheKey {
                snowflake: "ORG-ACCOUNT2.SNOWFLAKECOMPUTING.COM".to_string(),
                ..key1.clone()
            };

            assert_ne!(build_cache_key(&key1), build_cache_key(&key2));
        }
    }

    mod token_type_tests {
        use super::*;

        #[test]
        fn as_str_returns_correct_values() {
            assert_eq!(TokenType::IdToken.as_str(), "ID_TOKEN");
            assert_eq!(TokenType::MfaToken.as_str(), "MFA_TOKEN");
            assert_eq!(TokenType::OAuthAccessToken.as_str(), "OAUTH_ACCESS_TOKEN");
            assert_eq!(TokenType::OAuthRefreshToken.as_str(), "OAUTH_REFRESH_TOKEN");
            assert_eq!(
                TokenType::DpopBundledAccessToken.as_str(),
                "DPOP_BUNDLED_ACCESS_TOKEN"
            );
        }

        #[test]
        fn display_matches_as_str() {
            assert_eq!(format!("{}", TokenType::IdToken), "ID_TOKEN");
            assert_eq!(format!("{}", TokenType::MfaToken), "MFA_TOKEN");
            assert_eq!(
                format!("{}", TokenType::OAuthAccessToken),
                "OAUTH_ACCESS_TOKEN"
            );
            assert_eq!(
                format!("{}", TokenType::OAuthRefreshToken),
                "OAUTH_REFRESH_TOKEN"
            );
            assert_eq!(
                format!("{}", TokenType::DpopBundledAccessToken),
                "DPOP_BUNDLED_ACCESS_TOKEN"
            );
        }
    }

    mod normalize_url_tests {
        use super::*;

        #[test]
        fn strips_scheme_uppercases_host_port_and_path() {
            assert_eq!(
                normalize_url("https://login.microsoftonline.com:443/tenant-id/oauth2/v2.0"),
                "LOGIN.MICROSOFTONLINE.COM:443/TENANT-ID/OAUTH2/V2.0"
            );
        }

        #[test]
        fn omits_root_path_for_bare_host() {
            assert_eq!(
                normalize_url("https://myorg-myaccount.privatelink.snowflakecomputing.com"),
                "MYORG-MYACCOUNT.PRIVATELINK.SNOWFLAKECOMPUTING.COM"
            );
        }

        #[test]
        fn includes_explicit_port_and_non_root_path() {
            assert_eq!(
                normalize_url("https://account.snowflakecomputing.com/path/to/resource"),
                "ACCOUNT.SNOWFLAKECOMPUTING.COM/PATH/TO/RESOURCE"
            );
        }

        #[test]
        fn excludes_implicit_default_port() {
            assert_eq!(
                normalize_url("https://host.example.com"),
                "HOST.EXAMPLE.COM"
            );
        }

        #[test]
        fn unparseable_input_is_uppercased_verbatim() {
            assert_eq!(normalize_url("not a url"), "NOT A URL");
        }

        #[test]
        fn strips_userinfo_from_authority_only() {
            assert_eq!(
                normalize_url("https://user:pass@host.example.com/path"),
                "HOST.EXAMPLE.COM/PATH"
            );
        }

        #[test]
        fn preserves_at_sign_in_path() {
            // An '@' after the authority is part of the path and must survive:
            // the previous whole-string split would have truncated here.
            assert_eq!(
                normalize_url("https://host.example.com/oauth/@handle/token"),
                "HOST.EXAMPLE.COM/OAUTH/@HANDLE/TOKEN"
            );
        }
    }

    mod normalize_identifier_tests {
        use super::*;

        #[test]
        fn uppercases_plain_identifier() {
            assert_eq!(normalize_identifier("user@domain.com"), "USER@DOMAIN.COM");
        }

        #[test]
        fn preserves_double_quoted_segment() {
            assert_eq!(
                normalize_identifier("\"First Last\"@domain.com"),
                "\"First Last\"@DOMAIN.COM"
            );
        }

        #[test]
        fn handles_multiple_quoted_and_unquoted_segments() {
            assert_eq!(
                normalize_identifier("\"Analyst Role With Spaces\":north_america:prod"),
                "\"Analyst Role With Spaces\":NORTH_AMERICA:PROD"
            );
        }

        #[test]
        fn already_uppercased_input_is_unchanged() {
            assert_eq!(normalize_identifier("USER@DOMAIN.COM"), "USER@DOMAIN.COM");
        }

        #[test]
        fn preserves_sql_escaped_double_quotes_verbatim() {
            // `"Foo""Bar"` is a single quoted identifier whose `""` is an
            // escaped double-quote. Its content stays verbatim (case-preserved),
            // while a trailing unquoted segment is still uppercased. Locks the
            // quote-toggle behavior against future simplification.
            assert_eq!(
                normalize_identifier("\"Foo\"\"Bar\":baz"),
                "\"Foo\"\"Bar\":BAZ"
            );
        }
    }

    mod validation_tests {
        use super::*;

        fn valid_key() -> CacheKey {
            CacheKey {
                token_type: TokenType::IdToken,
                idp: "IDP.EXAMPLE.COM".to_string(),
                snowflake: "HOST.EXAMPLE.COM".to_string(),
                username: "testuser".to_string(),
                role: String::new(),
            }
        }

        #[test]
        fn validate_key_components_rejects_empty_snowflake() {
            let key = CacheKey {
                snowflake: String::new(),
                ..valid_key()
            };

            assert!(
                matches!(
                    validate_key_components(&key),
                    Err(TokenCacheError::InvalidKeyFormat { .. })
                ),
                "Expected InvalidKeyFormat for empty snowflake"
            );
        }

        #[test]
        fn validate_key_components_rejects_empty_username() {
            let key = CacheKey {
                username: String::new(),
                ..valid_key()
            };

            assert!(
                matches!(
                    validate_key_components(&key),
                    Err(TokenCacheError::InvalidKeyFormat { .. })
                ),
                "Expected InvalidKeyFormat for empty username"
            );
        }

        #[test]
        fn validate_key_components_accepts_valid_inputs() {
            assert!(validate_key_components(&valid_key()).is_ok());
        }

        #[test]
        fn validate_key_components_accepts_empty_role() {
            let key = CacheKey {
                role: String::new(),
                ..valid_key()
            };

            assert!(validate_key_components(&key).is_ok());
        }

        #[test]
        fn validate_key_components_rejects_empty_idp() {
            let key = CacheKey {
                idp: String::new(),
                ..valid_key()
            };

            assert!(
                matches!(
                    validate_key_components(&key),
                    Err(TokenCacheError::InvalidKeyFormat { .. })
                ),
                "Expected InvalidKeyFormat for empty idp"
            );
        }
    }

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    mod keyring_token_cache_tests {
        use super::*;

        fn unique_test_key(prefix: &str) -> String {
            use std::time::{SystemTime, UNIX_EPOCH};
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            format!("{prefix}_{timestamp}")
        }

        fn make_key(snowflake: &str, username: &str, token_type: TokenType) -> CacheKey {
            CacheKey {
                token_type,
                idp: snowflake.to_string(),
                snowflake: snowflake.to_string(),
                username: username.to_string(),
                role: String::new(),
            }
        }

        fn make_key_with_role(
            snowflake: &str,
            username: &str,
            token_type: TokenType,
            role: &str,
        ) -> CacheKey {
            CacheKey {
                token_type,
                idp: snowflake.to_string(),
                snowflake: snowflake.to_string(),
                username: username.to_string(),
                role: role.to_string(),
            }
        }

        fn cleanup_test_token(cache: &KeyringTokenCache, snowflake: &str, username: &str) {
            for &token_type in TokenType::all() {
                let key = make_key(snowflake, username, token_type);
                let _ = cache.remove_token(&key);
            }
        }

        #[test]
        fn add_and_get_token_succeeds() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");
            let snowflake = unique_test_key("test_host");
            let username = unique_test_key("test_user");
            let token_value = "test_token_value_12345";
            cleanup_test_token(&cache, &snowflake, &username);

            let key = make_key(&snowflake, &username, TokenType::IdToken);

            // When we add a token and then retrieve it
            let add_result = cache.add_token(&key, token_value);
            assert!(
                add_result.is_ok(),
                "Failed to add token: {:?}",
                add_result.err()
            );

            // Then the retrieved token should match
            let get_result = cache.get_token(&key);
            assert!(
                get_result.is_ok(),
                "Failed to get token: {:?}",
                get_result.err()
            );
            assert_eq!(get_result.unwrap(), Some(token_value.to_string()));

            cleanup_test_token(&cache, &snowflake, &username);
        }

        #[test]
        fn get_nonexistent_token_returns_none() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");
            let snowflake = unique_test_key("nonexistent_host");
            let username = unique_test_key("nonexistent_user");
            cleanup_test_token(&cache, &snowflake, &username);

            let key = make_key(&snowflake, &username, TokenType::MfaToken);

            // When we get a token
            let result = cache.get_token(&key);

            // Then None should be returned
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None);
        }

        #[test]
        #[cfg_attr(target_os = "windows", ignore)]
        fn flaky_remove_existing_token_succeeds() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");
            let snowflake = unique_test_key("remove_test_host");
            let username = unique_test_key("remove_test_user");
            let token_value = "token_to_be_removed";
            cleanup_test_token(&cache, &snowflake, &username);

            let key = make_key(&snowflake, &username, TokenType::OAuthAccessToken);
            cache
                .add_token(&key, token_value)
                .expect("Setup failed: could not add token");
            let get_result = cache.get_token(&key);
            assert_eq!(get_result.unwrap(), Some(token_value.to_string()));

            // When we remove the token
            let remove_result = cache.remove_token(&key);
            assert!(
                remove_result.is_ok(),
                "Failed to remove token: {:?}",
                remove_result.err()
            );

            // Then getting it should return None
            let get_after_remove = cache.get_token(&key);
            assert_eq!(get_after_remove.unwrap(), None);

            cleanup_test_token(&cache, &snowflake, &username);
        }

        #[test]
        fn remove_nonexistent_token_succeeds() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");
            let snowflake = unique_test_key("remove_nonexistent_host");
            let username = unique_test_key("remove_nonexistent_user");
            cleanup_test_token(&cache, &snowflake, &username);

            let key = make_key(&snowflake, &username, TokenType::OAuthRefreshToken);

            // When we remove a token
            let result = cache.remove_token(&key);

            // Then the operation should succeed
            assert!(
                result.is_ok(),
                "Remove nonexistent should succeed: {:?}",
                result.err()
            );
        }

        #[test]
        #[cfg_attr(target_os = "windows", ignore)]
        fn flaky_overwrite_token_succeeds() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");
            let snowflake = unique_test_key("overwrite_test_host");
            let username = unique_test_key("overwrite_test_user");
            let original_token = "original_token_value";
            let updated_token = "updated_token_value";
            cleanup_test_token(&cache, &snowflake, &username);

            let key = make_key(&snowflake, &username, TokenType::DpopBundledAccessToken);
            cache
                .add_token(&key, original_token)
                .expect("Failed to add original token");
            let first_get = cache.get_token(&key);
            assert_eq!(first_get.unwrap(), Some(original_token.to_string()));

            // When we add a new value for the same key
            let overwrite_result = cache.add_token(&key, updated_token);
            assert!(
                overwrite_result.is_ok(),
                "Failed to overwrite token: {:?}",
                overwrite_result.err()
            );

            // Then the new value should replace the old one
            let second_get = cache.get_token(&key);
            assert_eq!(second_get.unwrap(), Some(updated_token.to_string()));

            cleanup_test_token(&cache, &snowflake, &username);
        }

        #[test]
        fn different_token_types_stored_separately() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");
            let snowflake = unique_test_key("multi_type_host");
            let username = unique_test_key("multi_type_user");
            let id_token = "id_token_value";
            let mfa_token = "mfa_token_value";
            cleanup_test_token(&cache, &snowflake, &username);

            let id_key = make_key(&snowflake, &username, TokenType::IdToken);
            let mfa_key = make_key(&snowflake, &username, TokenType::MfaToken);

            // When we store tokens of different types for the same host and user
            cache
                .add_token(&id_key, id_token)
                .expect("Failed to add ID token");
            cache
                .add_token(&mfa_key, mfa_token)
                .expect("Failed to add MFA token");

            // Then each type should return its own value
            let get_id = cache.get_token(&id_key);
            let get_mfa = cache.get_token(&mfa_key);
            assert_eq!(get_id.unwrap(), Some(id_token.to_string()));
            assert_eq!(get_mfa.unwrap(), Some(mfa_token.to_string()));

            cleanup_test_token(&cache, &snowflake, &username);
        }

        #[test]
        fn add_token_with_empty_snowflake_fails() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");

            let key = CacheKey {
                token_type: TokenType::IdToken,
                idp: String::new(),
                snowflake: String::new(),
                username: "username".to_string(),
                role: String::new(),
            };

            // When we add a token with an empty snowflake host
            let result = cache.add_token(&key, "token_value");

            // Then an InvalidKeyFormat error should be returned
            assert!(result.is_err());
            assert!(matches!(
                result,
                Err(TokenCacheError::InvalidKeyFormat { .. })
            ));
        }

        #[test]
        fn add_token_with_empty_username_fails() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");

            let key = CacheKey {
                token_type: TokenType::IdToken,
                idp: "host.example.com".to_string(),
                snowflake: "host.example.com".to_string(),
                username: String::new(),
                role: String::new(),
            };

            // When we add a token with an empty username
            let result = cache.add_token(&key, "token_value");

            // Then an InvalidKeyFormat error should be returned
            assert!(result.is_err());
            assert!(matches!(
                result,
                Err(TokenCacheError::InvalidKeyFormat { .. })
            ));
        }

        #[test]
        fn get_token_with_empty_snowflake_fails() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");

            let key = CacheKey {
                token_type: TokenType::IdToken,
                idp: String::new(),
                snowflake: String::new(),
                username: "username".to_string(),
                role: String::new(),
            };

            // When we get a token with an empty snowflake host
            let result = cache.get_token(&key);

            // Then an InvalidKeyFormat error should be returned
            assert!(result.is_err());
            assert!(matches!(
                result,
                Err(TokenCacheError::InvalidKeyFormat { .. })
            ));
        }

        #[test]
        fn get_token_with_empty_username_fails() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");

            let key = CacheKey {
                token_type: TokenType::IdToken,
                idp: "host.example.com".to_string(),
                snowflake: "host.example.com".to_string(),
                username: String::new(),
                role: String::new(),
            };

            // When we get a token with an empty username
            let result = cache.get_token(&key);

            // Then an InvalidKeyFormat error should be returned
            assert!(result.is_err());
            assert!(matches!(
                result,
                Err(TokenCacheError::InvalidKeyFormat { .. })
            ));
        }

        #[test]
        fn remove_token_with_empty_snowflake_fails() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");

            let key = CacheKey {
                token_type: TokenType::IdToken,
                idp: String::new(),
                snowflake: String::new(),
                username: "username".to_string(),
                role: String::new(),
            };

            // When we remove a token with an empty snowflake host
            let result = cache.remove_token(&key);

            // Then an InvalidKeyFormat error should be returned
            assert!(result.is_err());
            assert!(matches!(
                result,
                Err(TokenCacheError::InvalidKeyFormat { .. })
            ));
        }

        #[test]
        fn remove_token_with_empty_username_fails() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");

            let key = CacheKey {
                token_type: TokenType::IdToken,
                idp: "host.example.com".to_string(),
                snowflake: "host.example.com".to_string(),
                username: String::new(),
                role: String::new(),
            };

            // When we remove a token with an empty username
            let result = cache.remove_token(&key);

            // Then an InvalidKeyFormat error should be returned
            assert!(result.is_err());
            assert!(matches!(
                result,
                Err(TokenCacheError::InvalidKeyFormat { .. })
            ));
        }

        #[test]
        #[cfg_attr(target_os = "windows", ignore)]
        fn flaky_multi_account_tokens_do_not_collide() {
            // Two different Snowflake accounts share the same IdP and user.
            // Each account's token must be stored and retrieved independently.
            let cache = KeyringTokenCache::new().expect("Failed to create cache");
            let idp = unique_test_key("idp_shared");
            let user = unique_test_key("multi_acct_user");
            let account1 = unique_test_key("account1");
            let account2 = unique_test_key("account2");
            let token1 = "token_for_account1";
            let token2 = "token_for_account2";

            let key1 = CacheKey {
                token_type: TokenType::OAuthAccessToken,
                idp: idp.clone(),
                snowflake: account1.clone(),
                username: user.clone(),
                role: String::new(),
            };
            let key2 = CacheKey {
                token_type: TokenType::OAuthAccessToken,
                idp: idp.clone(),
                snowflake: account2.clone(),
                username: user.clone(),
                role: String::new(),
            };

            // Store tokens for both accounts
            cache.add_token(&key1, token1).expect("add account1 token");
            cache.add_token(&key2, token2).expect("add account2 token");

            // Each account retrieves its own token
            assert_eq!(
                cache.get_token(&key1).unwrap(),
                Some(token1.to_string()),
                "account1 should retrieve its own token"
            );
            assert_eq!(
                cache.get_token(&key2).unwrap(),
                Some(token2.to_string()),
                "account2 should retrieve its own token"
            );

            // Evicting account1's token does not affect account2
            cache.remove_token(&key1).expect("remove account1 token");
            assert_eq!(
                cache.get_token(&key1).unwrap(),
                None,
                "account1 token should be gone after removal"
            );
            assert_eq!(
                cache.get_token(&key2).unwrap(),
                Some(token2.to_string()),
                "account2 token must survive account1 eviction"
            );

            let _ = cache.remove_token(&key2);
        }

        #[test]
        #[cfg_attr(target_os = "windows", ignore)]
        fn flaky_multi_role_tokens_do_not_collide() {
            // Same account and user, two different roles.
            // Each role's token must be stored and retrieved independently.
            let cache = KeyringTokenCache::new().expect("Failed to create cache");
            let snowflake = unique_test_key("multi_role_acct");
            let user = unique_test_key("multi_role_user");
            let token_a = "token_for_role_a";
            let token_b = "token_for_role_b";

            let key_a =
                make_key_with_role(&snowflake, &user, TokenType::OAuthAccessToken, "ROLE_A");
            let key_b =
                make_key_with_role(&snowflake, &user, TokenType::OAuthAccessToken, "ROLE_B");

            // Store tokens for both roles
            cache.add_token(&key_a, token_a).expect("add role_a token");
            cache.add_token(&key_b, token_b).expect("add role_b token");

            // Each role retrieves its own token
            assert_eq!(
                cache.get_token(&key_a).unwrap(),
                Some(token_a.to_string()),
                "role_a should retrieve its own token"
            );
            assert_eq!(
                cache.get_token(&key_b).unwrap(),
                Some(token_b.to_string()),
                "role_b should retrieve its own token"
            );

            // Evicting role_a's token does not affect role_b
            cache.remove_token(&key_a).expect("remove role_a token");
            assert_eq!(
                cache.get_token(&key_a).unwrap(),
                None,
                "role_a token should be gone after removal"
            );
            assert_eq!(
                cache.get_token(&key_b).unwrap(),
                Some(token_b.to_string()),
                "role_b token must survive role_a eviction"
            );

            let _ = cache.remove_token(&key_b);
        }
    }
}
