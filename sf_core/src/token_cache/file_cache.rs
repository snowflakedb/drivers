use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use snafu::Location;

use super::{TokenCache, TokenCacheError, TokenType, build_cache_key, validate_key_components};

const CACHE_FILE_NAME: &str = "credential_cache_v2.json";
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

/// A file-based token cache for environments where the OS keyring is unavailable.
///
/// Tokens are stored as plain text values in a JSON file keyed by the SHA-256
/// hash of the cache key. The file is protected with mode 0o600 on Unix.
pub struct FileTokenCache {
    cache_file_path: PathBuf,
    retry_count: u32,
    retry_delay: Duration,
    stale_lock_timeout: Duration,
}

impl FileTokenCache {
    /// Creates a new file-based token cache, resolving the cache directory
    /// from environment variables.
    pub fn new() -> Result<Self, TokenCacheError> {
        let cache_dir = resolve_cache_dir().ok_or(TokenCacheError::CacheDirectoryResolution {
            location: Location::default(),
        })?;
        Ok(Self {
            cache_file_path: cache_dir.join(CACHE_FILE_NAME),
            retry_count: DEFAULT_RETRY_COUNT,
            retry_delay: DEFAULT_RETRY_DELAY,
            stale_lock_timeout: DEFAULT_STALE_LOCK_TIMEOUT,
        })
    }

