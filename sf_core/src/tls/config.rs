use crate::config::ConfigError;
use crate::config::InvalidParameterValueSnafu;
use crate::config::param_names::{
    CUSTOM_ROOT_STORE_PATH, MAX_TLS_VERSION, MIN_TLS_VERSION, TLS_SKIP_VERIFY, VERIFY_CERTIFICATES,
    VERIFY_HOSTNAME,
};
use crate::config::settings::{Setting, Settings};
use crate::crl::config::CrlConfig;
use crate::sensitive::SensitiveString;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

impl TlsVersion {
    pub(crate) fn parse(value: &str, parameter: &str) -> Result<Self, crate::config::ConfigError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "tls12" => Ok(Self::Tls12),
            "tls13" => Ok(Self::Tls13),
            "tls11" | "tls10" => InvalidParameterValueSnafu {
                parameter,
                value,
                explanation: "TLS versions below 1.2 are not supported; use tls12 or tls13"
                    .to_string(),
            }
            .fail(),
            _ => InvalidParameterValueSnafu {
                parameter,
                value,
                explanation: "expected one of: tls12, tls13".to_string(),
            }
            .fail(),
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Tls12 => "TLS 1.2",
            Self::Tls13 => "TLS 1.3",
        }
    }

    pub(crate) fn to_reqwest(self) -> reqwest::tls::Version {
        match self {
            Self::Tls12 => reqwest::tls::Version::TLS_1_2,
            Self::Tls13 => reqwest::tls::Version::TLS_1_3,
        }
    }

    pub(crate) fn to_rustls(self) -> &'static rustls::SupportedProtocolVersion {
        match self {
            Self::Tls12 => &rustls::version::TLS12,
            Self::Tls13 => &rustls::version::TLS13,
        }
    }
}

/// The `[min, max]` TLS protocol-version window negotiated on the wire,
/// resolved from the `min_tls_version` / `max_tls_version` parameters.
///
/// Bundled into one value so it threads through config, `StageInfo`, and the
/// client builders as a single argument rather than a pair. Defaults to the
/// full `Tls12..=Tls13` range — exactly what every TLS backend negotiates by
/// default, so a default window requires no pinning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlsVersions {
    pub min: TlsVersion,
    pub max: TlsVersion,
}

impl Default for TlsVersions {
    fn default() -> Self {
        Self {
            min: TlsVersion::Tls12,
            max: TlsVersion::Tls13,
        }
    }
}

impl TlsVersions {
    /// Parse the `min_tls_version` / `max_tls_version` settings into a window,
    /// applying defaults for absent values and rejecting an inverted window
    /// (`min > max`). A bad spelling or sub-1.2 value fails here rather than
    /// being silently ignored.
    pub(crate) fn from_settings(
        settings: &dyn crate::config::settings::Settings,
    ) -> Result<Self, crate::config::ConfigError> {
        let min_raw = settings.get_string(MIN_TLS_VERSION.as_str());
        let max_raw = settings.get_string(MAX_TLS_VERSION.as_str());
        let min = match min_raw.as_deref() {
            Some(v) => TlsVersion::parse(v, MIN_TLS_VERSION.as_str())?,
            None => TlsVersion::Tls12,
        };
        let max = match max_raw.as_deref() {
            Some(v) => TlsVersion::parse(v, MAX_TLS_VERSION.as_str())?,
            None => TlsVersion::Tls13,
        };
        if min > max {
            return InvalidParameterValueSnafu {
                parameter: MAX_TLS_VERSION.as_str(),
                value: max_raw.unwrap_or_else(|| max.label().to_string()),
                explanation: format!(
                    "max_tls_version ({}) must be at least min_tls_version ({})",
                    max.label(),
                    min.label(),
                ),
            }
            .fail();
        }
        Ok(Self { min, max })
    }

    /// The ordered set of rustls protocol versions enabled by the window.
    ///
    /// [`from_settings`](Self::from_settings) rejects `min > max`, so for any
    /// window produced through it this is non-empty. Callers that build a
    /// window by hand must uphold the same invariant; the rustls/AWS client
    /// paths guard defensively against an empty result.
    pub(crate) fn enabled_rustls_versions(self) -> Vec<&'static rustls::SupportedProtocolVersion> {
        [TlsVersion::Tls12, TlsVersion::Tls13]
            .into_iter()
            .filter(|v| *v >= self.min && *v <= self.max)
            .map(TlsVersion::to_rustls)
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub crl_config: CrlConfig,
    pub custom_root_store_path: Option<PathBuf>,
    pub verify_hostname: bool,
    pub verify_certificates: bool,
    /// TLS protocol-version window to negotiate (default `Tls12..=Tls13`).
    pub versions: TlsVersions,
}

