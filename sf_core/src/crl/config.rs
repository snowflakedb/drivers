use crate::config::ConfigError;
use crate::config::settings::Settings;
use chrono::Duration;
use std::path::PathBuf;

/// Maximum CRL download size in bytes.
pub const DEFAULT_CRL_DOWNLOAD_MAX_SIZE_BYTES: usize = 20 * 1024 * 1024; // 20 MB

/// Default max cache age, in seconds.
pub const DEFAULT_CRL_VALIDITY_TIME_SECS: i64 = 24 * 60 * 60;

/// Default delay, in seconds, before an expired CRL is purged from disk (kept
/// for debuggability). Matches the old Python driver and the design doc (7 days).
pub const DEFAULT_CRL_ON_DISK_CACHE_REMOVAL_DELAY_SECS: i64 = 7 * 24 * 60 * 60;

/// Default interval, in seconds, between background CRL cache cleanup passes.
pub const DEFAULT_CRL_CACHE_CLEANUP_INTERVAL_SECS: i64 = 60 * 60;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum CertRevocationCheckMode {
    /// Default - disables CRL checking (TLS handshake still in place)
    #[default]
    Disabled,
    /// Fails the connection if certificate is revoked or there is other revocation status check issue
    Enabled,
    /// Fails the request for revoked certificate only. In case of any other problems
    /// (like connection issues with CRL endpoints, CRL parsing errors etc) assumes
    /// that the certificate is not revoked and allows to connect.
    Advisory,
}

