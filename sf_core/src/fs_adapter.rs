use std::io;
use std::path::Path;

/// Abstraction for filesystem reads, allowing tests to inject mocks and avoid real disk access.
/// All file reads must use this trait—not `std::fs`—to allow future extensions such as:
/// - Checking file permissions and ownership
/// - Enforcing file size limits
/// - Centralized logging and error handling
pub trait FsAdapter: Send + Sync {
    fn read_to_string(&self, path: &Path) -> io::Result<String>;
}

/// Default production adapter that delegates directly to `std::fs`.
pub struct RealFs;

impl FsAdapter for RealFs {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub mod mock {
    use std::collections::HashMap;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use super::FsAdapter;

    #[derive(Default)]
    pub struct MockFs {
        files: Mutex<HashMap<PathBuf, String>>,
    }

    impl MockFs {
        pub fn new() -> Self {
            Self::default()
        }

        /// Builder-style: register a file and return `self`.
        pub fn with_file(self, path: impl Into<PathBuf>, contents: impl Into<String>) -> Self {
            self.insert(path, contents);
            self
        }

        /// Register (or overwrite) the contents at `path`.
        pub fn insert(&self, path: impl Into<PathBuf>, contents: impl Into<String>) {
            self.files
                .lock()
                .unwrap()
                .insert(path.into(), contents.into());
        }
    }

    impl FsAdapter for MockFs {
        fn read_to_string(&self, path: &Path) -> io::Result<String> {
            self.files
                .lock()
                .unwrap()
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
        }
    }
}
