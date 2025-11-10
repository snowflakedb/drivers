use std::time::Duration;

/// Global retry policy with per-category overrides.
#[derive(Clone, Debug)]
pub struct RetryPolicy {
    pub http: HttpPolicy,
    pub submission: CategoryPolicy,
    pub poll: CategoryPolicy,
    pub chunk: CategoryPolicy,
    pub auth: CategoryPolicy,
    pub inline_short_poll: InlineShortPoll,
    pub deadline: Duration,
}

#[derive(Clone, Debug)]
pub struct CategoryPolicy {
    pub max_attempts: u32,
    pub backoff: BackoffStrategy,
}

#[derive(Clone, Debug)]
pub struct BackoffStrategy {
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
pub struct InlineShortPoll {
    pub delays: Vec<Duration>,
    pub budget: Duration,
}

#[derive(Clone, Debug)]
pub struct HttpPolicy {
    /// Enable retries for GET/HEAD/OPTIONS
    pub retry_safe_reads: bool,
    /// Enable retries for PUT/DELETE (idempotent operations)
    pub retry_idempotent_writes: bool,
    /// Enable retries for POST/PATCH (generally off). Snowflake submission will override safely.
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
            submission: CategoryPolicy {
                max_attempts: 6,
                backoff: BackoffStrategy {
                    base: Duration::from_millis(50),
                    factor: 2.0,
                    cap: Duration::from_millis(1500),
                    jitter: Jitter::Decorrelated,
                },
            },
            poll: CategoryPolicy {
                max_attempts: 1000, // governed by deadline primarily
                backoff: BackoffStrategy {
                    base: Duration::from_millis(250),
                    factor: 1.6,
                    cap: Duration::from_millis(2000),
                    jitter: Jitter::Full,
                },
            },
            chunk: CategoryPolicy {
                max_attempts: 8,
                backoff: BackoffStrategy {
                    base: Duration::from_millis(50),
                    factor: 2.0,
                    cap: Duration::from_millis(2000),
                    jitter: Jitter::Decorrelated,
                },
            },
            auth: CategoryPolicy {
                max_attempts: 3,
                backoff: BackoffStrategy {
                    base: Duration::from_millis(100),
                    factor: 2.0,
                    cap: Duration::from_millis(1000),
                    jitter: Jitter::Full,
                },
            },
            inline_short_poll: InlineShortPoll {
                delays: vec![
                    Duration::from_millis(0),
                    Duration::from_millis(20),
                    Duration::from_millis(50),
                    Duration::from_millis(100),
                ],
                budget: Duration::from_millis(150),
            },
            deadline: Duration::from_secs(120),
        }
    }
}
