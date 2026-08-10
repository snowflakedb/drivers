use crate::config::ConfigError;
use crate::config::ParamStore;
use crate::config::config_manager;
use crate::config::param_names;
use crate::config::param_registry;
use crate::config::path_resolver::ConfigPaths;
use crate::config::settings::Setting;
use crate::config::toml_loader::FilePermissionCheck;
use crate::env_vars;

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
/// derive the hostname from the account identifier — matching the legacy
/// `snowflake-connector-python` driver behavior where `account="myaccount"`
/// yields host `"myaccount.snowflakecomputing.com"`.
///
/// Account identifiers that already encode a region (e.g. `"myaccount.us-east-1"`)
/// are passed through unchanged, producing `"myaccount.us-east-1.snowflakecomputing.com"`.
pub(crate) fn derive_host_from_account(store: &mut ParamStore) {
    if store.get_string(param_names::HOST).is_some()
        || store.get_string(param_names::SERVER_URL).is_some()
    {
        return;
    }

    let Some(account) = store.get_string(param_names::ACCOUNT) else {
        return;
    };

    // SECURITY (SNOW-3663586, CWE-918): `account` is interpolated into the host
    // verbatim, so its character set is restricted to a safe allow-list in
    // `connection_config::validate_settings`, which runs in
    // `ConnectionConfig::build` before any network I/O.
    let host = format!("{account}.snowflakecomputing.com");
    tracing::debug!(derived_host = %host, account = %account, "Derived host from account");
    store.insert(param_names::HOST.into(), Setting::String(host));
}

/// When running inside a Snowpark Container Services (SPCS) container, read
/// the SPCS-injected connection env vars and populate the store. Only fires
/// when `SNOWFLAKE_RUNNING_INSIDE_SPCS` is set. Empty values are ignored.
///
/// Priority: above registry defaults but below TOML profiles and explicit
/// programmatic settings — a connection profile or explicit param overrides
/// the container-injected values.
fn apply_spcs_env_vars(store: &mut ParamStore) {
    if std::env::var_os(env_vars::SNOWFLAKE_RUNNING_INSIDE_SPCS).is_none() {
        return;
    }
    for (env_var, param) in [
        (env_vars::SNOWFLAKE_ACCOUNT, param_names::ACCOUNT),
        (env_vars::SNOWFLAKE_HOST, param_names::HOST),
        (env_vars::SNOWFLAKE_DATABASE, param_names::DATABASE),
        (env_vars::SNOWFLAKE_SCHEMA, param_names::SCHEMA),
    ] {
        if let Ok(value) = std::env::var(env_var)
            && !value.is_empty()
        {
            store.insert(param.into(), Setting::String(value));
        }
    }
}

/// Resolve final settings by merging explicit settings with file-based
/// config and registry defaults.
///
/// Precedence (highest to lowest):
/// 1. Explicit programmatic settings (from `SetOptions` / `SetOption*` RPCs)
/// 2. TOML file: `connections.toml` `[connection_name]` section
/// 3. TOML file: `config.toml` `[connections.connection_name]` section
///    3.5. SPCS environment variables (`SNOWFLAKE_ACCOUNT` / `HOST` / `DATABASE` / `SCHEMA`)
/// 4. Registry defaults (`ParamDef::default`)
///
/// `explicit` contains values set via the programmatic API (already
/// alias-resolved and type-checked by `connection_set_options`).
///
/// When `no_connection_details` is `true`, the caller supplied no
/// connection-identifying options (a bare `connect()`), so the default
/// connection profile from `connections.toml` is loaded and merged
/// underneath — honoring `SNOWFLAKE_DEFAULT_CONNECTION_NAME` and
/// `config.toml`'s `default_connection_name`. This mirrors the legacy Python
/// driver's `is_kwargs_empty` contract. The signal is computed by each
/// language wrapper (which alone can see the raw caller input before
/// bookkeeping params are injected) and carried as a typed field on
/// `ConnectionSetOptionsRequest`, not inferred from the merged params here.
pub fn resolve(
    explicit: &ParamStore,
    no_connection_details: bool,
) -> Result<ParamStore, ConfigError> {
    let paths = crate::config::path_resolver::get_config_paths()?;
    resolve_with_paths(explicit, &paths, no_connection_details)
}

