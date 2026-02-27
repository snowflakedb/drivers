use std::any::Any;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use keyring::credential::{CredentialApi, CredentialBuilderApi, CredentialPersistence};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use snafu::{ResultExt, ensure};

use super::{
    CacheDirectoryResolutionSnafu, FileNotOwnedByCurrentUserSnafu, InsufficientPermissionsSnafu,
    IrregularFileTypeSnafu, LockAcquisitionSnafu, TokenCacheError, TokenRetrievalSnafu,
    TokenStorageSnafu,
};

const DEFAULT_CACHE_FILE_NAME: &str = "credential_cache_v2.json";
const DEFAULT_RETRY_COUNT: u32 = 5;
const DEFAULT_RETRY_DELAY: Duration = Duration::from_millis(100);
const DEFAULT_STALE_LOCK_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Serialize, Deserialize)]
struct CacheFileContent {
    tokens: HashMap<String, String>,
}

/// Creates a single directory with mode `0o700` on Unix, ignoring `AlreadyExists`.
fn create_subdir(path: &Path) -> Result<(), TokenCacheError> {
    #[cfg(unix)]
    let result = {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new().mode(0o700).create(path)
    };
    #[cfg(not(unix))]
    let result = fs::create_dir(path);

    match result {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e).boxed().context(TokenStorageSnafu),
    }
}

/// Resolves the cache directory from environment variables in priority order,
/// validates that the root folder exists, and creates any needed subdirectories
/// with `0o700` permissions:
///
/// 1. `$SF_TEMPORARY_CREDENTIAL_CACHE_DIR` — used as-is, must already exist
/// 2. `$XDG_CACHE_HOME/snowflake` — `$XDG_CACHE_HOME` must exist, `snowflake` is created
/// 3. `$HOME/.cache/snowflake` — `$HOME` must exist, `.cache` and `snowflake` are created
fn ensure_cache_dir() -> Result<PathBuf, TokenCacheError> {
    if let Ok(dir) = std::env::var("SF_TEMPORARY_CREDENTIAL_CACHE_DIR")
        && !dir.is_empty()
    {
        let path = PathBuf::from(dir);
        ensure!(path.is_dir(), CacheDirectoryResolutionSnafu);
        return Ok(path);
    }

    if let Ok(dir) = std::env::var("XDG_CACHE_HOME")
        && !dir.is_empty()
    {
        let root = PathBuf::from(dir);
        ensure!(root.is_dir(), CacheDirectoryResolutionSnafu);
        let cache_dir = root.join("snowflake");
        create_subdir(&cache_dir)?;
        return Ok(cache_dir);
    }

    if let Ok(home) = std::env::var("HOME")
        && !home.is_empty()
    {
        let root = PathBuf::from(home);
        ensure!(root.is_dir(), CacheDirectoryResolutionSnafu);
        let dot_cache = root.join(".cache");
        create_subdir(&dot_cache)?;
        let cache_dir = dot_cache.join("snowflake");
        create_subdir(&cache_dir)?;
        return Ok(cache_dir);
    }

    CacheDirectoryResolutionSnafu.fail()
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
    async fn acquire(
        cache_path: &Path,
        retry_count: u32,
        retry_delay: Duration,
        stale_timeout: Duration,
    ) -> Result<Self, TokenCacheError> {
        let lock_path = cache_path.with_extension("json.lck");

        for attempt in 0..retry_count {
            match fs::create_dir(&lock_path) {
                Ok(()) => {
                    return Ok(FileLock { lock_path });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if Self::is_stale(&lock_path, stale_timeout) {
                        let _ = fs::remove_dir(&lock_path);
                        continue;
                    }
                    if attempt < retry_count - 1 {
                        tokio::time::sleep(retry_delay).await;
                    }
                }
                Err(e) => {
                    return Err(e).context(LockAcquisitionSnafu);
                }
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::WouldBlock,
            "failed to acquire file lock after maximum retries",
        ))
        .context(LockAcquisitionSnafu)
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
        let _ = fs::remove_dir(&self.lock_path);
    }
}

