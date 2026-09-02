//! Per-connection query context cache for HTAP workloads.
//!
//! Maintains a bounded set of [`CacheEntry`] values keyed by entry `id`.
//! An `eviction_order` vec keeps entry ids sorted by `(priority, -timestamp)`
//! ascending, so `pop()` directly yields the eviction target: highest priority
//! number, lowest (oldest) timestamp.
use super::query_response;
use crate::apis::database_driver_v1::WrapperPresets;
use crate::config::ParamStore;
use crate::rest::snowflake::query_request::{ContextData, QueryContext, QueryContextEntry};
use sf_params_spec::param_names;
use std::collections::BTreeMap;

/// Default maximum number of entries retained in the cache.
pub const DEFAULT_MAX_SIZE: usize = 5;

/// Name of the server-side parameter that overrides the cache capacity.
const CACHE_SIZE_PARAM: &str = "QUERY_CONTEXT_CACHE_SIZE";

#[derive(Clone, Debug)]
pub struct CacheEntry {
    pub id: i64,
    pub priority: i64,
    pub timestamp: i64,
    /// Opaque base64-encoded context blob forwarded verbatim to the server.
    pub context: Option<String>,
}

pub struct QueryContextCacheAdapter {
    query_context_cache: QueryContextCache,
    is_query_context_cache_disabled: bool,
    clear_query_context_on_null_entries: bool,
}

impl QueryContextCacheAdapter {
    pub(crate) fn new() -> Self {
        Self {
            query_context_cache: QueryContextCache::new(),
            is_query_context_cache_disabled: true,
            clear_query_context_on_null_entries: false,
        }
    }
    pub(crate) fn init(
        &mut self,
        param_store: Option<&ParamStore>,
        wrapper_presets: &WrapperPresets,
    ) {
        self.clear_query_context_on_null_entries =
            wrapper_presets.clear_query_context_on_null_entries;

        self.is_query_context_cache_disabled = param_store
            .and_then(|s| s.get_bool(param_names::DISABLE_QUERY_CONTEXT_CACHE))
            .unwrap_or(false)
    }
    pub(crate) async fn get_query_context_snapshot(&self) -> QueryContext {
        if self.is_query_context_cache_disabled {
            QueryContext::default()
        } else {
            self.query_context_cache.snapshot().unwrap_or_default()
        }
    }
    pub(crate) async fn update_query_context_cache(
        &mut self,
        query_context: Option<&query_response::QueryContext>,
        parameters: Option<&Vec<query_response::NameValueParameter>>,
    ) {
        if self.is_query_context_cache_disabled {
            return;
        }

        let cache = &mut self.query_context_cache;
        cache.update(query_context, self.clear_query_context_on_null_entries);

        if let Some(params) = parameters {
            for param in params {
                if param.name.eq_ignore_ascii_case(CACHE_SIZE_PARAM) {
                    if let Some(new_size) = parse_cache_size(&param.value) {
                        cache.update_max_size(new_size);
                    }
                    return;
                }
            }
        }
    }
}

/// Query context cache with support for duplicate priorities.
///
/// Two structures:
/// - `entries` (id → CacheEntry): primary store, O(log n) lookup by id.
/// - `eviction_order` (Vec<id>): ids sorted by `(priority, -timestamp)` ascending.
///   `pop()` yields the eviction target: highest priority number, oldest timestamp.
pub struct QueryContextCache {
    /// Primary store: entry id → CacheEntry.
    entries: BTreeMap<i64, CacheEntry>,
    /// Entry ids sorted by `(priority, -timestamp)` ascending.
    /// Last element = highest priority + oldest timestamp = eviction target.
    eviction_order: Vec<i64>,
    max_size: usize,
}

impl QueryContextCache {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            eviction_order: Vec::new(),
            max_size: DEFAULT_MAX_SIZE,
        }
    }

    pub fn update(
        &mut self,
        query_ctx: Option<&query_response::QueryContext>,
        clear_query_context_on_null_entries: bool,
    ) {
        let Some(query_ctx) = query_ctx else {
            tracing::debug!(
                "query_context_cache: no queryContext in response, keeping cache unchanged"
            );
            return;
        };

        let Some(entries) = query_ctx.entries.as_deref() else {
            if clear_query_context_on_null_entries {
                tracing::debug!("query_context_cache: entries is null, clearing cache");
                self.clear();
            }

            return;
        };

        if entries.is_empty() {
            tracing::debug!("query_context_cache: entries is empty, clearing cache");
            self.clear();
            return;
        }

        for e in entries {
            if let Some(existing) = self.entries.get_mut(&e.id) {
                if e.timestamp < existing.timestamp {
                    continue;
                }
                if existing.priority != e.priority {
                    tracing::debug!(
                        id = e.id,
                        old_priority = existing.priority,
                        new_priority = e.priority,
                        "query_context_cache: priority changed for entry"
                    );
                }
                existing.priority = e.priority;
                existing.timestamp = e.timestamp;
                if existing.context != e.context {
                    existing.context = e.context.clone();
                }
            } else {
                tracing::debug!(
                    id = e.id,
                    priority = e.priority,
                    "query_context_cache: inserting new entry"
                );
                self.entries.insert(
                    e.id,
                    CacheEntry {
                        id: e.id,
                        priority: e.priority,
                        timestamp: e.timestamp,
                        context: e.context.clone(),
                    },
                );
                self.eviction_order.push(e.id);
            }
        }

        self.sort_eviction_order();
        self.enforce_capacity();
    }

    pub(crate) fn update_max_size(&mut self, new_size: usize) {
        self.max_size = new_size;
        self.enforce_capacity();
    }

    /// Returns a [`QueryContext`] snapshot for embedding in
    /// the next outgoing query request, or `None` when the cache is empty.
    pub fn snapshot(&self) -> Option<QueryContext> {
        if self.entries.is_empty() {
            return None;
        }

        let entries: Vec<QueryContextEntry> = self
            .entries
            .values()
            .map(|e| QueryContextEntry {
                context: Some(ContextData {
                    base64_data: e.context.clone(),
                }),
                id: e.id,
                priority: e.priority,
                timestamp: Some(e.timestamp),
            })
            .collect();

        Some(QueryContext {
            entries: Some(entries),
        })
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.eviction_order.clear();
    }

    fn remove_entry(&mut self, id: i64) {
        self.entries.remove(&id);
        self.eviction_order.retain(|&x| x != id);
    }

    fn sort_eviction_order(&mut self) {
        self.eviction_order.sort_by_key(|&id| {
            self.entries
                .get(&id)
                .map_or((i64::MAX, 0), |e| (e.priority, -e.timestamp))
        });
    }

    fn enforce_capacity(&mut self) {
        while self.entries.len() > self.max_size {
            let Some(evict_id) = self.eviction_order.pop() else {
                tracing::error!(
                    entries_len = self.entries.len(),
                    max_size = self.max_size,
                    "query_context_cache: eviction_order empty but entries exceed max_size"
                );
                break;
            };
            if let Some(entry) = self.entries.remove(&evict_id) {
                tracing::debug!(
                    evict_id,
                    priority = entry.priority,
                    timestamp = entry.timestamp,
                    cache_size = self.entries.len(),
                    max_size = self.max_size,
                    "query_context_cache: evicting entry to enforce capacity"
                );
            }
        }
    }
}

