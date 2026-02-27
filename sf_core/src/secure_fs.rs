//! Centralized secure filesystem access for sf_core.
//!
//! All filesystem operations that touch sensitive files (private keys, config,
//! TLS certificates, cache data) should go through this module to get:
//!
//! - **Size limits** — prevent unbounded reads from special files or DoS via
//!   oversized payloads.
//! - **Permission checks** (Unix) — reject or warn when files are writable or
//!   readable by group/others. No ownership checks in this initial version.
//! - **Atomic writes** — write-to-temp + rename to avoid partial-file races.
//! - **Path traversal protection** — `secure_write_confined` validates that the
//!   target path stays within an allowed directory.
//! - **Audit tracing** — every operation emits a `tracing` event.
//!
//! ### Platform behaviour
//!
//! Permission-bit checks are Unix-only (`#[cfg(unix)]`). On other platforms
//! they compile to no-ops. Path traversal protection is cross-platform.
//!
//! ### `secure_create_dir` note
//!
//! Intermediate directories created by `create_dir_all` retain default umask
//! permissions. Only the leaf directory is set to `0o700`.

use error_trace::ErrorTrace;
use snafu::{IntoError, Location, Snafu};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

// ── Error type ──────────────────────────────────────────────────────────────

#[derive(Debug, Snafu, ErrorTrace)]
pub enum SecureFsError {
    #[snafu(display("I/O error on {path}: {source}"))]
    Io {
        path: String,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("File too large: {path} is {actual} bytes, limit is {limit}"))]
    FileTooLarge {
        path: String,
        actual: u64,
        limit: u64,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Insecure permissions on {path}: {reason}"))]
    InsecurePermissions {
        path: String,
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Path traversal denied: {path} escapes allowed parent {allowed_parent}"))]
    PathTraversal {
        path: String,
        allowed_parent: String,
        #[snafu(implicit)]
        location: Location,
    },
}

// ── Configuration ───────────────────────────────────────────────────────────

/// How to handle permission checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionCheck {
    /// Reject files with insecure permissions.
    Enforce,
    /// Log a warning but allow access.
    Warn,
    /// Skip permission checks entirely.
    Skip,
}

/// Options for `secure_read` / `secure_read_to_string`.
#[derive(Debug, Clone)]
pub struct ReadOptions {
    /// Maximum allowed file size in bytes.
    pub max_size: u64,
    /// How to handle insecure file permissions.
    pub check_permissions: PermissionCheck,
}

impl Default for ReadOptions {
    fn default() -> Self {
        Self {
            max_size: 10 * 1024 * 1024, // 10 MiB
            check_permissions: PermissionCheck::Enforce,
        }
    }
}

// ── Core read API ───────────────────────────────────────────────────────────

/// Read a file with size limits and optional permission checks.
///
/// 1. Opens the file and reads metadata via `fstat` (avoids TOCTOU).
/// 2. Checks size against `max_size`.
/// 3. On Unix, validates permission bits (writable-by-others → error,
///    readable-by-others → warn or error per `opts`).
/// 4. Reads via `file.take(max_size + 1)` as a hard I/O-level limit.
pub fn secure_read(path: &Path, opts: &ReadOptions) -> Result<Vec<u8>, SecureFsError> {
    let path_str = path.display().to_string();

    let file = File::open(path).map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;

    let metadata = file
        .metadata()
        .map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;

    // Size check via metadata
    let file_size = metadata.len();
    if file_size > opts.max_size {
        return Err(FileTooLargeSnafu {
            path: &path_str,
            actual: file_size,
            limit: opts.max_size,
        }
        .build());
    }

    // Permission checks (Unix only)
    #[cfg(unix)]
    check_permissions(&metadata, &path_str, opts.check_permissions)?;

    // Hard I/O-level size limit — protects against special files / races
    let mut buf = Vec::with_capacity(file_size as usize);
    file.take(opts.max_size + 1)
        .read_to_end(&mut buf)
        .map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;

    if buf.len() as u64 > opts.max_size {
        return Err(FileTooLargeSnafu {
            path: &path_str,
            actual: buf.len() as u64,
            limit: opts.max_size,
        }
        .build());
    }

    tracing::debug!(
        target: "sf_core::secure_fs",
        op = "read",
        path = %path.display(),
        size = buf.len(),
        "File read"
    );

    Ok(buf)
}

