use crate::config::path_resolver::expand_tilde;
use snafu::{Location, ResultExt, Snafu};
use std::collections::HashSet;

/// Expands file names using glob patterns, returning a list of valid file paths.
///
/// A leading `~` or `~/` in the pattern is first expanded to the user's home
/// directory (matching JDBC's `expandFileNames` and Python's
/// `os.path.expanduser`); a `~` that is not the leading path segment is left
/// literal. When the home directory cannot be determined the `~` is left
/// untouched and globbed verbatim. See [`expand_tilde`] for the exact `~`
/// semantics (and its unit tests for coverage of the expansion itself).
///
/// Matches are deduplicated by canonical path: if a glob matches two entries
/// that resolve to the same real file (e.g. a symlink and its target, or two
/// symlinks to the same target), only the first is kept — matching legacy
/// JDBC's `HashSet<String>` dedup keyed on canonical path.
pub fn expand_filenames(pattern: &str) -> Result<Vec<ValidatedFilePath>, PathExpansionError> {
    let expanded = expand_tilde(pattern, dirs::home_dir().as_deref())
        .to_string_lossy()
        .into_owned();

    let mut expanded_file_paths = Vec::new();
    let mut seen_canonical_paths = HashSet::new();
    let paths = glob::glob(&expanded).context(InvalidPatternSnafu { pattern })?;

    for path in paths {
        if let Ok(p) = path {
            let validated_path = ValidatedFilePath::new(p)?;
            if seen_canonical_paths.insert(validated_path.path.clone()) {
                expanded_file_paths.push(validated_path);
            }
        } else {
            InvalidPathSnafu {
                path: "Unknown - check the glob error".to_string(),
                glob_error: path.err(),
            }
            .fail()?;
        }
    }

    Ok(expanded_file_paths)
}

pub struct ValidatedFilePath {
    pub path: String,
    pub filename: String,
}

