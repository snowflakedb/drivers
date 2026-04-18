//! Abstraction layer for disk reads within `DatabaseDriverV1`.
//!
//! This module defines the interface for all filesystem reads performed by the
//! driver. Its primary purpose is to allow integration tests to inject a
//! [`MockDiskReader`] and avoid actual filesystem access—for example, to supply
//! canned data for paths such as `/etc/hosts` or simulated private key files.
//!
//! The [`DiskReader`] trait is deliberately minimal for now but is expected
//! to evolve with additional safety and auditing features that should apply
//! uniformly to all disk reads, such as:
//!
//! - Verifying file ownership and permissions before reading;
//! - Enforcing a file size limit;
//! - Centralized logging and error handling.
//!
//! Important: To ensure all future safeguards are applied consistently, production code
//! should never use [`std::fs`] directly. All file reads must go through a [`DiskReader`].
//!
//! In normal operation, a [`DiskReader`] instance is managed by
//! [`crate::apis::database_driver_v1::DatabaseDriverV1`].
//! For proper testability, all subcomponents of `DatabaseDriverV1` which
//! need to read from disk should receive a reference to the `DiskReader` via
//! dependency injection, ensuring they always use the correct mechanism.

use std::io;
use std::path::Path;

pub trait DiskReader: Send + Sync {
    fn read_to_string(&self, path: &Path) -> io::Result<String>;
}

#[derive(Default)]
pub struct RealDiskReader;

impl DiskReader for RealDiskReader {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }
}

#[cfg(test)]
pub use mock::MockDiskReader;

#[cfg(test)]
mod mock {
    use super::DiskReader;
    use std::collections::HashMap;
    use std::io;
    use std::path::{Path, PathBuf};

    /// In-memory [`DiskReader`] used by tests.
    ///
    /// Construct with [`MockDiskReader::new`] and register canned responses
    /// via [`MockDiskReader::with_file`]. Reads for unregistered paths
    /// return [`io::ErrorKind::NotFound`], matching the behavior production
    /// code already expects from [`std::fs::read_to_string`].
    #[derive(Default)]
    pub struct MockDiskReader {
        files: HashMap<PathBuf, String>,
    }

    impl MockDiskReader {
        pub fn new() -> Self {
            Self::default()
        }

        #[must_use]
        pub fn with_file(mut self, path: impl Into<PathBuf>, contents: impl Into<String>) -> Self {
            self.files.insert(path.into(), contents.into());
            self
        }
    }

    impl DiskReader for MockDiskReader {
        fn read_to_string(&self, path: &Path) -> io::Result<String> {
            self.files.get(path).cloned().ok_or_else(|| {
                let display = path.display();
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("MockDiskReader: no file registered for {display}"),
                )
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_returns_registered_contents() {
        let reader = MockDiskReader::new()
            .with_file("/etc/hosts", "127.0.0.1 localhost\n")
            .with_file("/etc/resolv.conf", "nameserver 1.1.1.1\n");

        let hosts = reader
            .read_to_string(Path::new("/etc/hosts"))
            .expect("hosts entry should exist");
        assert_eq!(hosts, "127.0.0.1 localhost\n");

        let resolv = reader
            .read_to_string(Path::new("/etc/resolv.conf"))
            .expect("resolv entry should exist");
        assert_eq!(resolv, "nameserver 1.1.1.1\n");
    }

    #[test]
    fn mock_returns_not_found_for_unknown_paths() {
        let reader = MockDiskReader::new();
        let err = reader
            .read_to_string(Path::new("/does/not/exist"))
            .expect_err("unknown path should error");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}
