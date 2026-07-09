use super::param_registry::{
    DEFAULT_PUT_GET_MAX_ATTEMPTS, DEFAULT_RETRY_BACKOFF_BASE_MS, DEFAULT_RETRY_BACKOFF_CAP_MS,
    DEFAULT_RETRY_BACKOFF_FACTOR, DEFAULT_RETRY_MAX_ATTEMPTS, param_names,
};
use super::param_store::ParamStore;
use std::collections::BTreeSet;
use std::str::FromStr;
use std::time::Duration;

/// Global retry policy used by the driver. Keep it minimal at the HTTP layer;
/// layers above (Snowflake query logic, etc.) can compose their own semantics.
/// Cloning is cheap because the structure only stores durations, numbers, and
/// booleans, allowing call sites to snapshot per-request settings easily.
#[derive(Clone, Debug)]
pub struct RetryPolicy {
    /// Verb-aware HTTP retry gates.
    pub http: HttpPolicy,
    /// Maximum number of attempts for a request.
    pub max_attempts: u32,
    /// Configuration for exponential backoff between attempts.
    pub backoff: BackoffConfig,
    /// Maximum total duration spent on the operation before we stop retrying.
    pub max_elapsed: Duration,
    /// Optional per-request socket timeout. If Some, each HTTP request gets
    /// timeout = min(this, remaining_budget). If None, each request gets the
    /// remaining budget as its timeout (so max_elapsed is still enforced).
    pub per_request_timeout: Option<Duration>,
    /// Additional HTTP status codes to treat as retryable beyond the built-in set
    /// (408, 429, 307, 308, and 5xx). Cloud-specific policies extend this set
    /// rather than replacing it.
    pub extra_retryable_statuses: BTreeSet<u16>,
}

#[derive(Clone, Debug)]
pub struct BackoffConfig {
    pub base: Duration,
    pub factor: f64,
    pub cap: Duration,
    pub jitter: Jitter,
}

#[derive(Clone, Debug)]
pub enum Jitter {
    None,
    Full,
    Decorrelated,
}

impl FromStr for Jitter {
    type Err = ();

    /// Parse a jitter strategy name case-insensitively. Unknown values return
    /// `Err(())` so callers fall back to the default rather than guessing.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "none" => Ok(Jitter::None),
            "full" => Ok(Jitter::Full),
            "decorrelated" => Ok(Jitter::Decorrelated),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct HttpPolicy {
    /// Enable retries for GET/HEAD/OPTIONS
    pub retry_safe_reads: bool,
    /// Enable retries for PUT/DELETE (idempotent operations)
    pub retry_idempotent_writes: bool,
    /// Enable retries for POST/PATCH (generally off).
    pub retry_post_patch: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            http: HttpPolicy {
                retry_safe_reads: true,
                retry_idempotent_writes: true,
                retry_post_patch: false,
            },
            max_attempts: 6,
            backoff: default_backoff(),
            max_elapsed: Duration::from_secs(120),
            per_request_timeout: None,
            extra_retryable_statuses: BTreeSet::new(),
        }
    }
}

/// The common exponential-backoff curve shared by every retry pipeline.
///
/// Values come from the `DEFAULT_RETRY_BACKOFF_*` constants, which are also
/// the registered defaults for the `retry_backoff_*` connection parameters, so
/// there is a single source of truth for the shape.
fn default_backoff() -> BackoffConfig {
    BackoffConfig {
        base: Duration::from_millis(DEFAULT_RETRY_BACKOFF_BASE_MS),
        factor: DEFAULT_RETRY_BACKOFF_FACTOR,
        cap: Duration::from_millis(DEFAULT_RETRY_BACKOFF_CAP_MS),
        jitter: Jitter::Decorrelated,
    }
}

/// Upper bound (5 minutes) for the configurable `base`/`cap` backoff durations.
///
/// A value above this is treated as misconfiguration and ignored in favour of
/// the default. Without it a fat-fingered parameter (e.g. seconds typed as
/// milliseconds) could silently disable retries — the first computed delay
/// would exceed the total budget and fail fast — or, on the AWS SDK PUT/GET
/// path where `base`/`cap` are handed to the SDK directly, cause an extreme
/// sleep.
const MAX_BACKOFF_MS: i64 = 300_000;

