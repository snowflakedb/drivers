use crate::config::ConfigError;
use crate::config::ParamStore;
use crate::config::config_manager;
use crate::config::param_names;
use crate::config::param_registry;
use crate::config::path_resolver::ConfigPaths;
use crate::config::settings::Setting;

/// If `account` is not explicitly set but `host` is available,
/// derive the account identifier from the hostname — matching the legacy
/// `snowflake-odbc` driver behavior.
///
/// The algorithm is:
///   1. Take everything before the first `.` in the host.
///   2. If the host contains `.global.`, strip the external-ID suffix after the
///      last `-` in that first token.
///
/// Must be called **before** underscore normalization so that
/// `normalize_host_underscores` can see the derived account.
pub(crate) fn derive_account_from_host(store: &mut ParamStore) {
    if store.get_string(param_names::ACCOUNT).is_some() {
        return;
    }

    let host_opt = store.get(param_names::HOST).and_then(|h| h.as_string());

    let Some(host) = host_opt else {
        return;
    };

    let first_label = host.split('.').next().unwrap_or(host);
    if first_label.is_empty() {
        return;
    }

    let account = if host.contains(".global.") {
        first_label
            .rfind('-')
            .map_or(first_label, |i| &first_label[..i])
    } else {
        first_label
    };

    tracing::debug!(derived_account = %account, host = %host, "Derived account from host");
    store.insert(
        param_names::ACCOUNT.into(),
        Setting::String(account.to_owned()),
    );
}

/// If neither `host` nor `server_url` is explicitly set but `account` is,
/// derive the hostname from `account` and the optional `region` parameter —
/// matching the legacy `snowflake-connector-python` `construct_hostname()`
/// behavior.
///
/// Rules:
///   - `region == "us-west-2"` is treated as no region (legacy default AWS
///     region).
///   - When `region` starts with `"cn-"`, the TLD is `.cn` (China);
///     otherwise `.com`.
///   - When `region` is set and `account` contains a dot, only the part
///     before the first dot is used.
///   - When `region` is absent and `account` contains a dot whose second
///     segment starts with `"cn-"`, the TLD is `.cn`.
///   - Account identifiers that already encode a region (e.g.
///     `"myaccount.us-east-1"`) are passed through unchanged when no
///     explicit `region` is set, producing
///     `"myaccount.us-east-1.snowflakecomputing.com"`.
pub(crate) fn derive_host_from_account(store: &mut ParamStore) {
    if store.get_string(param_names::HOST).is_some()
        || store.get_string(param_names::SERVER_URL).is_some()
    {
        return;
    }

    let Some(account) = store.get_string(param_names::ACCOUNT) else {
        return;
    };

    let region = store.get_string(param_names::REGION);

    let is_china_region = |r: &str| r.to_ascii_lowercase().starts_with("cn-");

    // "us-west-2" is the legacy default AWS region — treat as empty.
    let effective_region = region.as_deref().and_then(|r| {
        if r.eq_ignore_ascii_case("us-west-2") || r.is_empty() {
            None
        } else {
            Some(r)
        }
    });

    let host = if let Some(region) = effective_region {
        let acct = account.split('.').next().unwrap_or(&account);
        let tld = if is_china_region(region) { "cn" } else { "com" };
        format!("{acct}.{region}.snowflakecomputing.{tld}")
    } else {
        let tld = if account.contains('.') {
            let segments: Vec<&str> = account.split('.').collect();
            if segments.len() > 1 && is_china_region(segments[1]) {
                "cn"
            } else {
                "com"
            }
        } else {
            "com"
        };
        format!("{account}.snowflakecomputing.{tld}")
    };

    tracing::debug!(derived_host = %host, account = %account, "Derived host from account");
    store.insert(param_names::HOST.into(), Setting::String(host));
}

/// Resolve final settings by merging explicit settings with file-based
/// config and registry defaults.
///
/// Precedence (highest to lowest):
/// 1. Explicit programmatic settings (from `SetOptions` / `SetOption*` RPCs)
/// 2. TOML file: `connections.toml` `[connection_name]` section
/// 3. TOML file: `config.toml` `[connections.connection_name]` section
/// 4. Registry defaults (`ParamDef::default`)
///
/// `explicit` contains values set via the programmatic API (already
/// alias-resolved and type-checked by `connection_set_options`).
///
/// If `connection_name` is present in `explicit`, file-based config is
/// loaded and merged underneath.
pub fn resolve(explicit: &ParamStore) -> Result<ParamStore, ConfigError> {
    let paths = crate::config::path_resolver::get_config_paths()?;
    resolve_with_paths(explicit, &paths)
}

