use std::time::{Duration, Instant};

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

/// A shared wall-clock deadline for an entire operation (retries + refresh included).
///
/// Created once at the top-level call site and threaded through every layer.
/// Each layer calls [`remaining()`](Deadline::remaining) to learn how much time
/// is left, so session refresh, Okta renewal, and HTTP retries all draw from
/// the same budget.
#[derive(Clone, Debug)]
pub struct Deadline {
    start: Instant,
    budget: Duration,
}

impl Deadline {
    /// Start a new deadline with the given total budget.
    pub fn new(budget: Duration) -> Self {
        Self {
            start: Instant::now(),
            budget,
        }
    }

    /// How much time remains before the deadline expires.
    /// Returns `Duration::ZERO` when expired.
    pub fn remaining(&self) -> Duration {
        self.budget.saturating_sub(self.start.elapsed())
    }

    /// Returns `true` when the budget has been fully consumed.
    pub fn is_expired(&self) -> bool {
        self.start.elapsed() >= self.budget
    }

    /// The original budget this deadline was created with.
    pub fn budget(&self) -> Duration {
        self.budget
    }

    /// How much time has elapsed since this deadline was created.
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
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
        }
    }
}
