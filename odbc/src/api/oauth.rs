//! Registry-driven secret policies for the ODBC wrapper: never persist
//! secrets to the DSN registry ([`should_persist_to_dsn`]) and redact them
//! from logs ([`redacted_param_map`]). Both delegate sensitivity to
//! `sf_core`'s parameter registry under the ODBC flavor, covering OAuth and
//! legacy PWD-family secrets in one place.

/// Returns `false` for every key the parameter registry marks sensitive, so no
/// secret reaches the on-disk DSN registry. Keys unknown to the registry are
/// persisted, so callers compose this with their own DSN-skip rules (`DSN`).
/// Case-insensitive in `key`.
#[allow(dead_code)] // consumed by setup_common.rs (windows-only DSN write path)
pub fn should_persist_to_dsn(key: &str) -> bool {
    use sf_core::config::param_registry::{Wrapper, registry};
    !registry().is_sensitive_for(Wrapper::Odbc, key)
}

/// Returns a borrowed view of `params` with the value of every key the
/// parameter registry marks sensitive replaced by `"****"`; no allocation is
/// performed in either branch.
///
/// Use this at every connection-string logging boundary so secrets never reach
/// `tracing` sinks.
pub fn redacted_param_map(
    params: &std::collections::HashMap<String, String>,
) -> std::collections::HashMap<&String, std::borrow::Cow<'_, str>> {
    use sf_core::config::param_registry::{Wrapper, registry};
    let registry = registry();
    params
        .iter()
        .map(|(k, v)| {
            let value: std::borrow::Cow<'_, str> = if registry.is_sensitive_for(Wrapper::Odbc, k) {
                std::borrow::Cow::Borrowed("****")
            } else {
                std::borrow::Cow::Borrowed(v.as_str())
            };
            (k, value)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_persist_to_dsn_returns_false_only_for_secrets() {
        assert!(!should_persist_to_dsn("OAUTH_CLIENT_SECRET"));
        assert!(!should_persist_to_dsn("oauth_client_secret"));
        assert!(!should_persist_to_dsn("TOKEN"));
        assert!(!should_persist_to_dsn("PROXY"));
        assert!(should_persist_to_dsn("OAUTH_CLIENT_ID"));
        assert!(should_persist_to_dsn("OAUTH_REDIRECT_URI"));
        assert!(should_persist_to_dsn("OAUTH_SCOPE"));
        assert!(should_persist_to_dsn("UID"));
    }

    #[test]
    fn redacted_param_map_redacts_legacy_and_oauth_secrets() {
        use std::collections::HashMap;
        let params: HashMap<String, String> = [
            ("UID", "joe"),
            ("PWD", "hunter2"),
            ("PRIV_KEY_FILE_PWD", "kpwd"),
            ("PRIV_KEY_PWD", "kpwd2"),
            ("PRIV_KEY_BASE64", "AAA="),
            ("PASSCODE", "123456"),
            ("OAUTH_CLIENT_ID", "abc"),
            ("OAUTH_CLIENT_SECRET", "shhh"),
            ("TOKEN", "jwt.value"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();

        let redacted = redacted_param_map(&params);

        assert_eq!(
            redacted.get(&"UID".to_owned()).map(|v| v.as_ref()),
            Some("joe")
        );
        assert_eq!(
            redacted
                .get(&"OAUTH_CLIENT_ID".to_owned())
                .map(|v| v.as_ref()),
            Some("abc")
        );
        for sensitive in [
            "PWD",
            "PRIV_KEY_FILE_PWD",
            "PRIV_KEY_PWD",
            "PRIV_KEY_BASE64",
            "PASSCODE",
            "OAUTH_CLIENT_SECRET",
            "TOKEN",
        ] {
            let key = sensitive.to_string();
            assert_eq!(
                redacted.get(&key).map(|v| v.as_ref()),
                Some("****"),
                "expected key {sensitive} to be redacted"
            );
        }
    }

    #[test]
    fn redacted_param_map_redaction_is_case_insensitive() {
        use std::collections::HashMap;
        let params: HashMap<String, String> = [
            ("oauth_client_secret", "shhh"),
            ("Pwd", "hunter2"),
            ("token", "jwt.value"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();

        let redacted = redacted_param_map(&params);
        for k in params.keys() {
            assert_eq!(
                redacted.get(k).map(|v| v.as_ref()),
                Some("****"),
                "expected key {k} to be redacted regardless of case"
            );
        }
    }
}
