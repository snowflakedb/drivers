use std::collections::HashMap;
use std::fmt;

use super::error::{ApiError, InvalidArgumentSnafu};
use crate::config::ParamStore;
use crate::config::param_registry::{self, ValueType};
use crate::config::settings::Setting;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationCode {
    Unspecified,
    MissingRequired,
    InvalidType,
    InvalidValue,
    UnknownParameter,
    DeprecatedParameter,
    ConflictingParameters,
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub parameter: String,
    pub message: String,
    pub code: ValidationCode,
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{:?}] {}: {}",
            self.severity, self.parameter, self.message
        )
    }
}

fn setting_matches_value_type(setting: &Setting, expected: ValueType) -> bool {
    matches!(
        (setting, expected),
        (Setting::String(_), ValueType::String)
            | (Setting::Int(_), ValueType::Int)
            | (Setting::Double(_), ValueType::Double)
            | (Setting::Bytes(_), ValueType::Bytes)
            | (Setting::Bool(_), ValueType::Bool)
    )
}

fn value_type_name(vt: ValueType) -> &'static str {
    match vt {
        ValueType::String => "String",
        ValueType::Int => "Int",
        ValueType::Double => "Double",
        ValueType::Bytes => "Bytes",
        ValueType::Bool => "Bool",
    }
}

fn setting_type_name(setting: &Setting) -> &'static str {
    match setting {
        Setting::String(_) => "String",
        Setting::Int(_) => "Int",
        Setting::Double(_) => "Double",
        Setting::Bytes(_) => "Bytes",
        Setting::Bool(_) => "Bool",
    }
}

/// Attempt to coerce a `Setting::String` to the expected `ValueType`.
///
/// Connection strings (ODBC, JDBC, TOML files) are inherently stringly-typed,
/// so values like `"443"` (port) or `"true"` (verify_hostname) arrive as
/// strings even when the registry expects Int or Bool.  This function
/// converts them when the parse is unambiguous, returning `None` if the
/// string cannot be parsed.
fn try_coerce_setting(setting: &Setting, expected: ValueType) -> Option<Setting> {
    let s = match setting {
        Setting::String(s) => s,
        _ => return None,
    };
    match expected {
        ValueType::Int => s.parse::<i64>().ok().map(Setting::Int),
        ValueType::Double => s.parse::<f64>().ok().map(Setting::Double),
        ValueType::Bool => match s.to_lowercase().as_str() {
            "true" => Some(Setting::Bool(true)),
            "false" => Some(Setting::Bool(false)),
            _ => None,
        },
        _ => None,
    }
}

/// Validate and resolve a batch of options through the `ParamRegistry`.
///
/// Returns `(resolved_options, issues)` where `resolved_options` contains
/// canonical-name → value pairs for all valid entries, and `issues` contains
/// any warnings or errors encountered.
pub fn resolve_options(
    options: HashMap<String, Setting>,
) -> (HashMap<String, Setting>, Vec<ValidationIssue>) {
    let registry = param_registry::registry();
    let mut resolved = HashMap::new();
    let mut issues = Vec::new();
    let mut canonical_sources: HashMap<String, String> = HashMap::new();

    for (key, value) in options {
        match registry.resolve(&key) {
            Some(param_def) => {
                let canonical_name = param_def.canonical_name.to_string();
                if let Some(existing_key) = canonical_sources.get(&canonical_name) {
                    issues.push(ValidationIssue {
                        severity: ValidationSeverity::Error,
                        parameter: key.clone(),
                        message: format!(
                            "Parameters '{existing_key}' and '{key}' both resolve to '{}'. Provide only one key for that setting.",
                            param_def.canonical_name
                        ),
                        code: ValidationCode::ConflictingParameters,
                    });
                    continue;
                }

                if let Some(replacement) = param_def.deprecated_by {
                    issues.push(ValidationIssue {
                        severity: ValidationSeverity::Warning,
                        parameter: key.clone(),
                        message: format!(
                            "Parameter '{key}' is deprecated, use '{replacement}' instead"
                        ),
                        code: ValidationCode::DeprecatedParameter,
                    });
                }

                let final_value = if setting_matches_value_type(&value, param_def.value_type) {
                    value
                } else if let Some(coerced) = try_coerce_setting(&value, param_def.value_type) {
                    coerced
                } else {
                    issues.push(ValidationIssue {
                        severity: ValidationSeverity::Error,
                        parameter: key.clone(),
                        message: format!(
                            "Expected type {} for parameter '{}', got {}",
                            value_type_name(param_def.value_type),
                            param_def.canonical_name,
                            setting_type_name(&value),
                        ),
                        code: ValidationCode::InvalidType,
                    });
                    continue;
                };

                canonical_sources.insert(canonical_name.clone(), key);
                resolved.insert(canonical_name, final_value);
            }
            None => {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Warning,
                    parameter: key.clone(),
                    message: format!("Unknown parameter '{key}'"),
                    code: ValidationCode::UnknownParameter,
                });
                resolved.insert(key, value);
            }
        }
    }

    (resolved, issues)
}

