use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

use crate::token_cache::TokenType;

/// Shared mutable state for all per-`(host, user, token-type)` prompt locks.
///
/// An `Arc` of this type is stored on [`crate::apis::database_driver_v1::global_state::DatabaseDriverV1`]
/// and passed into `snowflake_login_with_client`, making the lock process-global
/// without a `static`.  Each map entry carries a waiter count so entries are
/// removed as soon as the last holder/waiter leaves, preventing unbounded growth
/// in long-lived processes.
///
/// Exposed as `pub` so that `DriverProviders` can accept an externally-created
/// map (e.g. to share one map across multiple driver instances in tests).
pub type PromptLockMap = Mutex<HashMap<String, (Arc<AsyncMutex<()>>, usize)>>;

pub(crate) struct PromptGuard {
    _inner: OwnedMutexGuard<()>,
    map: Arc<PromptLockMap>,
    key: String,
}

impl Drop for PromptGuard {
    fn drop(&mut self) {
        // Decrement the waiter count; remove the entry when this was the last
        // holder so the map does not grow unboundedly for long-lived processes.
        // Recover from a poisoned mutex rather than crashing — the critical
        // section is a simple HashMap mutation that cannot panic in practice.
        let mut guard = self.map.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = guard.get_mut(&self.key) {
            entry.1 -= 1;
            if entry.1 == 0 {
                guard.remove(&self.key);
            }
        }
    }
}

/// Acquire the per-`(host, user, token_type)` prompt lock.
///
/// Returns an infallible `PromptGuard`.  The waiter count for the key is
/// incremented before awaiting the async mutex so that concurrent callers
/// do not race to remove the entry while others are still queued.
pub(crate) async fn acquire(
    locks: &Arc<PromptLockMap>,
    host: &str,
    user: &str,
    token_type: TokenType,
) -> PromptGuard {
    let key = crate::token_cache::build_cache_key(host, user, token_type);
    let arc = {
        let mut guard = locks.lock().unwrap_or_else(|e| e.into_inner());
        let (mutex, count) = guard
            .entry(key.clone())
            .or_insert_with(|| (Arc::new(AsyncMutex::new(())), 0));
        *count += 1;
        Arc::clone(mutex)
    };
    PromptGuard {
        _inner: arc.lock_owned().await,
        map: Arc::clone(locks),
        key,
    }
}

pub(crate) fn is_eligible(
    client_store_temporary_credential: bool,
    disable_parallel_user_prompt: bool,
    user: &str,
) -> bool {
    client_store_temporary_credential && disable_parallel_user_prompt && !user.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc as StdArc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    fn make_locks() -> Arc<PromptLockMap> {
        Arc::new(Mutex::new(HashMap::new()))
    }

    #[tokio::test]
    async fn same_key_serializes() {
        let locks = make_locks();
        let counter = StdArc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        for _ in 0..5 {
            let c = counter.clone();
            let l = Arc::clone(&locks);
            handles.push(tokio::spawn(async move {
                let _guard = acquire(
                    &l,
                    "host.snowflakecomputing.com",
                    "alice",
                    TokenType::IdToken,
                )
                .await;
                let prev = c.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(10)).await;
                let after = c.load(Ordering::SeqCst);
                assert_eq!(after, prev + 1, "concurrent access inside lock window");
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
    }

    #[tokio::test]
    async fn different_keys_do_not_block_each_other() {
        let locks = make_locks();
        let peak = StdArc::new(AtomicUsize::new(0));
        let active = StdArc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for token_type in [TokenType::IdToken, TokenType::MfaToken] {
            let peak_c = peak.clone();
            let active_c = active.clone();
            let l = Arc::clone(&locks);
            handles.push(tokio::spawn(async move {
                let _guard = acquire(&l, "host.snowflakecomputing.com", "alice", token_type).await;
                let now = active_c.fetch_add(1, Ordering::SeqCst) + 1;
                let mut cur = peak_c.load(Ordering::SeqCst);
                while cur < now {
                    match peak_c.compare_exchange(cur, now, Ordering::SeqCst, Ordering::SeqCst) {
                        Ok(_) => break,
                        Err(x) => cur = x,
                    }
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
                active_c.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(
            peak.load(Ordering::SeqCst),
            2,
            "different token-type keys must not block each other"
        );
    }

    #[tokio::test]
    async fn map_entry_removed_after_last_holder_drops() {
        let locks = make_locks();
        {
            let _g = acquire(
                &locks,
                "host.snowflakecomputing.com",
                "alice",
                TokenType::IdToken,
            )
            .await;
            assert_eq!(
                locks.lock().unwrap().len(),
                1,
                "entry should exist while guard is held"
            );
        }
        assert_eq!(
            locks.lock().unwrap().len(),
            0,
            "entry should be removed after last guard is dropped"
        );
    }

    #[test]
    fn is_eligible_requires_all_conditions() {
        // All conditions met: caching on, locking enabled, user present
        assert!(is_eligible(true, true, "alice"));

        // caching off
        assert!(!is_eligible(false, true, "alice"));

        // locking disabled (DISABLE_PARALLEL_USER_PROMPT=false)
        assert!(!is_eligible(true, false, "alice"));

        // empty user
        assert!(!is_eligible(true, true, ""));
    }
}
