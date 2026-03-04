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