/// Same as [`resolve`] but accepts explicit config file paths (for testing).
pub fn resolve_with_paths(
    explicit: &ParamStore,
    paths: &ConfigPaths,
) -> Result<ParamStore, ConfigError> {
    let mut merged = ParamStore::new();

    // Layer 4: Registry defaults (lowest priority)
    for param in param_registry::registry().all_params() {
        if let Some(default_fn) = param.default {
            merged.insert(param.canonical_name.to_owned(), default_fn());
        }
    }

    // Layer 3+2: TOML files (if connection_name is set)
    if let Some(Setting::String(name)) = explicit.get(param_names::CONNECTION_NAME) {
        let file_settings = config_manager::load_connection_config_with_paths(name, paths)?;
        for (k, v) in file_settings {
            merged.insert(k, v);
        }
    }

    // Layer 1: Explicit programmatic settings (highest priority)
    merged.extend_from(explicit);

    derive_account_from_host(&mut merged);
    derive_host_from_account(&mut merged);

    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::param_names;
    use crate::config::path_resolver::ConfigPaths;
    use crate::config::settings::Setting;
    use std::fs;
    use tempfile::TempDir;
    use test_case::test_case;

    #[test_case("myaccount.snowflakecomputing.com", "myaccount" ; "standard host")]
    #[test_case("myaccount.us-east-1.snowflakecomputing.com", "myaccount" ; "host with region")]
    #[test_case("myaccount.privatelink.snowflakecomputing.com", "myaccount" ; "privatelink host")]
    #[test_case("myaccount", "myaccount" ; "bare account no dots")]
    fn derive_account_from_host_extracts_first_label(host: &str, expected: &str) {
        let mut store = ParamStore::new();
        store.insert(param_names::HOST.into(), Setting::String(host.to_owned()));

        derive_account_from_host(&mut store);

        assert_eq!(
            store.get(param_names::ACCOUNT),
            Some(&Setting::String(expected.to_owned())),
        );
    }

    #[test]
    fn derive_account_from_host_strips_global_external_id() {
        let mut store = ParamStore::new();
        store.insert(
            param_names::HOST.into(),
            Setting::String("myaccount-extid.global.snowflake.com".to_owned()),
        );

        derive_account_from_host(&mut store);

        assert_eq!(
            store.get(param_names::ACCOUNT),
            Some(&Setting::String("myaccount".to_owned())),
        );
    }

    #[test]
    fn derive_account_from_host_skips_when_account_present() {
        let mut store = ParamStore::new();
        store.insert(
            param_names::ACCOUNT.into(),
            Setting::String("explicit".to_owned()),
        );
        store.insert(
            param_names::HOST.into(),
            Setting::String("other.snowflakecomputing.com".to_owned()),
        );

        derive_account_from_host(&mut store);

        assert_eq!(
            store.get(param_names::ACCOUNT),
            Some(&Setting::String("explicit".to_owned())),
        );
    }

    #[test]
    fn derive_account_from_host_noop_when_no_host() {
        let mut store = ParamStore::new();

        derive_account_from_host(&mut store);

        assert_eq!(store.get(param_names::ACCOUNT), None);
    }

    // --- derive_host_from_account tests ---

    #[test_case("myaccount", "myaccount.snowflakecomputing.com" ; "simple account")]
    #[test_case("myaccount.us-east-1", "myaccount.us-east-1.snowflakecomputing.com" ; "account with region")]
    #[test_case("myorg-myaccount", "myorg-myaccount.snowflakecomputing.com" ; "org-account format")]
    fn derive_host_from_account_constructs_host(account: &str, expected_host: &str) {
        let mut store = ParamStore::new();
        store.insert(
            param_names::ACCOUNT.into(),
            Setting::String(account.to_owned()),
        );

        derive_host_from_account(&mut store);

        assert_eq!(
            store.get(param_names::HOST),
            Some(&Setting::String(expected_host.to_owned())),
        );
    }

    #[test]
    fn derive_host_from_account_skips_when_host_present() {
        let mut store = ParamStore::new();
        store.insert(
            param_names::ACCOUNT.into(),
            Setting::String("myaccount".to_owned()),
        );
        store.insert(
            param_names::HOST.into(),
            Setting::String("custom.host.com".to_owned()),
        );

        derive_host_from_account(&mut store);

        assert_eq!(
            store.get(param_names::HOST),
            Some(&Setting::String("custom.host.com".to_owned())),
        );
    }

    #[test]
    fn derive_host_from_account_skips_when_server_url_present() {
        let mut store = ParamStore::new();
        store.insert(
            param_names::ACCOUNT.into(),
            Setting::String("myaccount".to_owned()),
        );
        store.insert(
            param_names::SERVER_URL.into(),
            Setting::String("https://custom.url".to_owned()),
        );

        derive_host_from_account(&mut store);

        assert_eq!(store.get(param_names::HOST), None);
    }

    #[test]
    fn derive_host_from_account_noop_when_no_account() {
        let mut store = ParamStore::new();

        derive_host_from_account(&mut store);

        assert_eq!(store.get(param_names::HOST), None);
    }

    // --- derive_host_from_account: region handling ---

    #[test_case(
        "myaccount", Some("us-east-1"), "myaccount.us-east-1.snowflakecomputing.com"
        ; "account with explicit region"
    )]
    #[test_case(
        "myaccount", Some("eu-central-1"), "myaccount.eu-central-1.snowflakecomputing.com"
        ; "eu region"
    )]
    #[test_case(
        "myaccount", Some("ap-southeast-2"), "myaccount.ap-southeast-2.snowflakecomputing.com"
        ; "ap region"
    )]
    #[test_case(
        "myaccount", Some("eu-central-1.privatelink"), "myaccount.eu-central-1.privatelink.snowflakecomputing.com"
        ; "privatelink region"
    )]
    fn derive_host_from_account_with_region(
        account: &str,
        region: Option<&str>,
        expected_host: &str,
    ) {
        let mut store = ParamStore::new();
        store.insert(
            param_names::ACCOUNT.into(),
            Setting::String(account.to_owned()),
        );
        if let Some(r) = region {
            store.insert(param_names::REGION.into(), Setting::String(r.to_owned()));
        }

        derive_host_from_account(&mut store);

        assert_eq!(
            store.get(param_names::HOST),
            Some(&Setting::String(expected_host.to_owned())),
        );
    }

    #[test_case("us-west-2" ; "lowercase")]
    #[test_case("US-WEST-2" ; "uppercase")]
    #[test_case("Us-West-2" ; "mixed case")]
    fn derive_host_us_west_2_treated_as_no_region(region: &str) {
        let mut store = ParamStore::new();
        store.insert(
            param_names::ACCOUNT.into(),
            Setting::String("myaccount".to_owned()),
        );
        store.insert(
            param_names::REGION.into(),
            Setting::String(region.to_owned()),
        );

        derive_host_from_account(&mut store);

        assert_eq!(
            store.get(param_names::HOST),
            Some(&Setting::String(
                "myaccount.snowflakecomputing.com".to_owned()
            )),
        );
    }

    #[test_case(
        "myaccount", Some("cn-northwest-1"), "myaccount.cn-northwest-1.snowflakecomputing.cn"
        ; "china region uses cn TLD"
    )]
    #[test_case(
        "myaccount", Some("CN-NORTHWEST-1"), "myaccount.CN-NORTHWEST-1.snowflakecomputing.cn"
        ; "china region detection is case insensitive"
    )]
    #[test_case(
        "myaccount.cn-northwest-1", None, "myaccount.cn-northwest-1.snowflakecomputing.cn"
        ; "china inferred from dotted account"
    )]
    fn derive_host_china_region(account: &str, region: Option<&str>, expected_host: &str) {
        let mut store = ParamStore::new();
        store.insert(
            param_names::ACCOUNT.into(),
            Setting::String(account.to_owned()),
        );
        if let Some(r) = region {
            store.insert(param_names::REGION.into(), Setting::String(r.to_owned()));
        }

        derive_host_from_account(&mut store);

        assert_eq!(
            store.get(param_names::HOST),
            Some(&Setting::String(expected_host.to_owned())),
        );
    }

    #[test_case(
        "myaccount.us-east-1", Some("eu-central-1"), "myaccount.eu-central-1.snowflakecomputing.com"
        ; "dotted account truncated when region set"
    )]
    #[test_case(
        "a.b.c", Some("eu-west-1"), "a.eu-west-1.snowflakecomputing.com"
        ; "multi-dot account uses first segment only"
    )]
    fn derive_host_dotted_account_with_region(
        account: &str,
        region: Option<&str>,
        expected_host: &str,
    ) {
        let mut store = ParamStore::new();
        store.insert(
            param_names::ACCOUNT.into(),
            Setting::String(account.to_owned()),
        );
        if let Some(r) = region {
            store.insert(param_names::REGION.into(), Setting::String(r.to_owned()));
        }

        derive_host_from_account(&mut store);

        assert_eq!(
            store.get(param_names::HOST),
            Some(&Setting::String(expected_host.to_owned())),
        );
    }

    #[test]
    fn derive_host_empty_region_treated_as_absent() {
        let mut store = ParamStore::new();
        store.insert(
            param_names::ACCOUNT.into(),
            Setting::String("myaccount".to_owned()),
        );
        store.insert(param_names::REGION.into(), Setting::String(String::new()));

        derive_host_from_account(&mut store);

        assert_eq!(
            store.get(param_names::HOST),
            Some(&Setting::String(
                "myaccount.snowflakecomputing.com".to_owned()
            )),
        );
    }

    #[test]
    fn derive_host_explicit_host_not_overridden_by_region() {
        let mut store = ParamStore::new();
        store.insert(
            param_names::ACCOUNT.into(),
            Setting::String("myaccount".to_owned()),
        );
        store.insert(
            param_names::REGION.into(),
            Setting::String("eu-central-1".to_owned()),
        );
        store.insert(
            param_names::HOST.into(),
            Setting::String("custom.host.com".to_owned()),
        );

        derive_host_from_account(&mut store);

        assert_eq!(
            store.get(param_names::HOST),
            Some(&Setting::String("custom.host.com".to_owned())),
        );
    }

    fn make_paths(dir: &TempDir) -> ConfigPaths {
        ConfigPaths {
            config_file: Some(dir.path().join("config.toml")),
            connections_file: Some(dir.path().join("connections.toml")),
        }
    }

    fn write_config(dir: &TempDir, filename: &str, content: &str) {
        let path = dir.path().join(filename);
        fs::write(&path, content).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
    }

    #[test]
    fn explicit_settings_override_file_settings() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[testconn]