/// Build the backoff curve from connection parameters, falling back to
/// [`default_backoff`] for any field that is absent or out of range.
///
/// The four `retry_backoff_*` parameters form a single shared set: whatever is
/// configured applies to both the HTTP and PUT/GET pipelines.
fn backoff_from_params(params: &ParamStore) -> BackoffConfig {
    let defaults = default_backoff();
    BackoffConfig {
        base: params
            .get_int(param_names::RETRY_BACKOFF_BASE_MS)
            .filter(|v| (0..=MAX_BACKOFF_MS).contains(v))
            .map(|v| Duration::from_millis(v as u64))
            .unwrap_or(defaults.base),
        factor: params
            .get_double(param_names::RETRY_BACKOFF_FACTOR)
            .filter(|v| *v > 0.0)
            .unwrap_or(defaults.factor),
        cap: params
            .get_int(param_names::RETRY_BACKOFF_CAP_MS)
            .filter(|v| (0..=MAX_BACKOFF_MS).contains(v))
            .map(|v| Duration::from_millis(v as u64))
            .unwrap_or(defaults.cap),
        jitter: params
            .get_string(param_names::RETRY_BACKOFF_JITTER)
            .and_then(|s| s.parse().ok())
            .unwrap_or(defaults.jitter),
    }
}

impl RetryPolicy {
    /// Builds the retry policy for general HTTP calls (login, query, logout, etc.).
    ///
    /// Reads `retry_max_attempts` from the [`ParamStore`] (falling back to
    /// [`DEFAULT_RETRY_MAX_ATTEMPTS`]), the shared `retry_backoff_*` curve via
    /// [`backoff_from_params`], and `retry_extra_status_codes` for any
    /// user-configured extra retryable statuses; remaining fields stay at their
    /// defaults.
    pub fn http(params: &ParamStore) -> Self {
        let max_attempts = params
            .get_int(param_names::RETRY_MAX_ATTEMPTS)
            .filter(|v| *v > 0 && *v <= u32::MAX as i64)
            .map(|v| v as u32)
            .unwrap_or(DEFAULT_RETRY_MAX_ATTEMPTS);

        Self {
            max_attempts,
            backoff: backoff_from_params(params),
            extra_retryable_statuses: parse_extra_statuses(params),
            ..Self::default()
        }
    }

    /// Builds the base put/get retry policy from connection parameters.
    ///
    /// Reads `put_get_max_attempts` from the [`ParamStore`] (falling back to
    /// [`DEFAULT_PUT_GET_MAX_ATTEMPTS`] for absent or out-of-range values) and
    /// the shared `retry_backoff_*` curve via [`backoff_from_params`], and seeds
    /// user-configured `retry_extra_status_codes` into `extra_retryable_statuses`.
    /// Uses a larger total budget (600s) than general HTTP calls.
    ///
    /// Cloud-specific code clones this base and adds its own tweaks (extra
    /// retryable statuses on top of the user-configured ones, per-request
    /// timeout).
    pub fn put_get(params: &ParamStore) -> Self {
        let max_attempts = params
            .get_int(param_names::PUT_GET_MAX_ATTEMPTS)
            .filter(|v| *v > 0 && *v <= u32::MAX as i64)
            .map(|v| v as u32)
            .unwrap_or(DEFAULT_PUT_GET_MAX_ATTEMPTS);

        Self {
            max_attempts,
            backoff: backoff_from_params(params),
            max_elapsed: Duration::from_secs(600),
            extra_retryable_statuses: parse_extra_statuses(params),
            ..Self::default()
        }
    }
}