#[derive(Debug, Clone)]
pub struct CrlConfig {
    pub check_mode: CertRevocationCheckMode,
    pub enable_disk_caching: bool,
    pub enable_memory_caching: bool,
    pub cache_dir: Option<PathBuf>,
    pub allow_certificates_without_crl_url: bool,
    /// Maximum number of bytes to download for a single CRL before aborting.
    pub max_download_size: usize,
    /// Maximum age of a cached CRL (in memory or on disk) before it must be
    /// re-fetched, regardless of the CRL's own `nextUpdate`.
    pub validity_time: Duration,
    /// How long an expired CRL is retained on disk (past its `nextUpdate`)
    /// before the background cleaner removes it.
    pub on_disk_cache_removal_delay: Duration,
    /// Interval between background cache cleanup passes.
    pub cache_cleanup_interval: Duration,
    /// Whether the background cache cleanup runs at all.
    pub cache_start_cleanup: bool,
    /// Skip verification that on-disk cache files/dir are owner-only (0600/0700).
    pub unsafe_skip_file_permissions_check: bool,
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
            allow_certificates_without_crl_url: false,
            max_download_size: DEFAULT_CRL_DOWNLOAD_MAX_SIZE_BYTES,
            validity_time: Duration::seconds(DEFAULT_CRL_VALIDITY_TIME_SECS),
            on_disk_cache_removal_delay: Duration::seconds(
                DEFAULT_CRL_ON_DISK_CACHE_REMOVAL_DELAY_SECS,
            ),
            cache_cleanup_interval: Duration::seconds(DEFAULT_CRL_CACHE_CLEANUP_INTERVAL_SECS),
            cache_start_cleanup: false,
            unsafe_skip_file_permissions_check: false,
            http_timeout: Duration::seconds(10),
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
        let allow_certificates_without_crl_url = settings
            .get_string("crl_allow_certificates_without_crl_url")
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(false);
        // Configured directly in bytes (matching gosnowflake and the old
        // Python driver, which both use bytes).
        let max_download_size = settings
            .get_int("crl_max_download_size")
            .filter(|v| *v > 0)
            .map(|b| b as usize)
            .unwrap_or(DEFAULT_CRL_DOWNLOAD_MAX_SIZE_BYTES);
        // Cache lifetimes are configured in whole seconds.
        let validity_time = settings
            .get_int("crl_validity_time")
            .filter(|v| *v >= 0)
            .map(Duration::seconds)
            .unwrap_or(Duration::seconds(DEFAULT_CRL_VALIDITY_TIME_SECS));
        let on_disk_cache_removal_delay = settings
            .get_int("crl_on_disk_cache_removal_delay")
            .filter(|v| *v >= 0)
            .map(Duration::seconds)
            .unwrap_or(Duration::seconds(
                DEFAULT_CRL_ON_DISK_CACHE_REMOVAL_DELAY_SECS,
            ));
        let cache_cleanup_interval = settings
            .get_int("crl_cache_cleanup_interval")
            .filter(|v| *v > 0)
            .map(Duration::seconds)
            .unwrap_or(Duration::seconds(DEFAULT_CRL_CACHE_CLEANUP_INTERVAL_SECS));
        let cache_start_cleanup = settings
            .get_string("crl_cache_start_cleanup")
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(false);
        let unsafe_skip_file_permissions_check = settings
            .get_string("crl_unsafe_skip_file_permissions_check")
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(false);
        let http_timeout = settings
            .get_int("crl_http_timeout")
            .map(Duration::seconds)
            .unwrap_or(Duration::seconds(10));
        let connection_timeout = settings
            .get_int("crl_connection_timeout")
            .map(Duration::seconds)
            .unwrap_or(Duration::seconds(10));
        Ok(Self {
            check_mode,
            enable_disk_caching,
            enable_memory_caching,
            cache_dir,
            allow_certificates_without_crl_url,
            max_download_size,
            validity_time,
            on_disk_cache_removal_delay,
            cache_cleanup_interval,
            cache_start_cleanup,
            unsafe_skip_file_permissions_check,
            http_timeout,
            connection_timeout,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::Setting;
    use std::collections::HashMap;

    #[test]
    fn defaults_match_gosnowflake() {
        let cfg = CrlConfig::default();
        assert_eq!(cfg.validity_time, Duration::hours(24));
        // Removal delay follows the old Python driver / design doc (7 days),
        // not gosnowflake's 7 hours.
        assert_eq!(cfg.on_disk_cache_removal_delay, Duration::days(7));
        assert_eq!(cfg.max_download_size, 20 * 1024 * 1024);
        assert_eq!(cfg.http_timeout, Duration::seconds(10));
        assert!(!cfg.unsafe_skip_file_permissions_check);
    }

    #[test]
    fn from_settings_reads_seconds_and_bytes() {
        let mut map: HashMap<String, Setting> = HashMap::new();
        map.insert("crl_validity_time".into(), Setting::Int(3_600));
        map.insert("crl_on_disk_cache_removal_delay".into(), Setting::Int(600));
        map.insert("crl_max_download_size".into(), Setting::Int(1_048_576));
        map.insert("crl_cache_cleanup_interval".into(), Setting::Int(120));
        map.insert(
            "crl_cache_start_cleanup".into(),
            Setting::String("true".into()),
        );
        let cfg = CrlConfig::from_settings(&map).unwrap();
        assert_eq!(cfg.validity_time, Duration::seconds(3_600));
        assert_eq!(cfg.on_disk_cache_removal_delay, Duration::seconds(600));
        assert_eq!(cfg.max_download_size, 1_048_576);
        assert_eq!(cfg.cache_cleanup_interval, Duration::seconds(120));
        assert!(cfg.cache_start_cleanup);
    }

    #[test]
    fn from_settings_uses_defaults_when_absent() {
        let map: HashMap<String, Setting> = HashMap::new();
        let cfg = CrlConfig::from_settings(&map).unwrap();
        assert_eq!(cfg.validity_time, Duration::hours(24));
        assert_eq!(cfg.on_disk_cache_removal_delay, Duration::days(7));
        assert_eq!(cfg.max_download_size, 20 * 1024 * 1024);
        assert_eq!(cfg.cache_cleanup_interval, Duration::hours(1));
        assert!(!cfg.cache_start_cleanup);
        assert!(!cfg.unsafe_skip_file_permissions_check);
    }
}
