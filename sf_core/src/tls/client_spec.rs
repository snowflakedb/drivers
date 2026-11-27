use std::ffi::OsString;
use std::time::Duration;

use reqwest::{ClientBuilder, NoProxy, Proxy};
use snafu::ResultExt;

use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
use crate::sensitive::SensitiveString;
use crate::tls::cipher_policy::CipherSetting;
use crate::tls::client::{RootCertificates, build_proxy_url, load_root_certificates};
use crate::tls::config::{ProxyConfig, TlsConfig, TlsVersions};
use crate::tls::crypto_module::KeyShareOrder;
use crate::tls::error::{ProxyBuildSnafu, RedactedUrl, TlsError};

/// The settings a client build reads per build, resolved by
/// the caller, so two equal specs on one driver build interchangeable clients.
///
/// The native root store and the CRL cache settings are read once per driver
/// and stay outside the spec. In [`ProxySpec::Env`] mode reqwest reads the
/// proxy variables while building, which [`ProxyEnv`] snapshots, and on macOS
/// and Windows the system proxy settings, which the spec does not capture.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ClientSpec {
    pub(crate) tls: TlsSpec,
    pub(crate) proxy: ProxySpec,
    pub(crate) connect_timeout: Option<Duration>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct TlsSpec {
    pub(crate) verify_certificates: bool,
    pub(crate) verify_hostname: bool,
    pub(crate) versions: TlsVersions,
    pub(crate) ciphers: CipherSetting,
    /// [`RootCertificates::Default`] when certificate verification is off: the
    /// insecure client never reads the root files.
    pub(crate) roots: RootCertificates,
    pub(crate) crl: CrlSpec,
    pub(crate) key_share_order: KeyShareOrder,
}

/// The CRL settings a client build reads. The driver opens its CRL cache from
/// the first config it sees, so only the settings the verifier reads per client
/// tell two specs apart.
#[derive(Clone)]
pub(crate) struct CrlSpec(CrlConfig);

impl CrlSpec {
    pub(crate) fn config(&self) -> &CrlConfig {
        &self.0
    }

    fn per_client(&self) -> (&CertRevocationCheckMode, bool) {
        let CrlConfig {
            check_mode,
            allow_certificates_without_crl_url,
            enable_disk_caching: _,
            enable_memory_caching: _,
            cache_dir: _,
            max_download_size: _,
            validity_time: _,
            on_disk_cache_removal_delay: _,
            cache_cleanup_interval: _,
            cache_start_cleanup: _,
            unsafe_skip_file_permissions_check: _,
            http_timeout: _,
            connection_timeout: _,
        } = &self.0;
        (check_mode, *allow_certificates_without_crl_url)
    }
}

impl PartialEq for CrlSpec {
    fn eq(&self, other: &Self) -> bool {
        self.per_client() == other.per_client()
    }
}

impl Eq for CrlSpec {}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum ProxySpec {
    /// All schemes go through `url`, which carries the credentials. reqwest's
    /// environment detection is off.
    Explicit {
        url: SensitiveString,
        no_proxy: Option<String>,
    },
    /// reqwest detects the proxy from the environment and, on macOS and
    /// Windows, the system settings. The snapshot only tells specs apart.
    Env(ProxyEnv),
    Disabled,
}

/// The proxy variables reqwest reads today, in its own order, plus
/// `REQUEST_METHOD`, which makes it ignore `HTTP_PROXY` (CGI). These are
/// reqwest's names, not driver settings, so they have no `env_vars` constants.
const PROXY_ENV_VARS: [&str; 9] = [
    "ALL_PROXY",
    "all_proxy",
    "HTTP_PROXY",
    "http_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "NO_PROXY",
    "no_proxy",
    "REQUEST_METHOD",
];

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ProxyEnv([Option<OsString>; PROXY_ENV_VARS.len()]);

