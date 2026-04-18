use tokio::sync::Mutex;

use super::connection::Connection;
use super::database::Database;
use super::disk_reader::{DiskReader, RealDiskReader};
use super::statement::Statement;
use crate::handle_manager::HandleManager;
use crate::token_cache::{KeyringTokenCache, TokenCacheError};

#[derive(Default)]
pub struct DatabaseDriverV1 {
    pub(super) databases: HandleManager<Mutex<Database>>,
    pub(super) connections: HandleManager<Mutex<Connection>>,
    pub(super) statements: HandleManager<Mutex<Statement>>,
    token_cache: once_cell::sync::OnceCell<KeyringTokenCache>,
    disk_reader: once_cell::sync::OnceCell<Box<dyn DiskReader>>,
}

impl DatabaseDriverV1 {
    pub const fn new() -> Self {
        Self {
            databases: HandleManager::new(),
            connections: HandleManager::new(),
            statements: HandleManager::new(),
            token_cache: once_cell::sync::OnceCell::new(),
            disk_reader: once_cell::sync::OnceCell::new(),
        }
    }

    pub fn token_cache(&self) -> Result<&KeyringTokenCache, TokenCacheError> {
        self.token_cache.get_or_try_init(KeyringTokenCache::new)
    }

    pub fn disk_reader(&self) -> &dyn DiskReader {
        self.disk_reader
            .get_or_init(|| Box::new(RealDiskReader))
            .as_ref()
    }

    #[cfg(test)]
    pub fn set_disk_reader(&self, reader: Box<dyn DiskReader>) {
        if self.disk_reader.set(reader).is_err() {
            panic!("set_disk_reader called after disk_reader() was already initialized");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::disk_reader::MockDiskReader;
    use super::*;
    use std::path::Path;

    static DRIVER_STATE: DatabaseDriverV1 = DatabaseDriverV1::new();

    #[test]
    fn token_cache_lazy_init_succeeds() {
        let result = DRIVER_STATE.token_cache();
        assert!(
            result.is_ok(),
            "token_cache() should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn token_cache_returns_same_instance() {
        let first = DRIVER_STATE.token_cache().expect("first call failed");
        let second = DRIVER_STATE.token_cache().expect("second call failed");
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

    #[test]
    fn disk_reader_defaults_to_real_reader() {
        let driver = DatabaseDriverV1::new();
        let err = driver
            .disk_reader()
            .read_to_string(Path::new(
                "/this/path/should/not/exist/for/universal_driver/tests",
            ))
            .expect_err("real reader should fail on a nonexistent path");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn set_disk_reader_injects_mock() {
        let driver = DatabaseDriverV1::new();
        driver.set_disk_reader(Box::new(
            MockDiskReader::new().with_file("/etc/hosts", "127.0.0.1 localhost\n"),
        ));

        let contents = driver
            .disk_reader()
            .read_to_string(Path::new("/etc/hosts"))
            .expect("mock should return canned /etc/hosts");
        assert_eq!(contents, "127.0.0.1 localhost\n");
    }
}
