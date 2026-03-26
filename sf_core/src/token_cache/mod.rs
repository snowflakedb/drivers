pub(crate) mod file_cache;
mod keyring_cache;

use std::path::PathBuf;

use snafu::{Location, Snafu};

pub use keyring_cache::KeyringTokenCache;

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

/// A trait for implementing token caching functionality.
///
/// Implementations of this trait provide secure storage for authentication tokens,
/// using the host, username, and token type as the key identifier.
///
/// The key is constructed by concatenating host, username, and token type with semicolons:
/// `"{host};{username};{token_type}"`
pub trait TokenCache: Send + Sync {
    /// Adds a token to the keystore.
    ///
    /// # Arguments
    /// * `host` - The host associated with the token
    /// * `username` - The username associated with the token
    /// * `token_type` - The type of token being stored
    /// * `token_value` - The actual token value to store
    ///
    /// # Returns
    /// * `Ok(())` if the token was successfully stored
    /// * `Err(TokenCacheError)` if the operation failed
    fn add_token(
        &self,
        host: &str,
        username: &str,
        token_type: TokenType,
        token_value: &str,
    ) -> Result<(), TokenCacheError>;

    /// Removes a token from the keystore.
    ///
    /// # Arguments
    /// * `host` - The host associated with the token
    /// * `username` - The username associated with the token
    /// * `token_type` - The type of token to remove
    ///
    /// # Returns
    /// * `Ok(())` if the token was successfully removed or did not exist
    /// * `Err(TokenCacheError)` if the operation failed
    fn remove_token(
        &self,
        host: &str,
        username: &str,
        token_type: TokenType,
    ) -> Result<(), TokenCacheError>;

    /// Retrieves a token from the keystore.
    ///
    /// # Arguments
    /// * `host` - The host associated with the token
    /// * `username` - The username associated with the token
    /// * `token_type` - The type of token to retrieve
    ///
    /// # Returns
    /// * `Ok(Some(token))` if the token was found
    /// * `Ok(None)` if the token does not exist
    /// * `Err(TokenCacheError)` if the operation failed
    fn get_token(
        &self,
        host: &str,
        username: &str,
        token_type: TokenType,
    ) -> Result<Option<String>, TokenCacheError>;
}

/// Constructs a cache key from the host, username, and token type.
///
/// The key format is: `"{host};{username};{token_type}"`
pub fn build_cache_key(host: &str, username: &str, token_type: TokenType) -> String {
    format!("{};{};{}", host, username, token_type.as_str())
}

/// Validates that host and username are non-empty.
pub(super) fn validate_key_components(host: &str, username: &str) -> Result<(), TokenCacheError> {
    if host.is_empty() || host.contains(';') {
        return Err(TokenCacheError::InvalidKeyFormat {
            key: format!("{};{}", host, username),
            location: Location::new(file!(), line!(), 0),
        });
    }
    if username.is_empty() || username.contains(';') {
        return Err(TokenCacheError::InvalidKeyFormat {
            key: format!("{};{}", host, username),
            location: Location::new(file!(), line!(), 0),
        });
    }
    Ok(())
}

