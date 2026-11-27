//! Process-wide cipher allow-list from [`SNOWFLAKE_TLS_CIPHERS`](crate::env_vars::SNOWFLAKE_TLS_CIPHERS).
//!
//! Names that start with `TLS_` replace the TLS 1.3 suites. Every other name
//! replaces the TLS 1.2 suites. A version that does not appear in the list
//! keeps the suites the linked provider already offers for that version.
//! The shared [`CryptoModule`](super::crypto_module::CryptoModule) provider is
//! left unchanged; a client build clones it only when the variable is set.

use std::sync::Arc;

use crate::env_vars::SNOWFLAKE_TLS_CIPHERS;
use crate::tls::crypto_module::KeyShareOrder;
use crate::tls::error::{CipherPolicySnafu, TlsError};
use rustls::CipherSuite;
use rustls::SupportedCipherSuite;
use rustls::crypto::CryptoProvider;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CipherPolicy {
    tls12: Option<Vec<String>>,
    tls13: Option<Vec<String>>,
}

/// The raw [`SNOWFLAKE_TLS_CIPHERS`] value one client build applies. `None`
/// keeps the linked provider's suites. Parse errors surface when a provider
/// is built from it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CipherSetting(Option<String>);

impl CipherSetting {
    pub(crate) fn from_env() -> Result<Self, TlsError> {
        #[cfg(test)]
        let _guard = test_env::test_read_lock();
        Self::from_value(std::env::var_os(SNOWFLAKE_TLS_CIPHERS))
    }

    pub(crate) fn from_value(value: Option<std::ffi::OsString>) -> Result<Self, TlsError> {
        match value.map(std::ffi::OsString::into_string) {
            None => Ok(Self(None)),
            Some(Ok(raw)) => Ok(Self(Some(raw))),
            Some(Err(_)) => CipherPolicySnafu {
                message: format!("{SNOWFLAKE_TLS_CIPHERS} is not valid Unicode"),
            }
            .fail(),
        }
    }
}

