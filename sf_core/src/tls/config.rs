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

    /// Create TLS config with CRL validation enabled
    pub fn new_with_crl(crl_mode: CertRevocationCheckMode) -> Self {
        Self {
            crl_config: CrlConfig {
                check_mode: crl_mode,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Set custom root certificate store path
    pub fn with_custom_root_store<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.custom_root_store_path = Some(path.into());
        self
    }

    /// Disable hostname verification (dangerous)
    pub fn without_hostname_verification(mut self) -> Self {
        self.verify_hostname = false;
        self
    }

    /// Disable certificate verification entirely (very dangerous)
    pub fn without_certificate_verification(mut self) -> Self {
        self.verify_certificates = false;
        self
    }

    /// Check if any form of TLS validation is disabled
    pub fn is_insecure(&self) -> bool {
        !self.verify_certificates || !self.verify_hostname
    }

    /// Check if CRL validation is enabled
    pub fn has_crl_validation(&self) -> bool {
        self.crl_config.check_mode != CertRevocationCheckMode::Disabled
    }
}

/// Simple builder-style configuration for common use cases
impl TlsConfig {
    /// Production configuration: full validation + CRL checking
    pub fn production() -> Self {
        Self::new_with_crl(CertRevocationCheckMode::Enabled)
    }

    /// Development configuration: full validation, CRL advisory mode
    pub fn development() -> Self {
        Self::new_with_crl(CertRevocationCheckMode::Advisory)
    }

    /// Testing configuration: no CRL validation, but still verify certificates
    pub fn testing() -> Self {
        Self::new_without_crl()
    }

    /// Insecure configuration: no validation at all (for testing only)
    pub fn insecure() -> Self {
        Self::new_without_crl()
            .without_certificate_verification()
            .without_hostname_verification()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TlsConfig::default();
        assert_eq!(
            config.crl_config.check_mode,
            CertRevocationCheckMode::Disabled
        );
        assert!(config.verify_hostname);
        assert!(config.verify_certificates);
        assert!(!config.is_insecure());
        assert!(!config.has_crl_validation());
    }

    #[test]
    fn test_production_config() {
        let config = TlsConfig::production();
        assert_eq!(
            config.crl_config.check_mode,
            CertRevocationCheckMode::Enabled
        );
        assert!(config.verify_hostname);
        assert!(config.verify_certificates);
        assert!(!config.is_insecure());
        assert!(config.has_crl_validation());
    }

    #[test]
    fn test_insecure_config() {
        let config = TlsConfig::insecure();
        assert!(!config.verify_hostname);
        assert!(!config.verify_certificates);
        assert!(config.is_insecure());
    }

    #[test]
    fn test_builder_pattern() {
        let config = TlsConfig::production().with_custom_root_store("/path/to/certs.pem");

        assert!(config.has_crl_validation());
        assert_eq!(
            config.custom_root_store_path,
            Some(PathBuf::from("/path/to/certs.pem"))
        );
    }
}