impl ProxyEnv {
    fn capture(env: &dyn Fn(&str) -> Option<OsString>) -> Self {
        Self(PROXY_ENV_VARS.map(env))
    }
}

impl ClientSpec {
    /// Reads `SNOWFLAKE_TLS_CIPHERS`, the proxy variables when the proxy is left
    /// to the environment, and, when certificate verification is on, the root
    /// PEM file named in `tls`.
    pub(crate) fn resolve(
        tls: &TlsConfig,
        proxy: Option<&ProxyConfig>,
        connect_timeout: Option<Duration>,
    ) -> Result<Self, TlsError> {
        Ok(Self {
            tls: TlsSpec::resolve(tls, CipherSetting::from_env()?)?,
            proxy: ProxySpec::resolve(proxy, &|name| std::env::var_os(name)),
            connect_timeout,
        })
    }
}

impl TlsSpec {
    pub(crate) fn resolve(tls: &TlsConfig, ciphers: CipherSetting) -> Result<Self, TlsError> {
        let TlsConfig {
            crl_config,
            custom_root_store_path: _,
            extra_root_store_path: _,
            verify_hostname,
            verify_certificates,
            versions,
            prefer_post_quantum,
        } = tls;
        let roots = if *verify_certificates {
            load_root_certificates(tls)?
        } else {
            RootCertificates::Default
        };
        Ok(Self {
            verify_certificates: *verify_certificates,
            verify_hostname: *verify_hostname,
            versions: *versions,
            ciphers,
            roots,
            crl: CrlSpec(crl_config.clone()),
            key_share_order: KeyShareOrder::from_prefer_post_quantum(*prefer_post_quantum),
        })
    }
}

impl ProxySpec {
    /// `None` keeps reqwest's environment detection, like a builder no proxy
    /// was ever applied to.
    pub(crate) fn resolve(
        proxy: Option<&ProxyConfig>,
        env: &dyn Fn(&str) -> Option<OsString>,
    ) -> Self {
        let Some(config) = proxy else {
            return Self::Env(ProxyEnv::capture(env));
        };
        let ProxyConfig {
            host,
            port: _,
            scheme,
            user: _,
            password: _,
            no_proxy,
            use_proxy_env,
            allow_empty_proxy: _,
            explicitly_disabled,
        } = config;
        tracing::debug!(
            host = ?host,
            scheme = scheme.as_str(),
            use_proxy_env = use_proxy_env,
            explicitly_disabled = explicitly_disabled,
            "proxy config"
        );
        if let Some(host) = host.as_deref().filter(|s| !s.is_empty()) {
            return Self::Explicit {
                url: SensitiveString::from(build_proxy_url(host, config)),
                no_proxy: no_proxy.clone(),
            };
        }
        if *use_proxy_env && !*explicitly_disabled {
            tracing::warn!("Proxy configuration: using environment variable proxy settings");
            Self::Env(ProxyEnv::capture(env))
        } else {
            Self::Disabled
        }
    }

    pub(crate) fn apply(&self, builder: ClientBuilder) -> Result<ClientBuilder, TlsError> {
        match self {
            Self::Explicit { url, no_proxy } => {
                let url = url.reveal();
                let proxy = Proxy::all(url)
                    .context(ProxyBuildSnafu {
                        redacted_url: RedactedUrl::new(url),
                    })?
                    .no_proxy(no_proxy.as_deref().and_then(NoProxy::from_string));
                Ok(builder.proxy(proxy))
            }
            Self::Env(_) => Ok(builder),
            Self::Disabled => Ok(builder.no_proxy()),
        }
    }

