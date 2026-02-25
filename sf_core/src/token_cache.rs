use snafu::{Location, Snafu};

/// The service name used for keyring entries.
const KEYRING_SERVICE_NAME: &str = "snowflake_credential_cache";

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
pub trait TokenCache {
    /// Adds a token to the keystore.
    ///
    /// # Arguments
    /// * `host` - The Snowflake host associated with the token
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
    /// * `host` - The Snowflake host associated with the token
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
    /// * `host` - The Snowflake host associated with the token
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
fn validate_key_components(host: &str, username: &str) -> Result<(), TokenCacheError> {
    if host.is_empty() {
        return Err(TokenCacheError::InvalidKeyFormat {
            key: format!("{};{}", host, username),
            location: Location::new(file!(), line!(), 0),
        });
    }
    if username.is_empty() {
        return Err(TokenCacheError::InvalidKeyFormat {
            key: format!("{};{}", host, username),
            location: Location::new(file!(), line!(), 0),
        });
    }
    Ok(())
}

#[derive(Debug, Snafu)]
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
}

/// A token cache implementation using the system keyring.
///
/// This implementation uses the `keyring` crate to store tokens securely
/// in the platform-specific credential store:
/// - macOS: Keychain
/// - Windows: Credential Manager
/// - Linux: Secret Service (via D-Bus) or kernel keyutils
pub struct KeyringTokenCache;

impl KeyringTokenCache {
    /// Creates a new keyring-based token cache.
    pub fn new() -> Self {
        Self
    }

    /// Creates a keyring entry for the given key components.
    fn create_entry(
        &self,
        host: &str,
        username: &str,
        token_type: TokenType,
    ) -> Result<keyring::Entry, TokenCacheError> {
        let key = build_cache_key(host, username, token_type);
        keyring::Entry::new(KEYRING_SERVICE_NAME, &key).map_err(|e| {
            TokenCacheError::KeystoreAccess {
                source: Box::new(e),
                location: Location::default(),
            }
        })
    }
}

impl Default for KeyringTokenCache {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenCache for KeyringTokenCache {
    fn add_token(
        &self,
        host: &str,
        username: &str,
        token_type: TokenType,
        token_value: &str,
    ) -> Result<(), TokenCacheError> {
        validate_key_components(host, username)?;

        let entry = self.create_entry(host, username, token_type)?;
        entry
            .set_password(token_value)
            .map_err(|e| TokenCacheError::TokenStorage {
                source: Box::new(e),
                location: Location::default(),
            })
    }

    fn remove_token(
        &self,
        host: &str,
        username: &str,
        token_type: TokenType,
    ) -> Result<(), TokenCacheError> {
        validate_key_components(host, username)?;

        let entry = self.create_entry(host, username, token_type)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // Token doesn't exist, which is fine for removal
            Err(e) => Err(TokenCacheError::TokenRemoval {
                source: Box::new(e),
                location: Location::default(),
            }),
        }
    }