#[derive(Debug, Snafu)]
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

        #[test]
        fn builds_correct_key_format() {
            let key = build_cache_key("myhost.snowflake.com", "testuser", TokenType::IdToken);

            assert_eq!(key, "myhost.snowflake.com;testuser;ID_TOKEN");
        }

        #[test]
        fn builds_key_for_all_token_types() {
            let host = "host.example.com";
            let user = "user1";

            for &token_type in TokenType::all() {
                assert_eq!(
                    build_cache_key(host, user, token_type),
                    format!("{host};{user};{}", token_type.as_str())
                );
            }
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

    mod validation_tests {
        use super::*;

        #[test]
        fn validate_key_components_rejects_empty_values() {
            for (host, username, token_key_component_name) in [
                ("", "username", "host"),
                ("host.example.com", "", "username"),
            ] {
                let result = validate_key_components(host, username);

                assert!(
                    matches!(result, Err(TokenCacheError::InvalidKeyFormat { .. })),
                    "Expected InvalidKeyFormat for empty {token_key_component_name}"
                );
            }
        }

        #[test]
        fn validate_key_components_accepts_valid_inputs() {
            let result = validate_key_components("host.example.com", "testuser");

            assert!(result.is_ok());
        }

        #[test]
        fn validate_key_components_disallows_invalid_host() {
            let result = validate_key_components("host.example.com;", "testuser");

            assert!(result.is_err());
        }

        #[test]
        fn validate_key_components_disallows_invalid_user() {
            let result = validate_key_components("host.example.com", "test;user");

            assert!(result.is_err());
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

        fn cleanup_test_token(cache: &KeyringTokenCache, host: &str, username: &str) {
            for &token_type in TokenType::all() {
                let _ = cache.remove_token(host, username, token_type);
            }
        }

        #[test]
        fn add_and_get_token_succeeds() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");
            let host = unique_test_key("test_host");
            let username = unique_test_key("test_user");
            let token_value = "test_token_value_12345";
            cleanup_test_token(&cache, &host, &username);

            // When we add a token and then retrieve it
            let add_result = cache.add_token(&host, &username, TokenType::IdToken, token_value);
            assert!(
                add_result.is_ok(),
                "Failed to add token: {:?}",
                add_result.err()
            );

            // Then the retrieved token should match
            let get_result = cache.get_token(&host, &username, TokenType::IdToken);
            assert!(
                get_result.is_ok(),
                "Failed to get token: {:?}",
                get_result.err()
            );
            assert_eq!(get_result.unwrap(), Some(token_value.to_string()));

            cleanup_test_token(&cache, &host, &username);
        }

        #[test]
        fn get_nonexistent_token_returns_none() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");
            let host = unique_test_key("nonexistent_host");
            let username = unique_test_key("nonexistent_user");
            cleanup_test_token(&cache, &host, &username);

            // When we get a token
            let result = cache.get_token(&host, &username, TokenType::MfaToken);

            // Then None should be returned
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None);
        }

        #[test]
        fn remove_existing_token_succeeds() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");
            let host = unique_test_key("remove_test_host");
            let username = unique_test_key("remove_test_user");
            let token_value = "token_to_be_removed";
            cleanup_test_token(&cache, &host, &username);
            cache
                .add_token(&host, &username, TokenType::OAuthAccessToken, token_value)
                .expect("Setup failed: could not add token");
            let get_result = cache.get_token(&host, &username, TokenType::OAuthAccessToken);
            assert_eq!(get_result.unwrap(), Some(token_value.to_string()));

            // When we remove the token
            let remove_result = cache.remove_token(&host, &username, TokenType::OAuthAccessToken);
            assert!(
                remove_result.is_ok(),
                "Failed to remove token: {:?}",
                remove_result.err()
            );

            // Then getting it should return None
            let get_after_remove = cache.get_token(&host, &username, TokenType::OAuthAccessToken);
            assert_eq!(get_after_remove.unwrap(), None);

            cleanup_test_token(&cache, &host, &username);
        }

        #[test]
        fn remove_nonexistent_token_succeeds() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");
            let host = unique_test_key("remove_nonexistent_host");
            let username = unique_test_key("remove_nonexistent_user");
            cleanup_test_token(&cache, &host, &username);

            // When we remove a token
            let result = cache.remove_token(&host, &username, TokenType::OAuthRefreshToken);

            // Then the operation should succeed
            assert!(
                result.is_ok(),
                "Remove nonexistent should succeed: {:?}",
                result.err()
            );
        }

        #[test]
        fn overwrite_token_succeeds() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");
            let host = unique_test_key("overwrite_test_host");
            let username = unique_test_key("overwrite_test_user");
            let original_token = "original_token_value";
            let updated_token = "updated_token_value";
            cleanup_test_token(&cache, &host, &username);
            cache
                .add_token(
                    &host,
                    &username,
                    TokenType::DpopBundledAccessToken,
                    original_token,
                )
                .expect("Failed to add original token");
            let first_get = cache.get_token(&host, &username, TokenType::DpopBundledAccessToken);
            assert_eq!(first_get.unwrap(), Some(original_token.to_string()));

            // When we add a new value for the same key
            let overwrite_result = cache.add_token(
                &host,
                &username,
                TokenType::DpopBundledAccessToken,
                updated_token,
            );
            assert!(
                overwrite_result.is_ok(),
                "Failed to overwrite token: {:?}",
                overwrite_result.err()
            );

            // Then the new value should replace the old one
            let second_get = cache.get_token(&host, &username, TokenType::DpopBundledAccessToken);
            assert_eq!(second_get.unwrap(), Some(updated_token.to_string()));

            cleanup_test_token(&cache, &host, &username);
        }

        #[test]
        fn different_token_types_stored_separately() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");
            let host = unique_test_key("multi_type_host");
            let username = unique_test_key("multi_type_user");
            let id_token = "id_token_value";
            let mfa_token = "mfa_token_value";
            cleanup_test_token(&cache, &host, &username);

            // When we store tokens of different types for the same host and user
            cache
                .add_token(&host, &username, TokenType::IdToken, id_token)
                .expect("Failed to add ID token");
            cache
                .add_token(&host, &username, TokenType::MfaToken, mfa_token)
                .expect("Failed to add MFA token");

            // Then each type should return its own value
            let get_id = cache.get_token(&host, &username, TokenType::IdToken);
            let get_mfa = cache.get_token(&host, &username, TokenType::MfaToken);
            assert_eq!(get_id.unwrap(), Some(id_token.to_string()));
            assert_eq!(get_mfa.unwrap(), Some(mfa_token.to_string()));

            cleanup_test_token(&cache, &host, &username);
        }

        #[test]
        fn add_token_with_empty_host_fails() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");

            // When we add a token with an empty host
            let result = cache.add_token("", "username", TokenType::IdToken, "token_value");

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

            // When we add a token with an empty username
            let result = cache.add_token("host.example.com", "", TokenType::IdToken, "token_value");

            // Then an InvalidKeyFormat error should be returned
            assert!(result.is_err());
            assert!(matches!(
                result,
                Err(TokenCacheError::InvalidKeyFormat { .. })
            ));
        }

        #[test]
        fn get_token_with_empty_host_fails() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");

            // When we get a token with an empty host
            let result = cache.get_token("", "username", TokenType::IdToken);

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

            // When we get a token with an empty username
            let result = cache.get_token("host.example.com", "", TokenType::IdToken);

            // Then an InvalidKeyFormat error should be returned
            assert!(result.is_err());
            assert!(matches!(
                result,
                Err(TokenCacheError::InvalidKeyFormat { .. })
            ));
        }

        #[test]
        fn remove_token_with_empty_host_fails() {
            let cache = KeyringTokenCache::new().expect("Failed to create cache");

            // When we remove a token with an empty host
            let result = cache.remove_token("", "username", TokenType::IdToken);

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

            // When we remove a token with an empty username
            let result = cache.remove_token("host.example.com", "", TokenType::IdToken);

            // Then an InvalidKeyFormat error should be returned
            assert!(result.is_err());
            assert!(matches!(
                result,
                Err(TokenCacheError::InvalidKeyFormat { .. })
            ));
        }
    }
}
