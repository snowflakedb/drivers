use std::fs;
use std::path::Path;

use snafu::OptionExt;

use crate::config::param_names::{TOKEN, TOKEN_FILE_PATH};
use crate::config::settings::Settings;
use crate::config::toml_loader::{FilePermissionCheck, check_file_permissions};
use crate::config::{ConfigError, InvalidParameterValueSnafu, MissingParameterSnafu};
use crate::sensitive::SensitiveString;

/// Whether `token` or `token_file_path` is set to a non-empty value.
pub(super) fn has_bearer_token(settings: &dyn Settings) -> bool {
    settings
        .get_string(TOKEN.as_str())
        .is_some_and(|value| !value.is_empty())
        || settings
            .get_string(TOKEN_FILE_PATH.as_str())
            .is_some_and(|value| !value.is_empty())
}

/// Resolve an optional bearer token from `token` / `token_file_path`.
///
/// When `token_file_path` is set, the file is read and used even if `token` is
/// also present — matching legacy snowflake-connector-python.
pub(super) fn read_optional_bearer_token(
    settings: &dyn Settings,
) -> Result<Option<SensitiveString>, ConfigError> {
    if let Some(path) = settings
        .get_string(TOKEN_FILE_PATH.as_str())
        .filter(|value| !value.is_empty())
    {
        return Ok(Some(read_token_file(&path, settings)?));
    }

    Ok(settings
        .get_string(TOKEN.as_str())
        .filter(|value| !value.is_empty())
        .map(SensitiveString::from))
}

/// Resolve a required bearer token from `token` / `token_file_path`.
pub(super) fn read_required_bearer_token(
    settings: &dyn Settings,
) -> Result<SensitiveString, ConfigError> {
    read_optional_bearer_token(settings)?.context(MissingParameterSnafu {
        parameter: format!("'{TOKEN}' (or '{TOKEN_FILE_PATH}')"),
    })
}

fn read_token_file(
    token_file_path: &str,
    settings: &dyn Settings,
) -> Result<SensitiveString, ConfigError> {
    let permission_check =
        if settings.get_bool("unsafe_skip_config_file_permissions_check") == Some(true) {
            FilePermissionCheck::UnsafeDisabled
        } else {
            FilePermissionCheck::Enabled
        };

    let path = Path::new(token_file_path);
    // Same order as `private_key_file` / `load_toml_file`: only permission-check
    // files that exist so a missing path surfaces a token-file read error.
    if path.try_exists().unwrap_or(false) {
        check_file_permissions(path, permission_check)?;
    }

    let contents = fs::read_to_string(path).map_err(|err| {
        InvalidParameterValueSnafu {
            parameter: String::from(TOKEN_FILE_PATH),
            value: token_file_path.to_string(),
            explanation: format!("Could not read token file: {err}"),
        }
        .build()
    })?;

    // Token files almost always end with a newline (`echo token > file`).
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return InvalidParameterValueSnafu {
            parameter: String::from(TOKEN_FILE_PATH),
            value: token_file_path.to_string(),
            explanation: "Token file is empty".to_string(),
        }
        .fail();
    }

    Ok(SensitiveString::from(trimmed.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::param_store::ParamStore;
    use crate::config::settings::Setting;
    use std::io::Write;

    fn settings_with(pairs: &[(&str, Setting)]) -> ParamStore {
        let mut store = ParamStore::with_registry_defaults();
        for (key, value) in pairs {
            store.insert((*key).to_string(), value.clone());
        }
        store
    }

    fn write_token_file(contents: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().expect("temp token file");
        file.write_all(contents.as_bytes()).expect("write token");
        file.flush().expect("flush token");
        file
    }

    #[test]
    fn has_bearer_token_from_inline_token() {
        let settings = settings_with(&[("token", Setting::String("abc".into()))]);
        assert!(has_bearer_token(&settings));
    }

    #[test]
    fn has_bearer_token_from_file_path() {
        let settings = settings_with(&[("token_file_path", Setting::String("/tmp/t".into()))]);
        assert!(has_bearer_token(&settings));
    }

    #[test]
    fn has_bearer_token_false_when_both_empty() {
        let settings = settings_with(&[]);
        assert!(!has_bearer_token(&settings));
    }

    #[test]
    fn read_optional_uses_inline_token() {
        let settings = settings_with(&[("token", Setting::String("inline-tok".into()))]);
        assert_eq!(
            read_optional_bearer_token(&settings)
                .unwrap()
                .unwrap()
                .reveal(),
            "inline-tok"
        );
    }

    #[test]
    fn read_optional_uses_file_and_trims_whitespace() {
        let file = write_token_file("file-tok\n");
        let settings = settings_with(&[(
            "token_file_path",
            Setting::String(file.path().to_str().unwrap().into()),
        )]);
        assert_eq!(
            read_optional_bearer_token(&settings)
                .unwrap()
                .unwrap()
                .reveal(),
            "file-tok"
        );
    }

    #[test]
    fn read_optional_file_wins_over_inline_token() {
        let file = write_token_file("from-file");
        let settings = settings_with(&[
            ("token", Setting::String("from-inline".into())),
            (
                "token_file_path",
                Setting::String(file.path().to_str().unwrap().into()),
            ),
        ]);
        assert_eq!(
            read_optional_bearer_token(&settings)
                .unwrap()
                .unwrap()
                .reveal(),
            "from-file"
        );
    }

    #[test]
    fn read_optional_missing_file_returns_read_error() {
        let settings = settings_with(&[(
            "token_file_path",
            Setting::String("/nonexistent/definitely_missing.token".into()),
        )]);
        let err = read_optional_bearer_token(&settings).unwrap_err();
        assert!(
            matches!(
                err,
                ConfigError::InvalidParameterValue { ref explanation, .. }
                    if explanation.contains("Could not read token file")
            ),
            "expected token-file read error, got: {err}"
        );
    }

    #[test]
    fn read_optional_empty_file_returns_error() {
        let file = write_token_file("  \n");
        let settings = settings_with(&[(
            "token_file_path",
            Setting::String(file.path().to_str().unwrap().into()),
        )]);
        let err = read_optional_bearer_token(&settings).unwrap_err();
        assert!(
            matches!(
                err,
                ConfigError::InvalidParameterValue { ref explanation, .. }
                    if explanation.contains("Token file is empty")
            ),
            "expected empty-file error, got: {err}"
        );
    }

    #[test]
    fn read_required_missing_returns_missing_parameter() {
        let settings = settings_with(&[]);
        let err = read_required_bearer_token(&settings).unwrap_err();
        assert!(
            matches!(
                err,
                ConfigError::MissingParameter { ref parameter, .. }
                    if parameter == "'token' (or 'token_file_path')"
            ),
            "expected MissingParameter naming both token sources, got: {err}"
        );
    }
}