/// Same as [`resolve`] but accepts explicit config file paths (for testing).
pub fn resolve_with_paths(
    explicit: &ParamStore,
    paths: &ConfigPaths,
    no_connection_details: bool,
) -> Result<ParamStore, ConfigError> {
    let mut merged = ParamStore::new();

    // Layer 4: Registry defaults (lowest priority)
    for param in param_registry::registry().all_params() {
        if let Some(default) = param.default {
            merged.insert(param.canonical_name.to_owned(), default.into());
        }
    }

    // Layer 3.5: SPCS environment variables — fills in connection details when
    // running inside a Snowpark Container Services container.  Overrides
    // registry defaults; overridden by TOML profiles and explicit params.
    apply_spcs_env_vars(&mut merged);

    // Layer 3+2: TOML files.
    //
    // Load file-based config if:
    //   a) caller explicitly named a connection (`connection_name` param), OR
    //   b) `no_connection_details` is `true` — a bare connect() that should
    //      fall back to the default profile, honoring
    //      `SNOWFLAKE_DEFAULT_CONNECTION_NAME` and `config.toml`'s
    //      `default_connection_name`.
    //
    // `no_connection_details` is the authoritative signal from the wrapper: it
    // is `true` only when the caller supplied no connection options at all.
    // The core does not re-derive this from the presence/absence of locator
    // params, because wrappers always inject bookkeeping params (application,
    // client_app_id, …) even on a bare call — so the merged `explicit` store
    // can never look "empty" here, and a locator heuristic would diverge from
    // the legacy `is_kwargs_empty` contract (e.g. `connect(user="alice")`).
    let permission_check = if matches!(
        explicit.get(param_names::UNSAFE_SKIP_CONFIG_FILE_PERMISSIONS_CHECK),
        Some(Setting::Bool(true))
    ) {
        FilePermissionCheck::UnsafeDisabled
    } else {
        FilePermissionCheck::Enabled
    };

    let connection_name: Option<String> =
        if let Some(Setting::String(name)) = explicit.get(param_names::CONNECTION_NAME) {
            Some(name.clone())
        } else if no_connection_details {
            Some(config_manager::get_default_connection_name_with_paths(
                paths,
                permission_check,
            )?)
        } else {
            None
        };

    if let Some(ref name) = connection_name {
        let file_settings =
            config_manager::load_connection_config_with_paths(name, paths, permission_check)?;
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

        let resolved = resolve_with_paths(&explicit, &paths, false).unwrap();

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

        let resolved = resolve_with_paths(&explicit, &paths, false).unwrap();

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

        let resolved = resolve_with_paths(&explicit, &paths, false).unwrap();

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

        let resolved = resolve_with_paths(&explicit, &paths, false).unwrap();

        if let Some(Setting::String(account)) = resolved.get(param_names::ACCOUNT) {
            assert_eq!(account, "explicit_account");
        } else {
            panic!("Expected account setting");
        }

        // protocol has no registry default (handled by consumption code)
        assert_eq!(resolved.get(param_names::PROTOCOL), None);
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

        let resolved = resolve_with_paths(&explicit, &paths, false).unwrap();

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
        // protocol has no registry default
        assert_eq!(get_str(&resolved, param_names::PROTOCOL), None);
    }

    // --- Default-profile fallback tests (SNOW-3647714) ---

    #[test]
    fn bare_connect_loads_default_profile() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[default]
account = "default_acct"
user = "default_user"
"#,
        );

        let explicit = ParamStore::new();
        let resolved = resolve_with_paths(&explicit, &paths, true).unwrap();

        assert_eq!(
            get_str(&resolved, param_names::ACCOUNT),
            Some("default_acct".to_owned())
        );
        assert_eq!(
            get_str(&resolved, param_names::USER),
            Some("default_user".to_owned())
        );
    }

    #[test]
    fn bare_connect_honors_default_connection_name_from_env_via_config_manager() {
        // The env-var branch is tested directly in config_manager unit tests.
        // Here we verify resolver wires it end-to-end via config.toml so the
        // test stays free of process-global env mutation (which races in
        // parallel test execution).
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"default_connection_name = "alt""#,
        );
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[default]
account = "should_not_be_used"
user = "wrong_user"

[alt]
account = "alt_acct"
user = "alt_user"
"#,
        );

        let explicit = ParamStore::new();
        let resolved = resolve_with_paths(&explicit, &paths, true).unwrap();
        assert_eq!(
            get_str(&resolved, param_names::ACCOUNT),
            Some("alt_acct".to_owned())
        );
        assert_eq!(
            get_str(&resolved, param_names::USER),
            Some("alt_user".to_owned())
        );
    }

    #[test]
    fn bare_connect_honors_default_connection_name_in_config_toml() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"
default_connection_name = "alt"
"#,
        );
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[default]
account = "should_not_be_used"

