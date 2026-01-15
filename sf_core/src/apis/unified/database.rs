//! Database handle management for the unified driver API.

use super::error::*;
use super::global_state::DB_HANDLE_MANAGER;
use crate::config::settings::Setting;
use crate::handle_manager::Handle;
use std::collections::HashMap;
use std::sync::Mutex;

/// Database state container.
pub struct Database {
    pub settings: HashMap<String, Setting>,
}

impl Database {
    pub fn new() -> Self {
        Database {
            settings: HashMap::new(),
        }
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new database handle.
pub fn database_new() -> Handle {
    DB_HANDLE_MANAGER.add_handle(Mutex::new(Database::new()))
}

/// Release a database handle.
pub fn database_release(db_handle: Handle) -> Result<(), ApiError> {
    match DB_HANDLE_MANAGER.delete_handle(db_handle) {
        true => Ok(()),
        false => InvalidArgumentSnafu {
            argument: "Failed to release database handle".to_string(),
        }
        .fail(),
    }
}

/// Set an option on a database handle.
pub fn database_set_option(handle: Handle, key: String, value: Setting) -> Result<(), ApiError> {
    match DB_HANDLE_MANAGER.get_obj(handle) {
        Some(db_ptr) => {
            let mut db = db_ptr.lock().map_err(|_| DatabaseLockingSnafu {}.build())?;
            db.settings.insert(key, value);
            Ok(())
        }
        None => InvalidArgumentSnafu {
            argument: "Database handle not found".to_string(),
        }
        .fail(),
    }
}

/// Initialize a database handle.
pub fn database_init(_db_handle: Handle) -> Result<(), ApiError> {
    // No-op for now, as database initialization logic is minimal
    Ok(())
}
