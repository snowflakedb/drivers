use tokio::sync::Mutex;

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
    token_cache: once_cell::sync::OnceCell<KeyringTokenCache>,
}

impl DatabaseDriverV1 {
    pub const fn new() -> Self {
        Self {
            databases: HandleManager::new(),
            connections: HandleManager::new(),
            statements: HandleManager::new(),
            token_cache: once_cell::sync::OnceCell::new(),
        }
    }

    pub fn token_cache(&self) -> Result<&KeyringTokenCache, TokenCacheError> {
        self.token_cache.get_or_try_init(KeyringTokenCache::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn driver_state() -> &'static DatabaseDriverV1 {
        thread_local! {
            static INSTANCE: DatabaseDriverV1 = const { DatabaseDriverV1::new() };
        }
        // SAFETY: thread_local storage lives for the thread's lifetime, which
        // outlives every test function running on that thread.
        INSTANCE.with(|s| unsafe { &*(s as *const DatabaseDriverV1) })
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