    fn get_token(
        &self,
        host: &str,
        username: &str,
        token_type: TokenType,
    ) -> Result<Option<String>, TokenCacheError> {
        validate_key_components(host, username)?;

        let entry = self.create_entry(host, username, token_type)?;
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(TokenCacheError::TokenRetrieval {
                source: Box::new(e),
                location: Location::default(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to generate unique test keys to avoid conflicts between tests
    fn unique_test_key(prefix: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{}_{}", prefix, timestamp)
    }

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

            assert_eq!(
                build_cache_key(host, user, TokenType::IdToken),
                "host.example.com;user1;ID_TOKEN"
            );
            assert_eq!(
                build_cache_key(host, user, TokenType::MfaToken),
                "host.example.com;user1;MFA_TOKEN"
            );
            assert_eq!(
                build_cache_key(host, user, TokenType::OAuthAccessToken),
                "host.example.com;user1;OAUTH_ACCESS_TOKEN"
            );
            assert_eq!(
                build_cache_key(host, user, TokenType::OAuthRefreshToken),
                "host.example.com;user1;OAUTH_REFRESH_TOKEN"
            );
            assert_eq!(
                build_cache_key(host, user, TokenType::DpopBundledAccessToken),
                "host.example.com;user1;DPOP_BUNDLED_ACCESS_TOKEN"
            );
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
        }
    }

    mod validation_tests {
        use super::*;

        #[test]
        fn validate_key_components_rejects_empty_host() {
            let result = validate_key_components("", "username");
            assert!(result.is_err());
            if let Err(TokenCacheError::InvalidKeyFormat { key, .. }) = result {
                assert!(key.contains(";username"));
            } else {
                panic!("Expected InvalidKeyFormat error");
            }
        }

        #[test]
        fn validate_key_components_rejects_empty_username() {
            let result = validate_key_components("host.example.com", "");
            assert!(result.is_err());
            if let Err(TokenCacheError::InvalidKeyFormat { key, .. }) = result {
                assert!(key.contains("host.example.com;"));
            } else {
                panic!("Expected InvalidKeyFormat error");
            }
        }

        #[test]
        fn validate_key_components_accepts_valid_inputs() {
            let result = validate_key_components("host.example.com", "testuser");
            assert!(result.is_ok());
        }
    }

    // Platform-specific keyring tests
    // These tests interact with the actual system keyring and require platform support
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    mod keyring_integration_tests {
        use super::*;

        fn cleanup_test_token(cache: &KeyringTokenCache, host: &str, username: &str) {
            // Clean up all token types for the test key
            let token_types = [
                TokenType::IdToken,
                TokenType::MfaToken,
                TokenType::OAuthAccessToken,
                TokenType::OAuthRefreshToken,
                TokenType::DpopBundledAccessToken,
            ];
            for token_type in token_types {
                let _ = cache.remove_token(host, username, token_type);
            }
        }

        #[test]
        fn add_and_get_token_succeeds() {
            let cache = KeyringTokenCache::new();
            let host = unique_test_key("test_host");
            let username = unique_test_key("test_user");
            let token_value = "test_token_value_12345";

            // Ensure clean state
            cleanup_test_token(&cache, &host, &username);

            // Add token
            let add_result = cache.add_token(&host, &username, TokenType::IdToken, token_value);
            assert!(
                add_result.is_ok(),
                "Failed to add token: {:?}",
                add_result.err()
            );

            // Get token
            let get_result = cache.get_token(&host, &username, TokenType::IdToken);
            assert!(
                get_result.is_ok(),
                "Failed to get token: {:?}",
                get_result.err()
            );
            assert_eq!(get_result.unwrap(), Some(token_value.to_string()));

            // Cleanup
            cleanup_test_token(&cache, &host, &username);
        }

        #[test]
        fn get_nonexistent_token_returns_none() {
            let cache = KeyringTokenCache::new();
            let host = unique_test_key("nonexistent_host");
            let username = unique_test_key("nonexistent_user");

            // Ensure token doesn't exist
            cleanup_test_token(&cache, &host, &username);

            let result = cache.get_token(&host, &username, TokenType::MfaToken);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None);
        }

        #[test]
        fn remove_existing_token_succeeds() {
            let cache = KeyringTokenCache::new();
            let host = unique_test_key("remove_test_host");
            let username = unique_test_key("remove_test_user");
            let token_value = "token_to_be_removed";

            // Setup: add a token first
            cleanup_test_token(&cache, &host, &username);
            cache
                .add_token(&host, &username, TokenType::OAuthAccessToken, token_value)
                .expect("Setup failed: could not add token");

            // Verify token exists
            let get_result = cache.get_token(&host, &username, TokenType::OAuthAccessToken);
            assert_eq!(get_result.unwrap(), Some(token_value.to_string()));

            // Remove token
            let remove_result = cache.remove_token(&host, &username, TokenType::OAuthAccessToken);
            assert!(
                remove_result.is_ok(),
                "Failed to remove token: {:?}",
                remove_result.err()
            );

            // Verify token is gone
            let get_after_remove = cache.get_token(&host, &username, TokenType::OAuthAccessToken);
            assert_eq!(get_after_remove.unwrap(), None);

            // Cleanup
            cleanup_test_token(&cache, &host, &username);
        }

        #[test]
        fn remove_nonexistent_token_succeeds() {
            let cache = KeyringTokenCache::new();
            let host = unique_test_key("remove_nonexistent_host");
            let username = unique_test_key("remove_nonexistent_user");

            // Ensure token doesn't exist
            cleanup_test_token(&cache, &host, &username);

            // Removing a non-existent token should succeed (idempotent operation)
            let result = cache.remove_token(&host, &username, TokenType::OAuthRefreshToken);
            assert!(
                result.is_ok(),
                "Remove nonexistent should succeed: {:?}",
                result.err()
            );
        }

        #[test]
        fn overwrite_token_succeeds() {
            let cache = KeyringTokenCache::new();
            let host = unique_test_key("overwrite_test_host");
            let username = unique_test_key("overwrite_test_user");
            let original_token = "original_token_value";
            let updated_token = "updated_token_value";

            // Ensure clean state
            cleanup_test_token(&cache, &host, &username);

            // Add original token
            cache
                .add_token(
                    &host,
                    &username,
                    TokenType::DpopBundledAccessToken,
                    original_token,
                )
                .expect("Failed to add original token");

            // Verify original value
            let first_get = cache.get_token(&host, &username, TokenType::DpopBundledAccessToken);
            assert_eq!(first_get.unwrap(), Some(original_token.to_string()));

            // Overwrite with new value
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

            // Verify updated value
            let second_get = cache.get_token(&host, &username, TokenType::DpopBundledAccessToken);
            assert_eq!(second_get.unwrap(), Some(updated_token.to_string()));

            // Cleanup
            cleanup_test_token(&cache, &host, &username);
        }

        #[test]
        fn different_token_types_stored_separately() {
            let cache = KeyringTokenCache::new();
            let host = unique_test_key("multi_type_host");
            let username = unique_test_key("multi_type_user");
            let id_token = "id_token_value";
            let mfa_token = "mfa_token_value";

            // Ensure clean state
            cleanup_test_token(&cache, &host, &username);

            // Add different token types
            cache
                .add_token(&host, &username, TokenType::IdToken, id_token)
                .expect("Failed to add ID token");
            cache
                .add_token(&host, &username, TokenType::MfaToken, mfa_token)
                .expect("Failed to add MFA token");

            // Verify each token type has its own value
            let get_id = cache.get_token(&host, &username, TokenType::IdToken);
            let get_mfa = cache.get_token(&host, &username, TokenType::MfaToken);

            assert_eq!(get_id.unwrap(), Some(id_token.to_string()));
            assert_eq!(get_mfa.unwrap(), Some(mfa_token.to_string()));

            // Cleanup
            cleanup_test_token(&cache, &host, &username);
        }

        #[test]
        fn add_token_with_empty_host_fails() {
            let cache = KeyringTokenCache::new();

            let result = cache.add_token("", "username", TokenType::IdToken, "token_value");
            assert!(result.is_err());
            assert!(matches!(
                result,
                Err(TokenCacheError::InvalidKeyFormat { .. })
            ));
        }

        #[test]
        fn add_token_with_empty_username_fails() {
            let cache = KeyringTokenCache::new();

            let result = cache.add_token("host.example.com", "", TokenType::IdToken, "token_value");
            assert!(result.is_err());
            assert!(matches!(
                result,
                Err(TokenCacheError::InvalidKeyFormat { .. })
            ));
        }

        #[test]
        fn get_token_with_empty_host_fails() {
            let cache = KeyringTokenCache::new();

            let result = cache.get_token("", "username", TokenType::IdToken);
            assert!(result.is_err());
            assert!(matches!(
                result,
                Err(TokenCacheError::InvalidKeyFormat { .. })
            ));
        }

        #[test]
        fn get_token_with_empty_username_fails() {
            let cache = KeyringTokenCache::new();

            let result = cache.get_token("host.example.com", "", TokenType::IdToken);
            assert!(result.is_err());
            assert!(matches!(
                result,
                Err(TokenCacheError::InvalidKeyFormat { .. })
            ));
        }

        #[test]
        fn remove_token_with_empty_host_fails() {
            let cache = KeyringTokenCache::new();

            let result = cache.remove_token("", "username", TokenType::IdToken);
            assert!(result.is_err());
            assert!(matches!(
                result,
                Err(TokenCacheError::InvalidKeyFormat { .. })
            ));
        }

        #[test]
        fn remove_token_with_empty_username_fails() {
            let cache = KeyringTokenCache::new();

            let result = cache.remove_token("host.example.com", "", TokenType::IdToken);
            assert!(result.is_err());
            assert!(matches!(
                result,
                Err(TokenCacheError::InvalidKeyFormat { .. })
            ));
        }
    }
}