impl ValidatedFilePath {
    pub fn new(path_buf: std::path::PathBuf) -> Result<Self, PathExpansionError> {
        // Resolve to an absolute, normalized path: CWD-relative → absolute,
        // `..` collapsed, symlinks followed. Matches legacy JDBC's
        // `file.getCanonicalPath()` and is parity with Python's `os.path.abspath`
        // for non-symlink inputs. `dunce::canonicalize` == `std::fs::canonicalize`
        // on Unix; on Windows it strips the `\\?\` verbatim prefix for ordinary
        // local-disk paths, so the common case stores a clean absolute path with
        // no verbatim prefix in the PUT result source. UNC paths (`\\server\share\...`)
        // and paths needing long-path support keep the verbatim prefix — see BD#131.
        let canonical = dunce::canonicalize(&path_buf).context(CanonicalizeSnafu {
            path: path_buf.to_string_lossy().to_string(),
        })?;

        if canonical.is_file()
            && let Some(path_str) = canonical.to_str()
            && let Some(filename) = canonical.file_name().and_then(|name| name.to_str())
        {
            return Ok(ValidatedFilePath {
                path: path_str.to_string(),
                filename: filename.to_string(),
            });
        }
        InvalidPathSnafu {
            path: canonical.to_string_lossy().to_string(),
            glob_error: None,
        }
        .fail()
    }
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
pub enum PathExpansionError {
    #[snafu(display("Pattern matched an invalid path {path}"))]
    InvalidPath {
        path: String,
        glob_error: Option<glob::GlobError>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to expand the pattern {pattern}"))]
    InvalidPattern {
        pattern: String,
        source: glob::PatternError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to canonicalize path {path}: {source}"))]
    Canonicalize {
        path: String,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn absolute_path_is_returned_unchanged() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("data.csv");
        fs::write(&file, b"").unwrap();
        let canonical = dunce::canonicalize(dir.path()).unwrap();

        let result = ValidatedFilePath::new(file).unwrap();
        assert_eq!(result.path, canonical.join("data.csv").to_str().unwrap());
        assert_eq!(result.filename, "data.csv");
    }

    #[test]
    fn dotdot_segments_are_collapsed() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        let file = dir.path().join("data.csv");
        fs::write(&file, b"").unwrap();

        // path_buf = <tempdir>/sub/../data.csv — contains a `..` segment
        let dotdot_path = sub.join("..").join("data.csv");
        let result = ValidatedFilePath::new(dotdot_path).unwrap();

        let canonical_base = dunce::canonicalize(dir.path()).unwrap();
        assert_eq!(
            result.path,
            canonical_base.join("data.csv").to_str().unwrap(),
            ".. must be collapsed to canonical form"
        );
        assert_eq!(result.filename, "data.csv");
    }

    // SNOW-3704998: symlink source uploads under target's basename (JDBC parity)
    #[cfg(unix)]
    #[test]
    fn symlink_resolves_to_target_path_and_filename() {
        let dir = tempdir().unwrap();
        let real_file = dir.path().join("real.csv");
        fs::write(&real_file, b"").unwrap();
        let link = dir.path().join("link.csv");
        std::os::unix::fs::symlink(&real_file, &link).unwrap();

        let result = ValidatedFilePath::new(link).unwrap();

        let canonical_real = dunce::canonicalize(&real_file).unwrap();
        assert_eq!(result.path, canonical_real.to_str().unwrap());
        assert_eq!(
            result.filename, "real.csv",
            "symlink uploads under target basename"
        );
    }

    #[test]
    fn directory_is_rejected() {
        let dir = tempdir().unwrap();
        let result = ValidatedFilePath::new(dir.path().to_path_buf());
        assert!(
            matches!(result, Err(PathExpansionError::InvalidPath { .. })),
            "directory must be rejected by is_file() guard"
        );
    }

    // Exercises `ValidatedFilePath::new` directly with an input that `glob`
    // itself would never produce (a literal nonexistent path yields zero
    // matches and never reaches here — see `NoFilesMatched`). Still valid
    // unit-level coverage of the guard clause itself.
    #[test]
    fn nonexistent_path_returns_canonicalize_error() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("does-not-exist.csv");
        let result = ValidatedFilePath::new(missing);
        assert!(
            matches!(result, Err(PathExpansionError::Canonicalize { .. })),
            "nonexistent path must return Canonicalize error"
        );
    }

    // The reachable equivalent through the real entry point: a dangling
    // symlink is enqueued by `glob` (its `symlink_metadata` succeeds) but
    // fails to canonicalize because its target does not exist.
    #[cfg(unix)]
    #[test]
    fn dangling_symlink_matched_by_glob_returns_canonicalize_error() {
        let dir = tempdir().unwrap();
        let missing_target = dir.path().join("does-not-exist.csv");
        let link = dir.path().join("link.csv");
        std::os::unix::fs::symlink(&missing_target, &link).unwrap();

        let pattern = dir.path().join("*.csv").to_str().unwrap().to_string();
        let result = expand_filenames(&pattern);

        assert!(
            matches!(result, Err(PathExpansionError::Canonicalize { .. })),
            "a dangling symlink matched by glob must surface a canonicalize error"
        );
    }

    #[cfg(unix)]
    #[test]
    fn glob_matching_symlink_and_target_dedupes_by_canonical_path() {
        let dir = tempdir().unwrap();
        let real_file = dir.path().join("real.csv");
        fs::write(&real_file, b"").unwrap();
        let link = dir.path().join("link.csv");
        std::os::unix::fs::symlink(&real_file, &link).unwrap();

        let pattern = dir.path().join("*.csv").to_str().unwrap().to_string();
        let result = expand_filenames(&pattern).unwrap();

        assert_eq!(
            result.len(),
            1,
            "symlink and its target canonicalize to the same file and must dedupe"
        );
        assert_eq!(result[0].filename, "real.csv");
    }
}
