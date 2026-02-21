use std::collections::HashMap;
use std::sync::Mutex;

use super::error::*;
use super::global_state::DatabaseDriverV1;
use super::validation::{ValidationIssue, resolve_and_apply_options};
use crate::config::ParamStore;
use crate::config::settings::Setting;
use crate::handle_manager::Handle;

impl DatabaseDriverV1 {
    pub fn database_new(&self) -> Handle {
        self.databases.add_handle(Mutex::new(Database::new()))
    }

    pub fn database_set_option(
        &self,
        db_handle: Handle,
        key: String,
        value: Setting,
    ) -> Result<(), ApiError> {
        match self.databases.get_obj(db_handle) {
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

    pub fn database_set_options(
        &self,
        db_handle: Handle,
        options: HashMap<String, Setting>,
    ) -> Result<Vec<ValidationIssue>, ApiError> {
        match self.databases.get_obj(db_handle) {
            Some(db_ptr) => {
                let mut db = db_ptr.lock().map_err(|_| DatabaseLockingSnafu {}.build())?;
                resolve_and_apply_options(&mut db.settings, options)
            }
            None => InvalidArgumentSnafu {
                argument: "Database handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub fn database_init(&self, db_handle: Handle) -> Result<(), ApiError> {
        match self.databases.get_obj(db_handle) {
            Some(_db_ptr) => Ok(()),
            None => InvalidArgumentSnafu {
                argument: "Database handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub fn database_release(&self, db_handle: Handle) -> Result<(), ApiError> {
        match self.databases.delete_handle(db_handle) {
            true => Ok(()),
            false => InvalidArgumentSnafu {
                argument: "Failed to release database handle".to_string(),
            }
            .fail(),
        }
    }
}

pub struct Database {
    pub(crate) settings: ParamStore,
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

impl Database {
    pub fn new() -> Self {
        Database {
            settings: ParamStore::new(),
        }
    }
}
