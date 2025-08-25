use snafu::{Location, ResultExt, Snafu};

/// Expands file names using glob patterns, returning a list of valid file paths
pub fn expand_filenames(pattern: &str) -> Result<Vec<ValidatedFilePath>, PathExpansionError> {
    let mut expanded_file_paths = Vec::new();
    let paths = glob::glob(pattern).context(InvalidPatternSnafu { pattern })?;

    for path in paths {
        if let Ok(p) = path {
            let validated_path = ValidatedFilePath::new(p)?;
            expanded_file_paths.push(validated_path);
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
        if path_buf.is_file()
            && let Some(path_str) = path_buf.to_str()
            && let Some(filename) = path_buf.file_name().and_then(|name| name.to_str())
        {
            return Ok(ValidatedFilePath {
                path: path_str.to_string(),
                filename: filename.to_string(),
            });
        }
        InvalidPathSnafu {
            path: path_buf.to_string_lossy().to_string(),
            glob_error: None,
        }
        .fail()
    }
}

#[derive(Snafu, Debug)]
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
}
