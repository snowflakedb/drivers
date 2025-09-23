use crate::api::{
    Connection, ConnectionState, Environment, OdbcResult, Statement, StatementState,
    conn_from_handle,
    diagnostic::DiagnosticInfo,
    error::{DisconnectedSnafu, InvalidHandleSnafu, Required},
};
use odbc_sys as sql;
use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf_gen::database_driver_v1::StatementNewRequest;
use tracing;

/// Allocate a new environment handle
pub fn alloc_environment() -> OdbcResult<*mut Environment> {
    tracing::info!("Allocating new environment handle");
    let env = Box::new(Environment {
        odbc_version: 3,
        diagnostic_info: DiagnosticInfo::default(),
    });
    Ok(Box::into_raw(env))
}

/// Allocate a new connection handle
pub fn alloc_connection() -> OdbcResult<*mut Connection> {
    tracing::info!("Allocating new connection handle");
    let dbc = Box::new(Connection {
        state: ConnectionState::Disconnected,
        diagnostic_info: DiagnosticInfo::default(),
    });
    Ok(Box::into_raw(dbc))
}

/// Allocate a new statement handle
pub fn alloc_statement(input_handle: sql::Handle) -> OdbcResult<*mut Statement<'static>> {
    tracing::info!("Allocating new statement handle");
    let conn = conn_from_handle(input_handle);
    match &mut conn.state {
        ConnectionState::Connected {
            db_handle: _,
            conn_handle,
        } => {
            let response = DatabaseDriverClient::statement_new(StatementNewRequest {
                conn_handle: Some(*conn_handle),
            })?;
            let stmt_handle = response
                .stmt_handle
                .required("Statement handle is required")?;

            let stmt = Box::new(Statement {
                conn,
                stmt_handle,
                state: StatementState::Created.into(),
                parameter_bindings: std::collections::HashMap::new(),
                diagnostic_info: DiagnosticInfo::default(),
            });
            Ok(Box::into_raw(stmt))
        }
        ConnectionState::Disconnected => {
            tracing::error!("Cannot allocate statement: connection is disconnected");
            DisconnectedSnafu.fail()
        }
    }
}

/// Free an environment handle
pub fn free_environment(handle: sql::Handle) -> OdbcResult<()> {
    if handle.is_null() {
        return InvalidHandleSnafu.fail();
    }

    tracing::info!("Freeing environment handle");
    unsafe {
        drop(Box::from_raw(handle as *mut Environment));
    }
    Ok(())
}

/// Free a connection handle
pub fn free_connection(handle: sql::Handle) -> OdbcResult<()> {
    if handle.is_null() {
        return InvalidHandleSnafu.fail();
    }

    tracing::info!("Freeing connection handle");
    unsafe {
        drop(Box::from_raw(handle as *mut Connection));
    }
    Ok(())
}

/// Free a statement handle
pub fn free_statement(handle: sql::Handle) -> OdbcResult<()> {
    if handle.is_null() {
        return InvalidHandleSnafu.fail();
    }

    tracing::info!("Freeing statement handle");
    unsafe {
        drop(Box::from_raw(handle as *mut Statement));
    }
    Ok(())
}

/// Initialize logging (helper function for allocation)
pub fn init_logging() {
    use lazy_static::lazy_static;

    lazy_static! {
        // TODO: This is a hack to initialize the logging system.
        // We should find a better way to do this.
        static ref LOGGING_RESULT: Result<(), sf_core::logging::LogError> = sf_core::logging::init(sf_core::logging::LoggingConfig::new(None, true, false));
    }

    if let Err(e) = LOGGING_RESULT.as_ref() {
        eprintln!("Failed to initialize logging: {e:?}");
    }
}

/// Allocate handle implementation (moved from api.rs)
pub fn sql_alloc_handle(
    handle_type: sql::HandleType,
    input_handle: sql::Handle,
    output_handle: *mut sql::Handle,
) -> OdbcResult<()> {
    init_logging();
    tracing::debug!("SQLAllocHandle: handle_type={:?}", handle_type);

    match handle_type {
        sql::HandleType::Env => {
            tracing::info!(
                "Allocating new env: SQLAllocHandle: handle_type={:?}",
                handle_type
            );
            let handle = alloc_environment()?;
            unsafe { std::ptr::write(output_handle, handle as sql::Handle) };
            Ok(())
        }
        sql::HandleType::Dbc => {
            tracing::info!(
                "Allocating new dbc: SQLAllocHandle: handle_type={:?}",
                handle_type
            );
            let handle = alloc_connection()?;
            unsafe { *output_handle = handle as sql::Handle };
            Ok(())
        }
        sql::HandleType::Stmt => {
            tracing::info!(
                "Allocating new stmt: SQLAllocHandle: handle_type={:?}",
                handle_type
            );
            let handle = alloc_statement(input_handle)?;
            unsafe { std::ptr::write(output_handle, handle as sql::Handle) };
            Ok(())
        }
        sql::HandleType::Desc => {
            // Not implemented yet
            tracing::warn!(
                "SQLAllocHandle: Desc handle type not implemented: {:?}",
                handle_type
            );
            InvalidHandleSnafu.fail()
        }
        _ => {
            tracing::error!("SQLAllocHandle: unknown handle type: {:?}", handle_type);
            InvalidHandleSnafu.fail()
        }
    }
}

/// Free handle implementation (moved from api.rs)
pub fn sql_free_handle(handle_type: sql::HandleType, handle: sql::Handle) -> OdbcResult<()> {
    if handle.is_null() {
        return InvalidHandleSnafu.fail();
    }

    match handle_type {
        sql::HandleType::Env => {
            tracing::info!("Freeing env: SQLFreeHandle: handle_type={:?}", handle_type);
            free_environment(handle)
        }
        sql::HandleType::Dbc => {
            tracing::info!("Freeing dbc: SQLFreeHandle: handle_type={:?}", handle_type);
            free_connection(handle)
        }
        sql::HandleType::Stmt => {
            tracing::info!("Freeing stmt: SQLFreeHandle: handle_type={:?}", handle_type);
            free_statement(handle)
        }
        sql::HandleType::Desc => {
            // Not implemented yet
            InvalidHandleSnafu.fail()
        }
        _ => InvalidHandleSnafu.fail(),
    }
}
