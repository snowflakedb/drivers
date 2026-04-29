use crate::api::error::{
    ConnectionStillConnectedSnafu, DisconnectedSnafu, EnvironmentHasConnectionsSnafu,
    InvalidHandleSnafu, OdbcRuntimeSnafu, Required,
};
use crate::api::handle_registry::HandleId;
use crate::api::{
    Connection, ConnectionState, Dbc, Env, Environment, OdbcResult, Statement, conn_from_handle,
    diagnostic::DiagnosticInfo,
    runtime::{env_allocated, env_freed, global},
};
use odbc_sys as sql;
use parking_lot::Mutex;
use sf_core::protobuf::generated::database_driver_v1::{
    StatementNewRequest, StatementReleaseRequest,
};
use snafu::ResultExt;

/// Allocate a new environment handle
pub fn alloc_environment() -> OdbcResult<sql::Handle> {
    tracing::info!("Allocating new environment handle");
    env_allocated().context(OdbcRuntimeSnafu)?;
    let env = Env {
        environment: Mutex::new(Environment {
            odbc_version: 3,
            connection_pooling: sql::AttrConnectionPooling::Off,
            connection_pool_match: sql::AttrCpMatch::Strict,
            diagnostic_info: DiagnosticInfo::default(),
            connections: vec![],
        }),
    };
    let handle = global().context(OdbcRuntimeSnafu)?.env_registry.add(env)?;
    Ok(handle.into())
}

/// Allocate a new connection handle
pub fn alloc_connection(env_id: HandleId) -> OdbcResult<sql::Handle> {
    tracing::info!("Allocating new connection handle");
    let env_guard = global()
        .context(OdbcRuntimeSnafu)?
        .env_registry
        .get(env_id)?;
    let dbc = Dbc {
        env_id,
        connection: Mutex::new(Connection {
            state: ConnectionState::Disconnected,
            diagnostic_info: DiagnosticInfo::default(),
            pre_connection_attrs: Default::default(),
            numeric_settings: Default::default(),
            access_mode: crate::api::types::AccessMode::ReadWrite,
            quiet_mode: std::ptr::null_mut(),
            packet_size: 0,
            child_statements: vec![],
            cached_autocommit: crate::api::types::AutocommitValue::On,
            current_catalog: None,
            metadata_id: false,
        }),
    };
    let dbc_handle = global().context(OdbcRuntimeSnafu)?.dbc_registry.add(dbc)?;
    env_guard.environment.lock().connections.push(dbc_handle);
    Ok(dbc_handle.into())
}

/// Allocate a new statement handle
pub fn alloc_statement(input_handle: sql::Handle) -> OdbcResult<sql::Handle> {
    tracing::info!("Allocating new statement handle");
    let conn_id = HandleId::from(input_handle);
    let dbc = conn_from_handle(input_handle)?;
    let (conn_handle, metadata_id) = {
        let connection = dbc.connection.lock();
        match &connection.state {
            ConnectionState::Connected {
                db_handle: _,
                conn_handle,
            } => (*conn_handle, connection.metadata_id),
            ConnectionState::Disconnected => {
                tracing::error!("Cannot allocate statement: connection is disconnected");
                return DisconnectedSnafu.fail();
            }
        }
    };
    let response = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
        c.statement_new(StatementNewRequest {
            conn_handle: Some(conn_handle),
        })
        .await
    })?;

    let stmt_handle = response
        .stmt_handle
        .required("Statement handle is required")?;

    let stmt = Statement::new(conn_id, stmt_handle, metadata_id);
    let g = global().context(OdbcRuntimeSnafu)?;
    let stmt_id = g.stmt_registry.add(stmt)?;

    // Set descriptor back-pointers to the stmt HandleId so that
    // check_need_data in descriptor.rs can look up the statement.
    {
        let guard = g.stmt_registry.get(stmt_id)?;
        let mut inner = guard.inner.lock();
        inner.ard.stmt_id = stmt_id;
        inner.ird.stmt_id = stmt_id;
        inner.apd.stmt_id = stmt_id;
        inner.ipd.stmt_id = stmt_id;
    }

    dbc.connection.lock().child_statements.push(stmt_id);
    Ok(stmt_id.into())
}

/// Free an environment handle
pub fn free_environment(handle: sql::Handle) -> OdbcResult<()> {
    let handle_id = HandleId::from(handle);
    let delete_guard = global()
        .context(OdbcRuntimeSnafu)?
        .env_registry
        .get_for_delete(handle_id)?;
    let environment = delete_guard.value().environment.lock();
    if !environment.connections.is_empty() {
        return EnvironmentHasConnectionsSnafu.fail();
    }
    drop(environment);
    delete_guard.delete();
    env_freed().context(OdbcRuntimeSnafu)?;
    Ok(())
}

