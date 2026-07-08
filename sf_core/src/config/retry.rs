use super::param_registry::{
    DEFAULT_PUT_GET_MAX_ATTEMPTS, DEFAULT_RETRY_MAX_ATTEMPTS, param_names,
};
use super::param_store::ParamStore;
use std::collections::BTreeSet;
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
            backoff: BackoffConfig {
                base: Duration::from_millis(50),
                factor: 2.0,
                cap: Duration::from_millis(1500),
                jitter: Jitter::Decorrelated,
            },
            max_elapsed: Duration::from_secs(120),
            per_request_timeout: None,
            extra_retryable_statuses: BTreeSet::new(),
        }
    }
}

impl RetryPolicy {
    /// Builds the retry policy for general HTTP calls (login, query, logout, etc.).
    ///
    /// Reads `retry_max_attempts` from the [`ParamStore`] (falling back to
    /// [`DEFAULT_RETRY_MAX_ATTEMPTS`]) and returns the default policy shape
    /// with only `max_attempts` overridden.
    pub fn http(params: &ParamStore) -> Self {
        let max_attempts = params
            .get_int(param_names::RETRY_MAX_ATTEMPTS)
            .filter(|v| *v > 0 && *v <= u32::MAX as i64)
            .map(|v| v as u32)
            .unwrap_or(DEFAULT_RETRY_MAX_ATTEMPTS);

        Self {
            max_attempts,
            ..Self::default()
        }
    }

    /// Builds the base put/get retry policy from connection parameters.
    ///
    /// Reads `put_get_max_attempts` from the [`ParamStore`] (falling back to
    /// [`DEFAULT_PUT_GET_MAX_ATTEMPTS`] for absent or out-of-range values) and
    /// applies the shared backoff shape (1s base, 2× factor, 16s cap, no jitter,
    /// 600s total budget).
    ///
    /// Cloud-specific code clones this base and adds its own tweaks
    /// (extra retryable statuses, per-request timeout).
    pub fn put_get(params: &ParamStore) -> Self {
        let max_attempts = params
            .get_int(param_names::PUT_GET_MAX_ATTEMPTS)
            .filter(|v| *v > 0 && *v <= u32::MAX as i64)
            .map(|v| v as u32)
            .unwrap_or(DEFAULT_PUT_GET_MAX_ATTEMPTS);

        Self {
            max_attempts,
            backoff: BackoffConfig {
                base: Duration::from_secs(1),
                factor: 2.0,
                cap: Duration::from_secs(16),
                jitter: Jitter::None,
            },
            max_elapsed: Duration::from_secs(600),
            ..Self::default()
        }
    }
}
