//! Shared hardened [`OpenOptions`] builders and lock-contention detection
//! for advisory file locking via [`File::try_lock`].

use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::Path;
use std::time::SystemTime;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

/// Takes a non-blocking exclusive advisory lock, reporting contention as
/// [`io::ErrorKind::WouldBlock`].
pub(crate) fn try_lock_exclusive(file: &File) -> io::Result<()> {
    file.try_lock().map_err(|e| match e {
        TryLockError::WouldBlock => io::ErrorKind::WouldBlock.into(),
        TryLockError::Error(e) => e,
    })
}

/// Returns `true` when the error indicates the lock is held by another
/// handle/process and the caller should retry.
pub(crate) fn is_lock_contention(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::WouldBlock
}

/// Opens an existing path read-only for advisory locking.
///
/// On Unix, sets `O_NOFOLLOW` (reject symlinks) and `O_NONBLOCK` (never
/// block on FIFOs/devices).
pub(crate) fn open_read_nofollow_nonblock(path: &Path) -> io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.read(true);
    #[cfg(unix)]
    opts.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    opts.open(path)
}

/// Creates a new file exclusively (`O_EXCL` / `create_new`).
///
/// On Unix, sets `O_NOFOLLOW` and optionally the initial file mode.
#[cfg(unix)]
pub(crate) fn create_new_nofollow(path: &Path, unix_mode: Option<u32>) -> io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        opts.custom_flags(libc::O_NOFOLLOW);
        if let Some(mode) = unix_mode {
            opts.mode(mode);
        }
    }
    opts.open(path)
}

/// Opens an existing file with optional read/write access.
///
/// On Unix, sets `O_NOFOLLOW` to reject symlinks at the target path.
#[cfg(unix)]
pub(crate) fn open_existing_nofollow(path: &Path, read: bool, write: bool) -> io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.read(read).write(write);
    #[cfg(unix)]
    opts.custom_flags(libc::O_NOFOLLOW);
    opts.open(path)
}

/// Opens (or creates) a lock file for advisory locking.
///
/// Uses `create(true).truncate(false).write(true)`. On Unix, sets
/// `O_NOFOLLOW` and optionally the initial file mode.
pub(crate) fn open_lock_file(path: &Path, _unix_mode: Option<u32>) -> io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.create(true).truncate(false).write(true);
    #[cfg(unix)]
    {
        opts.custom_flags(libc::O_NOFOLLOW);
        if let Some(mode) = _unix_mode {
            opts.mode(mode);
        }
    }
    opts.open(path)
}

/// Returns the hosting filesystem's current time by creating a short-lived,
/// uniquely-named temp file in `dir` (name `{prefix}{random}`) and reading back
/// its freshly-stamped `mtime`.
///
/// The point is skew-immunity. The returned [`SystemTime`] is stamped by the
/// *same* filesystem/server that stamps every other file in that directory, so
/// comparing another file's `mtime` against this value stays within a single
/// clock domain — it cancels client/server clock skew (the classic NFS
/// problem), attribute-cache lag, and local-vs-remote drift. File creation is a
/// portable metadata operation that stamps `mtime = now` on every backend, so
/// no timestamp syscall wrapper is needed.
///
/// Security: a fresh `O_EXCL` temp with a random name (via `tempfile`) is used
/// rather than a fixed probe path so there is nothing for a co-tenant to
/// pre-plant in a shared directory. `O_EXCL` creation fails on any pre-existing
/// entry — including a squatted symlink, FIFO, or hardlink — and `tempfile`
/// retries a new name, so this never opens or writes an attacker-controlled
/// file, and never blocks on a FIFO. The temp is unlinked on drop. Any failure
/// is propagated so callers fail closed (disable the age heuristic) rather than
/// fall back to a local clock.
pub(crate) fn filesystem_now(dir: &Path, prefix: &str) -> io::Result<SystemTime> {
    let tmp = tempfile::Builder::new().prefix(prefix).tempfile_in(dir)?;
    tmp.as_file().metadata()?.modified()
}
