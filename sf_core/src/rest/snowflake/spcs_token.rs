use crate::rest::snowflake::RuntimePaths;

pub(crate) fn read_spcs_token(paths: &RuntimePaths) -> Option<String> {
    if std::env::var("SNOWFLAKE_RUNNING_INSIDE_SPCS").is_err() {
        return None;
    }

    let path = &paths.spcs_token_file;
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            let trimmed = contents.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "Failed to read SPCS token file"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn spcs_token_file(content: &str) -> (tempfile::NamedTempFile, RuntimePaths) {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(f, "{content}").unwrap();
        let paths = RuntimePaths {
            spcs_token_file: f.path().to_path_buf(),
        };
        (f, paths)
    }

    #[test]
    fn returns_none_when_env_var_not_set() {
        let (_f, paths) = spcs_token_file("my-spcs-token");

        temp_env::with_var_unset("SNOWFLAKE_RUNNING_INSIDE_SPCS", || {
            assert!(read_spcs_token(&paths).is_none());
        });
    }

    #[test]
    fn reads_token_when_env_var_is_set() {
        let (_f, paths) = spcs_token_file("my-spcs-token");

        temp_env::with_var("SNOWFLAKE_RUNNING_INSIDE_SPCS", Some("true"), || {
            assert_eq!(read_spcs_token(&paths).unwrap(), "my-spcs-token");
        });
    }

    #[test]
    fn trims_whitespace_from_token() {
        let (_f, paths) = spcs_token_file("  my-token \n");

        temp_env::with_var("SNOWFLAKE_RUNNING_INSIDE_SPCS", Some("true"), || {
            assert_eq!(read_spcs_token(&paths).unwrap(), "my-token");
        });
    }

    #[test]
    fn returns_none_for_empty_file() {
        let (_f, paths) = spcs_token_file("");

        temp_env::with_var("SNOWFLAKE_RUNNING_INSIDE_SPCS", Some("true"), || {
            assert!(read_spcs_token(&paths).is_none());
        });
    }

    #[test]
    fn returns_none_when_env_set_but_file_missing() {
        let paths = RuntimePaths {
            spcs_token_file: "/nonexistent/spcs_token".into(),
        };

        temp_env::with_var("SNOWFLAKE_RUNNING_INSIDE_SPCS", Some("true"), || {
            assert!(read_spcs_token(&paths).is_none());
        });
    }
}
