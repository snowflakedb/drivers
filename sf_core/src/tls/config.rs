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

    pub fn from_settings(
        settings: &dyn crate::config::settings::Settings,
    ) -> Result<Self, crate::config::ConfigError> {
        let crl_config = CrlConfig::from_settings(settings)?;
        let custom_root_store_path = settings
            .get_string("custom_root_store_path")
            .map(PathBuf::from);
        // Accept both a typed `Bool` setting and a `"true"`/`"false"` string so
        // this path agrees with the `ParamStore`-backed `build_tls_config`
        // (which coerces both); otherwise a `Setting::Bool` would be ignored here.
        let bool_setting = |key: &str, default: bool| -> bool {
            settings
                .get_bool(key)
                .or_else(|| {
                    settings
                        .get_string(key)
                        .map(|s| s.eq_ignore_ascii_case("true"))
                })
                .unwrap_or(default)
        };
        let skip_tls_verify = bool_setting("insecure_skip_tls_verify", false);
        let verify_hostname = !skip_tls_verify && bool_setting("verify_hostname", true);
        let verify_certificates = !skip_tls_verify && bool_setting("verify_certificates", true);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::Setting;
    use std::collections::HashMap;

    #[test]
    fn from_settings_skip_disables_both_for_bool_and_string() {
        for skip in [Setting::Bool(true), Setting::String("true".into())] {
            let mut s: HashMap<String, Setting> = HashMap::new();
            s.insert("insecure_skip_tls_verify".into(), skip);
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
}