pub(crate) fn provider_for_versions(
    versions: &[&'static rustls::SupportedProtocolVersion],
    order: KeyShareOrder,
    ciphers: &CipherSetting,
) -> Result<Arc<CryptoProvider>, TlsError> {
    let base = super::crypto_module::CryptoModule::get().provider_for(order);
    let Some(policy) = ciphers
        .0
        .as_deref()
        .map(parse_policy)
        .transpose()?
        .flatten()
    else {
        return Ok(base);
    };
    tracing::debug!(
        tls13 = ?policy.tls13,
        tls12 = ?policy.tls12,
        "applying {SNOWFLAKE_TLS_CIPHERS}"
    );
    let mut owned = (*base).clone();
    owned.cipher_suites = restrict(&base.cipher_suites, &policy, versions)?;
    Ok(Arc::new(owned))
}

fn parse_policy(raw: &str) -> Result<Option<CipherPolicy>, TlsError> {
    if raw.trim().is_empty() {
        return Ok(None);
    }
    let names: Vec<String> = raw
        .split(':')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect();
    if names.is_empty() {
        return CipherPolicySnafu {
            message: format!(
                "{SNOWFLAKE_TLS_CIPHERS} contains no cipher names. \
                 Expected a colon-separated list, for example \
                 'TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384'."
            ),
        }
        .fail();
    }
    let mut tls13 = Vec::new();
    let mut tls12 = Vec::new();
    for name in names {
        if name.starts_with("TLS_") {
            tls13.push(name);
        } else {
            tls12.push(name);
        }
    }
    Ok(Some(CipherPolicy {
        tls12: nonempty(tls12),
        tls13: nonempty(tls13),
    }))
}

fn nonempty(names: Vec<String>) -> Option<Vec<String>> {
    if names.is_empty() { None } else { Some(names) }
}

fn restrict(
    offered: &[SupportedCipherSuite],
    policy: &CipherPolicy,
    versions: &[&'static rustls::SupportedProtocolVersion],
) -> Result<Vec<SupportedCipherSuite>, TlsError> {
    let mut selected = suites_for(offered, &rustls::version::TLS13, policy.tls13.as_deref())?;
    selected.extend(suites_for(
        offered,
        &rustls::version::TLS12,
        policy.tls12.as_deref(),
    )?);
    if versions.is_empty() {
        return Ok(selected);
    }
    let enabled: Vec<_> = selected
        .into_iter()
        .filter(|suite| {
            versions
                .iter()
                .any(|version| suite.version().version == version.version)
        })
        .collect();
    for version in versions {
        if !enabled
            .iter()
            .any(|suite| suite.version().version == version.version)
        {
            return CipherPolicySnafu {
                message: format!(
                    "{SNOWFLAKE_TLS_CIPHERS} leaves {} with no cipher suite",
                    protocol_label(version)
                ),
            }
            .fail();
        }
    }
    Ok(enabled)
}

fn suites_for(
    offered: &[SupportedCipherSuite],
    version: &'static rustls::SupportedProtocolVersion,
    names: Option<&[String]>,
) -> Result<Vec<SupportedCipherSuite>, TlsError> {
    let Some(names) = names else {
        return Ok(offered
            .iter()
            .copied()
            .filter(|suite| suite.version().version == version.version)
            .collect());
    };
    let mut selected: Vec<SupportedCipherSuite> = Vec::with_capacity(names.len());
    for name in names {
        let suite_id = known_suite(name).ok_or_else(|| unrecognized(name))?;
        let Some(suite) = offered
            .iter()
            .copied()
            .find(|suite| suite.suite() == suite_id)
        else {
            return Err(not_offered(name));
        };
        if suite.version().version != version.version {
            return Err(unrecognized(name));
        }
        if selected
            .iter()
            .all(|existing| existing.suite() != suite.suite())
        {
            selected.push(suite);
        }
    }
    Ok(selected)
}

fn known_suite(name: &str) -> Option<CipherSuite> {
    Some(match name {
        "TLS_AES_128_GCM_SHA256" => CipherSuite::TLS13_AES_128_GCM_SHA256,
        "TLS_AES_256_GCM_SHA384" => CipherSuite::TLS13_AES_256_GCM_SHA384,
        "TLS_CHACHA20_POLY1305_SHA256" => CipherSuite::TLS13_CHACHA20_POLY1305_SHA256,
        "ECDHE-ECDSA-AES128-GCM-SHA256" => CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
        "ECDHE-ECDSA-AES256-GCM-SHA384" => CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
        "ECDHE-RSA-AES128-GCM-SHA256" => CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
        "ECDHE-RSA-AES256-GCM-SHA384" => CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
        "ECDHE-ECDSA-CHACHA20-POLY1305" => {
            CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256
        }
        "ECDHE-RSA-CHACHA20-POLY1305" => CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
        _ => return None,
    })
}

fn unrecognized(name: &str) -> TlsError {
    CipherPolicySnafu {
        message: format!("{SNOWFLAKE_TLS_CIPHERS} does not recognize '{name}'"),
    }
    .build()
}

fn not_offered(name: &str) -> TlsError {
    CipherPolicySnafu {
        message: format!(
            "{SNOWFLAKE_TLS_CIPHERS} requests '{name}', which this build does not offer"
        ),
    }
    .build()
}

fn protocol_label(version: &rustls::SupportedProtocolVersion) -> &'static str {
    if version.version == rustls::ProtocolVersion::TLSv1_3 {
        "TLS 1.3"
    } else if version.version == rustls::ProtocolVersion::TLSv1_2 {
        "TLS 1.2"
    } else {
        "an enabled TLS version"
    }
}

#[cfg(test)]
mod test_env {
    use std::cell::Cell;
    use std::sync::{Mutex, MutexGuard};

    use crate::utils::sync::MutexRecoverExt;

    use super::SNOWFLAKE_TLS_CIPHERS;

    static ENV_LOCK: Mutex<()> = Mutex::new(());
    thread_local! {
        static ENV_LOCK_HELD: Cell<bool> = const { Cell::new(false) };
    }

    pub(super) fn test_read_lock() -> Option<MutexGuard<'static, ()>> {
        if ENV_LOCK_HELD.with(Cell::get) {
            None
        } else {
            Some(ENV_LOCK.lock_recover())
        }
    }

    pub(crate) struct TlsCipherEnvGuard {
        _lock: MutexGuard<'static, ()>,
        previous: Option<std::ffi::OsString>,
    }

    impl TlsCipherEnvGuard {
        fn acquire(value: Option<&str>) -> Self {
            let lock = ENV_LOCK.lock_recover();
            ENV_LOCK_HELD.with(|held| held.set(true));
            let previous = std::env::var_os(SNOWFLAKE_TLS_CIPHERS);
            match value {
                // SAFETY: `ENV_LOCK` is held, and this variable is read only
                // through `CipherSetting::from_env` while the guard is alive.
                Some(value) => unsafe { std::env::set_var(SNOWFLAKE_TLS_CIPHERS, value) },
                None => unsafe { std::env::remove_var(SNOWFLAKE_TLS_CIPHERS) },
            }
            Self {
                _lock: lock,
                previous,
            }
        }
    }

    impl Drop for TlsCipherEnvGuard {
        fn drop(&mut self) {
            // SAFETY: `ENV_LOCK` is still held; field drop releases it after this.
            match self.previous.take() {
                Some(value) => unsafe { std::env::set_var(SNOWFLAKE_TLS_CIPHERS, value) },
                None => unsafe { std::env::remove_var(SNOWFLAKE_TLS_CIPHERS) },
            }
            ENV_LOCK_HELD.with(|held| held.set(false));
        }
    }

    pub(crate) fn with_tls_ciphers_env<T>(value: Option<&str>, body: impl FnOnce() -> T) -> T {
        let _guard = TlsCipherEnvGuard::acquire(value);
        body()
    }
}