fn cleanup_connection(dbc: &Dbc) -> OdbcResult<()> {
    // Release any outstanding statements whose ODBC handles were never freed.
    let child_ids: Vec<_> = dbc.connection.lock().child_statements.drain(..).collect();
    let g = global().context(OdbcRuntimeSnafu)?;
    for child_id in child_ids {
        let delete_guard = match g.stmt_registry.get_for_delete(child_id) {
            Ok(guard) => guard,
            Err(e) => {
                tracing::error!(
                    "free_connection: statement {child_id:?} already deleted — skipping: {e:?}"
                );
                continue;
            }
        };
        let stmt_handle = delete_guard.value().stmt_handle;
        if let Err(e) = g.block_on(async |c| {
            c.statement_release(StatementReleaseRequest {
                stmt_handle: Some(stmt_handle),
            })
            .await
        }) {
            tracing::warn!("free_connection: failed to release statement {stmt_handle:?}: {e:?}");
        }
        delete_guard.delete();
    }
    Ok(())
}

/// Free a connection handle
pub fn free_connection(handle: sql::Handle) -> OdbcResult<()> {
    if handle.is_null() {
        return InvalidHandleSnafu.fail();
    }

    tracing::info!("Freeing connection handle");
    let handle_id = HandleId::from(handle);
    let delete_guard = global()
        .context(OdbcRuntimeSnafu)?
        .dbc_registry
        .get_for_delete(handle_id)?;
    let dbc = delete_guard.value();

    if matches!(
        dbc.connection.lock().state,
        ConnectionState::Connected { .. }
    ) {
        return ConnectionStillConnectedSnafu.fail();
    }

    // Remove from parent env's connections list.
    let env_id = dbc.env_id;
    let env_guard = global()
        .context(OdbcRuntimeSnafu)?
        .env_registry
        .get(env_id)?;
    env_guard
        .environment
        .lock()
        .connections
        .retain(|id| *id != handle_id);
    drop(env_guard);

    cleanup_connection(delete_guard.value())?;
    delete_guard.delete();
    Ok(())
}

/// Free a statement handle
pub fn free_statement(handle: sql::Handle) -> OdbcResult<()> {
    if handle.is_null() {
        return InvalidHandleSnafu.fail();
    }

    tracing::info!("Freeing statement handle");
    let handle_id = HandleId::from(handle);
    let g = global().context(OdbcRuntimeSnafu)?;

    // Take exclusive ownership via write lock (waits for all readers to finish).
    let delete_guard = g.stmt_registry.get_for_delete(handle_id)?;
    let stmt = delete_guard.value();
    let stmt_handle = stmt.stmt_handle;
    let conn_id = stmt.conn_id;

    // Release the server-side handle first; only delete on success so that
    // free_connection's cleanup loop can still find the handle on failure.
    let release_result = g.block_on(async |c| {
        c.statement_release(StatementReleaseRequest {
            stmt_handle: Some(stmt_handle),
        })
        .await?;
        Ok(())
    });

    if release_result.is_ok() {
        // Remove from parent connection's child_statements list.
        if let Ok(dbc) = g.dbc_registry.get(conn_id) {
            dbc.connection
                .lock()
                .child_statements
                .retain(|id| *id != handle_id);
        }
        delete_guard.delete();
    }
    // On failure: drop delete_guard without calling delete() — this restores
    // the handle so cleanup_connection can retry later.
    release_result
}

/// Allocate handle implementation (moved from api.rs)
pub fn sql_alloc_handle(
    handle_type: sql::HandleType,
    input_handle: sql::Handle,
    output_handle: *mut sql::Handle,
) -> OdbcResult<()> {
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
            let env_id = HandleId::from(input_handle);
            let handle = alloc_connection(env_id)?;
            unsafe { *output_handle = handle };
            Ok(())
        }
        sql::HandleType::Stmt => {
            tracing::info!(
                "Allocating new stmt: SQLAllocHandle: handle_type={:?}",
                handle_type
            );
            let handle = alloc_statement(input_handle)?;
            unsafe { std::ptr::write(output_handle, handle) };
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
            let guard = crate::api::stmt_from_handle(handle)?;
            if guard.inner.lock().state.as_ref().is_need_data() {
                return crate::api::error::InvalidDuringDaeSnafu.fail();
            }
            drop(guard);
            free_statement(handle)
        }
        sql::HandleType::Desc => InvalidHandleSnafu.fail(),
        _ => InvalidHandleSnafu.fail(),
    }
}