/// HTTP proxy settings, supporting two equivalent input forms:
///
/// - **Individual fields** (`host`, `port`, `user`, `password`) — legacy
///   snowflake-connector-python kwargs.
/// - **Full URL** (`url`) — legacy ODBC `PROXY` DSN entry,
///   `[scheme://][user:pass@]host[:port]`.
///
/// Both forms are merged in `build_proxy_config`: the URL is parsed as a
/// baseline and individual fields override the corresponding URL components
/// when both are set.  Once construction is finished, `host`/`port`/`user`/
/// `password` carry the effective values and `url` is informational only.
///
/// `use_proxy_env` controls whether the HTTP client falls back to
/// `HTTP_PROXY`/`HTTPS_PROXY`/`NO_PROXY` env vars when no explicit proxy is
/// configured.  Default is `false`: env vars are ignored unless opted in.
///
/// `allow_empty_proxy` mirrors the legacy ODBC `AllowEmptyProxy` knob: when
/// `true` (default), an empty `PROXY` value explicitly disables the proxy and
/// overrides any env-var or config-file setting.
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub host: Option<String>,
    pub port: Option<i64>,
    pub user: Option<String>,
    pub password: Option<SensitiveString>,
    pub no_proxy: Option<String>,
    pub use_proxy_env: bool,
    pub allow_empty_proxy: bool,
    /// `true` if the customer passed an empty `PROXY` value with
    /// `allow_empty_proxy = true`; signals "explicitly disable proxy".
    pub explicitly_disabled: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            host: None,
            port: None,
            user: None,
            password: None,
            no_proxy: None,
            use_proxy_env: false,
            allow_empty_proxy: true,
            explicitly_disabled: false,
        }
    }
}

impl ProxyConfig {
    /// Returns `true` if any explicit proxy host is configured (URL or
    /// individual field). When `false`, the HTTP client either disables the
    /// proxy entirely or falls back to env vars depending on `use_proxy_env`.
    pub fn is_explicit(&self) -> bool {
        self.host.is_some()
    }

    /// Build a [`ProxyConfig`] from any `Settings` bag, merging the legacy
    /// ODBC `PROXY` URL form with the individual `proxy_host`/`proxy_port`/
    /// `proxy_user`/`proxy_password` fields.  Individual fields override URL
    /// components when both are set.
    pub fn from_settings(settings: &dyn crate::config::settings::Settings) -> Self {
        let allow_empty_proxy = settings.get_bool_or("allow_empty_proxy", true);
        let use_proxy_env = settings.get_bool_or("use_proxy_env", false);
        let raw_url = settings.get_string("proxy");

        let explicitly_disabled = matches!(&raw_url, Some(s) if s.is_empty()) && allow_empty_proxy;

        let parsed = raw_url
            .as_deref()
            .filter(|s| !s.is_empty())
            .and_then(parse_legacy_proxy_url);

        let host = settings
            .get_string("proxy_host")
            .or_else(|| parsed.as_ref().and_then(|p| p.host.clone()));
        // proxy_port may arrive as a string from TOML/DSN strings; coerce.
        let port = settings
            .get_int("proxy_port")
            .or_else(|| {
                settings
                    .get_string("proxy_port")
                    .and_then(|s| s.parse::<i64>().ok())
            })
            .or_else(|| parsed.as_ref().and_then(|p| p.port));
        let user = settings
            .get_string("proxy_user")
            .or_else(|| parsed.as_ref().and_then(|p| p.user.clone()));
        let password = settings
            .get_string("proxy_password")
            .map(SensitiveString::from)
            .or_else(|| parsed.as_ref().and_then(|p| p.password.clone()));

        Self {
            host,
            port,
            user,
            password,
            no_proxy: settings.get_string("no_proxy"),
            use_proxy_env,
            allow_empty_proxy,
            explicitly_disabled,
        }
    }
}

/// Components extracted from a legacy ODBC `PROXY` URL.
struct ParsedProxyUrl {
    host: Option<String>,
    port: Option<i64>,
    user: Option<String>,
    password: Option<SensitiveString>,
}

/// Parse `[scheme://][user:pass@]host[:port]`.  Returns `None` when the host
/// is missing.  Credentials are percent-decoded so values like `user%40corp`
/// round-trip correctly.
fn parse_legacy_proxy_url(raw: &str) -> Option<ParsedProxyUrl> {
    let with_scheme = if raw.contains("://") {
        raw.to_owned()
    } else {
        format!("http://{raw}")
    };
    let url = url::Url::parse(&with_scheme).ok()?;
    let host = url.host_str().map(|h| h.to_owned());
    host.as_ref()?;
    let port = url.port().map(i64::from);
    let user = if url.username().is_empty() {
        None
    } else {
        Some(
            urlencoding::decode(url.username())
                .map(|c| c.into_owned())
                .unwrap_or_else(|_| url.username().to_owned()),
        )
    };
    let password = url.password().map(|p| {
        SensitiveString::from(
            urlencoding::decode(p)
                .map(|c| c.into_owned())
                .unwrap_or_else(|_| p.to_owned()),
        )
    });
    Some(ParsedProxyUrl {
        host,
        port,
        user,
        password,
    })
}

