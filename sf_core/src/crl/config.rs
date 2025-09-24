use crate::config::ConfigError;
use crate::config::settings::Settings;
use chrono::Duration;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum CertRevocationCheckMode {
    Disabled,
    Enabled,
    Advisory,
}

impl Default for CertRevocationCheckMode {
    fn default() -> Self {
        Self::Disabled
    }
}

#[derive(Debug, Clone)]
pub struct CrlConfig {
    pub check_mode: CertRevocationCheckMode,
    pub enable_disk_caching: bool,
    pub enable_memory_caching: bool,
    pub cache_dir: Option<PathBuf>,
    pub validity_time: Duration,
    pub allow_certificates_without_crl_url: bool,
    pub http_timeout: Duration,
    pub connection_timeout: Duration,
}

impl Default for CrlConfig {
    fn default() -> Self {
        Self {
            check_mode: CertRevocationCheckMode::Disabled,
            enable_disk_caching: true,
            enable_memory_caching: true,
            cache_dir: None,
            validity_time: Duration::days(10),
            allow_certificates_without_crl_url: false,
            http_timeout: Duration::seconds(30),
            connection_timeout: Duration::seconds(10),
        }
    }
}

impl CrlConfig {
    pub fn default_cache_dir() -> Option<PathBuf> {
        dirs::cache_dir().map(|mut p| {
            p.push("snowflake");
            p.push("crls");
            p
        })
    }

    pub fn get_cache_dir(&self) -> Option<PathBuf> {
        self.cache_dir.clone().or_else(Self::default_cache_dir)
    }

    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        let check_mode = match settings.get_string("crl_check_mode").as_deref() {
            Some("0") | Some("DISABLED") | None => CertRevocationCheckMode::Disabled,
            Some("1") | Some("ENABLED") => CertRevocationCheckMode::Enabled,
            Some("2") | Some("ADVISORY") => CertRevocationCheckMode::Advisory,
            Some(other) => {
                tracing::warn!("Unknown crl_check_mode: {other}, using DISABLED");
                CertRevocationCheckMode::Disabled
            }
        };
        let enable_disk_caching = settings
            .get_string("crl_enable_disk_caching")
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(true);
        let enable_memory_caching = settings
            .get_string("crl_enable_memory_caching")
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(true);
        let cache_dir = settings.get_string("crl_cache_dir").map(PathBuf::from);
        let validity_time = settings
            .get_int("crl_validity_time")
            .map(Duration::days)
            .unwrap_or(Duration::days(10));
        let allow_certificates_without_crl_url = settings
            .get_string("crl_allow_certificates_without_crl_url")
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(false);
        let http_timeout = settings
            .get_int("crl_http_timeout")
            .map(Duration::seconds)
            .unwrap_or(Duration::seconds(30));
        let connection_timeout = settings
            .get_int("crl_connection_timeout")
            .map(Duration::seconds)
            .unwrap_or(Duration::seconds(10));
        Ok(Self {
            check_mode,
            enable_disk_caching,
            enable_memory_caching,
            cache_dir,
            validity_time,
            allow_certificates_without_crl_url,
            http_timeout,
            connection_timeout,
        })
    }
}
