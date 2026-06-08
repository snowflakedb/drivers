use std::path::Path;

use crate::fs_adapter::FsAdapter;

const DEFAULT_SPCS_TOKEN_PATH: &str = "/snowflake/session/spcs_token";

pub(crate) fn read_spcs_token(fs: &dyn FsAdapter) -> Option<String> {
    std::env::var_os("SNOWFLAKE_RUNNING_INSIDE_SPCS")?;

    let path = Path::new(DEFAULT_SPCS_TOKEN_PATH);

    match fs.read_to_string(path) {
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
    use crate::fs_adapter::mock::MockFs;

    #[test]
    fn returns_none_when_env_var_not_set() {
        let fs = MockFs::new().with_file(DEFAULT_SPCS_TOKEN_PATH, "my-spcs-token");
        temp_env::with_var_unset("SNOWFLAKE_RUNNING_INSIDE_SPCS", || {
            assert!(read_spcs_token(&fs).is_none());
        });
    }

    #[test]
    fn returns_trimmed_token_when_env_var_is_set() {
        let fs = MockFs::new().with_file(DEFAULT_SPCS_TOKEN_PATH, "  my-spcs-token \n");
        temp_env::with_var("SNOWFLAKE_RUNNING_INSIDE_SPCS", Some("true"), || {
            assert_eq!(read_spcs_token(&fs).unwrap(), "my-spcs-token");
        });
    }

    #[test]
    fn returns_none_for_empty_file() {
        let fs = MockFs::new().with_file(DEFAULT_SPCS_TOKEN_PATH, "");
        temp_env::with_var("SNOWFLAKE_RUNNING_INSIDE_SPCS", Some("true"), || {
            assert!(read_spcs_token(&fs).is_none());
        });
    }

    #[test]
    fn returns_none_when_env_set_but_file_missing() {
        let fs = MockFs::new();
        temp_env::with_var("SNOWFLAKE_RUNNING_INSIDE_SPCS", Some("true"), || {
            assert!(read_spcs_token(&fs).is_none());
        });
    }
}
