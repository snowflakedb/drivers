use std::any::Any;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use keyring::credential::{CredentialApi, CredentialBuilderApi, CredentialPersistence};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use snafu::Location;

use super::TokenCacheError;

const DEFAULT_CACHE_FILE_NAME: &str = "credential_cache_v2.json";
const DEFAULT_RETRY_COUNT: u32 = 5;
const DEFAULT_RETRY_DELAY: Duration = Duration::from_millis(100);
const DEFAULT_STALE_LOCK_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Serialize, Deserialize)]
struct CacheFileContent {
    tokens: HashMap<String, String>,
}

/// Resolves the cache directory from environment variables in priority order:
/// 1. `$SF_TEMPORARY_CREDENTIAL_CACHE_DIR`
/// 2. `$XDG_CACHE_HOME/snowflake`
/// 3. `$HOME/.cache/snowflake`
fn resolve_cache_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("SF_TEMPORARY_CREDENTIAL_CACHE_DIR")
        && !dir.is_empty()
    {
        return Some(PathBuf::from(dir));
    }

    if let Ok(dir) = std::env::var("XDG_CACHE_HOME")
        && !dir.is_empty()
    {
        return Some(PathBuf::from(dir).join("snowflake"));
    }

    if let Ok(home) = std::env::var("HOME")
        && !home.is_empty()
    {
        return Some(PathBuf::from(home).join(".cache").join("snowflake"));
    }

    None
}

/// Resolves the cache file name from `$SF_TEMPORARY_CREDENTIAL_CACHE_FILE_NAME`,
/// falling back to [`DEFAULT_CACHE_FILE_NAME`].
fn resolve_cache_file_name() -> String {
    std::env::var("SF_TEMPORARY_CREDENTIAL_CACHE_FILE_NAME")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_CACHE_FILE_NAME.to_string())
}

fn hash_cache_key(key: &str) -> String {
    let hash = Sha256::digest(key.as_bytes());
    hex::encode(hash)
}

/// RAII file lock guard that uses a `.lck` file alongside the cache file.
///
/// The lock is released (file removed) when the guard is dropped.
struct FileLock {
    lock_path: PathBuf,
}

