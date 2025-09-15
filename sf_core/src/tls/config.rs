// TLS configuration that includes CRL settings and other TLS options
use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Certificate revocation list configuration
    pub crl_config: CrlConfig,

    /// Path to custom root certificate store (PEM format)
    /// If None, uses system default root certificates
    pub custom_root_store_path: Option<PathBuf>,

    /// Whether to verify hostnames (should usually be true in production)
    pub verify_hostname: bool,

    /// Whether to verify certificates at all (dangerous if false)
    pub verify_certificates: bool,
}

// Default impl below (single impl)

impl TlsConfig {
    /// Create TLS config with CRL validation disabled
    pub fn new_without_crl() -> Self {
        Self {
            crl_config: CrlConfig {
                check_mode: CertRevocationCheckMode::Disabled,
                ..Default::default()
            },
            ..Default::default()
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

impl TlsConfig {
    /// Build TlsConfig from generic Settings trait object
    pub fn from_settings_dyn(settings: &dyn crate::config::settings::Settings) -> Self {
        let mut cfg = TlsConfig::default();
        if let Some(path) = settings.get_string("custom_root_store_path") {
            cfg.custom_root_store_path = Some(PathBuf::from(path));
        }
        if let Some(v) = settings.get_string("verify_hostname") {
            cfg.verify_hostname = v.to_lowercase() == "true";
        }
        if let Some(v) = settings.get_string("verify_certificates") {
            cfg.verify_certificates = v.to_lowercase() == "true";
        }
        cfg
    }
}