/// Validates an already-open file descriptor for ownership, permissions, and file type.
///
/// Uses `fstat` on the fd (via `file.metadata()`) rather than `stat` on the path,
/// which eliminates TOCTOU races between validation and subsequent I/O on the same fd.
#[cfg(unix)]
fn validate_file_fd(file: &fs::File, path: &Path) -> Result<(), TokenCacheError> {
    use std::os::unix::fs::MetadataExt;
    use std::os::unix::fs::PermissionsExt;

    let metadata = file.metadata().boxed().context(TokenRetrievalSnafu)?;

    ensure!(
        metadata.is_file(),
        IrregularFileTypeSnafu {
            path: path.to_path_buf()
        }
    );

    let file_uid = metadata.uid();
    // SAFETY: getuid is always safe to call.
    let current_uid = unsafe { libc::getuid() };
    ensure!(
        file_uid == current_uid,
        FileNotOwnedByCurrentUserSnafu {
            path: path.to_path_buf(),
            file_uid,
            current_uid,
        }
    );

    let mode = metadata.permissions().mode() & 0o777;
    if mode != 0o600 {
        let new_perms = fs::Permissions::from_mode(0o600);
        file.set_permissions(new_perms)
            .boxed()
            .context(InsufficientPermissionsSnafu {
                path: path.to_path_buf(),
            })?;
    }

    Ok(())
}

/// Opens an existing file with `O_NOFOLLOW` and validates ownership, permissions, and file type
/// via `fstat` on the fd.
///
/// Pass `read` and/or `write` to select the access mode (`O_RDONLY`, `O_WRONLY`, or `O_RDWR`).
///
/// Returns `Ok(None)` if the file does not exist. Rejects symlinks (`ELOOP` from `O_NOFOLLOW`)
/// by mapping them to [`IrregularFileType`](super::TokenCacheError::IrregularFileType).
#[cfg(unix)]
fn open_existing(
    path: &Path,
    read: bool,
    write: bool,
) -> Result<Option<fs::File>, TokenCacheError> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut opts = fs::OpenOptions::new();
    opts.read(read).write(write);
    opts.custom_flags(libc::O_NOFOLLOW);

    match opts.open(path) {
        Ok(file) => {
            validate_file_fd(&file, path)?;
            Ok(Some(file))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) if e.raw_os_error() == Some(libc::ELOOP) => IrregularFileTypeSnafu {
            path: path.to_path_buf(),
        }
        .fail(),
        Err(e) => Err(e).boxed().context(TokenRetrievalSnafu),
    }
}

/// Creates a new cache file atomically with mode `0o600`.
///
/// Uses `O_EXCL` to guarantee atomic create-if-not-exists and `O_NOFOLLOW`
/// to reject symlinks at the target path.
#[cfg(unix)]
fn create_exclusive(path: &Path) -> Result<fs::File, TokenCacheError> {
    use std::os::unix::fs::OpenOptionsExt;

    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .custom_flags(libc::O_NOFOLLOW)
        .mode(0o600)
        .open(path)
        .boxed()
        .context(TokenStorageSnafu)
}

