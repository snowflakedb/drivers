use std::sync::Mutex;

use super::Handle;
use super::Setting;
use super::error::*;
use super::global_state::DB_HANDLE_MANAGER;

pub fn database_new() -> Handle {
    DB_HANDLE_MANAGER.add_handle(Mutex::new(Database::new()))
}

pub fn database_set_option(db_handle: Handle, key: String, value: Setting) -> Result<(), ApiError> {
    let handle = db_handle;
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

pub fn database_init(db_handle: Handle) -> Result<(), ApiError> {
    let handle = db_handle;
    match DB_HANDLE_MANAGER.get_obj(handle) {
        Some(_db_ptr) => Ok(()),
        None => InvalidArgumentSnafu {
            argument: "Database handle not found".to_string(),
        }
        .fail(),
    }
}

pub fn database_release(db_handle: Handle) -> Result<(), ApiError> {
    match DB_HANDLE_MANAGER.delete_handle(db_handle) {
        true => Ok(()),
        false => InvalidArgumentSnafu {
            argument: "Failed to release database handle".to_string(),
        }
        .fail(),
    }
}

use std::collections::HashMap;

pub struct Database {
    pub settings: HashMap<String, Setting>,
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

impl Database {
    pub fn new() -> Self {
        Database {
            settings: HashMap::new(),
        }
    }
}
