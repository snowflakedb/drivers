use std::sync::{Arc, Mutex};

use crate::api::error::{DisconnectedSnafu, InvalidHandleSnafu, OdbcRuntimeSnafu, Required};
use crate::api::handle::{
    ODBC_MAGIC_ALIVE, ODBC_MAGIC_DEAD, OdbcHandle, OdbcHandleWrapper, wrapper_from_handle,
};
use crate::api::{
    Connection, ConnectionState, Environment, OdbcResult, Statement,
    diagnostic::DiagnosticInfo,
    runtime::{env_allocated, env_freed, global},
};
use odbc_sys as sql;
use sf_core::protobuf::generated::database_driver_v1::{
    StatementNewRequest, StatementReleaseRequest,
};
use snafu::ResultExt;
use tracing;

/// Allocate a new environment handle, returning the opaque `sql::Handle`.
pub fn alloc_environment() -> OdbcResult<sql::Handle> {
    tracing::info!("Allocating new environment handle");
    env_allocated().context(OdbcRuntimeSnafu)?;
    let wrapper = Box::new(OdbcHandleWrapper {
        magic: ODBC_MAGIC_ALIVE,
        payload: OdbcHandle::Environment(Arc::new(Mutex::new(Environment {
            odbc_version: 3,
            connection_pooling: sql::AttrConnectionPooling::Off,
            connection_pool_match: sql::AttrCpMatch::Strict,
            diagnostic_info: DiagnosticInfo::default(),
            child_connections: vec![],
        }))),
    });
    Ok(Box::into_raw(wrapper) as sql::Handle)
}

/// Allocate a new connection handle, returning the opaque `sql::Handle`.
///
/// Extracts the `Arc<Mutex<Environment>>` from the parent environment
/// handle, downgrades it to `Weak<Mutex<Environment>>` for the
/// `Connection`, and pushes the `Arc<Mutex<Connection>>` to the
/// environment's `child_connections`.
pub fn alloc_connection(env_handle: sql::Handle) -> OdbcResult<sql::Handle> {
    tracing::info!("Allocating new connection handle");
    let env_wrapper = wrapper_from_handle(env_handle)?;
    let env_arc = match &env_wrapper.payload {
        OdbcHandle::Environment(arc) => arc,
        _ => {
            tracing::error!("alloc_connection: handle is not an Environment");
            return InvalidHandleSnafu.fail();
        }
    };
    let weak_env = Arc::downgrade(env_arc);

    let conn = Arc::new(Mutex::new(Connection::new(weak_env)));
    let weak_conn = Arc::downgrade(&conn);

    env_arc.lock().unwrap().child_connections.push(conn);

    let conn_wrapper = Box::new(OdbcHandleWrapper {
        magic: ODBC_MAGIC_ALIVE,
        payload: OdbcHandle::Connection(weak_conn),
    });
    Ok(Box::into_raw(conn_wrapper) as sql::Handle)
}

/// Allocate a new statement handle on the given connection, returning the opaque `sql::Handle`.
///
/// The `Arc<Mutex<Statement>>` is pushed to the parent connection's
/// `child_statements`; the wrapper stores a `Weak<Mutex<Statement>>`.
pub fn alloc_statement(conn_arc: &Arc<Mutex<Connection>>) -> OdbcResult<sql::Handle> {
    tracing::info!("Allocating new statement handle");
    let mut conn = conn_arc.lock().unwrap();
    match &mut conn.state {
        ConnectionState::Connected {
            db_handle: _,
            conn_handle,
        } => {
            let response = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
                c.statement_new(StatementNewRequest {
                    conn_handle: Some(*conn_handle),
                })
                .await
            })?;
            let stmt_handle = response
                .stmt_handle
                .required("Statement handle is required")?;

            let conn_weak = Arc::downgrade(conn_arc);
            let stmt = Statement::alloc(conn_weak, stmt_handle);
            let weak = Arc::downgrade(&stmt);
            conn.child_statements.push(stmt);
            let wrapper = Box::new(OdbcHandleWrapper {
                magic: ODBC_MAGIC_ALIVE,
                payload: OdbcHandle::Statement(weak),
            });
            let handle = Box::into_raw(wrapper) as sql::Handle;
            Ok(handle)
        }
        ConnectionState::Disconnected => {
            tracing::error!("Cannot allocate statement: connection is disconnected");
            DisconnectedSnafu.fail()
        }
    }
}

