use super::{connection::Connection, database::Database, statement::Statement};
use crate::handle_manager::HandleManager;
use std::sync::{LazyLock, Mutex};

pub static DB_HANDLE_MANAGER: LazyLock<HandleManager<Mutex<Database>>> =
    LazyLock::new(HandleManager::new);
pub static CONN_HANDLE_MANAGER: LazyLock<HandleManager<Mutex<Connection>>> =
    LazyLock::new(HandleManager::new);
pub static STMT_HANDLE_MANAGER: LazyLock<HandleManager<Mutex<Statement>>> =
    LazyLock::new(HandleManager::new);
