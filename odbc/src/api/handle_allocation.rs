use crate::api::error::{DisconnectedSnafu, InvalidHandleSnafu, OdbcRuntimeSnafu, Required};
use crate::api::{
    Connection, ConnectionState, Environment, OdbcResult, Statement, conn_from_handle,
    diagnostic::DiagnosticInfo,
    runtime::{env_allocated, env_freed, global},
};
use odbc_sys as sql;
use sf_core::protobuf::generated::database_driver_v1::{
    StatementNewRequest, StatementReleaseRequest,
};
use snafu::ResultExt;
use std::sync::Arc;
use tracing;

/// Allocate a new environment handle
pub fn alloc_environment() -> OdbcResult<*mut Environment> {
    tracing::info!("Allocating new environment handle");
    env_allocated().context(OdbcRuntimeSnafu)?;
    let env = Box::new(Environment {
        odbc_version: 3,
        connection_pooling: sql::AttrConnectionPooling::Off,
        connection_pool_match: sql::AttrCpMatch::Strict,
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
        pre_connection_attrs: Default::default(),
        numeric_settings: Default::default(),
        access_mode: crate::api::types::AccessMode::ReadWrite,
        quiet_mode: std::ptr::null_mut(), // no window handle
        packet_size: 0,                   // driver-defined
        child_statements: vec![],
        cached_autocommit: crate::api::types::AutocommitValue::On,
        current_catalog: None,
    });
    Ok(Box::into_raw(dbc))
}

/// Allocate a new statement handle
pub fn alloc_statement(input_handle: sql::Handle) -> OdbcResult<*mut Statement> {
    tracing::info!("Allocating new statement handle");
    let conn = conn_from_handle(input_handle);
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

            // Statement is Send but not Sync (conn is *mut Connection with no synchronisation).
            // Arc is used for reference-counted ownership and Weak tracking on Connection;
            // the Arc is converted to a raw pointer immediately — it is never cloned and
            // never shared across threads. Rc cannot be used because Connection: Send.
            #[allow(clippy::arc_with_non_send_sync)]
            let stmt = Arc::new(Statement::new(conn as *mut Connection, stmt_handle));
            // Defensive prune: in correct code free_statement removes each entry before
            // dropping the Arc, so Weaks here are always upgradeable. This retain is a
            // safety net against a future bug in the removal logic, not a routine cleanup.
            conn.child_statements.retain(|(w, raw_ptr)| {
                if w.upgrade().is_some() {
                    return true;
                }
                // A dead Weak means free_statement dropped the Arc without removing the
                // child_statements entry — this is a bug. Log it so leaks are visible.
                tracing::error!(
                    "alloc_statement: defensive prune found dead Weak (raw_ptr={raw_ptr:p}); \
                     server-side statement handle may have leaked"
                );
                false
            });
            let weak = Arc::downgrade(&stmt);
            // Transfer ownership to a raw pointer used as the opaque ODBC handle.
            // Store the same pointer in child_statements so free_connection can call
            // Arc::from_raw with a pointer that satisfies its documented contract.
            let raw_ptr = Arc::into_raw(stmt);
            conn.child_statements.push((weak, raw_ptr));
            Ok(raw_ptr as *mut Statement)
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
    env_freed().context(OdbcRuntimeSnafu)?;
    Ok(())
}

/// Free a connection handle
pub fn free_connection(handle: sql::Handle) -> OdbcResult<()> {
    if handle.is_null() {
        return InvalidHandleSnafu.fail();
    }

    tracing::info!("Freeing connection handle");

    // Release any outstanding statements whose ODBC handles were never freed.
    // Each Weak still in child_statements corresponds to an Arc::into_raw that was
    // never reclaimed by free_statement (strong count = 1, held by the raw ODBC handle).
    // Reconstruct the Arc to take back that ownership, then release the server resource.
    let stmts: Vec<_> = {
        let conn = unsafe { &mut *(handle as *mut Connection) };
        conn.child_statements.drain(..).collect()
    };
    for (w, raw_ptr) in stmts {
        // Safety: raw_ptr was obtained via Arc::into_raw in alloc_statement.
        // (Note: Arc::as_ptr and Arc::into_raw are NOT equivalent — as_ptr does not transfer
        // strong-count ownership; into_raw does. Only into_raw satisfies Arc::from_raw's contract.)
        // free_statement removes its child_statements entry before dropping the Arc, so any
        // entry still present here means the ODBC handle was never freed — strong count == 1.
        // Guard: verify the Weak is still live before dereferencing raw_ptr. A dead Weak means
        // the Arc was dropped (count → 0) without removing the child_statements entry — a bug.
        // In that case raw_ptr is dangling and Arc::from_raw would be UB; skip and log instead.
        if w.upgrade().is_none() {
            tracing::error!(
                "free_connection: dead Weak for raw_ptr={raw_ptr:p}; \
                 Arc was dropped without removing child_statements entry — skipping"
            );
            continue;
        }
        let stmt = unsafe { Arc::from_raw(raw_ptr) };
        let stmt_handle = stmt.stmt_handle;
        drop(stmt);
        match global().context(OdbcRuntimeSnafu) {
            Ok(g) => {
                if let Err(e) = g.block_on(async |c| {
                    c.statement_release(StatementReleaseRequest {
                        stmt_handle: Some(stmt_handle),
                    })
                    .await
                }) {
                    tracing::warn!(
                        "free_connection: failed to release statement {stmt_handle:?}: {e:?}"
                    );
                }
            }
            Err(e) => tracing::warn!("free_connection: runtime unavailable: {e:?}"),
        }
    }

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
    // Reconstruct the Arc to reclaim ownership and drop the Statement.
    let stmt = unsafe { Arc::from_raw(handle as *const Statement) };
    let stmt_handle = stmt.stmt_handle;
    // Release the server-side handle first; only remove the child_statements entry on success
    // so that free_connection's cleanup loop can still find and release the handle if this fails.
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
        // Release succeeded — remove the bookkeeping entry and drop the Arc.
        // Arc<Statement> doesn't implement DerefMut, so use conn_ptr() (takes &self via Arc::Deref)
        // to get an independent &mut Connection.
        unsafe { &mut *stmt.conn_ptr() }
            .child_statements
            .retain(|(_, raw_ptr)| *raw_ptr != handle as *const Statement);
        drop(stmt);
    } else {
        // Release failed — forget the Arc so the raw pointer in child_statements stays valid
        // for free_connection's cleanup loop to call Arc::from_raw on later.
        std::mem::forget(stmt);
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
        sql::HandleType::Desc => InvalidHandleSnafu.fail(),
        _ => InvalidHandleSnafu.fail(),
    }
}
