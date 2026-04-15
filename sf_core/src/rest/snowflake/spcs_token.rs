use std::path::PathBuf;

pub(crate) fn read_spcs_token() -> Option<String> {
    std::env::var_os("SNOWFLAKE_RUNNING_INSIDE_SPCS")?;

    let path = test_overrides::SPCS_TOKEN_PATH
        .with(|cell| cell.borrow().clone())
        .unwrap_or_else(|| PathBuf::from("/snowflake/session/spcs_token"));

    match std::fs::read_to_string(&path) {
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

pub mod test_overrides {
    use std::cell::RefCell;
    use std::path::PathBuf;

    thread_local! {
        pub static SPCS_TOKEN_PATH: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    }

    /// RAII guard that restores the previous `SPCS_TOKEN_PATH` on drop.
    pub struct SpcsTokenPathGuard {
        previous: Option<PathBuf>,
    }

    impl SpcsTokenPathGuard {
        pub fn set(path: PathBuf) -> Self {
            let previous = SPCS_TOKEN_PATH.with(|cell| cell.borrow_mut().replace(path));
            Self { previous }
        }
    }

    impl Drop for SpcsTokenPathGuard {
        fn drop(&mut self) {
            SPCS_TOKEN_PATH.with(|cell| {
                *cell.borrow_mut() = self.previous.take();
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use test_overrides::SpcsTokenPathGuard;

    fn spcs_token_file(content: &str) -> (tempfile::NamedTempFile, SpcsTokenPathGuard) {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(f, "{content}").unwrap();
        let guard = SpcsTokenPathGuard::set(f.path().to_path_buf());
        (f, guard)
    }

    #[test]
    fn returns_none_when_env_var_not_set() {
        let (_f, _guard) = spcs_token_file("my-spcs-token");
        temp_env::with_var_unset("SNOWFLAKE_RUNNING_INSIDE_SPCS", || {
            assert!(read_spcs_token().is_none());
        });
    }

    #[test]
    fn returns_trimmed_token_when_env_var_is_set() {
        let (_f, _guard) = spcs_token_file("  my-spcs-token \n");
        temp_env::with_var("SNOWFLAKE_RUNNING_INSIDE_SPCS", Some("true"), || {
            assert_eq!(read_spcs_token().unwrap(), "my-spcs-token");
        });
    }

    #[test]
    fn returns_none_for_empty_file() {
        let (_f, _guard) = spcs_token_file("");
        temp_env::with_var("SNOWFLAKE_RUNNING_INSIDE_SPCS", Some("true"), || {
            assert!(read_spcs_token().is_none());
        });
    }

    #[test]
    fn returns_none_when_env_set_but_file_missing() {
        let _guard = SpcsTokenPathGuard::set("/nonexistent/spcs_token".into());
        temp_env::with_var("SNOWFLAKE_RUNNING_INSIDE_SPCS", Some("true"), || {
            assert!(read_spcs_token().is_none());
        });
    }
}