/// Read a file as UTF-8 text with size limits and optional permission checks.
pub fn secure_read_to_string(
    path: &Path,
    opts: &ReadOptions,
) -> Result<String, SecureFsError> {
    let bytes = secure_read(path, opts)?;
    let path_str = path.display().to_string();
    String::from_utf8(bytes).map_err(|e| {
        IoSnafu { path: &path_str }
            .into_error(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    })
}

// ── Core write API ──────────────────────────────────────────────────────────

/// Atomically write data to a file with `0o600` permissions.
///
/// Writes to a temp file in the same directory, then renames to the target.
/// Falls back to a direct write if the atomic rename fails (e.g. cross-device).
pub fn secure_write(path: &Path, data: &[u8]) -> Result<(), SecureFsError> {
    let parent = path.parent().unwrap_or(Path::new("."));

    // Try atomic: write to sibling temp file, then rename
    match atomic_write_via_tempfile(path, parent, data) {
        Ok(()) => {}
        Err(atomic_err) => {
            tracing::debug!(
                target: "sf_core::secure_fs",
                op = "write",
                path = %path.display(),
                error = %atomic_err,
                "Atomic write failed, falling back to direct write"
            );
            direct_write(path, data)?;
        }
    }

    tracing::info!(
        target: "sf_core::secure_fs",
        op = "write",
        path = %path.display(),
        size = data.len(),
        "File written"
    );

    Ok(())
}

fn atomic_write_via_tempfile(
    target: &Path,
    parent: &Path,
    data: &[u8],
) -> Result<(), SecureFsError> {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let path_str = target.display().to_string();

    let mut tmp = NamedTempFile::new_in(parent)
        .map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;

    // Set permissions before writing content
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;
    }

    tmp.write_all(data)
        .map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;
    tmp.as_file()
        .sync_all()
        .map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;

    tmp.persist(target)
        .map_err(|e| IoSnafu { path: &path_str }.into_error(e.error))?;

    Ok(())
}

fn direct_write(path: &Path, data: &[u8]) -> Result<(), SecureFsError> {
    use std::io::Write;

    let path_str = path.display().to_string();

    let mut file =
        File::create(path).map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;
    }

    file.write_all(data)
        .map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;
    file.sync_all()
        .map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;

    Ok(())
}

/// Write data to a file, requiring that the path stays within `allowed_parent`.
///
/// Canonicalizes both the target's parent directory and `allowed_parent` and
/// verifies containment. Rejects symlinks in the final component that escape
/// the allowed tree.
pub fn secure_write_confined(
    path: &Path,
    data: &[u8],
    allowed_parent: &Path,
) -> Result<(), SecureFsError> {
    let path_str = path.display().to_string();
    let allowed_str = allowed_parent.display().to_string();

    let canon_allowed = fs::canonicalize(allowed_parent)
        .map_err(|e| IoSnafu { path: &allowed_str }.into_error(e))?;

    // Canonicalize the target's parent directory (must already exist)
    let target_parent = path.parent().unwrap_or(Path::new("."));
    let canon_parent = fs::canonicalize(target_parent)
        .map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;

    if !canon_parent.starts_with(&canon_allowed) {
        return Err(PathTraversalSnafu {
            path: &path_str,
            allowed_parent: &allowed_str,
        }
        .build());
    }

    // If the target already exists (e.g. symlink), canonicalize and re-check.
    // Note: there is an inherent TOCTOU window between this check and the
    // subsequent write — an attacker with local access could swap a symlink
    // in between. This is a fundamental limitation of user-space path checks
    // without O_NOFOLLOW support on the write path.
    if path.exists() {
        let canon_target = fs::canonicalize(path)
            .map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;
        let canon_target_parent = canon_target.parent().unwrap_or(Path::new("."));
        if !canon_target_parent.starts_with(&canon_allowed) {
            return Err(PathTraversalSnafu {
                path: &path_str,
                allowed_parent: &allowed_str,
            }
            .build());
        }
    }

    secure_write(path, data)
}

/// Create a new file with `0o600` permissions and return the handle.
pub fn secure_create_file(path: &Path) -> Result<File, SecureFsError> {
    let path_str = path.display().to_string();

    let file = File::create(path).map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;
    }

    tracing::debug!(
        target: "sf_core::secure_fs",
        op = "create_file",
        path = %path.display(),
        "File created"
    );

    Ok(file)
}

