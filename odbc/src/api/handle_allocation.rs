use crate::api::error::{
    ConnectionStillConnectedSnafu, DisconnectedSnafu, EnvironmentHasConnectionsSnafu,
    InvalidHandleSnafu, OdbcRuntimeSnafu, Required,
};
use crate::api::handle_registry::{DescLookup, HandleId};
use crate::api::types::DescriptorKind;
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

use super::runtime::GlobalsGuard;

fn register_desc_handles(
    g: &GlobalsGuard,
    stmt_id: HandleId,
) -> OdbcResult<(HandleId, HandleId, HandleId, HandleId)> {
    let ard = g.desc_manager.add(DescLookup::Implicit {
        stmt_id,
        kind: DescriptorKind::Ard,
    })?;
    let ird = match g.desc_manager.add(DescLookup::Implicit {
        stmt_id,
        kind: DescriptorKind::Ird,
    }) {
        Ok(id) => id,
        Err(e) => {
            let _ = g.desc_manager.get_for_delete(ard).map(|dg| dg.delete());
            return Err(e);
        }
    };
    let apd = match g.desc_manager.add(DescLookup::Implicit {
        stmt_id,
        kind: DescriptorKind::Apd,
    }) {
        Ok(id) => id,
        Err(e) => {
            let _ = g.desc_manager.get_for_delete(ard).map(|dg| dg.delete());
            let _ = g.desc_manager.get_for_delete(ird).map(|dg| dg.delete());
            return Err(e);
        }
    };
    let ipd = match g.desc_manager.add(DescLookup::Implicit {
        stmt_id,
        kind: DescriptorKind::Ipd,
    }) {
        Ok(id) => id,
        Err(e) => {
            let _ = g.desc_manager.get_for_delete(ard).map(|dg| dg.delete());
            let _ = g.desc_manager.get_for_delete(ird).map(|dg| dg.delete());
            let _ = g.desc_manager.get_for_delete(apd).map(|dg| dg.delete());
            return Err(e);
        }
    };
    Ok((ard, ird, apd, ipd))
}

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
            child_descriptors: vec![],
            cached_autocommit: crate::api::types::AutocommitValue::On,
            current_catalog: None,
            metadata_id: false,
            driver_section: None,
            dsn_name: None,
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
    let mut conn = dbc.connection.lock();
    let conn_handle = match conn.state {
        ConnectionState::Connected { conn_handle, .. } => conn_handle,
        ConnectionState::Disconnected => return DisconnectedSnafu.fail(),
    };

    let response = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
        c.statement_new(
            StatementNewRequest {
                conn_handle: Some(conn_handle),
            },
            tokio_util::sync::CancellationToken::new(),
        )
        .await
    })?;

    let stmt_handle = response
        .stmt_handle
        .required("Statement handle is required")?;

    let stmt = Statement::new(conn_id, stmt_handle, conn.metadata_id);
    let g = global().context(OdbcRuntimeSnafu)?;
    let stmt_id = g.stmt_registry.add(stmt)?;

    let desc_handles = register_desc_handles(&g, stmt_id);
    let (ard_handle, ird_handle, apd_handle, ipd_handle) = match desc_handles {
        Ok(handles) => handles,
        Err(e) => {
            if let Ok(dg) = g.stmt_registry.get_for_delete(stmt_id) {
                dg.delete();
            }
            return Err(e);
        }
    };

    let guard = g.stmt_registry.get(stmt_id)?;
    let mut inner = guard.inner.lock();
    inner.ard_handle = ard_handle;
    inner.ird_handle = ird_handle;
    inner.apd_handle = apd_handle;
    inner.ipd_handle = ipd_handle;

    conn.child_statements.push(stmt_id);
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
    let mut conn = dbc.connection.lock();
    // Release any outstanding statements whose ODBC handles were never freed.
    let child_ids: Vec<_> = conn.child_statements.drain(..).collect();
    let desc_ids: Vec<_> = conn.child_descriptors.drain(..).collect();
    drop(conn);

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
        let desc_handles = {
            let inner = delete_guard.value().inner.lock();
            [
                inner.ard_handle,
                inner.ird_handle,
                inner.apd_handle,
                inner.ipd_handle,
            ]
        };
        if let Err(e) = g.block_on(async |c| {
            c.statement_release(
                StatementReleaseRequest {
                    stmt_handle: Some(stmt_handle),
                },
                tokio_util::sync::CancellationToken::new(),
            )
            .await
        }) {
            tracing::warn!("free_connection: failed to release statement {stmt_handle:?}: {e:?}");
        }
        for desc_id in desc_handles {
            if let Ok(dg) = g.desc_manager.get_for_delete(desc_id) {
                dg.delete();
            }
        }
        delete_guard.delete();
    }

    // Free explicit descriptors allocated on this connection.
    // The Arc<Mutex<ArdDescriptor>> is dropped here (last owner), and the
    // desc_manager entry is removed so the HandleId can be recycled.
    for (desc_id, _arc) in desc_ids {
        if let Ok(dg) = g.desc_manager.get_for_delete(desc_id) {
            dg.delete();
        }
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
    let desc_handles = {
        let inner = stmt.inner.lock();
        [
            inner.ard_handle,
            inner.ird_handle,
            inner.apd_handle,
            inner.ipd_handle,
        ]
    };

    // Release the server-side handle first; only delete on success so that
    // free_connection's cleanup loop can still find the handle on failure.
    let release_result = g.block_on(async |c| {
        c.statement_release(
            StatementReleaseRequest {
                stmt_handle: Some(stmt_handle),
            },
            tokio_util::sync::CancellationToken::new(),
        )
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
        for desc_id in desc_handles {
            if let Ok(dg) = g.desc_manager.get_for_delete(desc_id) {
                dg.delete();
            }
        }
        delete_guard.delete();
    }
    // On failure: drop delete_guard without calling delete() — this restores
    // the handle so cleanup_connection can retry later.
    release_result
}

/// Allocate an explicit application descriptor on a connection.
pub fn alloc_descriptor(input_handle: sql::Handle) -> OdbcResult<sql::Handle> {
    tracing::info!("Allocating explicit descriptor handle");
    let conn_id = HandleId::from(input_handle);
    let dbc = conn_from_handle(input_handle)?;
    let mut conn = dbc.connection.lock();

    let g = global().context(OdbcRuntimeSnafu)?;
    let desc_handle_id = g.desc_manager.add(DescLookup::Explicit { conn_id })?;
    let arc = std::sync::Arc::new(parking_lot::Mutex::new(crate::api::ArdDescriptor::new()));
    conn.child_descriptors.push((desc_handle_id, arc));
    Ok(desc_handle_id.into())
}

/// Free an explicitly-allocated descriptor handle.
pub fn free_descriptor(handle: sql::Handle) -> OdbcResult<()> {
    if handle.is_null() {
        return InvalidHandleSnafu.fail();
    }
    tracing::info!("Freeing explicit descriptor handle");
    let desc_id = HandleId::from(handle);
    let g = global().context(OdbcRuntimeSnafu)?;

    // Validate this is an explicit descriptor
    let desc_guard = g.desc_manager.get(desc_id)?;
    let conn_id = match *desc_guard {
        DescLookup::Explicit { conn_id } => conn_id,
        DescLookup::Implicit { .. } => {
            return InvalidHandleSnafu.fail();
        }
    };
    drop(desc_guard);

    // Revert any statements using this descriptor, and remove from connection's list
    let dbc = g.dbc_registry.get(conn_id)?;
    let child_stmts: Vec<HandleId> = {
        let mut conn = dbc.connection.lock();
        conn.child_descriptors.retain(|(id, _)| *id != desc_id);
        conn.child_statements.clone()
    };
    drop(dbc);
    for stmt_id in child_stmts {
        if let Ok(stmt_guard) = g.stmt_registry.get(stmt_id) {
            let mut inner = stmt_guard.inner.lock();
            if inner
                .active_ard
                .as_ref()
                .is_some_and(|(id, _)| *id == desc_id)
            {
                inner.active_ard = None;
            }
            if inner
                .active_apd
                .as_ref()
                .is_some_and(|(id, _)| *id == desc_id)
            {
                inner.active_apd = None;
            }
        }
    }

    // Delete from desc_manager
    if let Ok(dg) = g.desc_manager.get_for_delete(desc_id) {
        dg.delete();
    }
    Ok(())
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
            tracing::info!(
                "Allocating new desc: SQLAllocHandle: handle_type={:?}",
                handle_type
            );
            let handle = alloc_descriptor(input_handle)?;
            unsafe { std::ptr::write(output_handle, handle) };
            Ok(())
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
            let mut inner = guard.inner.lock();
            if inner.state.as_ref().is_need_data() {
                return crate::api::error::InvalidDuringDaeSnafu.fail();
            }
            if inner.state.as_ref().is_async_executing() {
                if let Some(ref t) = *guard.cancel_token.lock() {
                    t.cancel();
                }
                match inner.state.take() {
                    crate::api::StatementState::AsyncExecDirect { join_handle } => {
                        join_handle.abort();
                    }
                    crate::api::StatementState::AsyncPrepare { join_handle } => {
                        join_handle.abort();
                    }
                    crate::api::StatementState::AsyncExecute { join_handle, .. } => {
                        join_handle.abort();
                    }
                    _ => unreachable!(),
                }
                inner.state.set(crate::api::StatementState::Error);
            }
            drop(inner);
            drop(guard);
            free_statement(handle)
        }
        sql::HandleType::Desc => free_descriptor(handle),
        _ => InvalidHandleSnafu.fail(),
    }
}
