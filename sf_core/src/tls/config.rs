// TLS configuration that includes CRL settings and other TLS options
use crate::config::ConfigError;
use crate::config::settings::Settings;
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

    /// Build TlsConfig from generic Settings
    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        let crl_config = CrlConfig::from_settings(settings)?;

        let custom_root_store_path = settings
            .get_string("custom_root_store_path")
            .map(PathBuf::from);

        let verify_hostname = settings
            .get_string("verify_hostname")
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(true);

        let verify_certificates = settings
            .get_string("verify_certificates")
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(true);

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

impl TlsConfig {}
