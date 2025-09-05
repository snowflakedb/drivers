use crate::config::ConfigError;
use crate::config::settings::Settings;
use chrono::Duration;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum CertRevocationCheckMode {
    /// Default - disables CRL checking (TLS handshake still in place)
    Disabled,
    /// Fails the connection if certificate is revoked or there is other revocation status check issue
    Enabled,
    /// Fails the request for revoked certificate only. In case of any other problems
    /// (like connection issues with CRL endpoints, CRL parsing errors etc) assumes
    /// that the certificate is not revoked and allows to connect.
    Advisory,
}

impl Default for CertRevocationCheckMode {
    fn default() -> Self {
        Self::Disabled
    }
}

#[derive(Debug, Clone)]
pub struct CrlConfig {
    /// Certificate revocation check mode
    pub check_mode: CertRevocationCheckMode,

    /// Whether CRL disk caching should be used - enabled by default
    pub enable_disk_caching: bool,

    /// Whether revocation status should be also cached in memory - enabled by default
    pub enable_memory_caching: bool,

    /// Optional cache dir (default explained in platform-specific defaults)
    pub cache_dir: Option<PathBuf>,

    /// How long should we keep the CRL version on disk before reaching the fresh one. Default: 10 days.
    pub validity_time: Duration,

    /// Allows to open connection in CRLMode = ENABLED in case of CRL Distribution Point URL link absent
    /// (meaning certificate may rely on OCSP only)
    pub allow_certificates_without_crl_url: bool,

    /// HTTP connection timeout for CRL fetching
    pub http_timeout: Duration,

    /// Socket connection timeout for CRL fetching
    pub connection_timeout: Duration,
}

impl Default for CrlConfig {
    fn default() -> Self {
        Self {
            check_mode: CertRevocationCheckMode::Disabled,
            enable_disk_caching: true,
            enable_memory_caching: true,
            cache_dir: None, // Will use platform default
            validity_time: Duration::days(10),
            allow_certificates_without_crl_url: false,
            http_timeout: Duration::seconds(30),
            connection_timeout: Duration::seconds(10),
        }
    }
}

impl CrlConfig {
    /// Get the default cache directory for the current platform
    pub fn default_cache_dir() -> Option<PathBuf> {
        dirs::cache_dir().map(|mut path| {
            path.push("snowflake");
            path.push("crls");
            path
        })
    }

    /// Get the actual cache directory to use (either configured or default)
    pub fn get_cache_dir(&self) -> Option<PathBuf> {
        self.cache_dir.clone().or_else(Self::default_cache_dir)
    }

    /// Create CrlConfig from settings
    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        let check_mode = match settings.get_string("cert_revocation_check_mode").as_deref() {
            Some("DISABLED") | None => CertRevocationCheckMode::Disabled,
            Some("ENABLED") => CertRevocationCheckMode::Enabled,
            Some("ADVISORY") => CertRevocationCheckMode::Advisory,
            Some(other) => {
                tracing::warn!("Unknown cert_revocation_check_mode: {other}, using DISABLED");
                CertRevocationCheckMode::Disabled
            }
        };

        let enable_disk_caching = settings
            .get_string("enable_crl_disk_caching")
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(true);

        let enable_memory_caching = settings
            .get_string("enable_crl_memory_caching")
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(true);

        let cache_dir = settings
            .get_string("sf_crl_response_cache_dir")
            .map(PathBuf::from);

        let validity_time = settings
            .get_int("sf_crl_validity_time")
            .map(Duration::days)
            .unwrap_or(Duration::days(10));

        let allow_certificates_without_crl_url = settings
            .get_string("allow_certificates_without_crl_url")
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