impl FileLock {
    fn acquire(
        cache_path: &Path,
        retry_count: u32,
        retry_delay: Duration,
        stale_timeout: Duration,
    ) -> Result<Self, TokenCacheError> {
        let lock_path = cache_path.with_extension("json.lck");

        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).map_err(|e| TokenCacheError::LockAcquisition {
                source: e,
                location: Location::default(),
            })?;
        }

        for attempt in 0..retry_count {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(_) => {
                    return Ok(FileLock { lock_path });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if Self::is_stale(&lock_path, stale_timeout) {
                        let _ = fs::remove_file(&lock_path);
                        continue;
                    }
                    if attempt < retry_count - 1 {
                        std::thread::sleep(retry_delay);
                    }
                }
                Err(e) => {
                    return Err(TokenCacheError::LockAcquisition {
                        source: e,
                        location: Location::default(),
                    });
                }
            }
        }

        Err(TokenCacheError::LockAcquisition {
            source: std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "failed to acquire file lock after maximum retries",
            ),
            location: Location::default(),
        })
    }

    fn is_stale(lock_path: &Path, stale_timeout: Duration) -> bool {
        fs::metadata(lock_path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|modified| SystemTime::now().duration_since(modified).ok())
            .map(|age| age > stale_timeout)
            .unwrap_or(true)
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

/// A file-based credential store for environments where the OS keyring is unavailable.
///
/// Secrets are stored as plain text values in a JSON file keyed by the SHA-256
/// hash of the credential key. The file is protected with mode 0o600 on Unix.
///
/// This struct provides low-level file operations (`set_secret`, `get_secret`,
/// `delete_credential`) that mirror the keyring `CredentialApi` verbs, and is
/// used as the backing store for [`FileCredentialBuilder`].
pub struct FileTokenCache {
    cache_file_path: PathBuf,
    retry_count: u32,
    retry_delay: Duration,
    stale_lock_timeout: Duration,
}

impl FileTokenCache {
    /// Creates a new file-based credential store, resolving the cache directory
    /// from environment variables.
    pub fn new() -> Result<Self, TokenCacheError> {
        let cache_dir = resolve_cache_dir().ok_or(TokenCacheError::CacheDirectoryResolution {
            location: Location::default(),
        })?;
        let file_name = resolve_cache_file_name();
        Ok(Self {
            cache_file_path: cache_dir.join(file_name),
            retry_count: DEFAULT_RETRY_COUNT,
            retry_delay: DEFAULT_RETRY_DELAY,
            stale_lock_timeout: DEFAULT_STALE_LOCK_TIMEOUT,
        })
    }

    /// Creates a file-based credential store using an explicit directory.
    pub fn with_directory(cache_dir: PathBuf) -> Self {
        let file_name = resolve_cache_file_name();
        Self {
            cache_file_path: cache_dir.join(file_name),
            retry_count: DEFAULT_RETRY_COUNT,
            retry_delay: DEFAULT_RETRY_DELAY,
            stale_lock_timeout: DEFAULT_STALE_LOCK_TIMEOUT,
        }
    }

    pub fn retry_count(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }

    pub fn retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    pub fn stale_lock_timeout(mut self, timeout: Duration) -> Self {
        self.stale_lock_timeout = timeout;
        self
    }

    /// Stores a secret under the given key. The key is SHA-256 hashed before
    /// storage. The secret bytes must be valid UTF-8.
    pub fn set_secret(&self, key: &str, secret: &[u8]) -> Result<(), TokenCacheError> {
        let value =
            String::from_utf8(secret.to_vec()).map_err(|e| TokenCacheError::TokenStorage {
                source: Box::new(e),
                location: Location::default(),
            })?;
        let _lock = self.acquire_lock()?;
        let hashed_key = hash_cache_key(key);

        let mut cache = self.read_cache()?;
        cache.tokens.insert(hashed_key, value);
        self.write_cache(&cache)
    }

    /// Retrieves a secret by key. Returns `None` if the key does not exist.
    pub fn get_secret(&self, key: &str) -> Result<Option<Vec<u8>>, TokenCacheError> {
        let _lock = self.acquire_lock()?;
        let hashed_key = hash_cache_key(key);

        let cache = self.read_cache()?;
        Ok(cache.tokens.get(&hashed_key).map(|v| v.as_bytes().to_vec()))
    }

    /// Deletes a credential by key. Returns `true` if the key existed.
    pub fn delete_credential(&self, key: &str) -> Result<bool, TokenCacheError> {
        let _lock = self.acquire_lock()?;
        let hashed_key = hash_cache_key(key);

        let mut cache = self.read_cache()?;
        let existed = cache.tokens.remove(&hashed_key).is_some();
        if existed {
            self.write_cache(&cache)?;
        }
        Ok(existed)
    }

    fn acquire_lock(&self) -> Result<FileLock, TokenCacheError> {
        FileLock::acquire(
            &self.cache_file_path,
            self.retry_count,
            self.retry_delay,
            self.stale_lock_timeout,
        )
    }

    fn read_cache(&self) -> Result<CacheFileContent, TokenCacheError> {
        if !self.cache_file_path.exists() {
            return Ok(CacheFileContent {
                tokens: HashMap::new(),
            });
        }

        #[cfg(unix)]
        self.ensure_file_ownership()?;
        #[cfg(unix)]
        self.ensure_file_permissions()?;

        let content = fs::read_to_string(&self.cache_file_path).map_err(|e| {
            TokenCacheError::TokenRetrieval {
                source: Box::new(e),
                location: Location::default(),
            }
        })?;

        if content.trim().is_empty() {
            return Ok(CacheFileContent {
                tokens: HashMap::new(),
            });
        }

        serde_json::from_str(&content).map_err(|e| TokenCacheError::TokenRetrieval {
            source: Box::new(e),
            location: Location::default(),
        })
    }

    fn write_cache(&self, cache: &CacheFileContent) -> Result<(), TokenCacheError> {
        if let Some(parent) = self.cache_file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| TokenCacheError::TokenStorage {
                source: Box::new(e),
                location: Location::default(),
            })?;
        }

        let content =
            serde_json::to_string_pretty(cache).map_err(|e| TokenCacheError::TokenStorage {
                source: Box::new(e),
                location: Location::default(),
            })?;

        self.write_with_permissions(&content)
    }

    #[cfg(unix)]
    fn write_with_permissions(&self, content: &str) -> Result<(), TokenCacheError> {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&self.cache_file_path)
            .map_err(|e| TokenCacheError::TokenStorage {
                source: Box::new(e),
                location: Location::default(),
            })?;
        file.write_all(content.as_bytes())
            .map_err(|e| TokenCacheError::TokenStorage {
                source: Box::new(e),
                location: Location::default(),
            })
    }

    #[cfg(not(unix))]
    fn write_with_permissions(&self, content: &str) -> Result<(), TokenCacheError> {
        fs::write(&self.cache_file_path, content).map_err(|e| TokenCacheError::TokenStorage {
            source: Box::new(e),
            location: Location::default(),
        })
    }

    #[cfg(unix)]
    fn ensure_file_permissions(&self) -> Result<(), TokenCacheError> {
        use std::os::unix::fs::PermissionsExt;
        let metadata =
            fs::metadata(&self.cache_file_path).map_err(|e| TokenCacheError::TokenRetrieval {
                source: Box::new(e),
                location: Location::default(),
            })?;
        let mode = metadata.permissions().mode() & 0o777;
        if mode != 0o600 {
            return Err(TokenCacheError::InsufficientPermissions {
                path: self.cache_file_path.clone(),
                location: Location::default(),
            });
        }
        Ok(())
    }

    #[cfg(unix)]
    fn ensure_file_ownership(&self) -> Result<(), TokenCacheError> {
        use std::os::unix::fs::MetadataExt;
        let metadata =
            fs::metadata(&self.cache_file_path).map_err(|e| TokenCacheError::TokenRetrieval {
                source: Box::new(e),
                location: Location::default(),
            })?;
        let file_uid = metadata.uid();
        // SAFETY: getuid is always safe to call.
        let current_uid = unsafe { libc::getuid() };
        if file_uid != current_uid {
            return Err(TokenCacheError::FileNotOwnedByCurrentUser {
                path: self.cache_file_path.clone(),
                file_uid,
                current_uid,
                location: Location::default(),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Keyring credential adapter
// ---------------------------------------------------------------------------

fn wrap_error(e: TokenCacheError) -> keyring::Error {
    keyring::Error::PlatformFailure(Box::new(e))
}

/// A keyring credential backed by the file-based credential store.
///
/// Implements [`keyring::credential::CredentialApi`] by delegating storage
/// operations to a shared [`FileTokenCache`], preserving all file locking,
/// SHA-256 key hashing, and permission enforcement logic.
struct FileCredential {
    #[allow(dead_code)]
    service: String,
    user: String,
    cache: Arc<FileTokenCache>,
}

impl CredentialApi for FileCredential {
    fn set_secret(&self, secret: &[u8]) -> keyring::Result<()> {
        self.cache
            .set_secret(&self.user, secret)
            .map_err(wrap_error)
    }

    fn get_secret(&self) -> keyring::Result<Vec<u8>> {
        match self.cache.get_secret(&self.user) {
            Ok(Some(secret)) => Ok(secret),
            Ok(None) => Err(keyring::Error::NoEntry),
            Err(e) => Err(wrap_error(e)),
        }
    }

    fn delete_credential(&self) -> keyring::Result<()> {
        match self.cache.delete_credential(&self.user) {
            Ok(true) => Ok(()),
            Ok(false) => Err(keyring::Error::NoEntry),
            Err(e) => Err(wrap_error(e)),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn debug_fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FileCredential {{ user: {:?} }}", self.user)
    }
}

/// A keyring credential builder that produces file-backed credentials.
///
/// When installed via [`keyring::set_default_credential_builder`], all
/// `keyring::Entry` operations will be backed by the file-based credential
/// store with the same file locking, SHA-256 key hashing, and permission
/// enforcement as [`FileTokenCache`].
pub struct FileCredentialBuilder {
    cache: Arc<FileTokenCache>,
}

impl FileCredentialBuilder {
    pub fn new(cache: Arc<FileTokenCache>) -> Self {
        Self { cache }
    }
}

impl CredentialBuilderApi for FileCredentialBuilder {
    fn build(
        &self,
        _target: Option<&str>,
        service: &str,
        user: &str,
    ) -> keyring::Result<Box<keyring::credential::Credential>> {
        Ok(Box::new(FileCredential {
            service: service.to_string(),
            user: user.to_string(),
            cache: Arc::clone(&self.cache),
        }))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn persistence(&self) -> CredentialPersistence {
        CredentialPersistence::UntilDelete
    }
}

impl std::fmt::Debug for FileCredentialBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileCredentialBuilder").finish()
    }
}

/// Checks whether the platform keyring provides persistent storage and
/// installs the file-based credential store as a fallback if it does not.
///
/// Call once at application startup, before creating any `keyring::Entry`.
pub fn install_file_credential_fallback() -> Result<(), TokenCacheError> {
    let default_persistence = keyring::default::default_credential_builder().persistence();
    if !matches!(default_persistence, CredentialPersistence::UntilDelete) {
        let cache = Arc::new(FileTokenCache::new()?);
        let builder = FileCredentialBuilder::new(cache);
        keyring::set_default_credential_builder(Box::new(builder));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod hash_cache_key_tests {
        use super::*;

        #[test]
        fn produces_deterministic_sha256() {
            let key = "myhost.snowflake.com;testuser;ID_TOKEN";
            let hash1 = hash_cache_key(key);
            let hash2 = hash_cache_key(key);
            assert_eq!(hash1, hash2);
            assert_eq!(hash1.len(), 64);
        }

        #[test]
        fn different_keys_produce_different_hashes() {
            let hash1 = hash_cache_key("host1;user1;ID_TOKEN");
            let hash2 = hash_cache_key("host2;user1;ID_TOKEN");
            assert_ne!(hash1, hash2);
        }
    }

    mod file_token_cache_tests {
        use super::*;

        fn create_temp_cache() -> (tempfile::TempDir, FileTokenCache) {
            let dir = tempfile::tempdir().expect("Failed to create temp dir");
            let cache = FileTokenCache::with_directory(dir.path().to_path_buf());
            (dir, cache)
        }

        #[test]
        fn set_and_get_secret() {
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("my_key", b"my_secret")
                .expect("Failed to set secret");

            let result = cache.get_secret("my_key").expect("Failed to get secret");
            assert_eq!(result, Some(b"my_secret".to_vec()));
        }

        #[test]
        fn get_missing_key_returns_none() {
            let (_dir, cache) = create_temp_cache();
            let result = cache
                .get_secret("nonexistent")
                .expect("Failed to get secret");
            assert_eq!(result, None);
        }

        #[test]
        fn delete_existing_credential() {
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("to_delete", b"val")
                .expect("Failed to set secret");

            let existed = cache
                .delete_credential("to_delete")
                .expect("Failed to delete");
            assert!(existed);

            let result = cache.get_secret("to_delete").expect("Failed to get");
            assert_eq!(result, None);
        }

        #[test]
        fn delete_nonexistent_returns_false() {
            let (_dir, cache) = create_temp_cache();
            let existed = cache
                .delete_credential("nonexistent")
                .expect("Failed to delete");
            assert!(!existed);
        }

        #[test]
        fn overwrite_secret() {
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("key", b"old")
                .expect("Failed to set secret");
            cache
                .set_secret("key", b"new")
                .expect("Failed to overwrite");

            let result = cache.get_secret("key").expect("Failed to get");
            assert_eq!(result, Some(b"new".to_vec()));
        }

        #[test]
        fn different_keys_stored_separately() {
            let (_dir, cache) = create_temp_cache();
            cache.set_secret("key_a", b"val_a").expect("Failed to set");
            cache.set_secret("key_b", b"val_b").expect("Failed to set");

            assert_eq!(cache.get_secret("key_a").unwrap(), Some(b"val_a".to_vec()));
            assert_eq!(cache.get_secret("key_b").unwrap(), Some(b"val_b".to_vec()));
        }

        #[test]
        fn cache_file_uses_correct_name() {
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("key", b"val")
                .expect("Failed to set secret");

            assert!(cache.cache_file_path.ends_with("credential_cache_v2.json"));
            assert!(cache.cache_file_path.exists());
        }

        #[test]
        fn cache_file_contains_valid_json() {
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("key", b"val")
                .expect("Failed to set secret");

            let content = fs::read_to_string(&cache.cache_file_path).expect("Failed to read file");
            let parsed: serde_json::Value =
                serde_json::from_str(&content).expect("Invalid JSON in cache file");
            assert!(parsed.get("tokens").is_some());
        }

        #[test]
        fn keys_are_sha256_hashed_in_file() {
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("my_raw_key", b"val")
                .expect("Failed to set secret");

            let content = fs::read_to_string(&cache.cache_file_path).expect("Failed to read file");
            let parsed: CacheFileContent =
                serde_json::from_str(&content).expect("Invalid JSON in cache file");

            let expected_key = hash_cache_key("my_raw_key");
            assert!(parsed.tokens.contains_key(&expected_key));
            assert_eq!(parsed.tokens.get(&expected_key).unwrap(), "val");
        }

        #[cfg(unix)]
        #[test]
        fn cache_file_has_mode_600() {
            use std::os::unix::fs::PermissionsExt;
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("key", b"val")
                .expect("Failed to set secret");

            let metadata =
                fs::metadata(&cache.cache_file_path).expect("Failed to read file metadata");
            let mode = metadata.permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }

        #[cfg(unix)]
        #[test]
        fn rejects_file_with_wrong_permissions() {
            use std::os::unix::fs::PermissionsExt;
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("key", b"val")
                .expect("Failed to set secret");

            fs::set_permissions(&cache.cache_file_path, fs::Permissions::from_mode(0o644))
                .expect("Failed to change permissions");

            let result = cache.get_secret("key");
            assert!(matches!(
                result,
                Err(TokenCacheError::InsufficientPermissions { .. })
            ));
        }

        #[cfg(unix)]
        #[test]
        fn accepts_file_owned_by_current_user() {
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("key", b"val")
                .expect("Failed to set secret");

            let result = cache.get_secret("key");
            assert!(
                result.is_ok(),
                "File created by current user should pass ownership check"
            );
        }

        #[cfg(unix)]
        #[test]
        fn rejects_file_not_owned_by_current_user() {
            use std::os::unix::fs::MetadataExt;
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("key", b"val")
                .expect("Failed to set secret");

            let metadata = fs::metadata(&cache.cache_file_path).unwrap();
            let current_uid = unsafe { libc::getuid() };
            assert_eq!(
                metadata.uid(),
                current_uid,
                "Temp file should be owned by current user — \
                 negative ownership test requires root to chown and is skipped"
            );
        }

        #[test]
        fn lock_file_removed_after_operation() {
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("key", b"val")
                .expect("Failed to set secret");

            let lock_path = cache.cache_file_path.with_extension("json.lck");
            assert!(
                !lock_path.exists(),
                "Lock file should be removed after operation"
            );
        }

        #[test]
        fn stale_lock_is_broken() {
            let dir = tempfile::tempdir().expect("Failed to create temp dir");
            let cache = FileTokenCache::with_directory(dir.path().to_path_buf())
                .stale_lock_timeout(Duration::from_millis(50));

            let lock_path = cache.cache_file_path.with_extension("json.lck");
            if let Some(parent) = lock_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&lock_path, "stale").expect("Failed to create stale lock");

            std::thread::sleep(Duration::from_millis(100));

            cache
                .set_secret("key", b"val")
                .expect("Should succeed after breaking stale lock");

            let result = cache.get_secret("key").expect("Failed to get secret");
            assert_eq!(result, Some(b"val".to_vec()));
        }

        #[test]
        fn configurable_retry_parameters() {
            let dir = tempfile::tempdir().expect("Failed to create temp dir");
            let cache = FileTokenCache::with_directory(dir.path().to_path_buf())
                .retry_count(10)
                .retry_delay(Duration::from_millis(50))
                .stale_lock_timeout(Duration::from_secs(30));

            assert_eq!(cache.retry_count, 10);
            assert_eq!(cache.retry_delay, Duration::from_millis(50));
            assert_eq!(cache.stale_lock_timeout, Duration::from_secs(30));
        }
    }

    mod file_credential_adapter_tests {
        use super::*;

        fn create_builder(dir: &tempfile::TempDir) -> FileCredentialBuilder {
            let cache = Arc::new(FileTokenCache::with_directory(dir.path().to_path_buf()));
            FileCredentialBuilder::new(cache)
        }

        #[test]
        fn set_and_get_password() {
            let dir = tempfile::tempdir().unwrap();
            let builder = create_builder(&dir);
            let cred = builder
                .build(None, "svc", "host.example.com;user1;ID_TOKEN")
                .unwrap();

            cred.set_password("secret123").unwrap();
            let password = cred.get_password().unwrap();
            assert_eq!(password, "secret123");
        }

        #[test]
        fn get_missing_entry_returns_no_entry() {
            let dir = tempfile::tempdir().unwrap();
            let builder = create_builder(&dir);
            let cred = builder
                .build(None, "svc", "host.example.com;user1;ID_TOKEN")
                .unwrap();

            let err = cred.get_password().unwrap_err();
            assert!(matches!(err, keyring::Error::NoEntry));
        }

        #[test]
        fn delete_existing_credential() {
            let dir = tempfile::tempdir().unwrap();
            let builder = create_builder(&dir);
            let cred = builder
                .build(None, "svc", "host.example.com;user1;MFA_TOKEN")
                .unwrap();

            cred.set_password("to_delete").unwrap();
            cred.delete_credential().unwrap();

            let err = cred.get_password().unwrap_err();
            assert!(matches!(err, keyring::Error::NoEntry));
        }

        #[test]
        fn delete_missing_credential_returns_no_entry() {
            let dir = tempfile::tempdir().unwrap();
            let builder = create_builder(&dir);
            let cred = builder
                .build(None, "svc", "host.example.com;user1;ID_TOKEN")
                .unwrap();

            let err = cred.delete_credential().unwrap_err();
            assert!(matches!(err, keyring::Error::NoEntry));
        }

        #[test]
        fn overwrite_password() {
            let dir = tempfile::tempdir().unwrap();
            let builder = create_builder(&dir);
            let cred = builder
                .build(None, "svc", "host.example.com;user1;ID_TOKEN")
                .unwrap();

            cred.set_password("first").unwrap();
            cred.set_password("second").unwrap();
            assert_eq!(cred.get_password().unwrap(), "second");
        }

        #[test]
        fn separate_credentials_are_independent() {
            let dir = tempfile::tempdir().unwrap();
            let builder = create_builder(&dir);
            let cred1 = builder
                .build(None, "svc", "host.example.com;user1;ID_TOKEN")
                .unwrap();
            let cred2 = builder
                .build(None, "svc", "host.example.com;user1;MFA_TOKEN")
                .unwrap();

            cred1.set_password("id_val").unwrap();
            cred2.set_password("mfa_val").unwrap();

            assert_eq!(cred1.get_password().unwrap(), "id_val");
            assert_eq!(cred2.get_password().unwrap(), "mfa_val");
        }

        #[test]
        fn persistence_is_until_delete() {
            let dir = tempfile::tempdir().unwrap();
            let builder = create_builder(&dir);
            assert!(matches!(
                builder.persistence(),
                CredentialPersistence::UntilDelete
            ));
        }

        #[test]
        fn credentials_share_same_backing_file() {
            let dir = tempfile::tempdir().unwrap();
            let builder = create_builder(&dir);

            let cred_write = builder
                .build(None, "svc", "host.example.com;user1;ID_TOKEN")
                .unwrap();
            cred_write.set_password("shared_val").unwrap();

            let cred_read = builder
                .build(None, "svc", "host.example.com;user1;ID_TOKEN")
                .unwrap();
            assert_eq!(cred_read.get_password().unwrap(), "shared_val");
        }
    }
}
