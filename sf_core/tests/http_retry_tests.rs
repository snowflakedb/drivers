use sf_core::config::retry::{Jitter, RetryPolicy};

#[test]
fn defaults_are_sane() {
    let p = RetryPolicy::default();
    assert!(p.http.retry_safe_reads);
    assert!(p.http.retry_idempotent_writes);
    assert!(!p.http.retry_post_patch);
    assert_eq!(p.submission.max_attempts, 6);
    assert!(matches!(p.submission.backoff.jitter, Jitter::Decorrelated));
    assert_eq!(p.chunk.max_attempts, 8);
}
