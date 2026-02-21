use std::collections::HashMap;

use super::error::{ApiError, InvalidArgumentSnafu};
use crate::config::ParamStore;
use crate::config::param_registry::{self, ParamScope, ValueType, param_names};
use crate::config::settings::{Setting, Settings};

pub use crate::config::connection_config::{ValidationCode, ValidationIssue, ValidationSeverity};

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

fn setting_matches_expected_types(
    setting: &Setting,
    primary: ValueType,
    additional: Option<ValueType>,
) -> bool {
    setting_matches_value_type(setting, primary)
        || additional.is_some_and(|value_type| setting_matches_value_type(setting, value_type))
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

fn value_type_names(primary: ValueType, additional: Option<ValueType>) -> String {
    match additional {
        Some(value_type) => format!(
            "{} or {}",
            value_type_name(primary),
            value_type_name(value_type)
        ),
        None => value_type_name(primary).to_string(),
    }
}

fn try_coerce_to_value_type(setting: &Setting, expected: ValueType) -> Option<Setting> {
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
        ValueType::String | ValueType::Bytes => None,
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

/// Attempt to coerce a `Setting::String` to the primary or additional `ValueType`.
///
/// Connection strings (ODBC, JDBC, TOML files) are inherently stringly-typed,
/// so values like `"443"` (port) or `"true"` (verify_hostname) arrive as
/// strings even when the registry expects Int or Bool.  This function
/// converts them when the parse is unambiguous, returning `None` if the
/// string cannot be parsed.
fn try_coerce_setting(
    setting: &Setting,
    primary: ValueType,
    additional: Option<ValueType>,
) -> Option<Setting> {
    try_coerce_to_value_type(setting, primary)
        .or_else(|| additional.and_then(|value_type| try_coerce_to_value_type(setting, value_type)))
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

                let final_value = if setting_matches_expected_types(
                    &value,
                    param_def.value_type,
                    param_def.additional_value_type,
                ) {
                    value
                } else if let Some(coerced) = try_coerce_setting(
                    &value,
                    param_def.value_type,
                    param_def.additional_value_type,
                ) {
                    coerced
                } else {
                    issues.push(ValidationIssue {
                        severity: ValidationSeverity::Error,
                        parameter: key.clone(),
                        message: format!(
                            "Expected type {} for parameter '{}', got {}",
                            value_type_names(param_def.value_type, param_def.additional_value_type),
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

/// Underscores are not valid in DNS labels (RFC 952/1123), but Snowflake account
/// names may contain them. When the host or server_url is derived from such an
/// account, the underscores must be replaced with hyphens so the hostname
/// resolves correctly.
/// Skipped when `preserve_underscores_in_hostname` is `true`.
///
/// Called from `connection_init` after all settings (explicit options, TOML
/// config) have been accumulated, so it always sees the complete picture.
pub(crate) fn normalize_host_underscores(settings: &mut dyn Settings) {
    if is_allow_underscores(settings) {
        return;
    }

    let account = match settings.get_string(param_names::ACCOUNT.as_str()) {
        Some(a) if a.contains('_') => a,
        _ => return,
    };

    let account_lower = account.to_ascii_lowercase();

    if let Some(host) = settings.get_string(param_names::HOST.as_str())
        && starts_with_account_label(&host, &account_lower)
    {
        let prefix = &host[..account.len()];
        let new_host = format!("{}{}", prefix.replace('_', "-"), &host[account.len()..]);
        tracing::debug!(old = %host, new = %new_host, "Replaced underscores with hyphens in host");
        settings.set(param_names::HOST.as_str(), Setting::String(new_host));
    }

    if let Some(url) = settings.get_string(param_names::SERVER_URL.as_str())
        && let Some(after_scheme) = url.find("://").map(|i| i + 3)
    {
        let host_part = &url[after_scheme..];
        if starts_with_account_label(host_part, &account_lower) {
            let prefix = &host_part[..account.len()];
            let new_url = format!(
                "{}{}{}",
                &url[..after_scheme],
                prefix.replace('_', "-"),
                &host_part[account.len()..]
            );
            tracing::debug!(old = %url, new = %new_url, "Replaced underscores with hyphens in server_url");
            settings.set(param_names::SERVER_URL.as_str(), Setting::String(new_url));
        }
    }
}

/// Returns `true` when `value` starts with `account_lower` and the next
/// character (if any) is a DNS label boundary (`.`, `:`, or end-of-string).
/// This prevents a prefix-only match from rewriting unrelated hosts (e.g.
/// account `abc_test` must not match host `abc_test2.example.com`).
fn starts_with_account_label(value: &str, account_lower: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if !lower.starts_with(account_lower) {
        return false;
    }
    matches!(
        lower.as_bytes().get(account_lower.len()),
        None | Some(b'.') | Some(b':')
    )
}

/// Check the opt-out flag across canonical key, all registered aliases, and
/// string-typed values so the flag works regardless of how the wrapper
/// delivered it. Aliases are read from the param registry so they stay in sync
/// automatically.
fn is_allow_underscores(settings: &dyn Settings) -> bool {
    let Some(param_def) =
        param_registry::registry().resolve(param_names::PRESERVE_UNDERSCORES_IN_HOSTNAME)
    else {
        tracing::warn!(
            "Parameter definition for preserve_underscores_in_hostname not found; treating flag as disabled"
        );
        return false;
    };

    std::iter::once(param_def.canonical_name)
        .chain(param_def.aliases.iter().copied())
        .any(|key| {
            settings.get_bool(key).unwrap_or(false)
                || settings
                    .get_string(key)
                    .is_some_and(|s| s.eq_ignore_ascii_case("true"))
        })
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

pub(crate) fn canonicalize_setting_key(
    key: &str,
) -> (String, Option<&'static param_registry::ParamDef>) {
    let reg = param_registry::registry();
    match reg.resolve(key) {
        Some(d) => (d.canonical_name.to_string(), Some(d)),
        None => (key.to_string(), None),
    }
}

pub(crate) fn validate_connection_seed_write(
    post_connect: bool,
    def: Option<&param_registry::ParamDef>,
) -> Result<(), ApiError> {
    let Some(d) = def else {
        return Ok(());
    };
    if d.scope == ParamScope::Statement {
        return InvalidArgumentSnafu {
            argument: format!(
                "Parameter '{}' is statement-scoped; set it on a statement handle",
                d.canonical_name
            ),
        }
        .fail();
    }
    if post_connect {
        if d.scope == ParamScope::Session {
            return InvalidArgumentSnafu {
                argument: format!(
                    "Parameter '{}' is session-scoped; use connection_set_session_option after connect",
                    d.canonical_name
                ),
            }
            .fail();
        }
        if !d.mutable_after_connect {
            return InvalidArgumentSnafu {
                argument: format!(
                    "Parameter '{}' cannot be changed after connect",
                    d.canonical_name
                ),
            }
            .fail();
        }
    }
    Ok(())
}

pub(crate) fn validate_session_override_write(
    def: Option<&param_registry::ParamDef>,
) -> Result<(), ApiError> {
    let Some(d) = def else {
        return Ok(());
    };
    if d.scope == ParamScope::Statement {
        return InvalidArgumentSnafu {
            argument: format!(
                "Parameter '{}' is statement-scoped; set it on a statement handle",
                d.canonical_name
            ),
        }
        .fail();
    }
    if d.scope == ParamScope::Connection {
        return InvalidArgumentSnafu {
            argument: format!(
                "Parameter '{}' is connection-scoped; set it via connection options before connect",
                d.canonical_name
            ),
        }
        .fail();
    }
    Ok(())
}

pub(crate) fn validate_statement_option_write(
    def: Option<&param_registry::ParamDef>,
) -> Result<(), ApiError> {
    let Some(d) = def else {
        return Ok(());
    };
    if d.scope != ParamScope::Statement {
        return InvalidArgumentSnafu {
            argument: format!(
                "Parameter '{}' is not statement-scoped; set it on the connection or session",
                d.canonical_name
            ),
        }
        .fail();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

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
    fn private_key_string_is_accepted() {
        let mut options = HashMap::new();
        options.insert(
            "private_key".to_string(),
            Setting::String("base64-encoded-key".to_string()),
        );

        let (resolved, issues) = resolve_options(options);

        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .collect();
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        assert_eq!(
            resolved.get("private_key"),
            Some(&Setting::String("base64-encoded-key".to_string()))
        );
    }

    #[test]
    fn private_key_bytes_are_accepted() {
        let mut options = HashMap::new();
        options.insert(
            "private_key".to_string(),
            Setting::Bytes(vec![0x01, 0x02, 0x03]),
        );

        let (resolved, issues) = resolve_options(options);

        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .collect();
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        assert_eq!(
            resolved.get("private_key"),
            Some(&Setting::Bytes(vec![0x01, 0x02, 0x03]))
        );
    }

    #[test]
    fn private_key_invalid_type_lists_all_supported_types() {
        let mut options = HashMap::new();
        options.insert("private_key".to_string(), Setting::Int(7));

        let (resolved, issues) = resolve_options(options);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, ValidationSeverity::Error);
        assert_eq!(issues[0].code, ValidationCode::InvalidType);
        assert_eq!(
            issues[0].message,
            "Expected type String or Bytes for parameter 'private_key', got Int"
        );
        assert!(!resolved.contains_key("private_key"));
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

    #[test_case("abc_test", "host", "abc_test.us-east-1.snowflakecomputing.com", "abc-test.us-east-1.snowflakecomputing.com" ; "host")]
    #[test_case("ABC_Test", "host", "abc_test.snowflakecomputing.com", "abc-test.snowflakecomputing.com" ; "host case insensitive")]
    #[test_case("foo_account_test_1", "host", "foo_account_test_1.snowflakecomputing.com", "foo-account-test-1.snowflakecomputing.com" ; "host multiple underscores")]
    #[test_case("org-account_test_1", "host", "org-account_test_1.snowflakecomputing.com", "org-account-test-1.snowflakecomputing.com" ; "host org-account regionless")]
    #[test_case("abc_test", "server_url", "https://abc_test.snowflakecomputing.com", "https://abc-test.snowflakecomputing.com" ; "server_url")]
    #[test_case("abc_test", "server_url", "https://abc_test.snowflakecomputing.com:443", "https://abc-test.snowflakecomputing.com:443" ; "server_url with port")]
    fn normalizes_underscores(account: &str, key: &str, input: &str, expected: &str) {
        let mut settings = HashMap::new();
        settings.insert("account".to_string(), Setting::String(account.to_string()));
        settings.insert(key.to_string(), Setting::String(input.to_string()));

        normalize_host_underscores(&mut settings);

        assert_eq!(
            Settings::get(&settings, key),
            Some(Setting::String(expected.to_string())),
        );
        assert_eq!(
            Settings::get(&settings, "account"),
            Some(Setting::String(account.to_string())),
        );
    }

    #[test_case("myaccount", Some("myaccount.snowflakecomputing.com"), None ; "no underscore in account")]
    #[test_case("abc_test", Some("192.168.1.1"), None ; "host not starting with account")]
    #[test_case("abc_test", None, None ; "no host or server_url set")]
    #[test_case("abc_test", None, Some("https://custom-host.example.com") ; "server_url not matching account")]
    #[test_case("abc_test", Some("abc_test2.snowflakecomputing.com"), None ; "account is only a prefix of host label")]
    #[test_case("abc_test", None, Some("https://abc_test2.snowflakecomputing.com") ; "account is only a prefix of server_url label")]
    fn skips_normalization(account: &str, host: Option<&str>, server_url: Option<&str>) {
        let mut settings = HashMap::new();
        settings.insert("account".to_string(), Setting::String(account.to_string()));
        if let Some(h) = host {
            settings.insert("host".to_string(), Setting::String(h.to_string()));
        }
        if let Some(u) = server_url {
            settings.insert("server_url".to_string(), Setting::String(u.to_string()));
        }

        normalize_host_underscores(&mut settings);

        assert_eq!(
            Settings::get(&settings, "host"),
            host.map(|h| Setting::String(h.to_string())),
        );
        assert_eq!(
            Settings::get(&settings, "server_url"),
            server_url.map(|u| Setting::String(u.to_string())),
        );
    }

    #[test_case("preserve_underscores_in_hostname", Setting::Bool(true) ; "bool on canonical key")]
    #[test_case("preserve_underscores_in_hostname", Setting::String("true".to_string()) ; "string on canonical key")]
    #[test_case("ALLOWUNDERSCORESINHOST", Setting::Bool(true) ; "bool on alias key")]
    fn opt_out_skips_normalization(key: &str, value: Setting) {
        let mut settings = HashMap::new();
        settings.insert(
            "account".to_string(),
            Setting::String("abc_test".to_string()),
        );
        settings.insert(
            "host".to_string(),
            Setting::String("abc_test.snowflakecomputing.com".to_string()),
        );
        settings.insert(key.to_string(), value);

        normalize_host_underscores(&mut settings);

        assert_eq!(
            Settings::get(&settings, "host"),
            Some(Setting::String(
                "abc_test.snowflakecomputing.com".to_string()
            )),
        );
    }
}
