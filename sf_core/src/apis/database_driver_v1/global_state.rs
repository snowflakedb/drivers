use tokio::sync::Mutex;

use super::connection::Connection;
use super::database::Database;
use super::statement::Statement;
use crate::handle_manager::HandleManager;

#[derive(Default)]
pub struct DatabaseDriverV1 {
    pub(super) databases: HandleManager<Mutex<Database>>,
    pub(super) connections: HandleManager<Mutex<Connection>>,
    pub(super) statements: HandleManager<Mutex<Statement>>,
}

impl DatabaseDriverV1 {
    pub const fn new() -> Self {
        Self {
            databases: HandleManager::new(),
            connections: HandleManager::new(),
            statements: HandleManager::new(),
        }
    }
}
