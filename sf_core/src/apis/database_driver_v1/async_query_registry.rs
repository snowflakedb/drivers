//! Async query registry for tracking running async queries
//!
//! Used by auto-detection logic to determine if logout should be skipped
//! to preserve running async queries (Fire & Forget semantics).

use snafu::{Location, Snafu};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// Error type for AsyncQueryRegistry operations
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum AsyncQueryRegistryError {
    #[snafu(display("Failed to acquire registry lock: mutex poisoned"))]
    RegistryLockPoisoned {
        #[snafu(implicit)]
        location: Location,
    },
}

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
    ///
    /// # Errors
    ///
    /// Returns `AsyncQueryRegistryError::RegistryLockPoisoned` if the mutex is poisoned
    pub fn register(&self, query_id: String) -> Result<(), AsyncQueryRegistryError> {
        let mut queries = self
            .queries
            .lock()
            .map_err(|_| RegistryLockPoisonedSnafu.build())?;
        tracing::debug!(query_id = %query_id, "Registered async query");
        queries.insert(query_id);
        Ok(())
    }

    /// Unregister an async query ID
    ///
    /// Called when a query completes or is cancelled
    ///
    /// # Errors
    ///
    /// Returns `AsyncQueryRegistryError::RegistryLockPoisoned` if the mutex is poisoned
    pub fn unregister(&self, query_id: &str) -> Result<(), AsyncQueryRegistryError> {
        let mut queries = self
            .queries
            .lock()
            .map_err(|_| RegistryLockPoisonedSnafu.build())?;
        let removed = queries.remove(query_id);
        if removed {
            tracing::debug!(query_id, "Unregistered async query");
        } else {
            tracing::warn!(query_id, "Attempted to unregister unknown async query");
        }
        Ok(())
    }

    /// Check if there are any running async queries
    ///
    /// Returns true immediately upon finding the first running query (early return optimization).
    /// This is more efficient than checking all queries when we only need to know if ANY exist.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - At least one async query is registered (still running)
    /// * `Ok(false)` - No async queries registered (all finished or none started)
    ///
    /// # Errors
    ///
    /// Returns `AsyncQueryRegistryError::RegistryLockPoisoned` if the mutex is poisoned
    pub fn has_running_queries(&self) -> Result<bool, AsyncQueryRegistryError> {
        let queries = self
            .queries
            .lock()
            .map_err(|_| RegistryLockPoisonedSnafu.build())?;
        let has_queries = !queries.is_empty();

        if has_queries {
            tracing::debug!(
                count = queries.len(),
                "Auto-detection found running async queries"
            );
        } else {
            tracing::debug!("Auto-detection found no running async queries");
        }

        Ok(has_queries)
    }

    /// Get count of registered queries (for testing/debugging)
    ///
    /// # Errors
    ///
    /// Returns `AsyncQueryRegistryError::RegistryLockPoisoned` if the mutex is poisoned
    pub fn count(&self) -> Result<usize, AsyncQueryRegistryError> {
        let queries = self
            .queries
            .lock()
            .map_err(|_| RegistryLockPoisonedSnafu.build())?;
        Ok(queries.len())
    }

    /// Clear all registered queries (for testing)
    ///
    /// # Errors
    ///
    /// Returns `AsyncQueryRegistryError::RegistryLockPoisoned` if the mutex is poisoned
    #[cfg(test)]
    pub fn clear(&self) -> Result<(), AsyncQueryRegistryError> {
        let mut queries = self
            .queries
            .lock()
            .map_err(|_| RegistryLockPoisonedSnafu.build())?;
        queries.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_has_running() {
        let registry = AsyncQueryRegistry::new();
        assert!(
            !registry.has_running_queries().unwrap(),
            "Should start empty"
        );

        registry.register("query1".to_string()).unwrap();
        assert!(
            registry.has_running_queries().unwrap(),
            "Should have running queries after register"
        );
        assert_eq!(registry.count().unwrap(), 1);
    }

    #[test]
    fn test_unregister() {
        let registry = AsyncQueryRegistry::new();
        registry.register("query1".to_string()).unwrap();
        registry.register("query2".to_string()).unwrap();
        assert_eq!(registry.count().unwrap(), 2);

        registry.unregister("query1").unwrap();
        assert_eq!(registry.count().unwrap(), 1);
        assert!(
            registry.has_running_queries().unwrap(),
            "Should still have query2"
        );

        registry.unregister("query2").unwrap();
        assert_eq!(registry.count().unwrap(), 0);
        assert!(!registry.has_running_queries().unwrap(), "Should be empty");
    }

    #[test]
    fn test_unregister_unknown() {
        let registry = AsyncQueryRegistry::new();
        registry.unregister("nonexistent").unwrap(); // Should not panic
        assert_eq!(registry.count().unwrap(), 0);
    }

    #[test]
    fn test_early_return_optimization() {
        let registry = AsyncQueryRegistry::new();
        registry.register("query1".to_string()).unwrap();
        registry.register("query2".to_string()).unwrap();
        registry.register("query3".to_string()).unwrap();

        // has_running_queries should return true immediately without checking all
        // (We can't directly test early return, but we verify the behavior)
        assert!(registry.has_running_queries().unwrap());
    }

    #[test]
    fn test_multiple_registers_same_id() {
        let registry = AsyncQueryRegistry::new();
        registry.register("query1".to_string()).unwrap();
        registry.register("query1".to_string()).unwrap(); // Duplicate

        // HashSet deduplicates
        assert_eq!(registry.count().unwrap(), 1);
    }
}