    pub(crate) fn mode(&self) -> &'static str {
        match self {
            Self::Explicit { .. } => "explicit",
            Self::Env(_) => "env",
            Self::Disabled => "disabled",
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::tls::config::{ProxyScheme, TlsVersion};
    use std::io::Write;
    use std::path::PathBuf;

    fn no_env(_: &str) -> Option<OsString> {
        None
    }

    pub(crate) fn spec(tls: &TlsConfig, proxy: Option<&ProxyConfig>) -> ClientSpec {
        ClientSpec {
            tls: TlsSpec::resolve(tls, CipherSetting::default()).expect("spec resolves"),
            proxy: ProxySpec::resolve(proxy, &no_env),
            connect_timeout: None,
        }
    }

    fn pem_file(contents: &[u8]) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        file.write_all(contents).expect("write pem");
        file
    }

    fn explicit_proxy() -> ProxyConfig {
        ProxyConfig {
            host: Some("proxy.example.com".to_string()),
            port: Some(8080),
            user: Some("alice".to_string()),
            password: Some(SensitiveString::from("pw")),
            ..Default::default()
        }
    }

    #[test]
    fn every_tls_config_field_reaches_the_spec() {
        let base = TlsConfig::default();
        let custom = pem_file(b"custom");
        let extra = pem_file(b"extra");
        let cases: Vec<(&str, TlsConfig)> = vec![
            (
                "crl_config",
                TlsConfig {
                    crl_config: CrlConfig {
                        check_mode: CertRevocationCheckMode::Enabled,
                        ..Default::default()
                    },
                    ..base.clone()
                },
            ),
            (
                "crl_config.allow_certificates_without_crl_url",
                TlsConfig {
                    crl_config: CrlConfig {
                        allow_certificates_without_crl_url: !base
                            .crl_config
                            .allow_certificates_without_crl_url,
                        ..Default::default()
                    },
                    ..base.clone()
                },
            ),
            (
                "custom_root_store_path",
                TlsConfig {
                    custom_root_store_path: Some(custom.path().to_path_buf()),
                    ..base.clone()
                },
            ),
            (
                "extra_root_store_path",
                TlsConfig {
                    extra_root_store_path: Some(extra.path().to_path_buf()),
                    ..base.clone()
                },
            ),
            (
                "verify_hostname",
                TlsConfig {
                    verify_hostname: !base.verify_hostname,
                    ..base.clone()
                },
            ),
            (
                "verify_certificates",
                TlsConfig {
                    verify_certificates: !base.verify_certificates,
                    ..base.clone()
                },
            ),
            (
                "versions",
                TlsConfig {
                    versions: TlsVersions {
                        min: TlsVersion::Tls13,
                        max: TlsVersion::Tls13,
                    },
                    ..base.clone()
                },
            ),
            (
                "prefer_post_quantum",
                TlsConfig {
                    prefer_post_quantum: !base.prefer_post_quantum,
                    ..base.clone()
                },
            ),
        ];
        let base_spec = spec(&base, None);
        for (field, config) in cases {
            assert!(
                spec(&config, None) != base_spec,
                "{field} is not in the spec"
            );
            assert!(
                spec(&config, None) == spec(&config, None),
                "{field} resolves differently twice"
            );
        }
    }

    #[test]
    fn crl_cache_settings_do_not_tell_specs_apart() {
        let base = TlsConfig {
            crl_config: CrlConfig {
                check_mode: CertRevocationCheckMode::Enabled,
                ..Default::default()
            },
            ..Default::default()
        };
        let cache_settings = TlsConfig {
            crl_config: CrlConfig {
                enable_disk_caching: !base.crl_config.enable_disk_caching,
                cache_dir: Some(PathBuf::from("/tmp/other-crl-cache")),
                max_download_size: base.crl_config.max_download_size + 1,
                http_timeout: base.crl_config.http_timeout + chrono::Duration::seconds(1),
                ..base.crl_config.clone()
            },
            ..base.clone()
        };
        assert!(spec(&base, None) == spec(&cache_settings, None));
    }

