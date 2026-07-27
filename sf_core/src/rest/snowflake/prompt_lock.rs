use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

use crate::token_cache::{CacheKey, build_cache_key};

/// Shared mutable state for all per-[`CacheKey`] prompt locks.
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

/// Acquire the per-`CacheKey` prompt lock.
///
/// The `key` must be identical to the [`CacheKey`] used by the corresponding
/// token-cache helpers so that the lock granularity (idp, snowflake, username,
/// role, token_type) exactly matches the cache entry being protected.
///
/// Returns an infallible `PromptGuard`.  The waiter count for the key is
/// incremented before awaiting the async mutex so that concurrent callers
/// do not race to remove the entry while others are still queued.
pub(crate) async fn acquire(locks: &Arc<PromptLockMap>, key: &CacheKey) -> PromptGuard {
    let key = build_cache_key(key);
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
    use crate::token_cache::{TokenType, normalize_identifier, normalize_url};
    use std::sync::Arc as StdArc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn make_locks() -> Arc<PromptLockMap> {
        Arc::new(Mutex::new(HashMap::new()))
    }

    fn make_key(snowflake_url: &str, username: &str, token_type: TokenType) -> CacheKey {
        CacheKey {
            token_type,
            idp: normalize_url(snowflake_url),
            snowflake: normalize_url(snowflake_url),
            username: normalize_identifier(username),
            role: String::new(),
        }
    }

    fn make_key_with_role(
        snowflake_url: &str,
        username: &str,
        token_type: TokenType,
        role: &str,
    ) -> CacheKey {
        CacheKey {
            token_type,
            idp: normalize_url(snowflake_url),
            snowflake: normalize_url(snowflake_url),
            username: normalize_identifier(username),
            role: normalize_identifier(role),
        }
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
                let key = make_key(
                    "https://host.snowflakecomputing.com",
                    "alice",
                    TokenType::IdToken,
                );
                let _guard = acquire(&l, &key).await;
                let prev = c.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
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
                let key = make_key("https://host.snowflakecomputing.com", "alice", token_type);
                let _guard = acquire(&l, &key).await;
                let now = active_c.fetch_add(1, Ordering::SeqCst) + 1;
                let mut cur = peak_c.load(Ordering::SeqCst);
                while cur < now {
                    match peak_c.compare_exchange(cur, now, Ordering::SeqCst, Ordering::SeqCst) {
                        Ok(_) => break,
                        Err(x) => cur = x,
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
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
            let key = make_key(
                "https://host.snowflakecomputing.com",
                "alice",
                TokenType::IdToken,
            );
            let _g = acquire(&locks, &key).await;
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

    #[tokio::test]
    async fn different_snowflake_hosts_do_not_block_each_other() {
        // Two keys that differ only in their Snowflake host must acquire their
        // own locks concurrently — neither should block the other.
        let locks = make_locks();
        let peak = StdArc::new(AtomicUsize::new(0));
        let active = StdArc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for host in [
            "https://account1.snowflakecomputing.com",
            "https://account2.snowflakecomputing.com",
        ] {
            let peak_c = peak.clone();
            let active_c = active.clone();
            let l = Arc::clone(&locks);
            handles.push(tokio::spawn(async move {
                let key = make_key(host, "alice", TokenType::IdToken);
                let _guard = acquire(&l, &key).await;
                let now = active_c.fetch_add(1, Ordering::SeqCst) + 1;
                let mut cur = peak_c.load(Ordering::SeqCst);
                while cur < now {
                    match peak_c.compare_exchange(cur, now, Ordering::SeqCst, Ordering::SeqCst) {
                        Ok(_) => break,
                        Err(x) => cur = x,
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                active_c.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(
            peak.load(Ordering::SeqCst),
            2,
            "different-host keys must not block each other"
        );
    }

    #[tokio::test]
    async fn different_roles_do_not_block_each_other() {
        // Two keys that differ only in role must each acquire their own lock
        // concurrently — they must not serialize behind a single lock entry.
        let locks = make_locks();
        let peak = StdArc::new(AtomicUsize::new(0));
        let active = StdArc::new(AtomicUsize::new(0));

        let host = "https://account.snowflakecomputing.com";
        let mut handles = Vec::new();
        for role in ["ANALYST", "ADMIN"] {
            let peak_c = peak.clone();
            let active_c = active.clone();
            let l = Arc::clone(&locks);
            handles.push(tokio::spawn(async move {
                let key = make_key_with_role(host, "alice", TokenType::OAuthAccessToken, role);
                let _guard = acquire(&l, &key).await;
                let now = active_c.fetch_add(1, Ordering::SeqCst) + 1;
                let mut cur = peak_c.load(Ordering::SeqCst);
                while cur < now {
                    match peak_c.compare_exchange(cur, now, Ordering::SeqCst, Ordering::SeqCst) {
                        Ok(_) => break,
                        Err(x) => cur = x,
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                active_c.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(
            peak.load(Ordering::SeqCst),
            2,
            "different-role keys must not block each other"
        );
    }
}
