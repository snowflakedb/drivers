use keyring::credential::CredentialPersistence;
use keyring::{CredentialBuilder, Entry};
use snafu::ResultExt;
use tracing::{debug, info};

use crate::token_cache::file_cache::FileTokenCache;

use super::{
    KeystoreAccessSnafu, TokenCache, TokenCacheError, TokenRemovalSnafu, TokenRetrievalSnafu,
    TokenStorageSnafu, TokenType, build_cache_key, validate_key_components,
};

const KEYRING_SERVICE_NAME: &str = "snowflake_credential_cache";

/// A token cache implementation using the system keyring.
///
/// This implementation uses the `keyring` crate to store tokens securely
/// in the platform-specific credential store:
/// - macOS: Keychain
/// - Windows: Credential Manager
/// - Linux: Secret Service (via D-Bus) or kernel keyutils
///
/// On platforms where the keyring does not provide persistent storage,
/// a file-based credential backend is used as a fallback.
pub struct KeyringTokenCache {
    cache: Box<CredentialBuilder>,
}

impl KeyringTokenCache {
    /// Creates a new keyring-based token cache.
    ///
    /// Checks whether the platform keyring provides persistent storage.
    /// If not, a file-based credential backend is used as a fallback.
    pub fn new() -> Result<Self, TokenCacheError> {
        let default_builder = keyring::default::default_credential_builder();
        let cache = if !matches!(
            default_builder.persistence(),
            CredentialPersistence::UntilDelete
        ) {
            let cache = FileTokenCache::new()?;
            Box::new(cache)
        } else {
            default_builder
        };
        Ok(Self { cache })
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
        self.cache
            .build(None, KEYRING_SERVICE_NAME, &key)
            .map(Entry::new_with_credential)
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