[alt]
account = "alt_acct"
user = "alt_user"
"#,
        );

        let explicit = ParamStore::new();
        let resolved = resolve_with_paths(&explicit, &paths, true).unwrap();

        assert_eq!(
            get_str(&resolved, param_names::ACCOUNT),
            Some("alt_acct".to_owned())
        );
    }

    #[test]
    fn explicit_params_do_not_trigger_default_profile_load() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[default]
account = "default_acct"
user = "profile_user"
password = "profile_pwd"
"#,
        );

        // Caller passes explicit account without bare_connect — must NOT merge [default]
        let mut explicit = ParamStore::new();
        explicit.insert(
            "account".to_owned(),
            Setting::String("explicit_acct".to_owned()),
        );

        let resolved = resolve_with_paths(&explicit, &paths, false).unwrap();

        assert_eq!(
            get_str(&resolved, param_names::ACCOUNT),
            Some("explicit_acct".to_owned())
        );
        // user must NOT have leaked in from [default]
        assert_eq!(get_str(&resolved, param_names::USER), None);
    }

    // --- apply_spcs_env_vars tests ---

    static SPCS_ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn spcs_env_vars_noop_when_gate_not_set() {
        let _lock = SPCS_ENV_MUTEX.lock().unwrap();
        // SAFETY: test-only; serialised by SPCS_ENV_MUTEX.
        unsafe {
            std::env::remove_var(env_vars::SNOWFLAKE_RUNNING_INSIDE_SPCS);
            std::env::remove_var(env_vars::SNOWFLAKE_ACCOUNT);
            std::env::remove_var(env_vars::SNOWFLAKE_HOST);
        }
        let mut store = ParamStore::new();
        apply_spcs_env_vars(&mut store);
        assert_eq!(store.get(param_names::ACCOUNT), None);
        assert_eq!(store.get(param_names::HOST), None);
    }

    #[test]
    fn spcs_env_vars_populate_account_and_host_when_gate_set() {
        let _lock = SPCS_ENV_MUTEX.lock().unwrap();
        // SAFETY: test-only; serialised by SPCS_ENV_MUTEX.
        unsafe {
            std::env::set_var(env_vars::SNOWFLAKE_RUNNING_INSIDE_SPCS, "true");
            std::env::set_var(env_vars::SNOWFLAKE_ACCOUNT, "myaccount");
            std::env::set_var(env_vars::SNOWFLAKE_HOST, "myaccount.snowflakecomputing.com");
            std::env::remove_var(env_vars::SNOWFLAKE_DATABASE);
            std::env::remove_var(env_vars::SNOWFLAKE_SCHEMA);
        }
        let mut store = ParamStore::new();
        apply_spcs_env_vars(&mut store);
        // SAFETY: cleanup.
        unsafe {
            std::env::remove_var(env_vars::SNOWFLAKE_RUNNING_INSIDE_SPCS);
            std::env::remove_var(env_vars::SNOWFLAKE_ACCOUNT);
            std::env::remove_var(env_vars::SNOWFLAKE_HOST);
        }
        assert_eq!(
            store.get(param_names::ACCOUNT),
            Some(&Setting::String("myaccount".into()))
        );
        assert_eq!(
            store.get(param_names::HOST),
            Some(&Setting::String("myaccount.snowflakecomputing.com".into()))
        );
        assert_eq!(store.get(param_names::DATABASE), None);
        assert_eq!(store.get(param_names::SCHEMA), None);
    }

    #[test]
    fn spcs_env_vars_database_and_schema_are_optional() {
        let _lock = SPCS_ENV_MUTEX.lock().unwrap();
        // SAFETY: test-only; serialised by SPCS_ENV_MUTEX.
        unsafe {
            std::env::set_var(env_vars::SNOWFLAKE_RUNNING_INSIDE_SPCS, "true");
            std::env::set_var(env_vars::SNOWFLAKE_ACCOUNT, "acct");
            std::env::set_var(env_vars::SNOWFLAKE_HOST, "acct.snowflakecomputing.com");
            std::env::set_var(env_vars::SNOWFLAKE_DATABASE, "mydb");
            std::env::set_var(env_vars::SNOWFLAKE_SCHEMA, "myschema");
        }
        let mut store = ParamStore::new();
        apply_spcs_env_vars(&mut store);
        // SAFETY: cleanup.
        unsafe {
            std::env::remove_var(env_vars::SNOWFLAKE_RUNNING_INSIDE_SPCS);
            std::env::remove_var(env_vars::SNOWFLAKE_ACCOUNT);
            std::env::remove_var(env_vars::SNOWFLAKE_HOST);
            std::env::remove_var(env_vars::SNOWFLAKE_DATABASE);
            std::env::remove_var(env_vars::SNOWFLAKE_SCHEMA);
        }
        assert_eq!(
            store.get(param_names::DATABASE),
            Some(&Setting::String("mydb".into()))
        );
        assert_eq!(
            store.get(param_names::SCHEMA),
            Some(&Setting::String("myschema".into()))
        );
    }

    #[test]
    fn spcs_env_vars_overridden_by_explicit_in_full_resolve() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);

        let _lock = SPCS_ENV_MUTEX.lock().unwrap();
        // SAFETY: test-only; serialised by SPCS_ENV_MUTEX.
        unsafe {
            std::env::set_var(env_vars::SNOWFLAKE_RUNNING_INSIDE_SPCS, "true");
            std::env::set_var(env_vars::SNOWFLAKE_ACCOUNT, "spcs-account");
            std::env::set_var(
                env_vars::SNOWFLAKE_HOST,
                "spcs-account.snowflakecomputing.com",
            );
            std::env::set_var(env_vars::SNOWFLAKE_DATABASE, "spcs-db");
            std::env::remove_var(env_vars::SNOWFLAKE_SCHEMA);
        }
        let mut explicit = ParamStore::new();
        explicit.insert(
            "account".to_owned(),
            Setting::String("explicit-account".to_owned()),
        );
        let resolved = resolve_with_paths(&explicit, &paths, false);
        // SAFETY: cleanup.
        unsafe {
            std::env::remove_var(env_vars::SNOWFLAKE_RUNNING_INSIDE_SPCS);
            std::env::remove_var(env_vars::SNOWFLAKE_ACCOUNT);
            std::env::remove_var(env_vars::SNOWFLAKE_HOST);
            std::env::remove_var(env_vars::SNOWFLAKE_DATABASE);
        }
        let resolved = resolved.unwrap();
        // Explicit wins over SPCS env var.
        assert_eq!(
            get_str(&resolved, param_names::ACCOUNT),
            Some("explicit-account".to_owned())
        );
        // SPCS HOST env var still populates when not explicitly overridden.
        assert_eq!(
            get_str(&resolved, param_names::HOST),
            Some("spcs-account.snowflakecomputing.com".to_owned())
        );
        // SPCS DATABASE env var populates.
        assert_eq!(
            get_str(&resolved, param_names::DATABASE),
            Some("spcs-db".to_owned())
        );
    }

    #[test]
    fn spcs_env_vars_overridden_by_toml_profile() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[myconn]
