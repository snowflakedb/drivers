use keyring::credential::CredentialPersistence;
use keyring::{CredentialBuilder, Entry};
use snafu::ResultExt;
use tracing::{debug, info, warn};

use crate::token_cache::file_cache::FileTokenCache;

use super::{
    CacheKey, KeystoreAccessSnafu, TokenCache, TokenCacheError, TokenRemovalSnafu,
    TokenRetrievalSnafu, TokenStorageSnafu, build_cache_key, validate_key_components,
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

    /// Creates a keyring entry for the given cache key.
    ///
    /// Validates the key and uses the output of [`build_cache_key`] verbatim
    /// as the credential name in the OS store.
    fn create_entry(&self, key: &CacheKey) -> Result<keyring::Entry, TokenCacheError> {
        validate_key_components(key)?;
        debug!("Creating secret for {:?}", key.token_type);
        let built_key = build_cache_key(key);
        self.cache
            .build(None, KEYRING_SERVICE_NAME, &built_key)
            .map(Entry::new_with_credential)
            .boxed()
            .context(KeystoreAccessSnafu)
    }
}

impl TokenCache for KeyringTokenCache {
    fn add_token(&self, key: &CacheKey, token_value: &str) -> Result<(), TokenCacheError> {
        info!("Saving secret for {:?}", key.token_type);
        let entry = self.create_entry(key)?;
        entry
            .set_password(token_value)
            .boxed()
            .context(TokenStorageSnafu)
    }

    fn remove_token(&self, key: &CacheKey) -> Result<(), TokenCacheError> {
        debug!("Removing secret for {:?}", key.token_type);
        let built_key = build_cache_key(key);
        let entry = self.create_entry(key)?;
        // TODO: SNOW-3552507
        // TEMP DIAGNOSTIC (SNOW-2314157, Windows x86 eviction regression):
        // Distinguish "actually deleted" from "backend reported NoEntry"
        // so we can tell whether `should_evict_refresh_token_when_idp_returns_invalid_grant`
        // fails because the OS credential store silently drops the delete
        // or because our default-target derivation diverges between
        // write/read and delete on `keyring v3` + windows-native. After
        // every reported-success delete, do a verify-read to catch a
        // backend that lies about deletion on the spot.
        // Remove once the root cause is confirmed.
        let delete_result = entry.delete_credential();
        match &delete_result {
            Ok(()) => info!(
                cache_key = %built_key,
                "delete_credential returned Ok for {:?}", key.token_type
            ),
            Err(keyring::Error::NoEntry) => warn!(
                cache_key = %built_key,
                "delete_credential returned NoEntry for {:?}; \
                 treating as success but credential may still be present",
                key.token_type
            ),
            Err(_) => {}
        }
        match delete_result {
            Ok(()) | Err(keyring::Error::NoEntry) => {
                let verify_entry = self.create_entry(key)?;
                match verify_entry.get_password() {
                    Ok(value) => warn!(
                        cache_key = %built_key,
                        leaked_byte_len = value.len(),
                        "post-delete verify-read FOUND credential for {:?}; \
                         keyring backend reported success but credential persists",
                        key.token_type
                    ),
                    Err(keyring::Error::NoEntry) => info!(
                        cache_key = %built_key,
                        "post-delete verify-read confirms credential gone for {:?}",
                        key.token_type
                    ),
                    Err(e) => warn!(
                        cache_key = %built_key,
                        error = %e,
                        "post-delete verify-read errored for {:?}", key.token_type
                    ),
                }
                Ok(())
            }
            Err(e) => Err(e).boxed().context(TokenRemovalSnafu),
        }
    }

    fn get_token(&self, key: &CacheKey) -> Result<Option<String>, TokenCacheError> {
        debug!("Retrieving secret for {:?}", key.token_type);
        let entry = self.create_entry(key)?;
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e).boxed().context(TokenRetrievalSnafu),
        }
    }
}