impl Default for QueryContextCache {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_cache_size(value: &serde_json::Value) -> Option<usize> {
    let n = match value {
        serde_json::Value::Number(n) => n.as_u64()?,
        serde_json::Value::String(s) => s.parse::<u64>().ok()?,
        _ => return None,
    };
    if n == 0 { None } else { Some(n as usize) }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use query_response::{QueryContext as RespCtx, QueryContextEntry as RespEntry};

    fn assert_eviction_order_synced(cache: &QueryContextCache, label: &str) {
        assert_eq!(
            cache.eviction_order.len(),
            cache.entries.len(),
            "{label}: eviction_order len must match entries len"
        );
        for &id in &cache.eviction_order {
            assert!(
                cache.entries.contains_key(&id),
                "{label}: eviction_order contains id={id} not in entries"
            );
        }
    }

    fn make_entry(id: i64, priority: i64, timestamp: i64) -> RespEntry {
        RespEntry {
            id,
            priority,
            timestamp,
            context: None,
        }
    }

    fn make_entry_with_ctx(id: i64, priority: i64, timestamp: i64, query_ctx: &str) -> RespEntry {
        RespEntry {
            id,
            priority,
            timestamp,
            context: Some(query_ctx.to_owned()),
        }
    }

    fn resp_ctx(entries: Vec<RespEntry>) -> RespCtx {
        RespCtx {
            entries: Some(entries),
        }
    }

    #[test]
    fn test_empty_cache_reports_zero_entries() {
        let cache = QueryContextCache::new();

        assert!(cache.entries.is_empty());
        assert!(cache.eviction_order.is_empty());
        assert!(cache.snapshot().is_none());
    }