account = "toml-account"
host = "toml-account.snowflakecomputing.com"
database = "toml-db"
"#,
        );

        let _lock = SPCS_ENV_MUTEX.lock().unwrap();
        // SAFETY: test-only; serialised by SPCS_ENV_MUTEX.
        unsafe {
            std::env::set_var(env_vars::SNOWFLAKE_RUNNING_INSIDE_SPCS, "true");
            std::env::set_var(env_vars::SNOWFLAKE_ACCOUNT, "spcs-account");
            std::env::set_var(
                env_vars::SNOWFLAKE_HOST,
                "spcs-account.snowflakecomputing.com",
            );
            std::env::set_var(env_vars::SNOWFLAKE_DATABASE, "spcs-db");
            std::env::remove_var(env_vars::SNOWFLAKE_SCHEMA);
        }
        let mut explicit = ParamStore::new();
        explicit.insert(
            "connection_name".to_owned(),
            Setting::String("myconn".to_owned()),
        );
        let resolved = resolve_with_paths(&explicit, &paths, false);
        // SAFETY: cleanup.
        unsafe {
            std::env::remove_var(env_vars::SNOWFLAKE_RUNNING_INSIDE_SPCS);
            std::env::remove_var(env_vars::SNOWFLAKE_ACCOUNT);
            std::env::remove_var(env_vars::SNOWFLAKE_HOST);
            std::env::remove_var(env_vars::SNOWFLAKE_DATABASE);
        }
        let resolved = resolved.unwrap();
        // TOML wins over SPCS env vars.
        assert_eq!(
            get_str(&resolved, param_names::ACCOUNT),
            Some("toml-account".to_owned())
        );
        assert_eq!(
            get_str(&resolved, param_names::DATABASE),
            Some("toml-db".to_owned())
        );
    }

    #[test]
    fn non_locator_user_params_do_not_trigger_default_profile_load() {
        // connect(user="alice") supplied an option, so the wrapper sets
        // no_connection_details=false. Even though no locator (account/host) is
        // present, the resolver must NOT load the default profile — legacy
        // `is_kwargs_empty` parity. The connect then fails on missing account.
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[default]
account = "default_acct"
user = "profile_user"
password = "profile_pwd"
"#,
        );

        let mut explicit = ParamStore::new();
        explicit.insert("user".to_owned(), Setting::String("alice".to_owned()));

        let resolved = resolve_with_paths(&explicit, &paths, false).unwrap();

        // account must NOT have been loaded from [default]
        assert_eq!(get_str(&resolved, param_names::ACCOUNT), None);
        // user comes from explicit, not from [default]
        assert_eq!(
            get_str(&resolved, param_names::USER),
            Some("alice".to_owned())
        );
    }

    #[test]
    fn bare_connect_with_no_default_profile_errors() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[other]
