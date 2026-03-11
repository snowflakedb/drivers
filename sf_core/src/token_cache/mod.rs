mod file_cache;
mod keyring_cache;

use std::path::PathBuf;

use snafu::{Location, Snafu};

pub use file_cache::{FileCredentialBuilder, FileTokenCache, install_file_credential_fallback};
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
pub trait TokenCache {
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
                // Given an empty <component> string
                // When we validate key components
                let result = validate_key_components(host, username);

                // Then an InvalidKeyFormat error should be returned
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
}
