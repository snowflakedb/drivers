use super::error::ApiError;
use super::global_state::DB_HANDLE_MANAGER;
use super::{Handle, Setting};
use std::sync::Mutex;

pub fn database_new() -> Handle {
    DB_HANDLE_MANAGER.add_handle(Mutex::new(Database::new()))
}

#[allow(clippy::result_large_err)]
pub fn database_set_option(db_handle: Handle, key: String, value: Setting) -> Result<(), ApiError> {
    match DB_HANDLE_MANAGER.get_obj(db_handle) {
        Some(db_ptr) => {
            let mut db = db_ptr.lock().unwrap();
            db.settings.insert(key, value);
            Ok(())
        }
        None => Err(ApiError::InvalidArgument {
            argument: "Database handle not found".to_string(),
            location: snafu::location!(),
        }),
    }
}

#[allow(clippy::result_large_err)]
pub fn database_init(_db_handle: Handle) -> Result<(), ApiError> {
    Ok(())
}

#[allow(clippy::result_large_err)]
pub fn database_release(db_handle: Handle) -> Result<(), ApiError> {
    match DB_HANDLE_MANAGER.delete_handle(db_handle) {
        true => Ok(()),
        false => Err(ApiError::InvalidArgument {
            argument: "Failed to release database handle".to_string(),
            location: snafu::location!(),
        }),
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
