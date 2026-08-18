//! Suffix-anchored allowlist that restricts Workload Identity attestation to
//! recognized Snowflake hosts before any cloud credential is fetched.
//!
//! [`is_snowflake_host_for_workload_identity`] is a pure, suffix-anchored
//! check that MUST be run (see [`super::create_attestation`]'s caller in
//! [`crate::rest::snowflake::auth_request_data`]) before any ambient
//! credential is fetched or minted. It is intentionally independent of the
//! other host helpers in this crate (account inference, OCSP/privatelink
//! detection, OAuth IdP checks) — those serve different purposes and are
//! tracked separately.
//!
//! The `SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES` environment variable is an additive
//! escape hatch for non-standard deployments (e.g. test doubles). Read only
//! from the process environment - never from the DSN, connection parameters
//! or configuration files - so connection configuration cannot influence the
//! allowlist. Entries are additive: they extend the recognized-host list and
//! cannot disable it.

/// Canonical Snowflake hostname suffixes eligible to receive a WIF
/// attestation. The apex (bare suffix, no subdomain) is accepted.
const ALLOWED_SUFFIXES: &[&str] = &[
    "snowflakecomputing.com",
    "snowflakecomputing.cn",
    "snowflakecomputing.mil",
];

/// Normalizes a host or suffix the way [`is_snowflake_host_for_workload_identity`]
/// does: trims whitespace, lowercases (ASCII), strips a trailing `:port`
/// (everything from the first ':' onward), and then strips exactly one
/// trailing '.' (FQDN form). The port MUST be stripped before the trailing
/// dot: for a host in FQDN form with an explicit port
/// (`acct.snowflakecomputing.com.:443`), stripping the dot first would
/// leave the dot attached to the port-stripped result and the host would
/// fail to match any suffix.
fn normalize(raw: &str) -> String {
    let trimmed = raw.trim();
    let lowered = trimmed.to_ascii_lowercase();
    let port_stripped = match lowered.find(':') {
        Some(idx) => lowered[..idx].to_string(),
        None => lowered,
    };
    port_stripped.strip_suffix('.').map_or_else(
        || port_stripped.clone(),
        |without_dot| without_dot.to_string(),
    )
}

/// Parses the additive `SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES` environment variable
/// into normalized suffixes, ignoring empty entries. Logs at `INFO` naming
/// the extra suffixes when the variable is set and non-empty.
fn extra_allowed_suffixes_from_env() -> Vec<String> {
    let Ok(raw) = std::env::var(crate::env_vars::SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES) else {
        return Vec::new();
    };
    let suffixes: Vec<String> = raw
        .split(',')
        .map(normalize)
        .filter(|s| !s.is_empty())
        .collect();
    if !suffixes.is_empty() {
        tracing::info!(
            extra_suffixes = %suffixes.join(","),
            "Workload Identity host allowlist extended via {}",
            crate::env_vars::SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES
        );
    }
    suffixes
}

/// Returns `true` if `host` is a Snowflake host eligible to receive a
/// Workload Identity Federation attestation.
///
/// Matching is anchored to a label boundary at the end of the host, so
/// only the listed suffixes and their subdomains are recognized.
///
/// IP literals are rejected implicitly: they never match any allowed
/// suffix, so no special-casing is needed.
pub fn is_snowflake_host_for_workload_identity(host: &str) -> bool {
    let normalized = normalize(host);
    if normalized.is_empty() {
        return false;
    }

    let extra = extra_allowed_suffixes_from_env();
    ALLOWED_SUFFIXES
        .iter()
        .map(|s| s.to_string())
        .chain(extra)
        .any(|suffix| normalized == suffix || normalized.ends_with(&format!(".{suffix}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serializes tests that touch SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES: std::env is
    // process-global and `cargo test` runs tests in parallel by default.
    static ENV_GUARD: Mutex<()> = Mutex::new(());

    fn with_env_var<T>(value: Option<&str>, f: impl FnOnce() -> T) -> T {
        let _guard = ENV_GUARD.lock().unwrap();
        match value {
            Some(v) => unsafe {
                std::env::set_var(crate::env_vars::SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES, v)
            },
            None => unsafe {
                std::env::remove_var(crate::env_vars::SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES)
            },
        }
        let result = f();
        unsafe { std::env::remove_var(crate::env_vars::SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES) };
        result
    }

    #[test]
    fn accepts_standard_snowflake_hosts() {
        with_env_var(None, || {
            for host in [
                "myorg-acct.snowflakecomputing.com",
                "myorg-acct.privatelink.snowflakecomputing.com",
                "acct.us-east-1.snowflakecomputing.com",
                "acct.snowflakecomputing.cn",
                "acct.snowflakecomputing.mil",
                "acct.some-region.privatelink.snowflakecomputing.mil",
                "snowflakecomputing.com",
                "ACCT.SnowflakeComputing.COM",
                "acct.snowflakecomputing.com.",
                // Vector 23: FQDN form (trailing dot) with an explicit port.
                // Port must be stripped before the trailing dot, else the
                // dot stays attached and the host fails to match.
                "acct.snowflakecomputing.com.:443",
            ] {
                assert!(
                    is_snowflake_host_for_workload_identity(host),
                    "expected ACCEPT for {host:?}"
                );
            }
        });
    }

    #[test]
    fn rejects_non_snowflake_hosts() {
        with_env_var(None, || {
            for host in [
                "evilsnowflakecomputing.com",
                "acct.snowflakecomputing.com.not-snowflake.example",
                "evil.snowflakecomputing.not-snowflake.example",
                "acct.snowflakecomputing.zip",
                "not-snowflake.example",
                "snowflakecomputing.com.evil.io",
                "",
                "127.0.0.1",
                "xsnowflakecomputing.mil",
                "acct.snowflakecomputing.co",
            ] {
                assert!(
                    !is_snowflake_host_for_workload_identity(host),
                    "expected REJECT for {host:?}"
                );
            }
        });
    }

    #[test]
    fn env_hatch_is_additive_and_scoped() {
        // Unset: wiremock.local rejected.
        with_env_var(None, || {
            assert!(!is_snowflake_host_for_workload_identity("wiremock.local"));
        });

        // Set: wiremock.local accepted...
        with_env_var(Some("wiremock.local"), || {
            assert!(is_snowflake_host_for_workload_identity("wiremock.local"));
            // ...but the hatch never disables the check for unrelated hosts.
            assert!(!is_snowflake_host_for_workload_identity(
                "not-snowflake.example"
            ));
        });
    }

    #[test]
    fn env_hatch_entries_are_normalized_and_empty_entries_ignored() {
        with_env_var(Some(" Wiremock.Local. , ,,example.internal."), || {
            assert!(is_snowflake_host_for_workload_identity("wiremock.local"));
            assert!(is_snowflake_host_for_workload_identity(
                "sub.example.internal"
            ));
        });
    }

    #[test]
    fn strips_port_defensively() {
        with_env_var(None, || {
            assert!(is_snowflake_host_for_workload_identity(
                "acct.snowflakecomputing.com:443"
            ));
            assert!(!is_snowflake_host_for_workload_identity(
                "not-snowflake.example:443"
            ));
        });
    }
}