    #[test]
    fn root_file_contents_reach_the_spec() {
        let file = pem_file(b"first");
        let config = TlsConfig {
            custom_root_store_path: Some(file.path().to_path_buf()),
            ..Default::default()
        };
        let before = spec(&config, None);
        std::fs::write(file.path(), b"second").expect("rewrite pem");
        assert!(spec(&config, None) != before);
        assert!(matches!(
            spec(&config, None).tls.roots,
            RootCertificates::Custom(ref bytes) if bytes == b"second"
        ));
    }

    #[test]
    fn root_files_are_read_only_when_verifying_certificates() {
        let verified = TlsConfig {
            custom_root_store_path: Some(PathBuf::from("/nonexistent/roots.pem")),
            ..Default::default()
        };
        assert!(matches!(
            TlsSpec::resolve(&verified, CipherSetting::default()),
            Err(TlsError::PemParse { .. })
        ));

        let insecure = TlsConfig {
            verify_certificates: false,
            ..verified
        };
        assert!(matches!(
            spec(&insecure, None).tls.roots,
            RootCertificates::Default
        ));
    }

    #[test]
    fn every_proxy_config_field_reaches_the_spec() {
        let base = explicit_proxy();
        let cases: Vec<(&str, ProxyConfig)> = vec![
            (
                "host",
                ProxyConfig {
                    host: Some("other.example.com".to_string()),
                    ..base.clone()
                },
            ),
            (
                "port",
                ProxyConfig {
                    port: Some(3128),
                    ..base.clone()
                },
            ),
            (
                "scheme",
                ProxyConfig {
                    scheme: ProxyScheme::Https,
                    ..base.clone()
                },
            ),
            (
                "user",
                ProxyConfig {
                    user: Some("bob".to_string()),
                    ..base.clone()
                },
            ),
            (
                "password",
                ProxyConfig {
                    password: Some(SensitiveString::from("other")),
                    ..base.clone()
                },
            ),
            (
                "no_proxy",
                ProxyConfig {
                    no_proxy: Some(".internal".to_string()),
                    ..base.clone()
                },
            ),
        ];
        let base_spec = ProxySpec::resolve(Some(&base), &no_env);
        for (field, config) in cases {
            assert!(
                ProxySpec::resolve(Some(&config), &no_env) != base_spec,
                "{field} is not in the spec"
            );
        }
    }

    #[test]
    fn proxy_mode_follows_the_config() {
        let explicit = ProxySpec::resolve(Some(&explicit_proxy()), &no_env);
        assert_eq!(explicit.mode(), "explicit");

        let empty_host = ProxyConfig {
            host: Some(String::new()),
            ..Default::default()
        };
        assert_eq!(
            ProxySpec::resolve(Some(&empty_host), &no_env).mode(),
            "disabled"
        );

        let env = ProxyConfig {
            use_proxy_env: true,
            ..Default::default()
        };
        assert_eq!(ProxySpec::resolve(Some(&env), &no_env).mode(), "env");

        let disabled = ProxyConfig {
            use_proxy_env: true,
            explicitly_disabled: true,
            ..Default::default()
        };
        assert_eq!(
            ProxySpec::resolve(Some(&disabled), &no_env).mode(),
            "disabled"
        );
        assert_eq!(ProxySpec::resolve(None, &no_env).mode(), "env");
    }

    #[test]
    fn proxy_env_affects_only_env_mode() {
        let env_mode = ProxyConfig {
            use_proxy_env: true,
            ..Default::default()
        };
        let set_https =
            |name: &str| (name == "HTTPS_PROXY").then(|| OsString::from("http://p.example:1"));
        assert!(
            ProxySpec::resolve(Some(&env_mode), &set_https)
                != ProxySpec::resolve(Some(&env_mode), &no_env)
        );
        assert!(
            ProxySpec::resolve(Some(&explicit_proxy()), &set_https)
                == ProxySpec::resolve(Some(&explicit_proxy()), &no_env)
        );
        assert!(
            ProxySpec::resolve(Some(&ProxyConfig::default()), &set_https)
                == ProxySpec::resolve(Some(&ProxyConfig::default()), &no_env)
        );
    }
}
