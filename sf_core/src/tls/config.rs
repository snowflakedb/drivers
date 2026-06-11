use crate::config::ConfigError;
use crate::config::param_names::{
    CUSTOM_ROOT_STORE_PATH, TLS_SKIP_VERIFY, VERIFY_CERTIFICATES, VERIFY_HOSTNAME,
};
use crate::config::param_registry::{ParamKey, registry};
use crate::config::settings::{Setting, Settings};
use crate::crl::config::CrlConfig;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub crl_config: CrlConfig,
    pub custom_root_store_path: Option<PathBuf>,
    pub verify_hostname: bool,
    pub verify_certificates: bool,
}

impl TlsConfig {
    pub fn insecure() -> Self {
        Self {
            crl_config: CrlConfig::default(),
            custom_root_store_path: None,
            verify_hostname: false,
            verify_certificates: false,
        }
    }

    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        let crl_config = CrlConfig::from_settings(settings)?;
        let custom_root_store_path = lookup_setting(settings, CUSTOM_ROOT_STORE_PATH)
            .and_then(|s| match s {
                Setting::String(path) => Some(path),
                _ => None,
            })
            .map(PathBuf::from);
        let skip_tls_verify = lookup_bool(settings, TLS_SKIP_VERIFY, false);
        let verify_hostname = !skip_tls_verify && lookup_bool(settings, VERIFY_HOSTNAME, true);
        let verify_certificates =
            !skip_tls_verify && lookup_bool(settings, VERIFY_CERTIFICATES, true);
        Ok(Self {
            crl_config,
            custom_root_store_path,
            verify_hostname,
            verify_certificates,
        })
    }
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            crl_config: CrlConfig::default(),
            custom_root_store_path: None,
            verify_hostname: true,
            verify_certificates: true,
        }
    }
}

/// Read a setting by canonical `ParamKey`, falling back to any aliases the
/// registry has for it. `build_tls_config` reads an already-canonicalized
/// `ParamStore`; this path may get a raw settings bag, so resolving aliases
/// here keeps both TLS-config builders honoring the same wrapper keys.
fn lookup_setting(settings: &dyn Settings, key: ParamKey) -> Option<Setting> {
    settings.get(key.as_str()).or_else(|| {
        registry()
            .resolve(key.as_str())
            .and_then(|def| def.aliases.iter().find_map(|&alias| settings.get(alias)))
    })
}

fn lookup_bool(settings: &dyn Settings, key: ParamKey, default: bool) -> bool {
    lookup_setting(settings, key)
        .and_then(|s| s.coerce_bool())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::Setting;
    use std::collections::HashMap;

    #[test]
    fn from_settings_skip_disables_both_for_bool_and_string() {
        for skip in [Setting::Bool(true), Setting::String("true".into())] {
            let mut s: HashMap<String, Setting> = HashMap::new();
            s.insert("tls_skip_verify".into(), skip);
            // Skip wins even when the individual flags are explicitly enabled.
            s.insert("verify_hostname".into(), Setting::Bool(true));
            s.insert("verify_certificates".into(), Setting::Bool(true));

            let cfg = TlsConfig::from_settings(&s).unwrap();
            assert!(!cfg.verify_hostname);
            assert!(!cfg.verify_certificates);
        }
    }

    #[test]
    fn from_settings_defaults_to_verifying() {
        let cfg = TlsConfig::from_settings(&HashMap::<String, Setting>::new()).unwrap();
        assert!(cfg.verify_hostname);
        assert!(cfg.verify_certificates);
    }

    #[test]
    fn from_settings_honors_registered_aliases() {
        // Raw bag using the registered alias (TLS_VERIFY_HOSTNAME) instead of the
        // canonical key must still take effect, matching the ParamStore path.
        let mut s: HashMap<String, Setting> = HashMap::new();
        s.insert("TLS_VERIFY_HOSTNAME".into(), Setting::Bool(false));

        let cfg = TlsConfig::from_settings(&s).unwrap();
        assert!(!cfg.verify_hostname);
        assert!(cfg.verify_certificates);
    }
}