/// Resolve, validate, and apply a batch of options to a settings map.
///
/// Calls [`resolve_options`] internally, rejects the batch if any errors are
/// found, and returns only the warnings on success.
pub fn resolve_and_apply_options(
    settings: &mut ParamStore,
    options: HashMap<String, Setting>,
) -> Result<Vec<ValidationIssue>, ApiError> {
    let (resolved, issues) = resolve_options(options);

    let error_messages: Vec<String> = issues
        .iter()
        .filter(|i| i.severity == ValidationSeverity::Error)
        .map(|i| i.to_string())
        .collect();

    if !error_messages.is_empty() {
        return InvalidArgumentSnafu {
            argument: error_messages.join("; "),
        }
        .fail();
    }

    for (key, value) in resolved {
        settings.insert(key, value);
    }

    Ok(issues
        .into_iter()
        .filter(|i| i.severity == ValidationSeverity::Warning)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_key_produces_warning() {
        let mut options = HashMap::new();
        options.insert(
            "nonexistent_key".to_string(),
            Setting::String("value".to_string()),
        );

        let (resolved, issues) = resolve_options(options);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, ValidationSeverity::Warning);
        assert_eq!(issues[0].code, ValidationCode::UnknownParameter);
        assert!(resolved.contains_key("nonexistent_key"));
    }

    #[test]
    fn type_mismatch_produces_error() {
        let mut options = HashMap::new();
        options.insert("host".to_string(), Setting::Int(42));

        let (resolved, issues) = resolve_options(options);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, ValidationSeverity::Error);
        assert_eq!(issues[0].code, ValidationCode::InvalidType);
        assert!(!resolved.contains_key("host"));
    }

    #[test]
    fn alias_resolves_to_canonical_name() {
        let mut options = HashMap::new();
        options.insert(
            "SERVER".to_string(),
            Setting::String("myhost.example.com".to_string()),
        );

        let (resolved, issues) = resolve_options(options);

        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .collect();
        assert!(errors.is_empty());
        assert!(resolved.contains_key("host"));
        assert!(!resolved.contains_key("SERVER"));
    }

    #[test]
    fn valid_options_produce_no_errors() {
        let mut options = HashMap::new();
        options.insert(
            "account".to_string(),
            Setting::String("myaccount".to_string()),
        );
        options.insert("user".to_string(), Setting::String("myuser".to_string()));

        let (resolved, issues) = resolve_options(options);

        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .collect();
        assert!(errors.is_empty());
        assert_eq!(resolved.len(), 2);
    }

    #[test]
    fn mixed_valid_and_invalid_options() {
        let mut options = HashMap::new();
        options.insert(
            "account".to_string(),
            Setting::String("myaccount".to_string()),
        );
        options.insert(
            "port".to_string(),
            Setting::String("not_a_number".to_string()),
        );
        options.insert(
            "unknown_param".to_string(),
            Setting::String("value".to_string()),
        );

        let (resolved, issues) = resolve_options(options);

        assert_eq!(issues.len(), 2);
        let type_errors: Vec<_> = issues
            .iter()
            .filter(|i| i.code == ValidationCode::InvalidType)
            .collect();
        assert_eq!(type_errors.len(), 1);
        let unknown_warnings: Vec<_> = issues
            .iter()
            .filter(|i| i.code == ValidationCode::UnknownParameter)
            .collect();
        assert_eq!(unknown_warnings.len(), 1);
        assert!(resolved.contains_key("account"));
        assert!(!resolved.contains_key("port"));
        assert!(resolved.contains_key("unknown_param"));
    }

    #[test]
    fn bool_type_accepted_for_bool_param() {
        let mut options = HashMap::new();
        options.insert("verify_hostname".to_string(), Setting::Bool(false));

        let (resolved, issues) = resolve_options(options);

        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .collect();
        assert!(errors.is_empty());
        assert!(resolved.contains_key("verify_hostname"));
    }

    #[test]
    fn string_port_coerced_to_int() {
        let mut options = HashMap::new();
        options.insert("port".to_string(), Setting::String("443".to_string()));

        let (resolved, issues) = resolve_options(options);

        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .collect();
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        assert_eq!(resolved.get("port"), Some(&Setting::Int(443)));
    }

    #[test]
    fn string_bool_coerced_to_bool() {
        let mut options = HashMap::new();
        options.insert(
            "verify_hostname".to_string(),
            Setting::String("true".to_string()),
        );

        let (resolved, issues) = resolve_options(options);

        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .collect();
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        assert_eq!(resolved.get("verify_hostname"), Some(&Setting::Bool(true)));
    }

    #[test]
    fn string_bool_case_insensitive() {
        let mut options = HashMap::new();
        options.insert(
            "verify_hostname".to_string(),
            Setting::String("FALSE".to_string()),
        );

        let (resolved, issues) = resolve_options(options);

        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .collect();
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        assert_eq!(resolved.get("verify_hostname"), Some(&Setting::Bool(false)));
    }

    #[test]
    fn unparseable_string_for_int_is_error() {
        let mut options = HashMap::new();
        options.insert("port".to_string(), Setting::String("abc".to_string()));

        let (resolved, issues) = resolve_options(options);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, ValidationSeverity::Error);
        assert_eq!(issues[0].code, ValidationCode::InvalidType);
        assert!(!resolved.contains_key("port"));
    }

    #[test]
    fn unparseable_string_for_bool_is_error() {
        let mut options = HashMap::new();
        options.insert(
            "verify_hostname".to_string(),
            Setting::String("yes".to_string()),
        );

        let (resolved, issues) = resolve_options(options);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, ValidationSeverity::Error);
        assert_eq!(issues[0].code, ValidationCode::InvalidType);
        assert!(!resolved.contains_key("verify_hostname"));
    }

    #[test]
    fn duplicate_alias_and_canonical_key_is_error() {
        let mut options = HashMap::new();
        options.insert(
            "SERVER".to_string(),
            Setting::String("dsn.example.com".to_string()),
        );
        options.insert(
            "host".to_string(),
            Setting::String("attr.example.com".to_string()),
        );

        let (_resolved, issues) = resolve_options(options);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, ValidationSeverity::Error);
        assert_eq!(issues[0].code, ValidationCode::ConflictingParameters);
        assert!(issues[0].message.contains("SERVER"));
        assert!(issues[0].message.contains("host"));
    }

    #[test]
    fn duplicate_case_variants_of_same_alias_are_error() {
        let mut options = HashMap::new();
        options.insert(
            "PRIV_KEY_BASE64".to_string(),
            Setting::String("dsn-key".to_string()),
        );
        options.insert(
            "priv_key_base64".to_string(),
            Setting::String("attr-key".to_string()),
        );

        let (_resolved, issues) = resolve_options(options);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, ValidationSeverity::Error);
        assert_eq!(issues[0].code, ValidationCode::ConflictingParameters);
        assert!(issues[0].message.contains("private_key"));
    }
}