account = "file_account"
user = "file_user"
"#,
        );

        let mut explicit = ParamStore::new();
        explicit.insert(
            "connection_name".to_owned(),
            Setting::String("testconn".to_owned()),
        );
        explicit.insert(
            "account".to_owned(),
            Setting::String("explicit_account".to_owned()),
        );

        let resolved = resolve_with_paths(&explicit, &paths).unwrap();

        if let Some(Setting::String(account)) = resolved.get(param_names::ACCOUNT) {
            assert_eq!(account, "explicit_account");
        } else {
            panic!("Expected account setting");
        }

        if let Some(Setting::String(user)) = resolved.get(param_names::USER) {
            assert_eq!(user, "file_user");
        } else {
            panic!("Expected user setting from file");
        }
    }

    #[test]
    fn file_settings_override_registry_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[testconn]
account = "file_account"
protocol = "http"
"#,
        );

        let mut explicit = ParamStore::new();
        explicit.insert(
            "connection_name".to_owned(),
            Setting::String("testconn".to_owned()),
        );

        let resolved = resolve_with_paths(&explicit, &paths).unwrap();

        if let Some(Setting::String(protocol)) = resolved.get(param_names::PROTOCOL) {
            assert_eq!(protocol, "http");
        } else {
            panic!("Expected protocol setting");
        }
    }

    #[test]
    fn connections_toml_overrides_config_toml() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"
