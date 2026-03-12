use std::sync::{LazyLock, Mutex};

use super::connection::Connection;
use super::database::Database;
use super::statement::Statement;
use crate::handle_manager::HandleManager;

pub struct DatabaseDriverV1 {
    pub(super) databases: HandleManager<Mutex<Database>>,
    pub(super) connections: HandleManager<Mutex<Connection>>,
    pub(super) statements: HandleManager<Mutex<Statement>>,
}

static INSTANCE: LazyLock<DatabaseDriverV1> = LazyLock::new(DatabaseDriverV1::new);

pub fn driver_state() -> &'static DatabaseDriverV1 {
    &INSTANCE
}

impl DatabaseDriverV1 {
    const fn new() -> Self {
        Self {
            databases: HandleManager::new(),
            connections: HandleManager::new(),
            statements: HandleManager::new(),
        }
    }
}