    /// Creates a file-based token cache using an explicit directory.
    pub fn with_directory(cache_dir: PathBuf) -> Self {
        Self {
            cache_file_path: cache_dir.join(CACHE_FILE_NAME),
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
}

impl TokenCache for FileTokenCache {
    fn add_token(
        &self,
        host: &str,
        username: &str,
        token_type: TokenType,
        token_value: &str,
    ) -> Result<(), TokenCacheError> {
        validate_key_components(host, username)?;
        let _lock = self.acquire_lock()?;

        let key = build_cache_key(host, username, token_type);
        let hashed_key = hash_cache_key(&key);

        let mut cache = self.read_cache()?;
        cache.tokens.insert(hashed_key, token_value.to_string());
        self.write_cache(&cache)
    }

    fn remove_token(
        &self,
        host: &str,
        username: &str,
        token_type: TokenType,
    ) -> Result<(), TokenCacheError> {
        validate_key_components(host, username)?;
        let _lock = self.acquire_lock()?;

        let key = build_cache_key(host, username, token_type);
        let hashed_key = hash_cache_key(&key);

        let mut cache = self.read_cache()?;
        cache.tokens.remove(&hashed_key);
        self.write_cache(&cache)
    }

    fn get_token(
        &self,
        host: &str,
        username: &str,
        token_type: TokenType,
    ) -> Result<Option<String>, TokenCacheError> {
        validate_key_components(host, username)?;
        let _lock = self.acquire_lock()?;

        let key = build_cache_key(host, username, token_type);
        let hashed_key = hash_cache_key(&key);

        let cache = self.read_cache()?;
        Ok(cache.tokens.get(&hashed_key).cloned())
    }
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
            assert_eq!(hash1.len(), 64); // SHA-256 hex = 64 chars
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
        fn add_and_get_token() {
            let (_dir, cache) = create_temp_cache();
            cache
                .add_token("host.example.com", "user1", TokenType::IdToken, "my_token")
                .expect("Failed to add token");

            let result = cache
                .get_token("host.example.com", "user1", TokenType::IdToken)
                .expect("Failed to get token");
            assert_eq!(result, Some("my_token".to_string()));
        }

        #[test]
        fn get_nonexistent_token_returns_none() {
            let (_dir, cache) = create_temp_cache();
            let result = cache
                .get_token("host.example.com", "user1", TokenType::IdToken)
                .expect("Failed to get token");
            assert_eq!(result, None);
        }

        #[test]
        fn remove_existing_token() {
            let (_dir, cache) = create_temp_cache();
            cache
                .add_token("host.example.com", "user1", TokenType::MfaToken, "tok123")
                .expect("Failed to add token");

            cache
                .remove_token("host.example.com", "user1", TokenType::MfaToken)
                .expect("Failed to remove token");

            let result = cache
                .get_token("host.example.com", "user1", TokenType::MfaToken)
                .expect("Failed to get token");
            assert_eq!(result, None);
        }

        #[test]
        fn remove_nonexistent_token_succeeds() {
            let (_dir, cache) = create_temp_cache();
            let result = cache.remove_token("host.example.com", "user1", TokenType::IdToken);
            assert!(result.is_ok());
        }

        #[test]
        fn overwrite_token() {
            let (_dir, cache) = create_temp_cache();
            cache
                .add_token("host.example.com", "user1", TokenType::IdToken, "old_tok")
                .expect("Failed to add token");
            cache
                .add_token("host.example.com", "user1", TokenType::IdToken, "new_tok")
                .expect("Failed to overwrite token");

            let result = cache
                .get_token("host.example.com", "user1", TokenType::IdToken)
                .expect("Failed to get token");
            assert_eq!(result, Some("new_tok".to_string()));
        }

        #[test]
        fn different_token_types_stored_separately() {
            let (_dir, cache) = create_temp_cache();
            cache
                .add_token("host.example.com", "user1", TokenType::IdToken, "id_val")
                .expect("Failed to add ID token");
            cache
                .add_token("host.example.com", "user1", TokenType::MfaToken, "mfa_val")
                .expect("Failed to add MFA token");

            let id = cache
                .get_token("host.example.com", "user1", TokenType::IdToken)
                .expect("Failed to get ID token");
            let mfa = cache
                .get_token("host.example.com", "user1", TokenType::MfaToken)
                .expect("Failed to get MFA token");

            assert_eq!(id, Some("id_val".to_string()));
            assert_eq!(mfa, Some("mfa_val".to_string()));
        }

        #[test]
        fn empty_host_rejected() {
            let (_dir, cache) = create_temp_cache();
            let result = cache.add_token("", "user1", TokenType::IdToken, "val");
            assert!(matches!(
                result,
                Err(TokenCacheError::InvalidKeyFormat { .. })
            ));
        }

        #[test]
        fn empty_username_rejected() {
            let (_dir, cache) = create_temp_cache();
            let result = cache.add_token("host.example.com", "", TokenType::IdToken, "val");
            assert!(matches!(
                result,
                Err(TokenCacheError::InvalidKeyFormat { .. })
            ));
        }

        #[test]
        fn cache_file_uses_correct_name() {
            let (_dir, cache) = create_temp_cache();
            cache
                .add_token("host.example.com", "user1", TokenType::IdToken, "val")
                .expect("Failed to add token");

            assert!(cache.cache_file_path.ends_with("credential_cache_v2.json"));
            assert!(cache.cache_file_path.exists());
        }

        #[test]
        fn cache_file_contains_valid_json() {
            let (_dir, cache) = create_temp_cache();
            cache
                .add_token("host.example.com", "user1", TokenType::IdToken, "val")
                .expect("Failed to add token");

            let content = fs::read_to_string(&cache.cache_file_path).expect("Failed to read file");
            let parsed: serde_json::Value =
                serde_json::from_str(&content).expect("Invalid JSON in cache file");
            assert!(parsed.get("tokens").is_some());
        }

        #[test]
        fn keys_are_sha256_hashed_in_file() {
            let (_dir, cache) = create_temp_cache();
            cache
                .add_token("host.example.com", "user1", TokenType::IdToken, "val")
                .expect("Failed to add token");

            let content = fs::read_to_string(&cache.cache_file_path).expect("Failed to read file");
            let parsed: CacheFileContent =
                serde_json::from_str(&content).expect("Invalid JSON in cache file");

            let expected_key = hash_cache_key(&build_cache_key(
                "host.example.com",
                "user1",
                TokenType::IdToken,
            ));
            assert!(parsed.tokens.contains_key(&expected_key));
            assert_eq!(parsed.tokens.get(&expected_key).unwrap(), "val");
        }

        #[cfg(unix)]
        #[test]
        fn cache_file_has_mode_600() {
            use std::os::unix::fs::PermissionsExt;
            let (_dir, cache) = create_temp_cache();
            cache
                .add_token("host.example.com", "user1", TokenType::IdToken, "val")
                .expect("Failed to add token");

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
                .add_token("host.example.com", "user1", TokenType::IdToken, "val")
                .expect("Failed to add token");

            fs::set_permissions(&cache.cache_file_path, fs::Permissions::from_mode(0o644))
                .expect("Failed to change permissions");

            let result = cache.get_token("host.example.com", "user1", TokenType::IdToken);
            assert!(matches!(
                result,
                Err(TokenCacheError::InsufficientPermissions { .. })
            ));
        }

        #[test]
        fn lock_file_removed_after_operation() {
            let (_dir, cache) = create_temp_cache();
            cache
                .add_token("host.example.com", "user1", TokenType::IdToken, "val")
                .expect("Failed to add token");

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
                .add_token("host.example.com", "user1", TokenType::IdToken, "val")
                .expect("Should succeed after breaking stale lock");

            let result = cache
                .get_token("host.example.com", "user1", TokenType::IdToken)
                .expect("Failed to get token");
            assert_eq!(result, Some("val".to_string()));
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
}
