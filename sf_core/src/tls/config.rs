use crate::crl::config::CrlConfig;
use crate::sensitive::SensitiveString;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub crl_config: CrlConfig,
    pub custom_root_store_path: Option<PathBuf>,
    pub verify_hostname: bool,
    pub verify_certificates: bool,
}

/// HTTP proxy server settings. All fields are optional; when `host` is `None`
/// the HTTP client falls back to reqwest's default env-var detection
/// (`HTTP_PROXY` / `HTTPS_PROXY` / `NO_PROXY`).
#[derive(Debug, Default, Clone)]
pub struct ProxyConfig {
    pub host: Option<String>,
    pub port: Option<i64>,
    pub user: Option<String>,
    pub password: Option<SensitiveString>,
    pub no_proxy: Option<String>,
}

impl ProxyConfig {
    /// Returns `true` if any explicit proxy connection parameter is set.
    /// When `false`, callers should leave reqwest's default env-var
    /// detection in place rather than calling `.proxy()`.
    pub fn is_explicit(&self) -> bool {
        self.host.is_some()
    }
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