    #[test]
    fn test_basic_round_trip() {
        let mut cache = QueryContextCache::new();
        let entry = make_entry(1, 10, 100);
        let context = resp_ctx(vec![entry]);
        cache.update(Some(&context), true);

        let snap = cache.snapshot().expect("should have snapshot");
        let entries = snap.entries.expect("should have entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, 1);
        assert_eq!(entries[0].priority, 10);
        assert_eq!(entries[0].timestamp, Some(100));
    }

    #[test]
    fn test_absent_context_keeps_cache() {
        let mut cache = QueryContextCache::new();
        let entry = make_entry(1, 10, 100);
        let context = resp_ctx(vec![entry]);
        cache.update(Some(&context), true);

        cache.update(None, true);

        assert_eq!(cache.entries.len(), 1);
    }

    #[test]
    fn test_null_entries_clears_cache_when_flag_true() {
        let mut cache = QueryContextCache::new();
        let entry = make_entry(1, 10, 100);
        let seed = resp_ctx(vec![entry]);
        cache.update(Some(&seed), true);

        let null_ctx = RespCtx { entries: None };
        cache.update(Some(&null_ctx), true);

        assert!(cache.entries.is_empty(), "cache should be cleared");
    }

    #[test]
    fn test_null_entries_preserves_cache_when_flag_false() {
        let mut cache = QueryContextCache::new();
        let entry = make_entry(1, 10, 100);
        let seed = resp_ctx(vec![entry]);
        cache.update(Some(&seed), false);

        let null_ctx = RespCtx { entries: None };
        cache.update(Some(&null_ctx), false);

        assert_eq!(cache.entries.len(), 1, "cache should be preserved");
        assert!(cache.entries.contains_key(&1));
    }

    #[test]
    fn test_empty_vec_clears_cache() {
        let mut cache = QueryContextCache::new();
        let entry = make_entry(1, 10, 100);
        let seed = resp_ctx(vec![entry]);
        cache.update(Some(&seed), true);

        let empty = resp_ctx(vec![]);
        cache.update(Some(&empty), true);

        assert!(cache.entries.is_empty());
    }

    #[test]
    fn test_merge_with_overlap() {
        let mut cache = QueryContextCache::new();
        let seed = resp_ctx(vec![make_entry(1, 10, 100), make_entry(2, 20, 200)]);
        cache.update(Some(&seed), true);

        // id=1 updates timestamp in place; id=3 is inserted as new
        let update = resp_ctx(vec![make_entry(1, 10, 999), make_entry(3, 30, 300)]);
        cache.update(Some(&update), true);

        assert_eq!(cache.entries.len(), 3, "ids 1, 2, 3 should all be present");
        let snap = cache.snapshot().unwrap();
        let entries = snap.entries.unwrap();
        let id1 = entries.iter().find(|e| e.id == 1).expect("id 1 missing");
        assert_eq!(id1.timestamp, Some(999), "id 1 timestamp should be updated");
    }

    #[test]
    fn test_eviction_removes_highest_priority_number() {
        let mut cache = QueryContextCache::new();
        let initial: Vec<_> = (0..5i64).map(|i| make_entry(i, i * 10, i)).collect();
        let seed = resp_ctx(initial);
        cache.update(Some(&seed), true);
        assert_eq!(cache.entries.len(), 5);

        // id=2 updates in place; id=5 is inserted as new.
        // Cache grows to 6 entries; enforce_capacity evicts highest priority number.
        let batch = resp_ctx(vec![make_entry(2, 20, 4), make_entry(5, 33, 5)]);
        cache.update(Some(&batch), true);

        assert_eq!(cache.entries.len(), 5);
        assert!(
            !cache.entries.contains_key(&4),
            "highest-priority-number entry (id=4, priority=40) should have been evicted"
        );
    }

    #[test]
    fn test_update_max_size() {
        let mut cache = QueryContextCache::new();
        assert_eq!(cache.max_size, DEFAULT_MAX_SIZE);

        cache.update_max_size(3);
        assert_eq!(cache.max_size, 3);
    }

    #[test]
    fn test_parse_cache_size() {
        assert_eq!(parse_cache_size(&serde_json::json!(3)), Some(3));
        assert_eq!(parse_cache_size(&serde_json::json!("7")), Some(7));
        assert_eq!(parse_cache_size(&serde_json::json!(0)), None);
        assert_eq!(parse_cache_size(&serde_json::json!("abc")), None);
        assert_eq!(parse_cache_size(&serde_json::json!(null)), None);
    }

    #[test]
    fn test_update_max_size_evicts_excess_immediately() {
        let mut cache = QueryContextCache::new();
        let initial: Vec<_> = (0..5i64).map(|i| make_entry(i, i, i)).collect();
        let seed = resp_ctx(initial);
        cache.update(Some(&seed), true);
        assert_eq!(cache.entries.len(), 5);

        // Shrink to 3 — highest-priority-number entries evicted
        cache.update_max_size(3);

        assert_eq!(cache.entries.len(), 3);
        assert!(!cache.entries.contains_key(&4), "priority-4 entry evicted");
        assert!(!cache.entries.contains_key(&3), "priority-3 entry evicted");
    }

    #[test]
    fn test_context_blob_forwarded() {
        let mut cache = QueryContextCache::new();
        let entry = make_entry_with_ctx(1, 10, 100, "abc123");
        let context = resp_ctx(vec![entry]);
        cache.update(Some(&context), true);

        let snap = cache.snapshot().unwrap();
        let result_entry = &snap.entries.unwrap()[0];
        let ctx_data = result_entry
            .context
            .as_ref()
            .expect("context should be present");
        assert_eq!(ctx_data.base64_data.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_same_priority_insert_evicts_oldest_by_capacity() {
        let mut cache = QueryContextCache::new();
        // All same priority — all coexist in priority_map with different timestamps
        let same_priority: Vec<_> = (0..5i64).map(|i| make_entry(i, 99, i)).collect();
        let seed = resp_ctx(same_priority);
        cache.update(Some(&seed), true);
        assert_eq!(cache.entries.len(), 5);

        // id=4 triggers merge (same id, same priority, same timestamp → no-op).
        // id=5 is a new id at priority=99 → inserted, capacity evicts oldest (id=0, ts=0).
        let update = resp_ctx(vec![make_entry(4, 99, 4), make_entry(5, 99, 5)]);
        cache.update(Some(&update), true);

        assert_eq!(cache.entries.len(), 5);
        assert!(
            !cache.entries.contains_key(&0),
            "id=0 should be evicted (oldest timestamp at highest priority number)"
        );
        assert!(cache.entries.contains_key(&5), "id=5 should be inserted");
    }

    #[test]
    fn test_stale_entry_does_not_block_batch() {
        let mut cache = QueryContextCache::new();
        let seed = resp_ctx(vec![make_entry(1, 10, 500), make_entry(2, 20, 100)]);
        cache.update(Some(&seed), true);

        // id=1 has stale timestamp (50 < 500) → skipped.
        // id=2 has newer timestamp (999 > 100) → should be processed.
        let batch = resp_ctx(vec![make_entry(1, 10, 50), make_entry(2, 20, 999)]);
        cache.update(Some(&batch), true);

        let snap = cache.snapshot().unwrap();
        let entries = snap.entries.unwrap();
        let id1 = entries.iter().find(|e| e.id == 1).unwrap();
        let id2 = entries.iter().find(|e| e.id == 2).unwrap();
        assert_eq!(
            id1.timestamp,
            Some(500),
            "stale entry should keep old timestamp"
        );
        assert_eq!(id2.timestamp, Some(999), "fresh entry should be updated");
    }

    #[test]
    fn test_timestamp_updated_on_same_id_same_priority() {
        let mut cache = QueryContextCache::new();
        let entry = make_entry(1, 10, 100);
        let seed = resp_ctx(vec![entry]);
        cache.update(Some(&seed), true);

        let newer = make_entry(1, 10, 999);
        let update = resp_ctx(vec![newer]);
        cache.update(Some(&update), true);

        let snap = cache.snapshot().unwrap();
        let result = &snap.entries.unwrap()[0];
        assert_eq!(result.id, 1);
        assert_eq!(result.timestamp, Some(999), "timestamp should be updated");
    }

    #[test]
    fn test_same_priority_timestamp_update_keeps_eviction_order_in_sync() {
        let mut cache = QueryContextCache::new();
        let seed = resp_ctx(vec![make_entry(1, 10, 100)]);
        cache.update(Some(&seed), true);

        let update = resp_ctx(vec![make_entry(1, 10, 300)]);
        cache.update(Some(&update), true);

        let entry = cache.entries.get(&1).expect("id=1 missing");
        assert_eq!(entry.timestamp, 300);
        assert_eviction_order_synced(&cache, "timestamp_update");
    }

    #[tokio::test]
    async fn test_disabled_cache_returns_empty_snapshot() {
        let mut adapter = QueryContextCacheAdapter::new();
        assert!(adapter.is_query_context_cache_disabled);

        // Even after updating the inner cache directly, snapshot should be empty
        {
            let entry = make_entry(1, 10, 100);
            let context = resp_ctx(vec![entry]);
            adapter.query_context_cache.update(Some(&context), true);
        }

        let snap = adapter.get_query_context_snapshot().await;
        assert!(
            snap.entries.is_none(),
            "disabled cache should return empty snapshot"
        );
    }

    #[tokio::test]
    async fn test_disabled_cache_does_not_update() {
        let mut adapter = QueryContextCacheAdapter::new();

        let entry = make_entry(1, 10, 100);
        let context = resp_ctx(vec![entry]);
        adapter
            .update_query_context_cache(Some(&context), None)
            .await;

        assert!(
            adapter.query_context_cache.entries.is_empty(),
            "disabled cache should not store entries"
        );
    }

    #[tokio::test]
    async fn test_separate_caches_are_isolated() {
        let mut adapter_a = QueryContextCacheAdapter::new();
        adapter_a.is_query_context_cache_disabled = false;
        let mut adapter_b = QueryContextCacheAdapter::new();
        adapter_b.is_query_context_cache_disabled = false;

        let ctx_a = resp_ctx(vec![make_entry_with_ctx(1, 4, 100, "connA")]);
        adapter_a
            .update_query_context_cache(Some(&ctx_a), None)
            .await;

        let ctx_b = resp_ctx(vec![make_entry_with_ctx(2, 5, 200, "connB")]);
        adapter_b
            .update_query_context_cache(Some(&ctx_b), None)
            .await;

        let snap_a = adapter_a.get_query_context_snapshot().await;
        let entries_a = snap_a.entries.unwrap();
        assert_eq!(entries_a.len(), 1);
        assert_eq!(entries_a[0].id, 1);

        let snap_b = adapter_b.get_query_context_snapshot().await;
        let entries_b = snap_b.entries.unwrap();
        assert_eq!(entries_b.len(), 1);
        assert_eq!(entries_b[0].id, 2);
    }

    #[tokio::test]
    async fn test_sequential_updates_with_overlap_merge() {
        let mut adapter = QueryContextCacheAdapter::new();
        adapter.is_query_context_cache_disabled = false;

        let first = resp_ctx(vec![make_entry_with_ctx(1, 4, 100, "q1")]);
        adapter.update_query_context_cache(Some(&first), None).await;

        // id=1 updates timestamp; id=2 is inserted as new
        let second = resp_ctx(vec![
            make_entry_with_ctx(1, 4, 200, "q1-updated"),
            make_entry_with_ctx(2, 5, 300, "q2"),
        ]);
        adapter
            .update_query_context_cache(Some(&second), None)
            .await;

        let snap = adapter.get_query_context_snapshot().await;
        let entries = snap.entries.unwrap();
        let ids: Vec<i64> = entries.iter().map(|e| e.id).collect();
        assert!(ids.contains(&1), "id=1 should remain after merge");
        assert!(ids.contains(&2), "id=2 should be added via merge");
    }

    #[tokio::test]
    async fn test_adapter_null_entries_with_jdbc_presets_preserves_cache() {
        use crate::apis::database_driver_v1::WrapperPresets;

        let mut adapter = QueryContextCacheAdapter::new();
        adapter.init(None, &WrapperPresets::jdbc());

        let entry = make_entry(1, 10, 100);
        let seed = resp_ctx(vec![entry]);
        adapter.update_query_context_cache(Some(&seed), None).await;

        let null_ctx = RespCtx { entries: None };
        adapter
            .update_query_context_cache(Some(&null_ctx), None)
            .await;

        let snap = adapter.get_query_context_snapshot().await;
        let entries = snap
            .entries
            .expect("JDBC should preserve cache on null entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, 1);
    }

    #[tokio::test]
    async fn test_adapter_null_entries_with_python_presets_clears_cache() {
        use crate::apis::database_driver_v1::WrapperPresets;

        let mut adapter = QueryContextCacheAdapter::new();
        adapter.init(None, &WrapperPresets::python());

        let entry = make_entry(1, 10, 100);
        let seed = resp_ctx(vec![entry]);
        adapter.update_query_context_cache(Some(&seed), None).await;

        let null_ctx = RespCtx { entries: None };
        adapter
            .update_query_context_cache(Some(&null_ctx), None)
            .await;

        let snap = adapter.get_query_context_snapshot().await;
        assert!(
            snap.entries.is_none(),
            "Python/ODBC should clear cache on null entries"
        );
    }

    #[test]
    fn test_fill_to_capacity() {
        let mut cache = QueryContextCache::new();
        let entries: Vec<_> = (0..5i64)
            .map(|i| make_entry_with_ctx(i, i * 10, 1000 + i, &format!("query_ctx{i}")))
            .collect();
        let context = resp_ctx(entries);
        cache.update(Some(&context), true);

        assert_eq!(cache.entries.len(), 5);
        for i in 0..5i64 {
            let entry = cache
                .entries
                .get(&i)
                .unwrap_or_else(|| panic!("id={i} missing"));
            assert_eq!(entry.priority, i * 10);
            assert_eq!(entry.timestamp, 1000 + i);
            assert_eq!(
                entry.context.as_deref(),
                Some(format!("query_ctx{i}").as_str())
            );
        }
    }

    #[test]
    fn test_insertion_order_independence() {
        let mut cache = QueryContextCache::new();
        let shuffled = vec![
            make_entry(3, 30, 103),
            make_entry(1, 10, 101),
            make_entry(4, 40, 104),
            make_entry(0, 0, 100),
            make_entry(2, 20, 102),
        ];
        let context = resp_ctx(shuffled);
        cache.update(Some(&context), true);

        assert_eq!(cache.entries.len(), 5);
        for i in 0..5i64 {
            let entry = cache
                .entries
                .get(&i)
                .unwrap_or_else(|| panic!("id={i} missing"));
            assert_eq!(entry.priority, i * 10);
            assert_eq!(entry.timestamp, 100 + i);
        }
    }

    #[test]
    fn test_priority_change_reindexes_entry() {
        let mut cache = QueryContextCache::new();
        let seed = resp_ctx(vec![
            make_entry(1, 10, 100),
            make_entry(2, 20, 200),
            make_entry(3, 30, 300),
        ]);
        cache.update(Some(&seed), true);

        // Update id=2 with new priority (was 20, now 5)
        let update = resp_ctx(vec![make_entry(2, 5, 400)]);
        cache.update(Some(&update), true);

        let entry = cache.entries.get(&2).expect("id=2 missing");
        assert_eq!(entry.priority, 5);
        assert_eq!(entry.timestamp, 400);
        assert_eviction_order_synced(&cache, "priority_change");
    }

    #[test]
    fn test_serialize_clear_deserialize_round_trip() {
        let mut cache = QueryContextCache::new();
        let seed = resp_ctx(vec![
            make_entry_with_ctx(1, 10, 100, "ctx1"),
            make_entry_with_ctx(2, 20, 200, "ctx2"),
            make_entry_with_ctx(3, 30, 300, "ctx3"),
        ]);
        cache.update(Some(&seed), true);

        // Snapshot (serialize)
        let snapshot = cache.snapshot().expect("should have snapshot");
        let snapshot_entries = snapshot.entries.expect("should have entries");
        assert_eq!(snapshot_entries.len(), 3);

        // Clear
        let null_ctx = RespCtx { entries: None };
        cache.update(Some(&null_ctx), true);
        assert!(cache.entries.is_empty());

        // Re-populate from snapshot (simulating deserialization)
        let restored: Vec<RespEntry> = snapshot_entries
            .iter()
            .map(|e| RespEntry {
                id: e.id,
                priority: e.priority,
                timestamp: e.timestamp.unwrap_or(0),
                context: e
                    .context
                    .as_ref()
                    .map(|c| c.base64_data.clone().unwrap_or_default()),
            })
            .collect();
        let restore_ctx = resp_ctx(restored);
        cache.update(Some(&restore_ctx), true);

        assert_eq!(cache.entries.len(), 3);
        let entry = cache.entries.get(&1).expect("id=1 missing");
        assert_eq!(entry.priority, 10);
        assert_eq!(entry.timestamp, 100);
    }

    #[test]
    fn test_duplicate_priorities_coexist() {
        let mut cache = QueryContextCache::new();
        // Insert entries with the same priority — all coexist in the cache
        let entries = vec![
            make_entry(1, 5, 100),
            make_entry(2, 5, 200),
            make_entry(3, 5, 300),
        ];
        let context = resp_ctx(entries);
        cache.update(Some(&context), true);

        assert_eq!(
            cache.entries.len(),
            3,
            "all entries with same priority should coexist"
        );
        assert_eviction_order_synced(&cache, "duplicate_priorities");
    }

    #[test]
    fn test_duplicate_priorities_eviction_order() {
        let mut cache = QueryContextCache::new();
        cache.max_size = 3;
        // 4 entries at same priority — evicts lowest timestamp at highest priority
        let entries = vec![
            make_entry(1, 5, 100),
            make_entry(2, 5, 200),
            make_entry(3, 5, 300),
            make_entry(4, 5, 400),
        ];
        let context = resp_ctx(entries);
        cache.update(Some(&context), true);

        assert_eq!(cache.entries.len(), 3);
        assert!(
            !cache.entries.contains_key(&1),
            "id=1 should be evicted (lowest timestamp at same priority)"
        );
    }

    #[test]
    fn test_new_id_at_occupied_priority_evicts_oldest_by_capacity() {
        let mut cache = QueryContextCache::new();
        // Seed with 5 entries (at capacity), two at priority=10
        let seed = resp_ctx(vec![
            make_entry(1, 10, 100),
            make_entry(2, 10, 200),
            make_entry(30, 1, 300),
            make_entry(40, 2, 300),
            make_entry(50, 3, 300),
        ]);
        cache.update(Some(&seed), true);

        // id=30 is a no-op (same data). id=4 is new at priority=10 → inserted,
        // cache exceeds capacity → evicts id=1 (oldest timestamp at highest priority number=10).
        let update = resp_ctx(vec![make_entry(30, 1, 300), make_entry(4, 10, 400)]);
        cache.update(Some(&update), true);

        assert!(
            !cache.entries.contains_key(&1),
            "id=1 (oldest timestamp at priority=10) should be evicted by capacity"
        );
        assert!(cache.entries.contains_key(&2), "id=2 should remain");
        assert!(cache.entries.contains_key(&4), "id=4 should be inserted");
    }

    #[test]
    fn test_same_timestamp_priority_change() {
        let mut cache = QueryContextCache::new();
        let seed = resp_ctx(vec![make_entry(1, 10, 100)]);
        cache.update(Some(&seed), true);

        // Same timestamp, different priority → should re-index
        let update = resp_ctx(vec![make_entry(1, 5, 100)]);
        cache.update(Some(&update), true);

        let entry = cache.entries.get(&1).expect("id=1 missing");
        assert_eq!(entry.priority, 5, "priority should be updated");
        assert_eviction_order_synced(&cache, "same_ts_priority_change");
    }

    // -----------------------------------------------------------------------
    // Key collision tests: entries sharing both priority and timestamp
    // -----------------------------------------------------------------------

    #[test]
    fn test_collision_same_priority_and_timestamp_coexist() {
        let mut cache = QueryContextCache::new();
        let seed = resp_ctx(vec![make_entry(1, 5, 100), make_entry(2, 5, 100)]);
        cache.update(Some(&seed), true);

        assert_eq!(cache.entries.len(), 2);
        assert!(cache.entries.contains_key(&1));
        assert!(cache.entries.contains_key(&2));
        assert!(cache.eviction_order.contains(&1));
        assert!(cache.eviction_order.contains(&2));
        assert_eq!(cache.eviction_order.len(), 2);
    }

    #[test]
    fn test_collision_three_entries_same_key() {
        let mut cache = QueryContextCache::new();
        let seed = resp_ctx(vec![
            make_entry(1, 5, 100),
            make_entry(2, 5, 100),
            make_entry(3, 5, 100),
        ]);
        cache.update(Some(&seed), true);

        assert_eq!(cache.entries.len(), 3);
        assert!(cache.eviction_order.contains(&1));
        assert!(cache.eviction_order.contains(&2));
        assert!(cache.eviction_order.contains(&3));
        assert_eq!(cache.eviction_order.len(), 3, "single key for all three");
    }

    #[test]
    fn test_collision_eviction_removes_one_not_both() {
        let mut cache = QueryContextCache::new();
        cache.max_size = 2;
        let seed = resp_ctx(vec![
            make_entry(1, 5, 100),
            make_entry(2, 5, 100),
            make_entry(3, 1, 200),
        ]);
        cache.update(Some(&seed), true);

        assert_eq!(cache.entries.len(), 2);
        assert!(
            cache.entries.contains_key(&3),
            "lower priority entry should survive"
        );
        let id1_alive = cache.entries.contains_key(&1);
        let id2_alive = cache.entries.contains_key(&2);
        assert!(
            id1_alive ^ id2_alive,
            "exactly one of the colliding entries should survive, not both or neither"
        );
        assert_eq!(cache.eviction_order.len(), cache.entries.len());
    }

    #[test]
    fn test_collision_remove_entry_preserves_sibling() {
        let mut cache = QueryContextCache::new();
        let seed = resp_ctx(vec![make_entry(1, 5, 100), make_entry(2, 5, 100)]);
        cache.update(Some(&seed), true);

        cache.remove_entry(1);

        assert_eq!(cache.entries.len(), 1);
        assert!(!cache.entries.contains_key(&1));
        assert!(cache.entries.contains_key(&2));
        assert!(!cache.eviction_order.contains(&1));
        assert!(cache.eviction_order.contains(&2));
        assert_eq!(cache.eviction_order.len(), 1);
    }

    #[test]
    fn test_collision_remove_both_cleans_key() {
        let mut cache = QueryContextCache::new();
        let seed = resp_ctx(vec![make_entry(1, 5, 100), make_entry(2, 5, 100)]);
        cache.update(Some(&seed), true);

        cache.remove_entry(1);
        cache.remove_entry(2);

        assert!(cache.entries.is_empty());
        assert!(cache.eviction_order.is_empty());
    }

    #[test]
    fn test_update_creates_collision() {
        let mut cache = QueryContextCache::new();
        let seed = resp_ctx(vec![make_entry(1, 5, 100), make_entry(2, 5, 200)]);
        cache.update(Some(&seed), true);

        assert!(cache.eviction_order.contains(&1));

        let update = resp_ctx(vec![make_entry(1, 5, 200)]);
        cache.update(Some(&update), true);

        assert_eq!(cache.entries.len(), 2);
        assert!(cache.eviction_order.contains(&1));
        assert!(cache.eviction_order.contains(&2));
    }

    #[test]
    fn test_update_resolves_collision() {
        let mut cache = QueryContextCache::new();
        let seed = resp_ctx(vec![make_entry(1, 5, 100), make_entry(2, 5, 100)]);
        cache.update(Some(&seed), true);
        assert_eq!(cache.eviction_order.len(), 2, "two keys before split");

        let update = resp_ctx(vec![make_entry(1, 5, 200)]);
        cache.update(Some(&update), true);

        assert_eq!(cache.entries.len(), 2);
        assert_eq!(cache.eviction_order[0], 1);
        assert_eq!(cache.eviction_order[1], 2);
        assert_eq!(
            cache.eviction_order.len(),
            2,
            "two distinct keys after split"
        );
    }

    // -----------------------------------------------------------------------
    // Parametrized merge scenarios
    // -----------------------------------------------------------------------

    #[derive(Debug, Clone)]
    struct ExpectedEntry {
        id: i64,
        priority: i64,
        timestamp: i64,
        query_ctx: Option<String>,
    }

    fn seed_cache() -> QueryContextCache {
        let mut cache = QueryContextCache::new();
        let seed = resp_ctx(vec![
            make_entry_with_ctx(40000, 1, 50000, "C1"),
            make_entry_with_ctx(50000, 2, 50001, "C2"),
            make_entry_with_ctx(60000, 3, 50002, "C3"),
        ]);
        cache.update(Some(&seed), true);
        assert_eq!(cache.entries.len(), 3);
        cache.max_size = 3;
        cache
    }

    fn assert_cache_matches(cache: &QueryContextCache, expected: &[ExpectedEntry], label: &str) {
        assert_eq!(
            cache.entries.len(),
            expected.len(),
            "{label}: expected {} entries, got {}",
            expected.len(),
            cache.entries.len()
        );

        for exp in expected {
            let entry = cache
                .entries
                .get(&exp.id)
                .unwrap_or_else(|| panic!("{label}: id= {} missing from cache.", exp.id));
            assert_eq!(
                entry.priority, exp.priority,
                "{label}: id={} priority mismatch",
                exp.id
            );
            assert_eq!(
                entry.timestamp, exp.timestamp,
                "{label}: id={} timestamp mismatch",
                exp.id
            );
            assert_eq!(
                entry.context, exp.query_ctx,
                "{label}: id={} context mismatch",
                exp.id
            );
        }
    }

    fn e(id: i64, priority: i64, timestamp: i64, query_ctx: &str) -> ExpectedEntry {
        ExpectedEntry {
            id,
            priority,
            timestamp,
            query_ctx: Some(query_ctx.to_owned()),
        }
    }

    // Unchanged seed entries reused across cases
    fn seed_unchanged() -> [ExpectedEntry; 3] {
        [
            e(40000, 1, 50000, "C1"),
            e(50000, 2, 50001, "C2"),
            e(60000, 3, 50002, "C3"),
        ]
    }

    #[test]
    fn merge_known_id_newer_ts_same_pri_same_ctx() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 1, 60000, "C1")]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &[
                e(40000, 1, 60000, "C1"),
                e(50000, 2, 50001, "C2"),
                e(60000, 3, 50002, "C3"),
            ],
            "known_id_newer_ts_same_pri_same_ctx",
        );
    }