#[cfg(test)]
pub(crate) use test_env::with_tls_ciphers_env;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::tls::crypto_module::KeyShareOrder;
    use rustls::CipherSuite;
    use rustls::crypto::CryptoProvider;

    use super::{CipherSetting, parse_policy, provider_for_versions, restrict};
    use crate::tls::crypto_module::CryptoModule;
    use crate::tls::error::TlsError;

    fn protocol_window(tls12: bool, tls13: bool) -> Vec<&'static rustls::SupportedProtocolVersion> {
        let mut out = Vec::new();
        if tls12 {
            out.push(&rustls::version::TLS12);
        }
        if tls13 {
            out.push(&rustls::version::TLS13);
        }
        out
    }

    fn suite_ids(provider: &CryptoProvider) -> Vec<CipherSuite> {
        provider
            .cipher_suites
            .iter()
            .map(|suite| suite.suite())
            .collect()
    }

    fn message(err: TlsError) -> String {
        err.to_string()
    }

    #[test]
    fn blank_value_leaves_the_provider_unchanged() {
        assert_eq!(parse_policy("   ").expect("blank"), None);
        assert_eq!(parse_policy("").expect("empty"), None);
    }

    #[test]
    fn a_value_with_no_names_fails() {
        let err = parse_policy(": :").expect_err("no names");
        assert!(
            message(err).contains("contains no cipher names"),
            "error should name the empty list"
        );
    }

    #[test]
    fn unrecognized_and_unsupported_names_fail() {
        let unrecognized = parse_policy("DHE-RSA-AES256-GCM-SHA384")
            .expect("parses")
            .expect("policy");
        let err = restrict(
            &CryptoModule::get().provider().cipher_suites,
            &unrecognized,
            &protocol_window(true, true),
        )
        .expect_err("DHE is not offered");
        let text = message(err);
        assert!(text.contains("does not recognize 'DHE-RSA-AES256-GCM-SHA384'"));
        assert!(text.contains(crate::env_vars::SNOWFLAKE_TLS_CIPHERS));

        let iana_tls12 = parse_policy("TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256")
            .expect("parses")
            .expect("policy");
        let err = restrict(
            &CryptoModule::get().provider().cipher_suites,
            &iana_tls12,
            &protocol_window(true, true),
        )
        .expect_err("IANA TLS 1.2 names are not TLS 1.3 suites");
        assert!(message(err).contains("does not recognize"));
    }

    #[test]
    fn a_suite_missing_from_this_build_fails() {
        let policy = parse_policy("TLS_CHACHA20_POLY1305_SHA256")
            .expect("parses")
            .expect("policy");
        let mut base = (*CryptoModule::get().provider()).clone();
        base.cipher_suites
            .retain(|suite| suite.suite() != CipherSuite::TLS13_CHACHA20_POLY1305_SHA256);
        let err = restrict(&base.cipher_suites, &policy, &protocol_window(false, true))
            .expect_err("ChaCha is absent from the provider");
        assert!(message(err).contains("does not offer"));
    }

    #[test]
    fn tls13_names_leave_tls12_defaults_in_place() {
        let policy = parse_policy(" TLS_AES_128_GCM_SHA256 : TLS_AES_256_GCM_SHA384 ")
            .expect("parses")
            .expect("policy");
        let offered = CryptoModule::get().provider();
        let both = restrict(
            &offered.cipher_suites,
            &policy,
            &protocol_window(true, true),
        )
        .expect("both versions");
        assert!(
            both.iter().any(|suite| {
                suite.suite() == CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
            })
        );
        assert!(
            !both
                .iter()
                .any(|suite| suite.suite() == CipherSuite::TLS13_CHACHA20_POLY1305_SHA256)
        );
        let tls13_only = restrict(
            &offered.cipher_suites,
            &policy,
            &protocol_window(false, true),
        )
        .expect("tls 1.3 window");
        assert_eq!(
            tls13_only
                .iter()
                .map(|suite| suite.suite())
                .collect::<Vec<_>>(),
            vec![
                CipherSuite::TLS13_AES_128_GCM_SHA256,
                CipherSuite::TLS13_AES_256_GCM_SHA384,
            ]
        );
    }

    #[test]
    fn duplicate_names_are_offered_once_in_first_seen_order() {
        let policy =
            parse_policy("TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384:TLS_AES_128_GCM_SHA256")
                .expect("parses")
                .expect("policy");
        let selected = restrict(
            &CryptoModule::get().provider().cipher_suites,
            &policy,
            &protocol_window(false, true),
        )
        .expect("duplicates");
        assert_eq!(
            selected
                .iter()
                .map(|suite| suite.suite())
                .collect::<Vec<_>>(),
            vec![
                CipherSuite::TLS13_AES_128_GCM_SHA256,
                CipherSuite::TLS13_AES_256_GCM_SHA384,
            ]
        );
    }

    #[test]
    fn tls12_names_leave_tls13_defaults_in_place() {
        let policy = parse_policy("ECDHE-RSA-AES256-GCM-SHA384")
            .expect("parses")
            .expect("policy");
        let selected = restrict(
            &CryptoModule::get().provider().cipher_suites,
            &policy,
            &protocol_window(true, true),
        )
        .expect("tls 1.2 name");
        assert!(
            selected.iter().any(|suite| {
                suite.suite() == CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384
            })
        );
        assert!(
            !selected.iter().any(|suite| {
                suite.suite() == CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
            })
        );
        assert!(
            selected
                .iter()
                .any(|suite| suite.version().version == rustls::ProtocolVersion::TLSv1_3)
        );
    }

    #[test]
    fn an_enabled_version_with_no_remaining_suite_fails() {
        let policy = parse_policy("TLS_AES_128_GCM_SHA256")
            .expect("parses")
            .expect("policy");
        let mut base = (*CryptoModule::get().provider()).clone();
        base.cipher_suites
            .retain(|suite| suite.version().version == rustls::ProtocolVersion::TLSv1_3);
        let err = restrict(&base.cipher_suites, &policy, &protocol_window(true, true))
            .expect_err("tls 1.2 window has no suite");
        let text = message(err);
        assert!(text.contains("leaves TLS 1.2 with no cipher suite"));
        assert!(text.contains(crate::env_vars::SNOWFLAKE_TLS_CIPHERS));
    }

    #[cfg(not(feature = "fips"))]
    #[test]
    fn unset_provider_still_offers_chacha() {
        let provider = provider_for_versions(
            &protocol_window(true, true),
            KeyShareOrder::ClassicalFirst,
            &CipherSetting(None),
        )
        .expect("default provider");
        assert!(suite_ids(&provider).contains(&CipherSuite::TLS13_CHACHA20_POLY1305_SHA256));
        assert!(Arc::ptr_eq(&provider, &CryptoModule::get().provider()));
    }

    #[test]
    fn the_setting_reaches_the_provider() {
        let setting = CipherSetting(Some("NOT-A-CIPHER".to_string()));
        let err = provider_for_versions(
            &protocol_window(false, true),
            KeyShareOrder::ClassicalFirst,
            &setting,
        )
        .expect_err("unknown name");
        assert!(message(err).contains("does not recognize 'NOT-A-CIPHER'"));
    }

    #[test]
    fn from_env_reads_the_environment_variable() {
        super::with_tls_ciphers_env(Some("TLS_AES_128_GCM_SHA256"), || {
            assert!(
                CipherSetting::from_env().expect("unicode")
                    == CipherSetting(Some("TLS_AES_128_GCM_SHA256".to_string()))
            );
        });
        super::with_tls_ciphers_env(None, || {
            assert!(CipherSetting::from_env().expect("unset") == CipherSetting(None));
        });
    }

    #[cfg(unix)]
    #[test]
    fn non_unicode_value_is_rejected() {
        use std::os::unix::ffi::OsStringExt;
        let err = CipherSetting::from_value(Some(std::ffi::OsString::from_vec(vec![0xff])))
            .expect_err("not unicode");
        assert!(message(err).contains("is not valid Unicode"));
    }
}
