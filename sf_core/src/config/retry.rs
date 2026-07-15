use super::param_registry::{
    DEFAULT_LOGIN_TIMEOUT_SECS, DEFAULT_PUT_GET_MAX_ATTEMPTS, DEFAULT_RETRY_BACKOFF_BASE_MS,
    DEFAULT_RETRY_BACKOFF_CAP_MS, DEFAULT_RETRY_BACKOFF_FACTOR, DEFAULT_RETRY_MAX_ATTEMPTS,
    param_names,
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
    /// Optional total-duration budget enforced within the retry loop itself.
    ///
    /// When `None`, the retry loop retries up to `max_attempts` with no
    /// internal deadline — the caller is expected to enforce a wall-clock
    /// budget via `tokio::time::timeout` (operation-level timeout).
    ///
    /// When `Some`, the retry loop checks the budget at the top of each
    /// iteration and applies `min(per_request_timeout, remaining_budget)` as
    /// the per-attempt timeout (backward-compatible path used by PUT/GET cloud
    /// transfer modules and other call sites that need a self-contained budget).
    ///
    /// `Some(Duration::ZERO)` means the budget is already exhausted on the first
    /// iteration, so the loop returns `DeadlineExceeded` immediately without
    /// issuing a request. The constructors never produce this: a configured
    /// timeout of `0` is interpreted as "no timeout" (`None`) when read from the
    /// [`ParamStore`], so `Some(0)` can only arise from a hand-built policy.
    pub max_elapsed: Option<Duration>,
    /// Optional per-request socket timeout. If Some, each HTTP request gets
    /// this timeout (clamped to remaining budget when `max_elapsed` is also
    /// set). If None and `max_elapsed` is also None, no per-attempt timeout
    /// is applied — the outer operation timeout is the only guard.
    /// Note: `cloud_http` enforces a `REQUEST_TIMEOUT_SECS` fallback in the
    /// `(None, None)` case regardless.
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
            max_elapsed: None,
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
    /// `max_elapsed` is `None`: the retry loop does not enforce an internal
    /// deadline. Callers wrap the operation with `tokio::time::timeout` using
    /// `login_timeout` / `query_timeout` / `request_timeout` to bound total
    /// wall-clock time.
    ///
    /// Reads `retry_max_attempts`, `retry_backoff_*`, and `retry_extra_status_codes`
    /// from the [`ParamStore`].
    pub fn http(params: &ParamStore) -> Self {
        let max_attempts = params
            .get_int(param_names::RETRY_MAX_ATTEMPTS)
            .filter(|v| *v > 0 && *v <= u32::MAX as i64)
            .map(|v| v as u32)
            .unwrap_or(DEFAULT_RETRY_MAX_ATTEMPTS);

        Self {
            max_attempts,
            backoff: backoff_from_params(params),
            max_elapsed: None,
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
            max_elapsed: Some(Duration::from_secs(600)),
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

// ── Timeout configuration helpers ──────────────────────────────────────

/// Read an optional positive-seconds duration from the [`ParamStore`].
///
/// Returns `None` when absent, zero, or negative — zero semantically means
/// "no timeout" throughout the timeout configuration surface.
fn read_optional_duration_secs(
    params: &ParamStore,
    key: super::param_registry::ParamKey,
) -> Option<Duration> {
    params
        .get_int(key)
        .filter(|v| *v > 0)
        .map(|v| Duration::from_secs(v as u64))
}

/// Resolved operation-level timeout configuration.
///
/// Built from the `ParamStore` once and carried through the connection
/// lifetime so each operation site can apply the right wall-clock budget
/// via `tokio::time::timeout`.
#[derive(Clone, Debug)]
pub struct TimeoutConfig {
    /// Wall-clock budget for the whole login operation, including every auth
    /// request and retry. The timer starts when login begins and ends when login
    /// succeeds or the budget elapses (enforced by an outer `tokio::time::timeout`
    /// wrapping the login future). `None` means no timeout; a configured value of
    /// `0` is read as `None`.
    pub login_timeout: Option<Duration>,
    /// Wall-clock budget for the whole query execution, spanning the initial
    /// submit plus the status-poll / token-refresh loop. The timer starts when
    /// execution begins and ends when the query returns or the budget elapses
    /// (enforced by `tokio::time::timeout_at` around the poll loop). `None` means
    /// no timeout; a configured value of `0` is read as `None`. Defaults to no
    /// timeout, matching the legacy drivers — queries can legitimately run for
    /// hours, so any finite default risks breaking existing clients.
    pub query_timeout: Option<Duration>,
    /// TCP connect timeout for the HTTP client. `None` means the system default.
    pub connect_timeout: Option<Duration>,
}

impl TimeoutConfig {
    /// Resolve the timeout configuration from connection parameters.
    ///
    /// Fields are wired up progressively across this stack; any parameter not
    /// yet read falls back to [`Self::default`].
    pub fn from_params(params: &ParamStore) -> Self {
        Self {
            login_timeout: read_optional_duration_secs(params, param_names::LOGIN_TIMEOUT),
            connect_timeout: read_optional_duration_secs(params, param_names::CONNECT_TIMEOUT),
            ..Self::default()
        }
    }
}

impl Default for TimeoutConfig {
    /// Defaults used when no `ParamStore` is available — chiefly tests, and the
    /// pre-connect state of the `Connection` before [`from_params`](Self::from_params)
    /// resolves the configured values.
    fn default() -> Self {
        Self {
            login_timeout: Some(Duration::from_secs(DEFAULT_LOGIN_TIMEOUT_SECS)),
            query_timeout: None,
            connect_timeout: None,
        }
    }
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
        let http = RetryPolicy::http(&ParamStore::new());
        let put_get = RetryPolicy::put_get(&ParamStore::new());
        for b in [&http.backoff, &put_get.backoff] {
            assert_eq!(b.base, Duration::from_millis(250));
            assert_eq!(b.cap, Duration::from_secs(16));
            assert_eq!(b.factor, 2.0);
            assert!(matches!(b.jitter, Jitter::Decorrelated));
        }
        // http() has no internal deadline (caller uses operation timeout).
        assert_eq!(http.max_elapsed, None);
        // put_get() keeps a self-contained 600s budget for cloud transfer modules.
        assert_eq!(put_get.max_elapsed, Some(Duration::from_secs(600)));

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
        assert_eq!(policy.max_elapsed, Some(Duration::from_secs(600)));
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
