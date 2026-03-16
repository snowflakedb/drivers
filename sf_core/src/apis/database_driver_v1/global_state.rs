use tokio::sync::{Mutex, OnceCell};

use super::connection::Connection;
use super::database::Database;
use super::statement::Statement;
use crate::handle_manager::HandleManager;
use crate::token_cache::{KeyringTokenCache, TokenCacheError};

#[derive(Default)]
pub struct DatabaseDriverV1 {
    pub(super) databases: HandleManager<Mutex<Database>>,
    pub(super) connections: HandleManager<Mutex<Connection>>,
    pub(super) statements: HandleManager<Mutex<Statement>>,
    token_cache: OnceCell<KeyringTokenCache>,
}

impl DatabaseDriverV1 {
    pub const fn new() -> Self {
        Self {
            databases: HandleManager::new(),
            connections: HandleManager::new(),
            statements: HandleManager::new(),
            token_cache: OnceCell::new(),
        }
    }

    pub fn token_cache(&self) -> Result<&KeyringTokenCache, TokenCacheError> {
        if let Some(cache) = self.token_cache.get() {
            return Ok(cache);
        }
        let cache = KeyringTokenCache::new()?;
        let _ = self.token_cache.set(cache);
        Ok(self.token_cache.get().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn driver_state() -> &'static DatabaseDriverV1 {
        &DatabaseDriverV1::new()
    }

    #[test]
    fn token_cache_lazy_init_succeeds() {
        let result = driver_state().token_cache();
        assert!(
            result.is_ok(),
            "token_cache() should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn token_cache_returns_same_instance() {
        let first = driver_state().token_cache().expect("first call failed");
        let second = driver_state().token_cache().expect("second call failed");
        assert!(
            std::ptr::eq(first, second),
            "token_cache() should return the same instance on repeated calls"
        );
    }

    #[test]
    fn driver_state_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DatabaseDriverV1>();
    }
}
