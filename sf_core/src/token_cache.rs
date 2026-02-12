use snafu::{Location, Snafu};

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
