use sf_core::config::retry::{Jitter, RetryPolicy};
use std::time::Duration;

#[test]
fn defaults_are_sane() {
    let p = RetryPolicy::default();
    assert!(p.http.retry_safe_reads);
    assert!(p.http.retry_idempotent_writes);
    assert!(!p.http.retry_post_patch);
    assert_eq!(p.max_attempts, 6);
    assert_eq!(p.backoff.base, Duration::from_millis(50));
    assert!(matches!(p.backoff.jitter, Jitter::Decorrelated));
    assert_eq!(p.max_elapsed, Duration::from_secs(120));
}
