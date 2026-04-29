//! Per-process cache of text file contents.
//!
//! The validator repeatedly asks the same question about the same test file
//! (e.g., "find methods", "find steps in method X", "find empty steps in
//! method X", etc.). Each of those call sites previously reopened the file and
//! re-read it into a `String`, which becomes the single largest source of I/O
//! for runs that cover a multi-MB test tree.
//!
//! This module provides [`read_to_string_cached`], a drop-in replacement for
//! [`std::fs::read_to_string`] that memoizes by absolute (or otherwise unique)
//! path. We use `Arc<String>` so callers can hold an owned reference without
//! copying the underlying bytes.
//!
//! The cache lives in a `thread_local!` so it never needs locking in this
//! single-threaded CLI, and it will be dropped automatically at process exit.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

thread_local! {
    static FILE_CACHE: RefCell<HashMap<PathBuf, Arc<String>>> =
        RefCell::new(HashMap::new());
}

/// Read a file's contents as a UTF-8 string, caching the result by path.
///
/// Subsequent calls with the same path return a cheap `Arc<String>` clone
/// rather than re-reading from disk.
pub fn read_to_string_cached(path: &Path) -> std::io::Result<Arc<String>> {
    FILE_CACHE.with(|cell| {
        if let Some(existing) = cell.borrow().get(path) {
            return Ok(existing.clone());
        }
        let content = std::fs::read_to_string(path)?;
        let arc = Arc::new(content);
        cell.borrow_mut().insert(path.to_path_buf(), arc.clone());
        Ok(arc)
    })
}