/// Free an environment handle. Validates the wrapper, marks it dead, and reclaims the Box.
pub fn free_environment(handle: sql::Handle) -> OdbcResult<()> {
    let wrapper = wrapper_from_handle(handle)?;
    match &wrapper.payload {
        OdbcHandle::Environment(_) => {}
        _ => {
            tracing::error!("free_environment: handle is not an Environment");
            return InvalidHandleSnafu.fail();
        }
    }

    tracing::info!("Freeing environment handle");
    let mut wrapper = unsafe { Box::from_raw(handle as *mut OdbcHandleWrapper) };
    wrapper.magic = ODBC_MAGIC_DEAD;
    drop(wrapper);
    env_freed().context(OdbcRuntimeSnafu)?;
    Ok(())
}

/// Free a connection handle. Releases any outstanding child statements first,
/// then removes the `Arc<Mutex<Connection>>` from the parent environment's list.
pub fn free_connection(handle: sql::Handle) -> OdbcResult<()> {
    let wrapper = wrapper_from_handle(handle)?;

    let conn_arc = match &wrapper.payload {
        OdbcHandle::Connection(weak) => weak.upgrade().ok_or_else(|| {
            tracing::error!("free_connection: connection already dropped by parent");
            InvalidHandleSnafu.build()
        })?,
        _ => {
            tracing::error!("free_connection: handle is not a Connection");
            return InvalidHandleSnafu.fail();
        }
    };

    tracing::info!("Freeing connection handle");

    // Drain child_statements first — release any outstanding statement handles
    // whose ODBC handles were never freed by the application.
    let child_statements: Vec<Arc<Mutex<Statement>>> = conn_arc
        .lock()
        .unwrap()
        .child_statements
        .drain(..)
        .collect();

    for stmt_arc in child_statements {
        let stmt_handle = stmt_arc.lock().unwrap().stmt_handle;
        global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
            let _ = c
                .statement_release(StatementReleaseRequest {
                    stmt_handle: Some(stmt_handle),
                })
                .await;
        });
    }

    // Get parent environment from connection's weak back-pointer.
    let env_arc = conn_arc
        .lock()
        .unwrap()
        .env_weak()
        .upgrade()
        .ok_or_else(|| {
            tracing::error!("free_connection: parent environment already dropped");
            InvalidHandleSnafu.build()
        })?;

    // Remove the Arc<Mutex<Connection>> from the parent environment's list.
    env_arc
        .lock()
        .unwrap()
        .child_connections
        .retain(|arc| !Arc::ptr_eq(arc, &conn_arc));

    // Now reclaim the connection wrapper itself.
    let mut wrapper = unsafe { Box::from_raw(handle as *mut OdbcHandleWrapper) };
    wrapper.magic = ODBC_MAGIC_DEAD;
    drop(wrapper);
    Ok(())
}

/// Free a statement handle. Releases the server-side handle and removes the
/// `Arc<Mutex<Statement>>` from the parent connection's `child_statements`.
pub fn free_statement(handle: sql::Handle) -> OdbcResult<()> {
    let wrapper = wrapper_from_handle(handle)?;

    let stmt_arc = match &wrapper.payload {
        OdbcHandle::Statement(weak) => weak.upgrade().ok_or_else(|| {
            tracing::error!("free_statement: statement already dropped by parent");
            InvalidHandleSnafu.build()
        })?,
        _ => {
            tracing::error!("free_statement: handle is not a Statement");
            return InvalidHandleSnafu.fail();
        }
    };

    let (stmt_handle, conn_weak) = {
        let stmt = stmt_arc.lock().unwrap();
        (stmt.stmt_handle, stmt.conn_weak())
    };

    tracing::info!("Freeing statement handle");

    // Release the server-side handle first; only remove the child_statements entry
    // and free the wrapper on success so that free_connection's cleanup loop can
    // still find and release the handle if this fails.
    let release_result = global().context(OdbcRuntimeSnafu).and_then(|rt| {
        rt.block_on(async |c| {
            c.statement_release(StatementReleaseRequest {
                stmt_handle: Some(stmt_handle),
            })
            .await?;
            Ok(())
        })
    });

    if release_result.is_ok() {
        if let Some(conn_arc) = conn_weak.upgrade() {
            conn_arc
                .lock()
                .unwrap()
                .child_statements
                .retain(|arc| !Arc::ptr_eq(arc, &stmt_arc));
        }

        // Mark dead and reclaim the Box.
        let mut wrapper = unsafe { Box::from_raw(handle as *mut OdbcHandleWrapper) };
        wrapper.magic = ODBC_MAGIC_DEAD;
        drop(wrapper);
    }

    release_result
}