account = "other_acct"
"#,
        );

        let explicit = ParamStore::new();
        let result = resolve_with_paths(&explicit, &paths, true);

        let err = result.unwrap_err();
        assert!(
            matches!(&err, ConfigError::ConnectionNotFound { name, .. } if name == "default"),
            "Expected ConnectionNotFound for 'default', got: {err}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn skip_flag_allows_loading_connection_from_world_writable_file() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);

        let connections_path = temp_dir.path().join("connections.toml");
        fs::write(
            &connections_path,
            "[myconn]\naccount = \"myaccount\"\nuser = \"myuser\"\n",
        )
        .unwrap();
        // World-writable: normally rejected by check_file_permissions
        fs::set_permissions(&connections_path, fs::Permissions::from_mode(0o666)).unwrap();

        // Without the flag: should fail with InsecurePermissions
        let mut explicit = ParamStore::new();
        explicit.insert(
            param_names::CONNECTION_NAME.into(),
            Setting::String("myconn".to_owned()),
        );
        assert!(
            matches!(
                resolve_with_paths(&explicit, &paths, false),
                Err(crate::config::ConfigError::InsecurePermissions { .. })
            ),
            "expected InsecurePermissions without skip flag"
        );

        // With the flag: should succeed
        explicit.insert(
            param_names::UNSAFE_SKIP_CONFIG_FILE_PERMISSIONS_CHECK.into(),
            Setting::Bool(true),
        );
        let resolved = resolve_with_paths(&explicit, &paths, false).unwrap();
        assert_eq!(
            get_str(&resolved, param_names::ACCOUNT),
            Some("myaccount".to_owned())
        );
    }

    // --- Connection diagnostic params via connections.toml (SNOW-3864169) ---

    #[test]
    fn connections_toml_profile_enables_diagnostic() {
        use crate::config::connection_config::{ConnectionConfig, DiagnosticConfig};

        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[production]
account = "myaccount"
user = "myuser"
password = "mypassword"
enable_connection_diag = true
connection_diag_log_path = "/var/log/sfdiag"
connection_diag_allowlist_path = "/var/snowflake/allowlist.json"
"#,
        );

        let mut explicit = ParamStore::new();
        explicit.insert(
            param_names::CONNECTION_NAME.into(),
            Setting::String("production".to_owned()),
        );

        let resolved = resolve_with_paths(&explicit, &paths, false).unwrap();
        let config = ConnectionConfig::build(&resolved).unwrap();

        match config.diagnostic {
            DiagnosticConfig::Enabled {
                log_path,
                allowlist_path,
            } => {
                assert_eq!(log_path, Some("/var/log/sfdiag".into()));
                assert_eq!(allowlist_path, Some("/var/snowflake/allowlist.json".into()));
            }
            DiagnosticConfig::Disabled => {
                panic!(
                    "enable_connection_diag = true in a connections.toml profile \
                     must produce DiagnosticConfig::Enabled"
                );
            }
        }
    }

    #[test]
    fn connections_toml_profile_leaves_diagnostic_disabled_by_default() {
        use crate::config::connection_config::{ConnectionConfig, DiagnosticConfig};

        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[production]
account = "myaccount"
user = "myuser"
password = "mypassword"
"#,
        );

        let mut explicit = ParamStore::new();
        explicit.insert(
            param_names::CONNECTION_NAME.into(),
            Setting::String("production".to_owned()),
        );

        let resolved = resolve_with_paths(&explicit, &paths, false).unwrap();
        let config = ConnectionConfig::build(&resolved).unwrap();

        assert!(matches!(config.diagnostic, DiagnosticConfig::Disabled));
    }
}