    #[test]
    fn merge_known_id_newer_ts_same_pri_diff_ctx() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 1, 60000, "C4")]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &[
                e(40000, 1, 60000, "C4"),
                e(50000, 2, 50001, "C2"),
                e(60000, 3, 50002, "C3"),
            ],
            "known_id_newer_ts_same_pri_diff_ctx",
        );
    }

    #[test]
    fn merge_known_id_newer_ts_higher_pri_same_ctx() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 2, 60000, "C1")]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &[
                e(40000, 2, 60000, "C1"),
                e(50000, 2, 50001, "C2"),
                e(60000, 3, 50002, "C3"),
            ],
            "known_id_newer_ts_higher_pri_same_ctx",
        );
    }

    #[test]
    fn merge_known_id_newer_ts_higher_pri_diff_ctx() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 2, 60000, "C4")]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &[
                e(40000, 2, 60000, "C4"),
                e(50000, 2, 50001, "C2"),
                e(60000, 3, 50002, "C3"),
            ],
            "known_id_newer_ts_higher_pri_diff_ctx",
        );
    }

    #[test]
    fn merge_known_id_newer_ts_lower_pri_same_ctx() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 0, 60000, "C1")]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &[
                e(40000, 0, 60000, "C1"),
                e(50000, 2, 50001, "C2"),
                e(60000, 3, 50002, "C3"),
            ],
            "known_id_newer_ts_lower_pri_same_ctx",
        );
    }

    #[test]
    fn merge_known_id_newer_ts_lower_pri_diff_ctx() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 0, 60000, "C4")]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &[
                e(40000, 0, 60000, "C4"),
                e(50000, 2, 50001, "C2"),
                e(60000, 3, 50002, "C3"),
            ],
            "known_id_newer_ts_lower_pri_diff_ctx",
        );
    }

    #[test]
    fn merge_known_id_equal_ts_same_pri_same_ctx_is_noop() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 1, 50000, "C1")]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &seed_unchanged(),
            "known_id_equal_ts_same_pri_same_ctx",
        );
    }

    #[test]
    fn merge_known_id_equal_ts_same_pri_updates_ctx() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 1, 50000, "C4")]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &[
                e(40000, 1, 50000, "C4"),
                e(50000, 2, 50001, "C2"),
                e(60000, 3, 50002, "C3"),
            ],
            "known_id_equal_ts_same_pri_diff_ctx",
        );
    }

    #[test]
    fn merge_known_id_equal_ts_higher_pri_reindexes_same_ctx() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 2, 50000, "C1")]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &[
                e(40000, 2, 50000, "C1"),
                e(50000, 2, 50001, "C2"),
                e(60000, 3, 50002, "C3"),
            ],
            "known_id_equal_ts_higher_pri_same_ctx",
        );
    }

    #[test]
    fn merge_known_id_equal_ts_higher_pri_reindexes_diff_ctx() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 2, 50000, "C4")]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &[
                e(40000, 2, 50000, "C4"),
                e(50000, 2, 50001, "C2"),
                e(60000, 3, 50002, "C3"),
            ],
            "known_id_equal_ts_higher_pri_diff_ctx",
        );
    }

    #[test]
    fn merge_known_id_equal_ts_lower_pri_reindexes_same_ctx() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 0, 50000, "C1")]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &[
                e(40000, 0, 50000, "C1"),
                e(50000, 2, 50001, "C2"),
                e(60000, 3, 50002, "C3"),
            ],
            "known_id_equal_ts_lower_pri_same_ctx",
        );
    }

    #[test]
    fn merge_known_id_equal_ts_lower_pri_reindexes_diff_ctx() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 0, 50000, "C4")]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &[
                e(40000, 0, 50000, "C4"),
                e(50000, 2, 50001, "C2"),
                e(60000, 3, 50002, "C3"),
            ],
            "known_id_equal_ts_lower_pri_diff_ctx",
        );
    }

    #[test]
    fn merge_known_id_stale_same_pri_same_ctx_is_noop() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 1, 40000, "C1")]);
        cache.update(Some(&update), true);
        assert_cache_matches(&cache, &seed_unchanged(), "stale_same_pri_same_ctx");
    }

    #[test]
    fn merge_known_id_stale_same_pri_diff_ctx_is_noop() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 1, 40000, "C4")]);
        cache.update(Some(&update), true);
        assert_cache_matches(&cache, &seed_unchanged(), "stale_same_pri_diff_ctx");
    }

    #[test]
    fn merge_known_id_stale_higher_pri_same_ctx_is_noop() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 2, 40000, "C1")]);
        cache.update(Some(&update), true);
        assert_cache_matches(&cache, &seed_unchanged(), "stale_higher_pri_same_ctx");
    }

    #[test]
    fn merge_known_id_stale_higher_pri_diff_ctx_is_noop() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 2, 40000, "C4")]);
        cache.update(Some(&update), true);
        assert_cache_matches(&cache, &seed_unchanged(), "stale_higher_pri_diff_ctx");
    }

    #[test]
    fn merge_known_id_stale_lower_pri_same_ctx_is_noop() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 0, 40000, "C1")]);
        cache.update(Some(&update), true);
        assert_cache_matches(&cache, &seed_unchanged(), "stale_lower_pri_same_ctx");
    }

    #[test]
    fn merge_known_id_stale_lower_pri_diff_ctx_is_noop() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(40000, 0, 40000, "C4")]);
        cache.update(Some(&update), true);
        assert_cache_matches(&cache, &seed_unchanged(), "stale_lower_pri_diff_ctx");
    }

    #[test]
    fn merge_unknown_id_at_occupied_pri_evicts_highest_pri_same_ctx() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(1, 1, 60000, "C1")]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &[
                e(1, 1, 60000, "C1"),
                e(50000, 2, 50001, "C2"),
                e(40000, 1, 50000, "C1"), // BD: old drivers would displace the incumbent at this priority instead of evicting by capacity
            ],
            "unknown_id_occupied_pri_same_ctx",
        );
    }

    #[test]
    fn merge_unknown_id_at_occupied_pri_evicts_highest_pri_diff_ctx() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(1, 1, 60000, "C4")]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &[
                e(1, 1, 60000, "C4"),
                e(50000, 2, 50001, "C2"),
                e(40000, 1, 50000, "C1"), // BD: old drivers would displace the incumbent at this priority instead of evicting by capacity
            ],
            "unknown_id_occupied_pri_diff_ctx",
        );
    }

    #[test]
    fn merge_unknown_id_at_different_occupied_pri_evicts_highest_pri() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(1, 2, 60000, "C2")]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &[
                e(40000, 1, 50000, "C1"),
                e(1, 2, 60000, "C2"),
                e(40000, 1, 50000, "C1"), // BD: old drivers would displace the incumbent at this priority instead of evicting by capacity
            ],
            "unknown_id_different_occupied_pri",
        );
    }

    #[test]
    fn merge_unknown_id_at_different_occupied_pri_evicts_highest_pri_diff_ctx() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(1, 2, 60000, "C4")]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &[
                e(40000, 1, 50000, "C1"),
                e(1, 2, 60000, "C4"),
                e(40000, 1, 50000, "C1"), // BD: old drivers would displace the incumbent at this priority instead of evicting by capacity
            ],
            "unknown_id_different_occupied_pri_diff_ctx",
        );
    }

    #[test]
    fn merge_unknown_id_at_new_pri_evicts_highest_pri() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![make_entry_with_ctx(1, 0, 60000, "C4")]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &[
                e(40000, 1, 50000, "C1"),
                e(50000, 2, 50001, "C2"),
                e(1, 0, 60000, "C4"),
            ],
            "unknown_id_new_pri_evicts_highest",
        );
    }

    #[test]
    fn merge_two_unknown_ids_at_same_pri_evicts_highest_pri() {
        let mut cache = seed_cache();
        let update = resp_ctx(vec![
            make_entry_with_ctx(1, 2, 60000, "C4"),
            make_entry_with_ctx(2, 2, 60000, "C5"),
        ]);
        cache.update(Some(&update), true);
        assert_cache_matches(
            &cache,
            &[
                e(40000, 1, 50000, "C1"),
                e(2, 2, 60000, "C5"),
                e(1, 2, 60000, "C4"), // BD: old drivers would displace the incumbent at this priority instead of evicting by capacity
            ],
            "two_unknown_ids_same_pri_evicts_highest",
        );
    }
}