/// Initialize logging (helper function for allocation)
pub fn init_logging() {
    use std::sync::LazyLock;

    // TODO: This is a hack to initialize the logging system.
    // We should find a better way to do this.
    static LOGGING_RESULT: LazyLock<Result<(), sf_core::logging::LogError>> = LazyLock::new(|| {
        sf_core::logging::init(sf_core::logging::LoggingConfig::new(
            Some("odbc.log".into()),
            false,
            false,
        ))
    });

    if let Err(e) = LOGGING_RESULT.as_ref() {
        eprintln!("Failed to initialize logging: {e:?}");
    }
}

/// Allocate handle implementation — dispatches on handle_type.
pub fn sql_alloc_handle(
    handle_type: sql::HandleType,
    input_handle: sql::Handle,
    output_handle: *mut sql::Handle,
) -> OdbcResult<()> {
    init_logging();
    tracing::debug!("SQLAllocHandle: handle_type={handle_type:?}");

    match handle_type {
        sql::HandleType::Env => {
            tracing::info!("Allocating new env: SQLAllocHandle: handle_type={handle_type:?}",);
            let handle = alloc_environment()?;
            unsafe { std::ptr::write(output_handle, handle) };
            Ok(())
        }
        sql::HandleType::Dbc => {
            tracing::info!("Allocating new dbc: SQLAllocHandle: handle_type={handle_type:?}",);
            let handle = alloc_connection(input_handle)?;
            unsafe { std::ptr::write(output_handle, handle) };
            Ok(())
        }
        sql::HandleType::Stmt => {
            tracing::info!("Allocating new stmt: SQLAllocHandle: handle_type={handle_type:?}",);
            let conn_arc = crate::api::handle::conn_from_handle(input_handle)?;
            let handle = alloc_statement(&conn_arc)?;
            unsafe { std::ptr::write(output_handle, handle) };
            Ok(())
        }
        sql::HandleType::Desc => {
            tracing::warn!("SQLAllocHandle: Desc handle type not implemented: {handle_type:?}",);
            InvalidHandleSnafu.fail()
        }
        _ => {
            tracing::error!("SQLAllocHandle: unknown handle type: {handle_type:?}");
            InvalidHandleSnafu.fail()
        }
    }
}

/// Free handle implementation — dispatches on handle_type.
pub fn sql_free_handle(handle_type: sql::HandleType, handle: sql::Handle) -> OdbcResult<()> {
    if handle.is_null() {
        return InvalidHandleSnafu.fail();
    }

    match handle_type {
        sql::HandleType::Env => {
            tracing::info!("Freeing env: SQLFreeHandle: handle_type={handle_type:?}");
            free_environment(handle)
        }
        sql::HandleType::Dbc => {
            tracing::info!("Freeing dbc: SQLFreeHandle: handle_type={handle_type:?}");
            free_connection(handle)
        }
        sql::HandleType::Stmt => {
            tracing::info!("Freeing stmt: SQLFreeHandle: handle_type={handle_type:?}");
            free_statement(handle)
        }
        sql::HandleType::Desc => InvalidHandleSnafu.fail(),
        _ => InvalidHandleSnafu.fail(),
    }
}
