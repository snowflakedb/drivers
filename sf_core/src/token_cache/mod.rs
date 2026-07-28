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
    /// Returns the PascalCase string used as the token-type segment in cache key
    /// prefixes (e.g. `"MfaToken"`, `"OauthAccessToken"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenType::IdToken => "IdToken",
            TokenType::MfaToken => "MfaToken",
            TokenType::OAuthAccessToken => "OauthAccessToken",
            TokenType::OAuthRefreshToken => "OauthRefreshToken",
            TokenType::DpopBundledAccessToken => "DpopBundledAccessToken",
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
/// Fields are normalized before construction. Use [`normalize_url`] for URL fields
/// (`idp`, `snowflake`) and [`normalize_identifier`] for identifier fields
/// (`username`, `role`).
///
/// **OAuth flows** (`OauthAccessToken`, `OauthRefreshToken`,
/// `DpopBundledAccessToken`): populate `idp` with the full token-endpoint URL,
/// `role` with the role name (or empty string if none), `snowflake` with the
/// Snowflake server URL, and `username`.
///
/// **MFA and ID token flows** (`MfaToken`, `IdToken`): set `idp` and `role` to
/// empty string. Only `snowflake` and `username` are included in the key hash for
/// these flows.
#[derive(Debug, Clone)]
pub struct CacheKey {
    pub token_type: TokenType,
    /// Normalized IdP token-endpoint URL. Empty string for MFA and ID token flows.
    pub idp: String,
    /// Normalized Snowflake server URL.
    pub snowflake: String,
    /// Normalized Snowflake username.
    pub username: String,
    /// Normalized role name. Empty string for MFA flows and when no role is configured.
    pub role: String,
}

/// Normalizes a URL for use as a cache key component.
///
/// Strips the URL scheme and any userinfo prefix, then lowercases the remaining
/// authority (host and any explicitly-stated port) and path. A trailing slash
/// on a root-only URL is omitted so bare host-only URLs produce a key without
/// a trailing slash.
///
/// The raw URL string is used rather than the parsed authority so that an
/// explicitly-stated default port (e.g., `:443` on an HTTPS URL) is preserved
/// verbatim — the `url` crate normalizes such ports away. Callers must supply
/// the raw connection-string URL, never a value produced by `url::Url::as_str()`.
///
/// Cross-driver spec: strip scheme, lowercase remainder, keep port and path.
///
/// # Examples
/// ```text
/// "https://login.microsoftonline.com:443/tenant/oauth2/v2.0"
///   → "login.microsoftonline.com:443/tenant/oauth2/v2.0"
/// "https://myorg-myaccount.snowflakecomputing.com"
///   → "myorg-myaccount.snowflakecomputing.com"
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
        .to_lowercase()
}

/// Normalizes a Snowflake identifier for use as a cache key component.
///
/// If the value contains any double-quote character (`"`), it is returned
/// verbatim — quoted identifiers carry case-sensitive semantics and must not
/// be altered. Otherwise the entire value is lowercased, since unquoted
/// Snowflake identifiers are case-insensitive.
///
/// # Examples
/// ```text
/// "first.last@domain.com"     → "first.last@domain.com"
/// "\"First Last\"@domain.com" → "\"First Last\"@domain.com"  (verbatim — has quotes)
/// ```
pub fn normalize_identifier(s: &str) -> String {
    if s.contains('"') {
        s.to_string()
    } else {
        s.to_lowercase()
    }
}

/// A trait for implementing token caching functionality.
///
/// Implementations provide secure storage for authentication tokens, keyed
/// by a versioned, uniformly-hashed [`CacheKey`]. The key format is
/// `SnowflakeTokenCache.v2.<TOKEN_TYPE>.<sha256hex(keyData)>`, where `keyData`
/// is flow-dependent (see [`build_cache_key`] and [`CacheKey`]).
///
/// Key format: `SnowflakeTokenCache.v2.<TokenType>.<sha256hex(keyData)>`.
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
/// SnowflakeTokenCache.v<VERSION>.<TOKEN_TYPE>.<sha256hex(compact_json_sorted_fields)>
/// ```
///
/// Fields serialized into `keyData` are flow-dependent:
/// - OAuth flows (`OauthAccessToken`, `OauthRefreshToken`, `DpopBundledAccessToken`):
///   `idp → role → snowflake → username` (4 fields, sorted)
/// - MFA and ID token flows (`MfaToken`, `IdToken`):
///   `snowflake → username` (2 fields, sorted)
///
/// `token_type` is NOT serialized into the JSON — it appears as the third
/// dot-separated segment of the key prefix, enabling per-type keystore cleanup.
pub fn build_cache_key(key: &CacheKey) -> String {
    let serialized = serialize_cache_key(key);
    let hash = hex::encode(Sha256::digest(serialized.as_bytes()));
    format!(
        "{KEY_PREFIX}.v{KEY_VERSION}.{}.{hash}",
        key.token_type.as_str()
    )
}