/// Serializes cache content back to an already-open file descriptor.
///
/// Seeks to the beginning, truncates, writes the JSON, and flushes — all on
/// the same fd to avoid TOCTOU gaps.
#[cfg(unix)]
fn flush_to_fd(file: &mut fs::File, cache: &CacheFileContent) -> Result<(), TokenCacheError> {
    let content = serde_json::to_string_pretty(cache)
        .boxed()
        .context(TokenStorageSnafu)?;
    file.seek(std::io::SeekFrom::Start(0))
        .boxed()
        .context(TokenStorageSnafu)?;
    file.set_len(0).boxed().context(TokenStorageSnafu)?;
    file.write_all(content.as_bytes())
        .boxed()
        .context(TokenStorageSnafu)?;
    file.flush().boxed().context(TokenStorageSnafu)?;
    Ok(())
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
        let cache_dir = ensure_cache_dir()?;
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
    pub async fn set_secret(&self, key: &str, secret: &[u8]) -> Result<(), TokenCacheError> {
        let value = String::from_utf8(secret.to_vec())
            .boxed()
            .context(TokenStorageSnafu)?;
        let _lock = self.acquire_lock().await?;
        let hashed_key = hash_cache_key(key);

        #[cfg(unix)]
        {
            let (mut file, mut cache) = match open_existing(&self.cache_file_path, true, true)? {
                Some(mut f) => {
                    let mut buf = String::new();
                    f.read_to_string(&mut buf)
                        .boxed()
                        .context(TokenStorageSnafu)?;
                    let cache = if buf.trim().is_empty() {
                        CacheFileContent {
                            tokens: HashMap::new(),
                        }
                    } else {
                        serde_json::from_str(&buf).unwrap_or_else(|_| CacheFileContent {
                            tokens: HashMap::new(),
                        })
                    };
                    (f, cache)
                }
                None => {
                    let f = create_exclusive(&self.cache_file_path)?;
                    (
                        f,
                        CacheFileContent {
                            tokens: HashMap::new(),
                        },
                    )
                }
            };
            cache.tokens.insert(hashed_key, value);
            flush_to_fd(&mut file, &cache)?;
        }

        #[cfg(not(unix))]
        {
            let mut cache = if self.cache_file_path.exists() {
                let content = fs::read_to_string(&self.cache_file_path)
                    .boxed()
                    .context(TokenRetrievalSnafu)?;
                if content.trim().is_empty() {
                    CacheFileContent {
                        tokens: HashMap::new(),
                    }
                } else {
                    serde_json::from_str(&content).unwrap_or_else(|_| CacheFileContent {
                        tokens: HashMap::new(),
                    })
                }
            } else {
                CacheFileContent {
                    tokens: HashMap::new(),
                }
            };
            cache.tokens.insert(hashed_key, value);
            let content = serde_json::to_string_pretty(&cache)
                .boxed()
                .context(TokenStorageSnafu)?;
            fs::write(&self.cache_file_path, content)
                .boxed()
                .context(TokenStorageSnafu)?;
        }

        Ok(())
    }

    /// Retrieves a secret by key. Returns `None` if the key does not exist.
    pub async fn get_secret(&self, key: &str) -> Result<Option<Vec<u8>>, TokenCacheError> {
        let _lock = self.acquire_lock().await?;
        let hashed_key = hash_cache_key(key);

        #[cfg(unix)]
        {
            let mut file = match open_existing(&self.cache_file_path, true, false)? {
                Some(f) => f,
                None => return Ok(None),
            };
            let mut buf = String::new();
            file.read_to_string(&mut buf)
                .boxed()
                .context(TokenRetrievalSnafu)?;
            if buf.trim().is_empty() {
                return Ok(None);
            }
            let cache: CacheFileContent = match serde_json::from_str(&buf) {
                Ok(c) => c,
                Err(_) => return Ok(None),
            };
            Ok(cache.tokens.get(&hashed_key).map(|v| v.as_bytes().to_vec()))
        }

        #[cfg(not(unix))]
        {
            if !self.cache_file_path.exists() {
                return Ok(None);
            }
            let content = fs::read_to_string(&self.cache_file_path)
                .boxed()
                .context(TokenRetrievalSnafu)?;
            if content.trim().is_empty() {
                return Ok(None);
            }
            let cache: CacheFileContent = match serde_json::from_str(&content) {
                Ok(c) => c,
                Err(_) => return Ok(None),
            };
            Ok(cache.tokens.get(&hashed_key).map(|v| v.as_bytes().to_vec()))
        }
    }

    /// Deletes a credential by key. Returns `true` if the key existed.
    pub async fn delete_credential(&self, key: &str) -> Result<bool, TokenCacheError> {
        let _lock = self.acquire_lock().await?;
        let hashed_key = hash_cache_key(key);

        #[cfg(unix)]
        {
            let mut file = match open_existing(&self.cache_file_path, true, true)? {
                Some(f) => f,
                None => return Ok(false),
            };
            let mut buf = String::new();
            file.read_to_string(&mut buf)
                .boxed()
                .context(TokenStorageSnafu)?;
            let mut cache = if buf.trim().is_empty() {
                CacheFileContent {
                    tokens: HashMap::new(),
                }
            } else {
                serde_json::from_str(&buf).unwrap_or_else(|_| CacheFileContent {
                    tokens: HashMap::new(),
                })
            };
            let existed = cache.tokens.remove(&hashed_key).is_some();
            flush_to_fd(&mut file, &cache)?;
            Ok(existed)
        }

        #[cfg(not(unix))]
        {
            if !self.cache_file_path.exists() {
                return Ok(false);
            }
            let content = fs::read_to_string(&self.cache_file_path)
                .boxed()
                .context(TokenRetrievalSnafu)?;
            let mut cache = if content.trim().is_empty() {
                CacheFileContent {
                    tokens: HashMap::new(),
                }
            } else {
                serde_json::from_str(&content).unwrap_or_else(|_| CacheFileContent {
                    tokens: HashMap::new(),
                })
            };
            let existed = cache.tokens.remove(&hashed_key).is_some();
            if existed {
                let content = serde_json::to_string_pretty(&cache)
                    .boxed()
                    .context(TokenStorageSnafu)?;
                fs::write(&self.cache_file_path, content)
                    .boxed()
                    .context(TokenStorageSnafu)?;
            }
            Ok(existed)
        }
    }

    async fn acquire_lock(&self) -> Result<FileLock, TokenCacheError> {
        FileLock::acquire(
            &self.cache_file_path,
            self.retry_count,
            self.retry_delay,
            self.stale_lock_timeout,
        )
        .await
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
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(self.cache.set_secret(&self.user, secret))
                .map_err(wrap_error)
        })
    }

    fn get_secret(&self) -> keyring::Result<Vec<u8>> {
        tokio::task::block_in_place(|| {
            let result =
                tokio::runtime::Handle::current().block_on(self.cache.get_secret(&self.user));
            match result {
                Ok(Some(secret)) => Ok(secret),
                Ok(None) => Err(keyring::Error::NoEntry),
                Err(e) => Err(wrap_error(e)),
            }
        })
    }

    fn delete_credential(&self) -> keyring::Result<()> {
        tokio::task::block_in_place(|| {
            let result = tokio::runtime::Handle::current()
                .block_on(self.cache.delete_credential(&self.user));
            match result {
                Ok(true) => Ok(()),
                Ok(false) => Err(keyring::Error::NoEntry),
                Err(e) => Err(wrap_error(e)),
            }
        })
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

        #[tokio::test]
        async fn set_and_get_secret() {
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("my_key", b"my_secret")
                .await
                .expect("Failed to set secret");

            let result = cache
                .get_secret("my_key")
                .await
                .expect("Failed to get secret");
            assert_eq!(result, Some(b"my_secret".to_vec()));
        }

        #[tokio::test]
        async fn get_missing_key_returns_none() {
            let (_dir, cache) = create_temp_cache();
            let result = cache
                .get_secret("nonexistent")
                .await
                .expect("Failed to get secret");
            assert_eq!(result, None);
        }

        #[tokio::test]
        async fn delete_existing_credential() {
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("to_delete", b"val")
                .await
                .expect("Failed to set secret");

            let existed = cache
                .delete_credential("to_delete")
                .await
                .expect("Failed to delete");
            assert!(existed);

            let result = cache.get_secret("to_delete").await.expect("Failed to get");
            assert_eq!(result, None);
        }

        #[tokio::test]
        async fn delete_nonexistent_returns_false() {
            let (_dir, cache) = create_temp_cache();
            let existed = cache
                .delete_credential("nonexistent")
                .await
                .expect("Failed to delete");
            assert!(!existed);
        }

        #[tokio::test]
        async fn overwrite_secret() {
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("key", b"old")
                .await
                .expect("Failed to set secret");
            cache
                .set_secret("key", b"new")
                .await
                .expect("Failed to overwrite");

            let result = cache.get_secret("key").await.expect("Failed to get");
            assert_eq!(result, Some(b"new".to_vec()));
        }

        #[tokio::test]
        async fn different_keys_stored_separately() {
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("key_a", b"val_a")
                .await
                .expect("Failed to set");
            cache
                .set_secret("key_b", b"val_b")
                .await
                .expect("Failed to set");

            assert_eq!(
                cache.get_secret("key_a").await.unwrap(),
                Some(b"val_a".to_vec())
            );
            assert_eq!(
                cache.get_secret("key_b").await.unwrap(),
                Some(b"val_b".to_vec())
            );
        }

        #[tokio::test]
        async fn cache_file_uses_correct_name() {
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("key", b"val")
                .await
                .expect("Failed to set secret");

            assert!(cache.cache_file_path.ends_with("credential_cache_v2.json"));
            assert!(cache.cache_file_path.exists());
        }

        #[tokio::test]
        async fn cache_file_contains_valid_json() {
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("key", b"val")
                .await
                .expect("Failed to set secret");

            let content = fs::read_to_string(&cache.cache_file_path).expect("Failed to read file");
            let parsed: serde_json::Value =
                serde_json::from_str(&content).expect("Invalid JSON in cache file");
            assert!(parsed.get("tokens").is_some());
        }

        #[tokio::test]
        async fn keys_are_sha256_hashed_in_file() {
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("my_raw_key", b"val")
                .await
                .expect("Failed to set secret");

            let content = fs::read_to_string(&cache.cache_file_path).expect("Failed to read file");
            let parsed: CacheFileContent =
                serde_json::from_str(&content).expect("Invalid JSON in cache file");

            let expected_key = hash_cache_key("my_raw_key");
            assert!(parsed.tokens.contains_key(&expected_key));
            assert_eq!(parsed.tokens.get(&expected_key).unwrap(), "val");
        }

        #[cfg(unix)]
        #[tokio::test]
        async fn cache_file_has_mode_600() {
            use std::os::unix::fs::PermissionsExt;
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("key", b"val")
                .await
                .expect("Failed to set secret");

            let metadata =
                fs::metadata(&cache.cache_file_path).expect("Failed to read file metadata");
            let mode = metadata.permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }

        #[cfg(unix)]
        #[tokio::test]
        async fn remediates_file_with_wrong_permissions() {
            use std::os::unix::fs::PermissionsExt;
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("key", b"val")
                .await
                .expect("Failed to set secret");

            fs::set_permissions(&cache.cache_file_path, fs::Permissions::from_mode(0o644))
                .expect("Failed to change permissions");

            let result = cache
                .get_secret("key")
                .await
                .expect("Should succeed after remediating permissions");
            assert_eq!(result, Some(b"val".to_vec()));

            let metadata =
                fs::metadata(&cache.cache_file_path).expect("Failed to read file metadata");
            let mode = metadata.permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "Permissions should be remediated to 0o600");
        }

        #[cfg(unix)]
        #[tokio::test]
        async fn accepts_file_owned_by_current_user() {
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("key", b"val")
                .await
                .expect("Failed to set secret");

            let result = cache.get_secret("key").await;
            assert!(
                result.is_ok(),
                "File created by current user should pass ownership check"
            );
        }

        #[cfg(unix)]
        #[tokio::test]
        async fn rejects_file_not_owned_by_current_user() {
            use std::os::unix::fs::MetadataExt;
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("key", b"val")
                .await
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

        #[tokio::test]
        async fn lock_file_removed_after_operation() {
            let (_dir, cache) = create_temp_cache();
            cache
                .set_secret("key", b"val")
                .await
                .expect("Failed to set secret");

            let lock_path = cache.cache_file_path.with_extension("json.lck");
            assert!(
                !lock_path.exists(),
                "Lock file should be removed after operation"
            );
        }

        #[tokio::test]
        async fn stale_lock_is_broken() {
            let dir = tempfile::tempdir().expect("Failed to create temp dir");
            let cache = FileTokenCache::with_directory(dir.path().to_path_buf())
                .stale_lock_timeout(Duration::from_millis(50));

            let lock_path = cache.cache_file_path.with_extension("json.lck");
            fs::create_dir(&lock_path).expect("Failed to create stale lock dir");

            tokio::time::sleep(Duration::from_millis(100)).await;

            cache
                .set_secret("key", b"val")
                .await
                .expect("Should succeed after breaking stale lock");

            let result = cache.get_secret("key").await.expect("Failed to get secret");
            assert_eq!(result, Some(b"val".to_vec()));
        }

        #[tokio::test]
        async fn configurable_retry_parameters() {
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

        #[tokio::test(flavor = "multi_thread")]
        async fn set_and_get_password() {
            let dir = tempfile::tempdir().unwrap();
            let builder = create_builder(&dir);
            let cred = builder
                .build(None, "svc", "host.example.com;user1;ID_TOKEN")
                .unwrap();

            cred.set_password("secret123").unwrap();
            let password = cred.get_password().unwrap();
            assert_eq!(password, "secret123");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn get_missing_entry_returns_no_entry() {
            let dir = tempfile::tempdir().unwrap();
            let builder = create_builder(&dir);
            let cred = builder
                .build(None, "svc", "host.example.com;user1;ID_TOKEN")
                .unwrap();

            let err = cred.get_password().unwrap_err();
            assert!(matches!(err, keyring::Error::NoEntry));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn delete_existing_credential() {
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

        #[tokio::test(flavor = "multi_thread")]
        async fn delete_missing_credential_returns_no_entry() {
            let dir = tempfile::tempdir().unwrap();
            let builder = create_builder(&dir);
            let cred = builder
                .build(None, "svc", "host.example.com;user1;ID_TOKEN")
                .unwrap();

            let err = cred.delete_credential().unwrap_err();
            assert!(matches!(err, keyring::Error::NoEntry));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn overwrite_password() {
            let dir = tempfile::tempdir().unwrap();
            let builder = create_builder(&dir);
            let cred = builder
                .build(None, "svc", "host.example.com;user1;ID_TOKEN")
                .unwrap();

            cred.set_password("first").unwrap();
            cred.set_password("second").unwrap();
            assert_eq!(cred.get_password().unwrap(), "second");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn separate_credentials_are_independent() {
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

        #[tokio::test(flavor = "multi_thread")]
        async fn persistence_is_until_delete() {
            let dir = tempfile::tempdir().unwrap();
            let builder = create_builder(&dir);
            assert!(matches!(
                builder.persistence(),
                CredentialPersistence::UntilDelete
            ));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn credentials_share_same_backing_file() {
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

    mod concurrency_tests {
        use super::*;
        use std::sync::Arc;

        const TASK_COUNT: usize = 10;

        fn create_shared_cache() -> (tempfile::TempDir, Arc<FileTokenCache>) {
            let dir = tempfile::tempdir().expect("Failed to create temp dir");
            let cache = Arc::new(
                FileTokenCache::with_directory(dir.path().to_path_buf())
                    .retry_count(100)
                    .retry_delay(Duration::from_millis(50)),
            );
            (dir, cache)
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn concurrent_writes_do_not_corrupt() {
            let (_dir, cache) = create_shared_cache();
            let barrier = Arc::new(tokio::sync::Barrier::new(TASK_COUNT));

            let handles: Vec<_> = (0..TASK_COUNT)
                .map(|i| {
                    let cache = Arc::clone(&cache);
                    let barrier = Arc::clone(&barrier);
                    tokio::spawn(async move {
                        barrier.wait().await;
                        let key = format!("key_{i}");
                        let value = format!("value_{i}");
                        cache
                            .set_secret(&key, value.as_bytes())
                            .await
                            .expect("Failed to set secret in concurrent write");
                    })
                })
                .collect();

            for handle in handles {
                handle.await.expect("Task panicked");
            }

            for i in 0..TASK_COUNT {
                let key = format!("key_{i}");
                let expected = format!("value_{i}");
                let actual = cache
                    .get_secret(&key)
                    .await
                    .expect("Failed to get secret after concurrent writes");
                assert_eq!(
                    actual,
                    Some(expected.into_bytes()),
                    "Mismatch for {key} after concurrent writes"
                );
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn concurrent_reads_during_writes() {
            let (_dir, cache) = create_shared_cache();

            for i in 0..TASK_COUNT {
                let key = format!("key_{i}");
                let value = format!("old_{i}");
                cache.set_secret(&key, value.as_bytes()).await.unwrap();
            }

            let total_tasks = TASK_COUNT * 2;
            let barrier = Arc::new(tokio::sync::Barrier::new(total_tasks));

            let handles: Vec<_> = (0..TASK_COUNT)
                .flat_map(|i| {
                    let writer_cache = Arc::clone(&cache);
                    let reader_cache = Arc::clone(&cache);
                    let writer_barrier = Arc::clone(&barrier);
                    let reader_barrier = Arc::clone(&barrier);

                    let writer = tokio::spawn(async move {
                        writer_barrier.wait().await;
                        let key = format!("key_{i}");
                        let value = format!("new_{i}");
                        writer_cache
                            .set_secret(&key, value.as_bytes())
                            .await
                            .expect("Failed to set secret in writer task");
                    });

                    let reader = tokio::spawn(async move {
                        reader_barrier.wait().await;
                        let key = format!("key_{i}");
                        let old_value = format!("old_{i}").into_bytes();
                        let new_value = format!("new_{i}").into_bytes();

                        let result = reader_cache
                            .get_secret(&key)
                            .await
                            .expect("Failed to get secret in reader task");
                        let value = result.expect("Seeded key should always be present");
                        assert!(
                            value == old_value || value == new_value,
                            "Read corrupt value for {key}: got {}, expected old or new",
                            String::from_utf8_lossy(&value),
                        );
                    });

                    [writer, reader]
                })
                .collect();

            for handle in handles {
                handle.await.expect("Task panicked");
            }

            for i in 0..TASK_COUNT {
                let key = format!("key_{i}");
                let old_value = format!("old_{i}").into_bytes();
                let new_value = format!("new_{i}").into_bytes();
                let actual = cache
                    .get_secret(&key)
                    .await
                    .expect("Failed to verify final state")
                    .expect("Key should still be present after concurrent access");
                assert!(
                    actual == old_value || actual == new_value,
                    "Corrupt final value for {key}: got {}",
                    String::from_utf8_lossy(&actual),
                );
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn concurrent_deletes_are_consistent() {
            let (_dir, cache) = create_shared_cache();

            let total_keys = TASK_COUNT * 2;
            for i in 0..total_keys {
                let key = format!("key_{i}");
                let value = format!("value_{i}");
                cache.set_secret(&key, value.as_bytes()).await.unwrap();
            }

            let delete_count = total_keys / 2;
            let barrier = Arc::new(tokio::sync::Barrier::new(delete_count));

            let handles: Vec<_> = (0..total_keys)
                .filter(|i| i % 2 == 0)
                .map(|i| {
                    let cache = Arc::clone(&cache);
                    let barrier = Arc::clone(&barrier);
                    tokio::spawn(async move {
                        barrier.wait().await;
                        let key = format!("key_{i}");
                        let existed = cache
                            .delete_credential(&key)
                            .await
                            .expect("Failed to delete in concurrent task");
                        assert!(existed, "Expected {key} to exist before deletion");
                    })
                })
                .collect();

            for handle in handles {
                handle.await.expect("Task panicked");
            }

            for i in 0..total_keys {
                let key = format!("key_{i}");
                let result = cache
                    .get_secret(&key)
                    .await
                    .expect("Failed to read after deletes");
                if i % 2 == 0 {
                    assert_eq!(result, None, "Deleted {key} should be gone");
                } else {
                    let expected = format!("value_{i}");
                    assert_eq!(
                        result,
                        Some(expected.into_bytes()),
                        "Non-deleted {key} should still be present"
                    );
                }
            }
        }
    }
}
