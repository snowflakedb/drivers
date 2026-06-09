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
        let allow_empty_proxy = settings
            .get_bool("allow_empty_proxy")
            .or_else(|| coerce_bool(settings.get_string("allow_empty_proxy").as_deref()))
            .unwrap_or(true);
        let use_proxy_env = settings
            .get_bool("use_proxy_env")
            .or_else(|| coerce_bool(settings.get_string("use_proxy_env").as_deref()))
            .unwrap_or(false);
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

/// Coerce a string like `"true"`/`"1"`/`"false"`/`"0"` to bool, matching
/// the `ParamStore::get_bool` rules. Returns `None` for unrecognised values
/// so callers can apply their own default.
fn coerce_bool(s: Option<&str>) -> Option<bool> {
    match s?.to_ascii_lowercase().as_str() {
        "true" | "1" | "on" => Some(true),
        "false" | "0" | "off" => Some(false),
        _ => None,
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
