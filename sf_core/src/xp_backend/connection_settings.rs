//! Connection settings a host-owned connection needs to pass validation. Lives
//! here so the only trace of the backend in [`crate::apis::database_driver_v1`]
//! is the one line calling [`seed_settings_for_host_backend`].

use crate::config::param_registry::param_names;
use crate::config::settings::Settings;

/// Distinctive so that if one escapes into a request or a log, its origin is
/// obvious.
const PLACEHOLDER_ACCOUNT: &str = "sf-core-host-backend";
const PLACEHOLDER_TOKEN: &str = "sf-core-host-backend-session";
/// `.invalid` (RFC 2606) can never resolve, so a stray connection attempt fails
/// at DNS rather than reaching a real host.
const PLACEHOLDER_HOST: &str = "sf-core-host-backend.invalid";

/// A host backend is already inside an authenticated session, so it has no
/// account to name and no credential to present — but validation rejects the
/// connection before [`crate::rest::snowflake::snowflake_login_with_client`],
/// where the backend takes over, is ever reached. Nothing reads these values;
/// they only need to be present.
///
/// `session_token` rather than user/password because it is the one shape
/// `validate_settings` accepts without a principal, which truthfully describes a
/// connection whose principal was established elsewhere.
///
/// `running_inside_xp` is the same flag [`super::registry::XpSlot`] already
/// resolved at driver construction. No-op when it is false, and only fills what
/// is missing.
pub(crate) fn seed_settings_for_host_backend(settings: &mut dyn Settings, running_inside_xp: bool) {
    if !running_inside_xp {
        return;
    }
    seed_placeholders(settings);
}

fn seed_placeholders(settings: &mut dyn Settings) {
    fill_if_unset(settings, param_names::ACCOUNT.as_str(), PLACEHOLDER_ACCOUNT);
    // A real session_token without a master_token is a misconfiguration;
    // inventing a placeholder master would pair them and pass `build_auth_config`.
    if is_unset(settings, param_names::SESSION_TOKEN.as_str())
        && is_unset(settings, param_names::MASTER_TOKEN.as_str())
    {
        fill_if_unset(
            settings,
            param_names::SESSION_TOKEN.as_str(),
            PLACEHOLDER_TOKEN,
        );
        fill_if_unset(
            settings,
            param_names::MASTER_TOKEN.as_str(),
            PLACEHOLDER_TOKEN,
        );
    }
    if is_unset(settings, param_names::SERVER_URL.as_str()) {
        fill_if_unset(settings, param_names::HOST.as_str(), PLACEHOLDER_HOST);
    }
}

fn is_unset(settings: &dyn Settings, key: &str) -> bool {
    settings
        .get_string(key)
        .is_none_or(|existing| existing.is_empty())
}

fn fill_if_unset(settings: &mut dyn Settings, key: &str, value: &str) {
    if is_unset(settings, key) {
        settings.set_string(key, value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ParamStore;
    use crate::config::connection_config::{ValidationSeverity, validate_settings};
    use crate::config::settings::Setting;
    use std::collections::HashMap;

    #[test]
    fn placeholders_satisfy_connection_config_validation() {
        let mut store = ParamStore::new();
        seed_placeholders(&mut store);

        let errors: Vec<_> = validate_settings(&store)
            .into_iter()
            .filter(|issue| issue.severity == ValidationSeverity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "seeded settings should leave no validation errors, got: {errors:?}"
        );
    }

    #[test]
    fn placeholders_never_overwrite_real_settings() {
        let mut settings: HashMap<String, Setting> = HashMap::new();
        settings.insert(
            "account".to_string(),
            Setting::String("realaccount".to_string()),
        );
        settings.insert(
            "host".to_string(),
            Setting::String("realaccount.snowflakecomputing.com".to_string()),
        );
        settings.insert(
            "session_token".to_string(),
            Setting::String("real-token".to_string()),
        );
        settings.insert(
            "master_token".to_string(),
            Setting::String("real-master".to_string()),
        );

        seed_placeholders(&mut settings);

        assert_eq!(
            Settings::get(&settings, "account"),
            Some(Setting::String("realaccount".to_string()))
        );
        assert_eq!(
            Settings::get(&settings, "host"),
            Some(Setting::String(
                "realaccount.snowflakecomputing.com".to_string()
            ))
        );
        assert_eq!(
            Settings::get(&settings, "session_token"),
            Some(Setting::String("real-token".to_string()))
        );
        assert_eq!(
            Settings::get(&settings, "master_token"),
            Some(Setting::String("real-master".to_string()))
        );
    }

    #[test]
    fn a_real_session_token_does_not_get_a_placeholder_master() {
        let mut settings: HashMap<String, Setting> = HashMap::new();
        settings.insert(
            "session_token".to_string(),
            Setting::String("real-token".to_string()),
        );

        seed_placeholders(&mut settings);

        assert_eq!(
            Settings::get(&settings, "session_token"),
            Some(Setting::String("real-token".to_string()))
        );
        assert_eq!(Settings::get(&settings, "master_token"), None);
    }

    #[test]
    fn a_supplied_server_url_suppresses_the_placeholder_host() {
        let mut settings: HashMap<String, Setting> = HashMap::new();
        settings.insert(
            "server_url".to_string(),
            Setting::String("https://real.snowflakecomputing.com".to_string()),
        );

        seed_placeholders(&mut settings);

        assert_eq!(Settings::get(&settings, "host"), None);
    }

    #[test]
    fn placeholder_host_can_never_resolve() {
        assert!(PLACEHOLDER_HOST.ends_with(".invalid"));
    }

    #[test]
    fn seeding_is_a_no_op_outside_xp() {
        let mut store = ParamStore::new();
        seed_settings_for_host_backend(&mut store, false);
        assert_eq!(store.get_string(param_names::ACCOUNT), None);
        assert_eq!(store.get_string(param_names::SESSION_TOKEN), None);
        assert_eq!(store.get_string(param_names::MASTER_TOKEN), None);
        assert_eq!(store.get_string(param_names::HOST), None);
    }

    #[test]
    fn seeding_inside_xp_fills_missing_placeholders() {
        let mut store = ParamStore::new();
        seed_settings_for_host_backend(&mut store, true);
        assert_eq!(
            store.get_string(param_names::ACCOUNT).as_deref(),
            Some(PLACEHOLDER_ACCOUNT)
        );
        assert_eq!(
            store.get_string(param_names::SESSION_TOKEN).as_deref(),
            Some(PLACEHOLDER_TOKEN)
        );
        assert_eq!(
            store.get_string(param_names::MASTER_TOKEN).as_deref(),
            Some(PLACEHOLDER_TOKEN)
        );
        assert_eq!(
            store.get_string(param_names::HOST).as_deref(),
            Some(PLACEHOLDER_HOST)
        );
    }
}
