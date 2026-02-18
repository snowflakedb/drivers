//! Async query registry for tracking running async queries
//!
//! Used by auto-detection logic to determine if logout should be skipped
//! to preserve running async queries (Fire & Forget semantics).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// Registry for tracking active async query IDs
#[derive(Debug, Clone)]
pub struct AsyncQueryRegistry {
    queries: Arc<Mutex<HashSet<String>>>,
}

impl Default for AsyncQueryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncQueryRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            queries: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Register an async query ID
    ///
    /// Called when a query is executed with asyncExec=true
    pub fn register(&self, query_id: String) {
        let mut queries = self
            .queries
            .lock()
            .expect("AsyncQueryRegistry mutex poisoned - cannot register query");
        queries.insert(query_id.clone());
        tracing::debug!(query_id, "Registered async query");
    }

    /// Unregister an async query ID
    ///
    /// Called when a query completes or is cancelled
    pub fn unregister(&self, query_id: &str) {
        let mut queries = self
            .queries
            .lock()
            .expect("AsyncQueryRegistry mutex poisoned - cannot unregister query");
        let removed = queries.remove(query_id);
        if removed {
            tracing::debug!(query_id, "Unregistered async query");
        } else {
            tracing::warn!(query_id, "Attempted to unregister unknown async query");
        }
    }

    /// Check if there are any running async queries
    ///
    /// Returns true immediately upon finding the first running query (early return optimization).
    /// This is more efficient than checking all queries when we only need to know if ANY exist.
    ///
    /// # Returns
    ///
    /// * `true` - At least one async query is registered (still running)
    /// * `false` - No async queries registered (all finished or none started)
    pub fn has_running_queries(&self) -> bool {
        let queries = self
            .queries
            .lock()
            .expect("AsyncQueryRegistry mutex poisoned - cannot check running queries");
        let has_queries = !queries.is_empty();

        if has_queries {
            tracing::debug!(
                count = queries.len(),
                "Auto-detection found running async queries"
            );
        } else {
            tracing::debug!("Auto-detection found no running async queries");
        }

        has_queries
    }

    /// Get count of registered queries (for testing/debugging)
    pub fn count(&self) -> usize {
        let queries = self
            .queries
            .lock()
            .expect("AsyncQueryRegistry mutex poisoned - cannot get count");
        queries.len()
    }

    /// Clear all registered queries (for testing)
    #[cfg(test)]
    pub fn clear(&self) {
        let mut queries = self
            .queries
            .lock()
            .expect("AsyncQueryRegistry mutex poisoned - cannot clear");
        queries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_has_running() {
        let registry = AsyncQueryRegistry::new();
        assert!(!registry.has_running_queries(), "Should start empty");

        registry.register("query1".to_string());
        assert!(
            registry.has_running_queries(),
            "Should have running queries after register"
        );
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_unregister() {
        let registry = AsyncQueryRegistry::new();
        registry.register("query1".to_string());
        registry.register("query2".to_string());
        assert_eq!(registry.count(), 2);

        registry.unregister("query1");
        assert_eq!(registry.count(), 1);
        assert!(registry.has_running_queries(), "Should still have query2");

        registry.unregister("query2");
        assert_eq!(registry.count(), 0);
        assert!(!registry.has_running_queries(), "Should be empty");
    }

    #[test]
    fn test_unregister_unknown() {
        let registry = AsyncQueryRegistry::new();
        registry.unregister("nonexistent"); // Should not panic
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_early_return_optimization() {
        let registry = AsyncQueryRegistry::new();
        registry.register("query1".to_string());
        registry.register("query2".to_string());
        registry.register("query3".to_string());

        // has_running_queries should return true immediately without checking all
        // (We can't directly test early return, but we verify the behavior)
        assert!(registry.has_running_queries());
    }

    #[test]
    fn test_multiple_registers_same_id() {
        let registry = AsyncQueryRegistry::new();
        registry.register("query1".to_string());
        registry.register("query1".to_string()); // Duplicate

        // HashSet deduplicates
        assert_eq!(registry.count(), 1);
    }
}
