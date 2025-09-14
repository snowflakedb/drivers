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
        settings: &std::collections::HashMap<String, crate::config::settings::Setting>,
    ) -> Self {
        let mut cfg = TlsConfig::default();
        if let Some(crate::config::settings::Setting::String(path)) =
            settings.get("custom_root_store_path")
        {
            cfg.custom_root_store_path = Some(std::path::PathBuf::from(path));
        }
        if let Some(crate::config::settings::Setting::String(v)) = settings.get("verify_hostname") {
            cfg.verify_hostname = v.to_lowercase() == "true";
        }
        if let Some(crate::config::settings::Setting::String(v)) =
            settings.get("verify_certificates")
        {
            cfg.verify_certificates = v.to_lowercase() == "true";
        }
        cfg
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
