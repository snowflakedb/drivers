//! Parameter metadata carried on [`ConfigError`].
//!
//! Adding a variant is a compile error until it is classified,
//! as there is no wildcard that silently drops those fields.

use super::ConfigError;
use super::connection_config::{ValidationCode, ValidationIssue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ConfigErrorClass {
    MissingParameter,
    InvalidParameterValue,
    #[default]
    InternalError,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ConfigErrorContext {
    pub parameter: Option<String>,
    pub parameter_value: Option<String>,
    pub validation_code: Option<ValidationCode>,
    pub class: ConfigErrorClass,
}

fn from_validation_issues(issues: &[ValidationIssue]) -> ConfigErrorContext {
    if let Some(issue) = issues
        .iter()
        .find(|issue| issue.code == ValidationCode::MissingRequired)
    {
        return ConfigErrorContext {
            parameter: Some(issue.parameter.clone()),
            parameter_value: None,
            validation_code: None,
            class: ConfigErrorClass::MissingParameter,
        };
    }
    // A WIF-conflict issue must win over blind `.first()` selection,
    // or `_is_wif_conflict` on the Python side silently misses it
    // whenever validate_settings also pushes an unrelated
    // Error-severity issue earlier in the same call.
    let chosen = issues
        .iter()
        .find(|issue| issue.code == ValidationCode::ConflictingWifParameters)
        .or_else(|| issues.first());
    match chosen {
        Some(issue) => ConfigErrorContext {
            parameter: Some(issue.parameter.clone()),
            parameter_value: None,
            validation_code: Some(issue.code),
            class: ConfigErrorClass::InvalidParameterValue,
        },
        None => ConfigErrorContext {
            parameter: None,
            parameter_value: None,
            validation_code: None,
            class: ConfigErrorClass::InvalidParameterValue,
        },
    }
}

impl ConfigError {
    pub(crate) fn exception_context(&self) -> ConfigErrorContext {
        match self {
            ConfigError::InvalidParameterValue {
                parameter, value, ..
            } => ConfigErrorContext {
                parameter: Some(parameter.clone()),
                parameter_value: Some(value.clone()),
                validation_code: None,
                class: ConfigErrorClass::InvalidParameterValue,
            },
            ConfigError::MissingParameter { parameter, .. } => ConfigErrorContext {
                parameter: Some(parameter.clone()),
                parameter_value: None,
                validation_code: None,
                class: ConfigErrorClass::MissingParameter,
            },
            ConfigError::ConflictingParameters {
                parameter, value, ..
            } => ConfigErrorContext {
                parameter: Some(parameter.clone()),
                parameter_value: Some(value.clone()),
                validation_code: None,
                class: ConfigErrorClass::InvalidParameterValue,
            },
            ConfigError::ConnectionNotFound { name, .. } => ConfigErrorContext {
                parameter: Some(format!("connection: {name}")),
                parameter_value: None,
                validation_code: None,
                class: ConfigErrorClass::MissingParameter,
            },
            ConfigError::Validation { issues, .. } => from_validation_issues(issues),
            // no wildcard - explicit empty arms
            ConfigError::ConfigFileRead { .. }
            | ConfigError::TomlParse { .. }
            | ConfigError::IniParse { .. }
            | ConfigError::IniAlreadyLoaded { .. }
            | ConfigError::InsecurePermissions { .. }
            | ConfigError::ConfigDirNotFound { .. } => ConfigErrorContext::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::connection_config::ValidationSeverity;
    use snafu::Location;

    fn loc() -> Location {
        Location::new("test", 0, 0)
    }

    fn issue(parameter: &str, code: ValidationCode) -> ValidationIssue {
        ValidationIssue {
            severity: ValidationSeverity::Error,
            parameter: parameter.to_owned(),
            message: "test".to_owned(),
            code,
        }
    }

    #[test]
    fn invalid_parameter_value_copies_name_and_value() {
        let err = ConfigError::InvalidParameterValue {
            parameter: "authenticator".to_owned(),
            value: "BAD".to_owned(),
            explanation: "nope".to_owned(),
            location: loc(),
        };
        assert_eq!(
            err.exception_context(),
            ConfigErrorContext {
                parameter: Some("authenticator".to_owned()),
                parameter_value: Some("BAD".to_owned()),
                validation_code: None,
                class: ConfigErrorClass::InvalidParameterValue,
            }
        );
    }

    #[test]
    fn missing_required_issue_wins_and_omits_validation_code() {
        let err = ConfigError::Validation {
            issues: vec![
                issue("token", ValidationCode::InvalidValue),
                issue("account", ValidationCode::MissingRequired),
            ],
            location: loc(),
        };
        let ctx = err.exception_context();
        assert_eq!(ctx.parameter.as_deref(), Some("account"));
        assert_eq!(ctx.validation_code, None);
        assert_eq!(ctx.class, ConfigErrorClass::MissingParameter);
    }

    #[test]
    fn wif_conflict_wins_over_earlier_unrelated_issue() {
        let err = ConfigError::Validation {
            issues: vec![
                issue("some_unrelated_param", ValidationCode::InvalidValue),
                issue(
                    "workload_identity_provider",
                    ValidationCode::ConflictingWifParameters,
                ),
            ],
            location: loc(),
        };
        let ctx = err.exception_context();
        assert_eq!(ctx.parameter.as_deref(), Some("workload_identity_provider"));
        assert_eq!(
            ctx.validation_code,
            Some(ValidationCode::ConflictingWifParameters)
        );
        assert_eq!(ctx.class, ConfigErrorClass::InvalidParameterValue);
    }

    #[test]
    fn connection_not_found_prefixes_parameter() {
        let err = ConfigError::ConnectionNotFound {
            name: "prod".to_owned(),
            location: loc(),
        };
        let ctx = err.exception_context();
        assert_eq!(ctx.parameter.as_deref(), Some("connection: prod"));
        assert_eq!(ctx.class, ConfigErrorClass::MissingParameter);
    }

    #[test]
    fn file_read_error_is_internal_with_no_parameter() {
        let err = ConfigError::ConfigDirNotFound { location: loc() };
        assert_eq!(err.exception_context(), ConfigErrorContext::default());
    }
}
