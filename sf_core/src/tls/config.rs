use crate::crl::config::CrlConfig;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub proxy_url: Option<String>,
    pub no_proxy: Option<String>,
    pub use_proxy_env: bool,
    /// When true (default), an empty proxy value in the connection string
    /// explicitly means "no proxy" — overriding env vars or config file settings.
    /// When false, an empty proxy value is ignored.
    pub allow_empty_proxy: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            proxy_url: None,
            no_proxy: None,
            use_proxy_env: false,
            allow_empty_proxy: true,
        }
    }
}

impl ProxyConfig {
    pub fn from_settings(settings: &dyn crate::config::settings::Settings) -> Self {
        let raw_proxy = settings.get_string("proxy");
        let allow_empty_proxy = settings
            .get_string("allow_empty_proxy")
            .map(|s| s.eq_ignore_ascii_case("true") || s == "1")
            .or_else(|| settings.get_bool("allow_empty_proxy"))
            .unwrap_or(true);
        let proxy_url = match raw_proxy {
            Some(ref s) if s.is_empty() && !allow_empty_proxy => None,
            other => other,
        };
        let no_proxy = settings.get_string("no_proxy");
        let use_proxy_env = settings
            .get_string("use_proxy_env")
            .map(|s| s.eq_ignore_ascii_case("true") || s == "1")
            .or_else(|| settings.get_bool("use_proxy_env"))
            .unwrap_or(false);
        Self {
            proxy_url,
            no_proxy,
            use_proxy_env,
            allow_empty_proxy,
        }
    }
}

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
