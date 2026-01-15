//! Global state management for the unified driver API.

use crate::handle_manager::HandleManager;
use lazy_static::lazy_static;
use std::sync::Mutex;

use super::connection::Connection;
use super::database::Database;
use super::statement::Statement;

lazy_static! {
    pub static ref DB_HANDLE_MANAGER: HandleManager<Mutex<Database>> = HandleManager::new();
    pub static ref CONN_HANDLE_MANAGER: HandleManager<Mutex<Connection>> = HandleManager::new();
    pub static ref STMT_HANDLE_MANAGER: HandleManager<Mutex<Statement>> = HandleManager::new();
}
