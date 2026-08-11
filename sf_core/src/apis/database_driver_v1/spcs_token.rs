use std::path::Path;

use crate::env_vars;
use crate::fs_adapter::FsAdapter;
use crate::sensitive::SensitiveString;

const DEFAULT_SPCS_TOKEN_PATH: &str = "/snowflake/session/spcs_token";

pub(crate) fn read_spcs_token(fs: &dyn FsAdapter) -> Option<SensitiveString> {
    std::env::var_os(env_vars::SNOWFLAKE_RUNNING_INSIDE_SPCS)?;

    let path = Path::new(DEFAULT_SPCS_TOKEN_PATH);

    match fs.read_to_string(path) {
        Ok(contents) => {
            let trimmed = contents.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(SensitiveString::from(trimmed))
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
        temp_env::with_var_unset(env_vars::SNOWFLAKE_RUNNING_INSIDE_SPCS, || {
            assert!(read_spcs_token(&fs).is_none());
        });
    }

    #[test]
    fn returns_trimmed_token_when_env_var_is_set() {
        let fs = MockFs::new().with_file(DEFAULT_SPCS_TOKEN_PATH, "  my-spcs-token \n");
        temp_env::with_var(
            env_vars::SNOWFLAKE_RUNNING_INSIDE_SPCS,
            Some("true"),
            || {
                assert_eq!(
                    read_spcs_token(&fs).unwrap().reveal().as_str(),
                    "my-spcs-token"
                );
            },
        );
    }

    #[test]
    fn returns_none_for_empty_file() {
        let fs = MockFs::new().with_file(DEFAULT_SPCS_TOKEN_PATH, "");
        temp_env::with_var(
            env_vars::SNOWFLAKE_RUNNING_INSIDE_SPCS,
            Some("true"),
            || {
                assert!(read_spcs_token(&fs).is_none());
            },
        );
    }

    #[test]
    fn returns_none_when_env_set_but_file_missing() {
        let fs = MockFs::new();
        temp_env::with_var(
            env_vars::SNOWFLAKE_RUNNING_INSIDE_SPCS,
            Some("true"),
            || {
                assert!(read_spcs_token(&fs).is_none());
            },
        );
    }

    #[test]
    fn should_redact_spcs_token_in_debug_output() {
        let fs = MockFs::new().with_file(DEFAULT_SPCS_TOKEN_PATH, "secret-spcs-token");
        temp_env::with_var(
            env_vars::SNOWFLAKE_RUNNING_INSIDE_SPCS,
            Some("true"),
            || {
                let token = read_spcs_token(&fs).unwrap();
                let debug_output = format!("{token:?}");
                assert!(!debug_output.contains("secret-spcs-token"));
                assert!(debug_output.contains("****"));
            },
        );
    }
}