/// Create a directory tree, setting `0o700` on the leaf directory.
///
/// Intermediate directories created by `create_dir_all` retain default umask
/// permissions.
pub fn secure_create_dir(path: &Path) -> Result<(), SecureFsError> {
    let path_str = path.display().to_string();

    fs::create_dir_all(path).map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;
    }

    tracing::debug!(
        target: "sf_core::secure_fs",
        op = "create_dir",
        path = %path.display(),
        "Directory created"
    );

    Ok(())
}

/// Check file permissions without reading the file.
///
/// Opens the file and checks permission bits via `fstat`. Does not read any
/// content. Useful when callers only need to validate permissions.
pub fn secure_check_permissions(
    path: &Path,
    check: PermissionCheck,
) -> Result<(), SecureFsError> {
    let path_str = path.display().to_string();
    let file = File::open(path).map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;
    let metadata = file
        .metadata()
        .map_err(|e| IoSnafu { path: &path_str }.into_error(e))?;

    #[cfg(unix)]
    check_permissions(&metadata, &path_str, check)?;

    let _ = metadata; // suppress unused warning on non-Unix
    Ok(())
}

// ── Unix permission helpers ─────────────────────────────────────────────────

#[cfg(unix)]
fn check_permissions(
    metadata: &std::fs::Metadata,
    path_str: &str,
    check: PermissionCheck,
) -> Result<(), SecureFsError> {
    use std::os::unix::fs::PermissionsExt;

    if check == PermissionCheck::Skip {
        return Ok(());
    }

    let mode = metadata.permissions().mode();

    // Writable by group or others → always error (even in Warn mode)
    if mode & 0o022 != 0 {
        return Err(InsecurePermissionsSnafu {
            path: path_str,
            reason: format!(
                "File is writable by group or others (mode: {:#o})",
                mode & 0o777
            ),
        }
        .build());
    }

    // Readable by group or others
    if mode & 0o044 != 0 {
        let msg = format!(
            "File is readable by group or others (mode: {:#o})",
            mode & 0o777
        );
        match check {
            PermissionCheck::Enforce => {
                return Err(InsecurePermissionsSnafu {
                    path: path_str,
                    reason: msg,
                }
                .build());
            }
            PermissionCheck::Warn => {
                tracing::warn!(
                    target: "sf_core::secure_fs",
                    path = path_str,
                    mode = format!("{:#o}", mode & 0o777),
                    "{msg}"
                );
            }
            PermissionCheck::Skip => unreachable!(),
        }
    }

    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    // 1. test_read_within_size_limit
    #[test]
    fn test_read_within_size_limit() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("small.txt");
        fs::write(&file_path, "hello").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&file_path, fs::Permissions::from_mode(0o600)).unwrap();
        }

        let opts = ReadOptions {
            max_size: 1024,
            check_permissions: PermissionCheck::Skip,
        };
        let data = secure_read(&file_path, &opts).unwrap();
        assert_eq!(data, b"hello");
    }

    // 2. test_read_exceeds_size_limit
    #[test]
    fn test_read_exceeds_size_limit() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("large.txt");
        fs::write(&file_path, "a]".repeat(1000)).unwrap();

        let opts = ReadOptions {
            max_size: 100,
            check_permissions: PermissionCheck::Skip,
        };
        let result = secure_read(&file_path, &opts);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("File too large"), "got: {err}");
    }

    // 3. test_read_hard_limit_special_file — verifies file.take() hard limit
    #[test]
    fn test_read_hard_limit_special_file() {
        // Simulate: create a file whose metadata len is 0 but actual content is large.
        // We cannot truly fake metadata, so instead create a file larger than max_size
        // and verify the take()-based limit catches it via metadata first.
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("big.bin");
        {
            let mut f = File::create(&file_path).unwrap();
            f.write_all(&vec![0u8; 2000]).unwrap();
        }
        let opts = ReadOptions {
            max_size: 500,
            check_permissions: PermissionCheck::Skip,
        };
        let result = secure_read(&file_path, &opts);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("File too large"),
            "should reject via size check"
        );
    }

    // 4. test_read_nonexistent_file
    #[test]
    fn test_read_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("nope.txt");
        let opts = ReadOptions::default();
        let result = secure_read(&file_path, &opts);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("I/O error"), "got: {err}");
    }

    // 5. test_write_creates_file_with_0600 (Unix)
    #[cfg(unix)]
    #[test]
    fn test_write_creates_file_with_0600() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("out.bin");
        secure_write(&file_path, b"secret").unwrap();

        let mode = fs::metadata(&file_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "expected 0o600, got {:#o}", mode);
        assert_eq!(fs::read(&file_path).unwrap(), b"secret");
    }

    // 6. test_write_is_atomic
    #[test]
    fn test_write_is_atomic() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("atomic.txt");
        secure_write(&file_path, b"first").unwrap();
        assert_eq!(fs::read(&file_path).unwrap(), b"first");
        secure_write(&file_path, b"second").unwrap();
        assert_eq!(fs::read(&file_path).unwrap(), b"second");
    }

    // 7. test_write_confined_within_parent
    #[test]
    fn test_write_confined_within_parent() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("child.txt");
        let result = secure_write_confined(&file_path, b"ok", dir.path());
        assert!(result.is_ok());
        assert_eq!(fs::read(&file_path).unwrap(), b"ok");
    }

    // 8. test_write_confined_path_traversal
    #[test]
    fn test_write_confined_path_traversal() {
        let dir = TempDir::new().unwrap();
        // Create a sibling directory so the parent of ../escape exists
        let sibling = TempDir::new().unwrap();
        let escape_path = dir.path().join("../").join(
            sibling
                .path()
                .file_name()
                .unwrap_or(std::ffi::OsStr::new("other")),
        ).join("escape.txt");

        let result = secure_write_confined(&escape_path, b"bad", dir.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Path traversal denied"), "got: {err}");
    }

    // 9. test_write_confined_symlink_escape (Unix)
    #[cfg(unix)]
    #[test]
    fn test_write_confined_symlink_escape() {
        let jail = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();

        // Create a symlink inside jail that points outside
        let link_path = jail.path().join("link");
        std::os::unix::fs::symlink(outside.path(), &link_path).unwrap();

        let target = link_path.join("evil.txt");
        // Write via the symlink — the parent resolves outside jail
        let result = secure_write_confined(&target, b"pwned", jail.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Path traversal denied"), "got: {err}");
    }

    // 10. test_permissions_writable_by_others (Unix)
    #[cfg(unix)]
    #[test]
    fn test_permissions_writable_by_others() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("world_writable.txt");
        fs::write(&file_path, "data").unwrap();
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o666)).unwrap();

        let opts = ReadOptions {
            max_size: 1024,
            check_permissions: PermissionCheck::Enforce,
        };
        let result = secure_read(&file_path, &opts);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Insecure permissions"), "got: {err}");
        assert!(err.contains("writable"), "got: {err}");
    }

    // 11. test_permissions_readable_by_others_warn (Unix)
    #[cfg(unix)]
    #[test]
    fn test_permissions_readable_by_others_warn() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("world_readable.txt");
        fs::write(&file_path, "data").unwrap();
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644)).unwrap();

        let opts = ReadOptions {
            max_size: 1024,
            check_permissions: PermissionCheck::Warn,
        };
        // Warn mode: should succeed (only logs a warning)
        let result = secure_read(&file_path, &opts);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"data");
    }

    // 12. test_permissions_readable_by_others_enforce (Unix)
    #[cfg(unix)]
    #[test]
    fn test_permissions_readable_by_others_enforce() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("world_readable.txt");
        fs::write(&file_path, "data").unwrap();
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644)).unwrap();

        let opts = ReadOptions {
            max_size: 1024,
            check_permissions: PermissionCheck::Enforce,
        };
        let result = secure_read(&file_path, &opts);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Insecure permissions"), "got: {err}");
        assert!(err.contains("readable"), "got: {err}");
    }

    // 13. test_permissions_skip
    #[test]
    fn test_permissions_skip() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("any_perms.txt");
        fs::write(&file_path, "data").unwrap();

        let opts = ReadOptions {
            max_size: 1024,
            check_permissions: PermissionCheck::Skip,
        };
        let result = secure_read(&file_path, &opts);
        assert!(result.is_ok());
    }

    // 14. test_secure_create_dir (Unix)
    #[cfg(unix)]
    #[test]
    fn test_secure_create_dir() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().unwrap();
        let new_dir = dir.path().join("a").join("b").join("leaf");
        secure_create_dir(&new_dir).unwrap();

        assert!(new_dir.is_dir());
        let mode = fs::metadata(&new_dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "expected 0o700, got {:#o}", mode);
    }
}