[connections.testconn]
account = "config_account"
user = "config_user"
"#,
        );
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[testconn]
account = "connections_account"
"#,
        );

        let mut explicit = ParamStore::new();
        explicit.insert(
            "connection_name".to_owned(),
            Setting::String("testconn".to_owned()),
        );

        let resolved = resolve_with_paths(&explicit, &paths).unwrap();

        if let Some(Setting::String(account)) = resolved.get(param_names::ACCOUNT) {
            assert_eq!(account, "connections_account");
        } else {
            panic!("Expected account setting");
        }

        if let Some(Setting::String(user)) = resolved.get(param_names::USER) {
            assert_eq!(user, "config_user");
        } else {
            panic!("Expected user setting");
        }
    }

    #[test]
    fn no_connection_name_uses_only_explicit_and_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[testconn]
account = "file_account"
"#,
        );

        let mut explicit = ParamStore::new();
        explicit.insert(
            "account".to_owned(),
            Setting::String("explicit_account".to_owned()),
        );

        let resolved = resolve_with_paths(&explicit, &paths).unwrap();

        if let Some(Setting::String(account)) = resolved.get(param_names::ACCOUNT) {
            assert_eq!(account, "explicit_account");
        } else {
            panic!("Expected account setting");
        }

        // Registry default for protocol should be present
        if let Some(Setting::String(protocol)) = resolved.get(param_names::PROTOCOL) {
            assert_eq!(protocol, "https");
        } else {
            panic!("Expected default protocol setting");
        }
    }

    fn get_str(map: &ParamStore, key: crate::config::param_registry::ParamKey) -> Option<String> {
        match map.get(key) {
            Some(Setting::String(v)) => Some(v.clone()),
            _ => None,
        }
    }

    #[test]
    fn integration_full_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"