/// Parses `retry_extra_status_codes` (comma-separated) into a set of extra
/// retryable HTTP statuses. Blank, non-numeric, and out-of-range (outside
/// 100–599) tokens are skipped with a warning rather than failing the
/// connection. Applied to both the general-HTTP and put/get policies;
/// cloud-specific policies extend the result further (e.g. GCS/Azure add 403).
fn parse_extra_statuses(params: &ParamStore) -> BTreeSet<u16> {
    params
        .get_string(param_names::RETRY_EXTRA_STATUS_CODES)
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .filter_map(|t| match t.parse::<u16>() {
                    Ok(code) if (100..=599).contains(&code) => Some(code),
                    _ => {
                        tracing::warn!(
                            token = t,
                            "ignoring invalid retry_extra_status_codes entry"
                        );
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::Setting;

    fn params(pairs: &[(&str, Setting)]) -> ParamStore {
        let mut p = ParamStore::new();
        for (k, v) in pairs {
            p.insert((*k).to_string(), v.clone());
        }
        p
    }

    #[test]
    fn backoff_from_params_applies_valid_values_and_ignores_invalid() {
        // Valid values are applied; an uppercase jitter name exercises the
        // case-insensitive parse.
        let good = backoff_from_params(&params(&[
            (
                param_names::RETRY_BACKOFF_BASE_MS.as_str(),
                Setting::Int(100),
            ),
            (
                param_names::RETRY_BACKOFF_CAP_MS.as_str(),
                Setting::Int(5000),
            ),
            (
                param_names::RETRY_BACKOFF_FACTOR.as_str(),
                Setting::Double(1.5),
            ),
            (
                param_names::RETRY_BACKOFF_JITTER.as_str(),
                Setting::String("FULL".to_string()),
            ),
        ]));
        assert_eq!(good.base, Duration::from_millis(100));
        assert_eq!(good.cap, Duration::from_millis(5000));
        assert_eq!(good.factor, 1.5);
        assert!(matches!(good.jitter, Jitter::Full));

        // Negative duration, non-positive factor, unknown jitter, and a value
        // past MAX_BACKOFF_MS each fall back to the common default; the bound
        // itself is accepted.
        let bad = backoff_from_params(&params(&[
            (
                param_names::RETRY_BACKOFF_BASE_MS.as_str(),
                Setting::Int(-1),
            ),
            (
                param_names::RETRY_BACKOFF_CAP_MS.as_str(),
                Setting::Int(MAX_BACKOFF_MS + 1),
            ),
            (
                param_names::RETRY_BACKOFF_FACTOR.as_str(),
                Setting::Double(0.0),
            ),
            (
                param_names::RETRY_BACKOFF_JITTER.as_str(),
                Setting::String("nonsense".to_string()),
            ),
        ]));
        assert_eq!(bad.base, Duration::from_millis(250));
        assert_eq!(bad.cap, Duration::from_secs(16));
        assert_eq!(bad.factor, 2.0);
        assert!(matches!(bad.jitter, Jitter::Decorrelated));

        let at_bound = backoff_from_params(&params(&[(
            param_names::RETRY_BACKOFF_CAP_MS.as_str(),
            Setting::Int(MAX_BACKOFF_MS),
        )]));
        assert_eq!(at_bound.cap, Duration::from_millis(MAX_BACKOFF_MS as u64));
    }

    #[test]
    fn http_and_put_get_derive_from_the_common_backoff() {
        // Empty params: both pipelines share the common default curve and
        // differ only in their total budget.
        let http = RetryPolicy::http(&ParamStore::new());
        let put_get = RetryPolicy::put_get(&ParamStore::new());
        for b in [&http.backoff, &put_get.backoff] {
            assert_eq!(b.base, Duration::from_millis(250));
            assert_eq!(b.cap, Duration::from_secs(16));
            assert_eq!(b.factor, 2.0);
            assert!(matches!(b.jitter, Jitter::Decorrelated));
        }
        assert_eq!(http.max_elapsed, Duration::from_secs(120));
        assert_eq!(put_get.max_elapsed, Duration::from_secs(600));

        // A configured override (single shared param set) reaches both.
        let store = params(&[(
            param_names::RETRY_BACKOFF_BASE_MS.as_str(),
            Setting::Int(42),
        )]);
        assert_eq!(
            RetryPolicy::http(&store).backoff.base,
            Duration::from_millis(42)
        );
        assert_eq!(
            RetryPolicy::put_get(&store).backoff.base,
            Duration::from_millis(42)
        );
    }

    fn store_with_status_codes(value: &str) -> ParamStore {
        let mut params = ParamStore::new();
        params.insert(
            param_names::RETRY_EXTRA_STATUS_CODES.as_str().to_string(),
            Setting::String(value.to_string()),
        );
        params
    }

    fn status_set(codes: &[u16]) -> BTreeSet<u16> {
        codes.iter().copied().collect()
    }

    #[test]
    fn http_reads_extra_status_codes() {
        let policy = RetryPolicy::http(&store_with_status_codes("404, 425"));
        assert_eq!(policy.extra_retryable_statuses, status_set(&[404, 425]));
    }

    #[test]
    fn http_absent_extra_statuses_is_empty() {
        let policy = RetryPolicy::http(&ParamStore::new());
        assert!(policy.extra_retryable_statuses.is_empty());
    }

    #[test]
    fn put_get_reads_extra_status_codes() {
        let policy = RetryPolicy::put_get(&store_with_status_codes("404,425"));
        assert_eq!(policy.extra_retryable_statuses, status_set(&[404, 425]));
        // put_get keeps its larger total budget; the backoff shape is covered
        // by `http_and_put_get_derive_from_the_common_backoff`.
        assert_eq!(policy.max_elapsed, Duration::from_secs(600));
    }

    #[test]
    fn put_get_absent_extra_statuses_is_empty() {
        let policy = RetryPolicy::put_get(&ParamStore::new());
        assert!(policy.extra_retryable_statuses.is_empty());
    }

    #[test]
    fn skips_blank_non_numeric_and_out_of_range_tokens() {
        // Empty tokens, garbage, out-of-range (<100 and >599), and overflow are dropped.
        let policy = RetryPolicy::http(&store_with_status_codes("404,abc,,600,99,700000"));
        assert_eq!(policy.extra_retryable_statuses, status_set(&[404]));
    }
}