impl TlsConfig {
    pub fn insecure() -> Self {
        Self {
            crl_config: CrlConfig::default(),
            custom_root_store_path: None,
            verify_hostname: false,
            verify_certificates: false,
            versions: TlsVersions::default(),
        }
    }

    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        let crl_config = CrlConfig::from_settings(settings)?;
        let custom_root_store_path = settings
            .get(CUSTOM_ROOT_STORE_PATH.as_str())
            .and_then(|s| match s {
                Setting::String(path) => Some(path),
                _ => None,
            })
            .map(PathBuf::from);
        let skip_tls_verify = settings.get_bool_or(TLS_SKIP_VERIFY.as_str(), false);
        if skip_tls_verify {
            tracing::warn!(
                "TLS verification disabled via tls_skip_verify: certificate, hostname, and CRL revocation checks are all bypassed. Do not use in production."
            );
        }
        let verify_hostname =
            !skip_tls_verify && settings.get_bool_or(VERIFY_HOSTNAME.as_str(), true);
        let verify_certificates =
            !skip_tls_verify && settings.get_bool_or(VERIFY_CERTIFICATES.as_str(), true);

        // The optional [min, max] TLS version window. Defaults match rustls'
        // effective default (1.2..=1.3), so behaviour is unchanged unless the
        // caller opts in; a bad spelling or inverted window fails here.
        let versions = TlsVersions::from_settings(settings)?;

        Ok(Self {
            crl_config,
            custom_root_store_path,
            verify_hostname,
            verify_certificates,
            versions,
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
            versions: TlsVersions::default(),
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
            s.insert("tls_skip_verify".into(), skip);
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

#[cfg(test)]
mod tls_version_tests {
    use super::*;
    use crate::config::settings::Setting;
    use std::collections::HashMap;

    fn settings(pairs: &[(&str, &str)]) -> HashMap<String, Setting> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), Setting::String(v.to_string())))
            .collect()
    }

    #[test]
    fn should_parse_tls12_and_tls13_case_insensitively() {
        for v in ["tls12", "TLS12"] {
            assert_eq!(
                TlsVersion::parse(v, "min_tls_version").unwrap(),
                TlsVersion::Tls12,
                "spelling {v:?} should parse as Tls12"
            );
        }
        for v in ["tls13", "TLS13"] {
            assert_eq!(
                TlsVersion::parse(v, "max_tls_version").unwrap(),
                TlsVersion::Tls13,
                "spelling {v:?} should parse as Tls13"
            );
        }
    }

    #[test]
    fn should_reject_sub_1_2_version_with_floor_message() {
        for v in ["tls11", "tls10", "TLS11", "TLs10"] {
            let err = TlsVersion::parse(v, "min_tls_version").unwrap_err();
            assert!(
                err.to_string().contains("below 1.2"),
                "value {v:?} should be rejected as below the TLS 1.2 floor, got: {err}"
            );
        }
    }

    #[test]
    fn should_reject_unrecognized_version() {
        let err = TlsVersion::parse("tls99", "min_tls_version").unwrap_err();
        assert!(err.to_string().contains("min_tls_version"));
    }

    #[test]
    fn should_default_to_full_window_when_unset() {
        let cfg = TlsConfig::from_settings(&settings(&[])).unwrap();
        assert_eq!(cfg.versions.min, TlsVersion::Tls12);
        assert_eq!(cfg.versions.max, TlsVersion::Tls13);
    }

    #[test]
    fn should_parse_window_from_settings() {
        let cfg = TlsConfig::from_settings(&settings(&[
            ("min_tls_version", "tls13"),
            ("max_tls_version", "tls13"),
        ]))
        .unwrap();
        assert_eq!(cfg.versions.min, TlsVersion::Tls13);
        assert_eq!(cfg.versions.max, TlsVersion::Tls13);
    }

    #[test]
    fn should_reject_window_when_min_exceeds_max() {
        let err = TlsConfig::from_settings(&settings(&[
            ("min_tls_version", "tls13"),
            ("max_tls_version", "tls12"),
        ]))
        .unwrap_err();
        assert!(
            err.to_string().contains("at least min_tls_version"),
            "min>max should be rejected, got: {err}"
        );
    }

    #[test]
    fn enabled_rustls_versions_reflects_window() {
        let full = TlsVersions::default();
        assert_eq!(full.enabled_rustls_versions().len(), 2);

        let tls13_only = TlsVersions {
            min: TlsVersion::Tls13,
            max: TlsVersion::Tls13,
        };
        let versions = tls13_only.enabled_rustls_versions();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, rustls::version::TLS13.version);
    }
}
