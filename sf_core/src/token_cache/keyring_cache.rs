use std::sync::OnceLock;

use snafu::ResultExt;
use tracing::{debug, info};

use super::file_cache::install_file_credential_fallback;
use super::{
    CacheDirectoryResolutionSnafu, KeystoreAccessSnafu, TokenCache, TokenCacheError,
    TokenRemovalSnafu, TokenRetrievalSnafu, TokenStorageSnafu, TokenType, build_cache_key,
    validate_key_components,
};

const KEYRING_SERVICE_NAME: &str = "snowflake_credential_cache";

/// Result of the one-time fallback installation attempt.
static FALLBACK_INIT: OnceLock<Result<(), String>> = OnceLock::new();

/// A token cache implementation using the system keyring.
///
/// This implementation uses the `keyring` crate to store tokens securely
/// in the platform-specific credential store:
/// - macOS: Keychain
/// - Windows: Credential Manager
/// - Linux: Secret Service (via D-Bus) or kernel keyutils
///
/// On platforms where the keyring does not provide persistent storage,
/// a file-based credential backend is automatically installed as a
/// fallback on first instantiation.
pub struct KeyringTokenCache;

impl KeyringTokenCache {
    /// Creates a new keyring-based token cache.
    ///
    /// On the first call, this checks whether the platform keyring provides
    /// persistent storage. If not, the file-based credential backend is
    /// installed as a transparent fallback so that all subsequent
    /// `keyring::Entry` operations are backed by the local cache file.
    pub fn new() -> Result<Self, TokenCacheError> {
        let result = FALLBACK_INIT
            .get_or_init(|| install_file_credential_fallback().map_err(|e| format!("{e}")));
        if let Err(error_msg) = result {
            debug!("Failed to install file credential fallback: {error_msg}");
            return CacheDirectoryResolutionSnafu.fail();
        }
        Ok(Self)
    }

    /// Creates a keyring entry for the given key components.
    fn create_entry(
        &self,
        host: &str,
        username: &str,
        token_type: TokenType,
    ) -> Result<keyring::Entry, TokenCacheError> {
        validate_key_components(host, username)?;
        debug!("Creating secret for {token_type:?}");
        let key = build_cache_key(host, username, token_type);
        keyring::Entry::new(KEYRING_SERVICE_NAME, &key)
            .boxed()
            .context(KeystoreAccessSnafu)
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
        info!("Saving secret for {token_type:?}");

        let entry = self.create_entry(host, username, token_type)?;
        entry
            .set_password(token_value)
            .boxed()
            .context(TokenStorageSnafu)
    }

    fn remove_token(
        &self,
        host: &str,
        username: &str,
        token_type: TokenType,
    ) -> Result<(), TokenCacheError> {
        validate_key_components(host, username)?;
        debug!("Removing secret for {token_type:?}");
        let entry = self.create_entry(host, username, token_type)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e).boxed().context(TokenRemovalSnafu),
        }
    }

    fn get_token(
        &self,
        host: &str,
        username: &str,
        token_type: TokenType,
    ) -> Result<Option<String>, TokenCacheError> {
        validate_key_components(host, username)?;
        debug!("Retrieving secret for {token_type:?}");

        let entry = self.create_entry(host, username, token_type)?;
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e).boxed().context(TokenRetrievalSnafu),
        }
    }
}

#[cfg(test)]
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
mod tests {
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

    // Scenario: Should add and get token via keyring
    //   Given a keyring-based token cache
    //   When we add a token and then retrieve it
    //   Then the retrieved token should match
    #[test]
    fn add_and_get_token_succeeds() {
        let cache = KeyringTokenCache::new().expect("Failed to create cache");
        let host = unique_test_key("test_host");
        let username = unique_test_key("test_user");
        let token_value = "test_token_value_12345";

        cleanup_test_token(&cache, &host, &username);

        let add_result = cache.add_token(&host, &username, TokenType::IdToken, token_value);
        assert!(
            add_result.is_ok(),
            "Failed to add token: {:?}",
            add_result.err()
        );

        let get_result = cache.get_token(&host, &username, TokenType::IdToken);
        assert!(
            get_result.is_ok(),
            "Failed to get token: {:?}",
            get_result.err()
        );
        assert_eq!(get_result.unwrap(), Some(token_value.to_string()));

        cleanup_test_token(&cache, &host, &username);
    }

    // Scenario: Should return none for nonexistent keyring token
    //   Given a keyring-based token cache with no stored token
    //   When we get a token
    //   Then None should be returned
    #[test]
    fn get_nonexistent_token_returns_none() {
        let cache = KeyringTokenCache::new().expect("Failed to create cache");
        let host = unique_test_key("nonexistent_host");
        let username = unique_test_key("nonexistent_user");

        cleanup_test_token(&cache, &host, &username);

        let result = cache.get_token(&host, &username, TokenType::MfaToken);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    // Scenario: Should remove existing token from keyring
    //   Given a keyring-based token cache with a stored token
    //   When we remove the token
    //   Then getting it should return None
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

        let remove_result = cache.remove_token(&host, &username, TokenType::OAuthAccessToken);
        assert!(
            remove_result.is_ok(),
            "Failed to remove token: {:?}",
            remove_result.err()
        );

        let get_after_remove = cache.get_token(&host, &username, TokenType::OAuthAccessToken);
        assert_eq!(get_after_remove.unwrap(), None);

        cleanup_test_token(&cache, &host, &username);
    }