fn is_oauth_type(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::OAuthAccessToken
            | TokenType::OAuthRefreshToken
            | TokenType::DpopBundledAccessToken
    )
}

/// Serializes `key` into compact JSON with lexicographically sorted field names.
///
/// For OAuth flows (`OauthAccessToken`, `OauthRefreshToken`, `DpopBundledAccessToken`):
/// `idp`, `role`, `snowflake`, `username` (4 fields).
/// For MFA and ID token flows (`MfaToken`, `IdToken`): `snowflake`, `username` (2 fields).
/// `token_type` is intentionally excluded — it appears in the key prefix.
fn serialize_cache_key(key: &CacheKey) -> String {
    let mut map = BTreeMap::new();
    if is_oauth_type(key.token_type) {
        map.insert("idp", key.idp.as_str());
        map.insert("role", key.role.as_str());
    }
    map.insert("snowflake", key.snowflake.as_str());
    map.insert("username", key.username.as_str());
    serde_json::to_string(&map).expect("BTreeMap<&str, &str> serialization is infallible")
}

/// Validates that `snowflake` and `username` are non-empty.
///
/// `idp` must be non-empty for OAuth flows; empty is allowed for MFA/ID (absent by
/// spec). `role` is allowed to be empty — absent for MFA/ID and when no role is
/// configured for OAuth.
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
    if is_oauth_type(key.token_type) && key.idp.is_empty() {
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

    mod golden_hash_tests {
        use super::*;

        /// OAuth golden vector — spec §3 Vector A.
        /// URL fields are lowercased; quoted identifier fields are verbatim
        /// (normalize_identifier returns them unchanged when they contain `"`).
        #[test]
        fn oauth_golden_hash_matches_spec() {
            let key = CacheKey {
                token_type: TokenType::DpopBundledAccessToken,
                idp: "login.microsoftonline.com:443/tenant-id/oauth2/v2.0".to_string(),
                snowflake: "myorg-myaccount.privatelink.snowflakecomputing.com".to_string(),
                username: "\"First Last\"@long-corporate-domain.example.com".to_string(),
                role: "\"Analyst Role With Spaces\":north_america:prod:readonly".to_string(),
            };
            assert_eq!(
                build_cache_key(&key),
                "SnowflakeTokenCache.v2.DpopBundledAccessToken.741b6d66d252666d6821bfd19e0151511cf4efdaaeba2b3c87673aa4de6d2c0b"
            );
        }

        /// MFA golden vector — spec §3 Vector B.
        /// idp and role are empty; key contains only snowflake + username.
        #[test]
        fn mfa_golden_hash_matches_spec() {
            let key = CacheKey {
                token_type: TokenType::MfaToken,
                idp: String::new(),
                snowflake: "myorg-myaccount.privatelink.snowflakecomputing.com".to_string(),
                username: "\"First Last\"@long-corporate-domain.example.com".to_string(),
                role: String::new(),
            };
            assert_eq!(
                build_cache_key(&key),
                "SnowflakeTokenCache.v2.MfaToken.10c5dde84bb8f584c0df06ea826d418c4f580e08f9db10187c0cb5e2a732a0d6"
            );
        }

        #[test]
        fn oauth_key_contains_token_type_as_readable_prefix_segment() {
            let key = CacheKey {
                token_type: TokenType::OAuthAccessToken,
                idp: "IDP.EXAMPLE.COM".to_string(),
                snowflake: "ACCOUNT.EXAMPLE.COM".to_string(),
                username: "USER".to_string(),
                role: String::new(),
            };
            let built = build_cache_key(&key);
            assert!(
                built.starts_with("SnowflakeTokenCache.v2.OauthAccessToken."),
                "unexpected key: {built}"
            );
        }

        #[test]
        fn mfa_key_contains_mfa_token_as_readable_prefix_segment() {
            let key = CacheKey {
                token_type: TokenType::MfaToken,
                idp: String::new(),
                snowflake: "ACCOUNT.EXAMPLE.COM".to_string(),
                username: "USER".to_string(),
                role: String::new(),
            };
            let built = build_cache_key(&key);
            assert!(
                built.starts_with("SnowflakeTokenCache.v2.MfaToken."),
                "unexpected key: {built}"
            );
        }
    }

    mod build_cache_key_tests {
        use super::*;

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
        fn mfa_key_does_not_include_idp_or_role() {
            let key = CacheKey {
                token_type: TokenType::MfaToken,
                idp: String::new(),
                snowflake: "ACCOUNT.SNOWFLAKECOMPUTING.COM".to_string(),
                username: "USER".to_string(),
                role: String::new(),
            };

            let result = build_cache_key(&key);
            assert!(result.starts_with("SnowflakeTokenCache.v2.MfaToken."));
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

        #[test]
        fn mfa_and_oauth_keys_differ_for_same_user_and_host() {
            let snowflake = "ACCOUNT.SNOWFLAKECOMPUTING.COM".to_string();
            let username = "USER".to_string();
            let oauth_key = CacheKey {
                token_type: TokenType::OAuthAccessToken,
                idp: snowflake.clone(),
                snowflake: snowflake.clone(),
                username: username.clone(),
                role: String::new(),
            };
            let mfa_key = CacheKey {
                token_type: TokenType::MfaToken,
                idp: String::new(),
                snowflake: snowflake.clone(),
                username: username.clone(),
                role: String::new(),
            };
            assert_ne!(build_cache_key(&oauth_key), build_cache_key(&mfa_key));
        }
    }

    mod serialize_cache_key_tests {
        use super::*;

        #[test]
        fn oauth_serialization_omits_token_type_includes_four_fields() {
            let key = CacheKey {
                token_type: TokenType::OAuthAccessToken,
                idp: "IDP.EXAMPLE.COM".to_string(),
                role: "ROLE".to_string(),
                snowflake: "SF.EXAMPLE.COM".to_string(),
                username: "USER".to_string(),
            };
            let json = serialize_cache_key(&key);
            assert_eq!(
                json,
                r#"{"idp":"IDP.EXAMPLE.COM","role":"ROLE","snowflake":"SF.EXAMPLE.COM","username":"USER"}"#
            );
            assert!(
                !json.contains("token_type"),
                "token_type must not appear in keyData"
            );
        }

        #[test]
        fn mfa_serialization_omits_idp_role_and_token_type() {
            let key = CacheKey {
                token_type: TokenType::MfaToken,
                idp: String::new(),
                role: String::new(),
                snowflake: "SF.EXAMPLE.COM".to_string(),
                username: "USER".to_string(),
            };
            let json = serialize_cache_key(&key);
            assert_eq!(json, r#"{"snowflake":"SF.EXAMPLE.COM","username":"USER"}"#);
            assert!(!json.contains("idp"), "idp must not appear in MFA keyData");
            assert!(
                !json.contains("role"),
                "role must not appear in MFA keyData"
            );
            assert!(
                !json.contains("token_type"),
                "token_type must not appear in keyData"
            );
        }
    }

    mod token_type_tests {
        use super::*;

        #[test]
        fn as_str_returns_pascal_case_values() {
            assert_eq!(TokenType::IdToken.as_str(), "IdToken");
            assert_eq!(TokenType::MfaToken.as_str(), "MfaToken");
            assert_eq!(TokenType::OAuthAccessToken.as_str(), "OauthAccessToken");
            assert_eq!(TokenType::OAuthRefreshToken.as_str(), "OauthRefreshToken");
            assert_eq!(
                TokenType::DpopBundledAccessToken.as_str(),
                "DpopBundledAccessToken"
            );
        }

        #[test]
        fn display_matches_as_str() {
            assert_eq!(format!("{}", TokenType::IdToken), "IdToken");
            assert_eq!(format!("{}", TokenType::MfaToken), "MfaToken");
            assert_eq!(
                format!("{}", TokenType::OAuthAccessToken),
                "OauthAccessToken"
            );
            assert_eq!(
                format!("{}", TokenType::OAuthRefreshToken),
                "OauthRefreshToken"
            );
            assert_eq!(
                format!("{}", TokenType::DpopBundledAccessToken),
                "DpopBundledAccessToken"
            );
        }
    }

    mod normalize_url_tests {
        use super::*;

        #[test]
        fn strips_scheme_lowercases_host_port_and_path() {
            assert_eq!(
                normalize_url("https://login.microsoftonline.com:443/tenant-id/oauth2/v2.0"),
                "login.microsoftonline.com:443/tenant-id/oauth2/v2.0"
            );
        }

        #[test]
        fn omits_root_path_for_bare_host() {
            assert_eq!(
                normalize_url("https://myorg-myaccount.privatelink.snowflakecomputing.com"),
                "myorg-myaccount.privatelink.snowflakecomputing.com"
            );
        }

        #[test]
        fn includes_explicit_port_and_non_root_path() {
            assert_eq!(
                normalize_url("https://account.snowflakecomputing.com/path/to/resource"),
                "account.snowflakecomputing.com/path/to/resource"
            );
        }

        #[test]
        fn excludes_implicit_default_port() {
            assert_eq!(
                normalize_url("https://host.example.com"),
                "host.example.com"
            );
        }

        #[test]
        fn unparseable_input_is_lowercased_verbatim() {
            assert_eq!(normalize_url("not a url"), "not a url");
        }

        #[test]
        fn strips_userinfo_from_authority_only() {
            assert_eq!(
                normalize_url("https://user:pass@host.example.com/path"),
                "host.example.com/path"
            );
        }

        #[test]
        fn preserves_at_sign_in_path() {
            // An '@' after the authority is part of the path and must survive:
            // the previous whole-string split would have truncated here.
            assert_eq!(
                normalize_url("https://host.example.com/oauth/@handle/token"),
                "host.example.com/oauth/@handle/token"
            );
        }
    }

    mod normalize_identifier_tests {
        use super::*;

        #[test]
        fn lowercases_plain_identifier() {
            assert_eq!(normalize_identifier("USER@DOMAIN.COM"), "user@domain.com");
        }

        #[test]
        fn returns_quoted_value_verbatim() {
            // Value contains a `"` → returned unchanged in its entirety.
            assert_eq!(
                normalize_identifier("\"First Last\"@domain.com"),
                "\"First Last\"@domain.com"
            );
        }

        #[test]
        fn returns_value_with_mid_string_quote_verbatim() {
            // The `"` does not have to be at position 0; any quote triggers verbatim.
            assert_eq!(
                normalize_identifier("\"Analyst Role With Spaces\":north_america:prod"),
                "\"Analyst Role With Spaces\":north_america:prod"
            );
        }

        #[test]
        fn lowercases_already_lowercase_input() {
            assert_eq!(normalize_identifier("user@domain.com"), "user@domain.com");
        }

        #[test]
        fn returns_sql_escaped_double_quotes_verbatim() {
            // `"Foo""Bar"` contains double-quote characters → entire value is verbatim.
            assert_eq!(
                normalize_identifier("\"Foo\"\"Bar\":baz"),
                "\"Foo\"\"Bar\":baz"
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
        fn validate_key_components_accepts_empty_idp_for_mfa() {
            let key = CacheKey {
                token_type: TokenType::MfaToken,
                idp: String::new(),
                snowflake: "HOST.EXAMPLE.COM".to_string(),
                username: "user".to_string(),
                role: String::new(),
            };
            assert!(validate_key_components(&key).is_ok());
        }

        #[test]
        fn validate_key_components_rejects_empty_idp_for_oauth() {
            let key = CacheKey {
                token_type: TokenType::DpopBundledAccessToken,
                idp: String::new(),
                snowflake: "HOST.EXAMPLE.COM".to_string(),
                username: "user".to_string(),
                role: String::new(),
            };

            assert!(
                matches!(
                    validate_key_components(&key),
                    Err(TokenCacheError::InvalidKeyFormat { .. })
                ),
                "Expected InvalidKeyFormat for empty idp on OAuth token type"
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

        fn make_mfa_key(snowflake: &str, username: &str) -> CacheKey {
            CacheKey {
                token_type: TokenType::MfaToken,
                idp: String::new(),
                snowflake: snowflake.to_string(),
                username: username.to_string(),
                role: String::new(),
            }
        }

        fn make_key(snowflake: &str, username: &str, token_type: TokenType) -> CacheKey {
            CacheKey {
                token_type,
                idp: if matches!(
                    token_type,
                    TokenType::OAuthAccessToken
                        | TokenType::OAuthRefreshToken
                        | TokenType::DpopBundledAccessToken
                ) {
                    snowflake.to_string()
                } else {
                    String::new()
                },
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

            let key = make_mfa_key(&snowflake, &username);

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
