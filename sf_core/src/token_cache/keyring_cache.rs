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

fn stores_as_utf8_blob(token_value: &str) -> bool {
    !token_value.is_empty() && token_value.is_ascii()
}

/// Decodes a Windows Credential Manager blob written either as UTF-8 bytes
/// (ASCII tokens) or as UTF-16LE (`set_password`). A blob made only of
/// non-NUL ASCII bytes is UTF-8; every other blob is UTF-16LE.
fn decode_windows_blob(bytes: Vec<u8>) -> keyring::Result<String> {
    if bytes.iter().all(|b| b.is_ascii() && *b != 0) {
        return String::from_utf8(bytes).map_err(|e| keyring::Error::BadEncoding(e.into_bytes()));
    }
    if !bytes.len().is_multiple_of(2) {
        return Err(keyring::Error::BadEncoding(bytes));
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16(&units).map_err(|_| keyring::Error::BadEncoding(bytes))
}

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
        if cfg!(windows) && stores_as_utf8_blob(token_value) {
            entry.set_secret(token_value.as_bytes())
        } else {
            entry.set_password(token_value)
        }
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
        let result = if cfg!(windows) {
            entry.get_secret().and_then(decode_windows_blob)
        } else {
            entry.get_password()
        };
        match result {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e).boxed().context(TokenRetrievalSnafu),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    use crate::token_cache::{CacheKey, TokenCache, TokenType};

    #[cfg(windows)]
    fn unique(prefix: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{prefix}_{timestamp}")
    }

    #[cfg(windows)]
    fn oauth_key(snowflake: &str, username: &str) -> CacheKey {
        CacheKey {
            token_type: TokenType::OAuthAccessToken,
            idp: snowflake.to_string(),
            snowflake: snowflake.to_string(),
            username: username.to_string(),
            role: String::new(),
        }
    }

    #[test]
    fn decodes_ascii_blob_as_utf8() {
        assert_eq!(decode_windows_blob(b"abc123".to_vec()).unwrap(), "abc123");
    }

    #[test]
    fn decodes_utf16le_blob() {
        let blob: Vec<u8> = "abc".encode_utf16().flat_map(u16::to_le_bytes).collect();
        assert_eq!(decode_windows_blob(blob).unwrap(), "abc");
        let blob: Vec<u8> = "zażółć".encode_utf16().flat_map(u16::to_le_bytes).collect();
        assert_eq!(decode_windows_blob(blob).unwrap(), "zażółć");
    }

    #[test]
    fn decodes_empty_blob_as_empty_string() {
        assert_eq!(decode_windows_blob(Vec::new()).unwrap(), "");
    }

    #[test]
    fn rejects_malformed_blob() {
        assert!(matches!(
            decode_windows_blob(vec![0x61, 0x00, 0x62]),
            Err(keyring::Error::BadEncoding(_))
        ));
        assert!(matches!(
            decode_windows_blob(vec![0x00, 0xD8]),
            Err(keyring::Error::BadEncoding(_))
        ));
    }

    #[test]
    fn only_non_empty_ascii_is_stored_as_utf8_blob() {
        assert!(stores_as_utf8_blob("token"));
        assert!(!stores_as_utf8_blob(""));
        assert!(!stores_as_utf8_blob("zażółć"));
    }

    #[test]
    #[cfg(windows)]
    fn stores_ascii_oauth_token_larger_than_utf16_limit() {
        let cache = KeyringTokenCache::new().expect("cache");
        let snowflake = unique("utf8_host");
        let username = unique("utf8_user");
        let key = oauth_key(&snowflake, &username);
        let token = "a".repeat(1583);
        let _ = cache.remove_token(&key);

        cache
            .add_token(&key, &token)
            .expect("ascii token should fit as UTF-8 bytes");
        assert_eq!(cache.get_token(&key).expect("read"), Some(token.clone()));

        cache.remove_token(&key).expect("remove");
        assert_eq!(cache.get_token(&key).expect("read after remove"), None);
    }

    #[test]
    #[cfg(windows)]
    fn reads_legacy_utf16_password_entry() {
        let cache = KeyringTokenCache::new().expect("cache");
        let snowflake = unique("legacy_host");
        let username = unique("legacy_user");
        let key = oauth_key(&snowflake, &username);
        let token = "legacy_utf16_token";
        let _ = cache.remove_token(&key);

        cache
            .create_entry(&key)
            .expect("legacy entry")
            .set_password(token)
            .expect("legacy write");
        assert_eq!(
            cache.get_token(&key).expect("read"),
            Some(token.to_string())
        );

        cache.remove_token(&key).expect("remove");
        assert_eq!(cache.get_token(&key).expect("read after remove"), None);
    }

    #[test]
    #[cfg(windows)]
    fn ascii_write_overwrites_legacy_utf16_entry() {
        let cache = KeyringTokenCache::new().expect("cache");
        let snowflake = unique("replace_host");
        let username = unique("replace_user");
        let key = oauth_key(&snowflake, &username);
        let _ = cache.remove_token(&key);

        let entry = cache.create_entry(&key).expect("entry");
        entry.set_password("old").expect("legacy write");
        cache.add_token(&key, "new_ascii").expect("utf8 write");
        assert_eq!(
            cache.get_token(&key).expect("read"),
            Some("new_ascii".to_string())
        );
        assert_eq!(entry.get_secret().expect("raw"), b"new_ascii".to_vec());

        cache.remove_token(&key).expect("remove");
    }

    #[test]
    #[cfg(windows)]
    fn non_ascii_token_round_trips_as_utf16() {
        let cache = KeyringTokenCache::new().expect("cache");
        let snowflake = unique("utf16_host");
        let username = unique("utf16_user");
        let key = oauth_key(&snowflake, &username);
        let _ = cache.remove_token(&key);

        cache.add_token(&key, "zażółć").expect("write");
        assert_eq!(
            cache.get_token(&key).expect("read"),
            Some("zażółć".to_string())
        );

        cache.remove_token(&key).expect("remove");
    }
}