    // Scenario: Should succeed when removing nonexistent keyring token
    //   Given a keyring-based token cache with no stored token
    //   When we remove a token
    //   Then the operation should succeed
    #[test]
    fn remove_nonexistent_token_succeeds() {
        let cache = KeyringTokenCache::new().expect("Failed to create cache");
        let host = unique_test_key("remove_nonexistent_host");
        let username = unique_test_key("remove_nonexistent_user");

        cleanup_test_token(&cache, &host, &username);

        let result = cache.remove_token(&host, &username, TokenType::OAuthRefreshToken);
        assert!(
            result.is_ok(),
            "Remove nonexistent should succeed: {:?}",
            result.err()
        );
    }

    // Scenario: Should overwrite token in keyring
    //   Given a keyring-based token cache with a stored token
    //   When we add a new value for the same key
    //   Then the new value should replace the old one
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

        let second_get = cache.get_token(&host, &username, TokenType::DpopBundledAccessToken);
        assert_eq!(second_get.unwrap(), Some(updated_token.to_string()));

        cleanup_test_token(&cache, &host, &username);
    }

    // Scenario: Should store different token types separately in keyring
    //   Given a keyring-based token cache
    //   When we store tokens of different types for the same host and user
    //   Then each type should return its own value
    #[test]
    fn different_token_types_stored_separately() {
        let cache = KeyringTokenCache::new().expect("Failed to create cache");
        let host = unique_test_key("multi_type_host");
        let username = unique_test_key("multi_type_user");
        let id_token = "id_token_value";
        let mfa_token = "mfa_token_value";

        cleanup_test_token(&cache, &host, &username);

        cache
            .add_token(&host, &username, TokenType::IdToken, id_token)
            .expect("Failed to add ID token");
        cache
            .add_token(&host, &username, TokenType::MfaToken, mfa_token)
            .expect("Failed to add MFA token");

        let get_id = cache.get_token(&host, &username, TokenType::IdToken);
        let get_mfa = cache.get_token(&host, &username, TokenType::MfaToken);

        assert_eq!(get_id.unwrap(), Some(id_token.to_string()));
        assert_eq!(get_mfa.unwrap(), Some(mfa_token.to_string()));

        cleanup_test_token(&cache, &host, &username);
    }

    // Scenario: Should fail to add keyring token with empty host
    //   Given a keyring-based token cache
    //   When we add a token with an empty host
    //   Then an InvalidKeyFormat error should be returned
    #[test]
    fn add_token_with_empty_host_fails() {
        let cache = KeyringTokenCache::new().expect("Failed to create cache");

        let result = cache.add_token("", "username", TokenType::IdToken, "token_value");
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(TokenCacheError::InvalidKeyFormat { .. })
        ));
    }

    // Scenario: Should fail to add keyring token with empty username
    //   Given a keyring-based token cache
    //   When we add a token with an empty username
    //   Then an InvalidKeyFormat error should be returned
    #[test]
    fn add_token_with_empty_username_fails() {
        let cache = KeyringTokenCache::new().expect("Failed to create cache");

        let result = cache.add_token("host.example.com", "", TokenType::IdToken, "token_value");
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(TokenCacheError::InvalidKeyFormat { .. })
        ));
    }

    // Scenario: Should fail to get keyring token with empty host
    //   Given a keyring-based token cache
    //   When we get a token with an empty host
    //   Then an InvalidKeyFormat error should be returned
    #[test]
    fn get_token_with_empty_host_fails() {
        let cache = KeyringTokenCache::new().expect("Failed to create cache");

        let result = cache.get_token("", "username", TokenType::IdToken);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(TokenCacheError::InvalidKeyFormat { .. })
        ));
    }

    // Scenario: Should fail to get keyring token with empty username
    //   Given a keyring-based token cache
    //   When we get a token with an empty username
    //   Then an InvalidKeyFormat error should be returned
    #[test]
    fn get_token_with_empty_username_fails() {
        let cache = KeyringTokenCache::new().expect("Failed to create cache");

        let result = cache.get_token("host.example.com", "", TokenType::IdToken);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(TokenCacheError::InvalidKeyFormat { .. })
        ));
    }

    // Scenario: Should fail to remove keyring token with empty host
    //   Given a keyring-based token cache
    //   When we remove a token with an empty host
    //   Then an InvalidKeyFormat error should be returned
    #[test]
    fn remove_token_with_empty_host_fails() {
        let cache = KeyringTokenCache::new().expect("Failed to create cache");

        let result = cache.remove_token("", "username", TokenType::IdToken);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(TokenCacheError::InvalidKeyFormat { .. })
        ));
    }

    // Scenario: Should fail to remove keyring token with empty username
    //   Given a keyring-based token cache
    //   When we remove a token with an empty username
    //   Then an InvalidKeyFormat error should be returned
    #[test]
    fn remove_token_with_empty_username_fails() {
        let cache = KeyringTokenCache::new().expect("Failed to create cache");

        let result = cache.remove_token("host.example.com", "", TokenType::IdToken);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(TokenCacheError::InvalidKeyFormat { .. })
        ));
    }
}