[connections.myconn]
account = "config_acct"
user = "config_user"
warehouse = "config_wh"
"#,
        );
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[myconn]
account = "conn_acct"
database = "conn_db"
"#,
        );

        let mut explicit = ParamStore::new();
        explicit.insert(
            "connection_name".to_owned(),
            Setting::String("myconn".to_owned()),
        );
        explicit.insert(
            "account".to_owned(),
            Setting::String("explicit_acct".to_owned()),
        );

        let resolved = resolve_with_paths(&explicit, &paths).unwrap();

        assert_eq!(
            get_str(&resolved, param_names::ACCOUNT),
            Some("explicit_acct".to_owned())
        );
        assert_eq!(
            get_str(&resolved, param_names::DATABASE),
            Some("conn_db".to_owned())
        );
        assert_eq!(
            get_str(&resolved, param_names::USER),
            Some("config_user".to_owned())
        );
        assert_eq!(
            get_str(&resolved, param_names::WAREHOUSE),
            Some("config_wh".to_owned())
        );
        assert_eq!(
            get_str(&resolved, param_names::PROTOCOL),
            Some("https".to_owned())
        );
    }

    // --- resolve_with_paths: region integration ---

    #[test]
    fn resolve_derives_host_from_account_and_region() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);

        let mut explicit = ParamStore::new();
        explicit.insert(
            "account".to_owned(),
            Setting::String("myaccount".to_owned()),
        );
        explicit.insert(
            "region".to_owned(),
            Setting::String("eu-central-1".to_owned()),
        );

        let resolved = resolve_with_paths(&explicit, &paths).unwrap();

        assert_eq!(
            get_str(&resolved, param_names::HOST),
            Some("myaccount.eu-central-1.snowflakecomputing.com".to_owned()),
        );
    }

    #[test]
    fn resolve_explicit_host_overrides_region_derivation() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);

        let mut explicit = ParamStore::new();
        explicit.insert(
            "account".to_owned(),
            Setting::String("myaccount".to_owned()),
        );
        explicit.insert(
            "region".to_owned(),
            Setting::String("eu-central-1".to_owned()),
        );
        explicit.insert(
            "host".to_owned(),
            Setting::String("custom.host.com".to_owned()),
        );

        let resolved = resolve_with_paths(&explicit, &paths).unwrap();

        assert_eq!(
            get_str(&resolved, param_names::HOST),
            Some("custom.host.com".to_owned()),
        );
    }

    #[test]
    fn resolve_region_from_toml_derives_host() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[regionconn]
account = "myaccount"
region = "ap-southeast-2"
"#,
        );

        let mut explicit = ParamStore::new();
        explicit.insert(
            "connection_name".to_owned(),
            Setting::String("regionconn".to_owned()),
        );

        let resolved = resolve_with_paths(&explicit, &paths).unwrap();

        assert_eq!(
            get_str(&resolved, param_names::HOST),
            Some("myaccount.ap-southeast-2.snowflakecomputing.com".to_owned()),
        );
        assert_eq!(
            get_str(&resolved, param_names::ACCOUNT),
            Some("myaccount".to_owned()),
        );
    }
}
