use crate::api::error::{
    ConnectionStillConnectedSnafu, DisconnectedSnafu, EnvironmentHasConnectionsSnafu,
    InvalidHandleSnafu, InvalidUseOfImplicitDescriptorSnafu, OdbcRuntimeSnafu, Required,
};
use crate::api::handle_registry::{DescLookup, HandleId, HandleKind};
use crate::api::types::DescriptorKind;
use crate::api::{
    Connection, ConnectionState, Dbc, Env, Environment, OdbcResult, Statement, conn_from_handle,
    diagnostic::DiagnosticInfo,
    runtime::{env_allocated, env_freed, global},
};
use crate::conversion::warning::{Warning, Warnings};
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
        telemetry_connection_cache: arc_swap::ArcSwapOption::empty(),
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
            open_transaction: false,
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
    let conn_id = HandleId::from(input_handle).require_kind(HandleKind::Dbc)?;
    let dbc = conn_from_handle(input_handle)?;
    let mut conn = dbc.connection.lock();
    let conn_handle = match conn.state {
        ConnectionState::Connected { conn_handle, .. } => conn_handle,
        ConnectionState::Disconnected => return DisconnectedSnafu.fail(),
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
    let handle_id = HandleId::from(handle).require_kind(HandleKind::Env)?;
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

/// Drain and free orphaned child statements and explicit descriptors on a
/// connection. Used by [`free_connection`] and by [`crate::api::connection::disconnect`]
/// after a successful disconnect (ODBC: free statements/explicit descs allocated
/// on the connection).
///
/// Failures releasing child statements in core are soft: local handles are still
/// deleted and [`Warning::DisconnectError`] is recorded (SNOW-3240576).
pub(crate) fn cleanup_connection(dbc: &Dbc, warnings: &mut Warnings) -> OdbcResult<()> {
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
            c.statement_release(StatementReleaseRequest {
                stmt_handle: Some(stmt_handle),
            })
            .await
        }) {
            tracing::warn!("free_connection: failed to release statement {stmt_handle:?}: {e:?}");
            warnings.push(Warning::DisconnectError);
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
    let handle_id = HandleId::from(handle).require_kind(HandleKind::Dbc)?;
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

    cleanup_connection(delete_guard.value(), &mut Vec::new())?;
    delete_guard.delete();
    Ok(())
}

/// Free a statement handle
pub fn free_statement(handle: sql::Handle) -> OdbcResult<()> {
    if handle.is_null() {
        return InvalidHandleSnafu.fail();
    }

    tracing::info!("Freeing statement handle");
    let handle_id = HandleId::from(handle).require_kind(HandleKind::Stmt)?;
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
    let conn_id = HandleId::from(input_handle).require_kind(HandleKind::Dbc)?;
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
    let desc_id = HandleId::from(handle).require_kind(HandleKind::Desc)?;
    let g = global().context(OdbcRuntimeSnafu)?;

    // Validate this is an explicit descriptor
    let desc_guard = g.desc_manager.get(desc_id)?;
    let conn_id = match *desc_guard {
        DescLookup::Explicit { conn_id } => conn_id,
        DescLookup::Implicit { .. } => {
            // Freeing an automatically-allocated (implicit) descriptor is invalid per the
            // ODBC spec: return SQL_ERROR with HY017 and leave the handle valid, not
            // SQL_INVALID_HANDLE. See SNOW-3240578. The HY017 diagnostic is posted onto the
            // handle by the SQLFreeHandle entry point. Return early without mutating any
            // connection/statement state so the descriptor stays usable.
            //
            // Note: unixODBC and iODBC both answer SQLFreeHandle(SQL_HANDLE_DESC, <implicit>)
            // from their own descriptor bookkeeping and never dispatch it to the driver, so
            // this arm is only reached by direct (driver-manager-less) callers. The
            // handle_allocation unit tests are its sole coverage for that reason.
            return InvalidUseOfImplicitDescriptorSnafu.fail();
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
            let env_id = HandleId::from(input_handle).require_kind(HandleKind::Env)?;
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
                if let Some(operation) = *guard.operation.lock()
                    && let Ok(g) = global()
                {
                    g.client().cancel_operation(operation);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::{DaeContext, ExecutionOrigin, StatementState};
    use crate::conversion::warning::Warning;
    use sf_core::protobuf::generated::database_driver_v1::{
        ConnectionNewRequest, DatabaseNewRequest, DatabaseReleaseRequest,
    };
    use std::collections::HashMap;

    fn with_env<F, R>(f: F) -> R
    where
        F: FnOnce(sql::Handle, HandleId) -> R,
    {
        // Every test in this module operates on the same process-global handle
        // registries. Serialize them so a concurrent test's live handles can't
        // perturb slot-index or registry-emptiness assertions under the parallel
        // test runner. Recover from a poisoned lock so one failing test doesn't
        // cascade into spurious failures in the rest of the module.
        static TEST_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());

        let env_handle = alloc_environment().expect("alloc_environment");
        let env_id = HandleId::from(env_handle);
        let result = f(env_handle, env_id);
        free_environment(env_handle).expect("free_environment — free child dbcs first");
        result
    }

    fn alloc_tracked_dbc(env_id: HandleId) -> sql::Handle {
        alloc_connection(env_id).expect("alloc_connection")
    }

    fn mark_dbc_connected(dbc_handle: sql::Handle) {
        let g = global().expect("globals");
        let (db_handle, conn_handle) = g.block_on(async |c| {
            let db_handle = c
                .database_new(DatabaseNewRequest {})
                .await
                .expect("database_new")
                .db_handle
                .expect("db_handle present");
            let conn_handle = c
                .connection_new(ConnectionNewRequest {})
                .await
                .expect("connection_new")
                .conn_handle
                .expect("conn_handle present");
            (db_handle, conn_handle)
        });
        let dbc = g
            .dbc_registry
            .get(HandleId::from(dbc_handle))
            .expect("dbc in registry");
        dbc.mark_connected(&mut dbc.connection.lock(), db_handle, conn_handle);
    }

    fn mark_dbc_disconnected(dbc_handle: sql::Handle) {
        let g = global().expect("globals");
        let dbc = g
            .dbc_registry
            .get(HandleId::from(dbc_handle))
            .expect("dbc in registry");
        dbc.mark_disconnected(&mut dbc.connection.lock());
    }

    fn env_connections(env_id: HandleId) -> Vec<HandleId> {
        let g = global().expect("globals");
        let env = g.env_registry.get(env_id).expect("env in registry");
        env.environment.lock().connections.clone()
    }

    fn child_statements(dbc_handle: sql::Handle) -> Vec<HandleId> {
        let g = global().expect("globals");
        let dbc = g
            .dbc_registry
            .get(HandleId::from(dbc_handle))
            .expect("dbc in registry");
        dbc.connection.lock().child_statements.clone()
    }

    fn child_descriptor_ids(dbc_handle: sql::Handle) -> Vec<HandleId> {
        let g = global().expect("globals");
        let dbc = g
            .dbc_registry
            .get(HandleId::from(dbc_handle))
            .expect("dbc in registry");
        dbc.connection
            .lock()
            .child_descriptors
            .iter()
            .map(|(id, _)| *id)
            .collect()
    }

    fn connection_is_connected(dbc_handle: sql::Handle) -> bool {
        let g = global().expect("globals");
        let dbc = g
            .dbc_registry
            .get(HandleId::from(dbc_handle))
            .expect("dbc in registry");
        matches!(
            dbc.connection.lock().state,
            ConnectionState::Connected { .. }
        )
    }

    fn stmt_desc_handles(stmt_handle: sql::Handle) -> [HandleId; 4] {
        let g = global().expect("globals");
        let stmt = g
            .stmt_registry
            .get(HandleId::from(stmt_handle))
            .expect("stmt in registry");
        let inner = stmt.inner.lock();
        [
            inner.ard_handle,
            inner.ird_handle,
            inner.apd_handle,
            inner.ipd_handle,
        ]
    }

    fn assert_implicit_descs_registered(stmt_id: HandleId, descs: [HandleId; 4]) {
        let g = global().expect("globals");
        let kinds = [
            DescriptorKind::Ard,
            DescriptorKind::Ird,
            DescriptorKind::Apd,
            DescriptorKind::Ipd,
        ];
        for (desc_id, kind) in descs.into_iter().zip(kinds) {
            let lookup = g
                .desc_manager
                .get(desc_id)
                .unwrap_or_else(|_| panic!("desc {desc_id:?} must be registered"));
            match *lookup {
                DescLookup::Implicit {
                    stmt_id: sid,
                    kind: k,
                } if sid == stmt_id && k == kind => {}
                other => panic!(
                    "expected Implicit {{ stmt_id: {stmt_id:?}, kind: {kind:?} }}, got {other:?}"
                ),
            }
        }
    }

    fn assert_handles_gone_from_registries(stmt_ids: &[HandleId], desc_ids: &[HandleId]) {
        let g = global().expect("globals");
        for &stmt_id in stmt_ids {
            assert!(
                g.stmt_registry.get(stmt_id).is_err(),
                "stmt {stmt_id:?} must be removed from stmt_registry"
            );
        }
        for &desc_id in desc_ids {
            assert!(
                g.desc_manager.get(desc_id).is_err(),
                "desc {desc_id:?} must be removed from desc_manager"
            );
        }
    }

    #[test]
    fn alloc_connection_registers_in_parent_env_connections() {
        with_env(|_env_handle, env_id| {
            let dbc_handle = alloc_tracked_dbc(env_id);
            let dbc_id = HandleId::from(dbc_handle);
            let connections = env_connections(env_id);
            assert!(
                connections.contains(&dbc_id),
                "env.connections must track allocated dbc {dbc_id:?}; got {connections:?}"
            );
            free_connection(dbc_handle).expect("free_connection");
        });
    }

    #[test]
    fn free_connection_removes_handle_from_env_connections() {
        with_env(|_env_handle, env_id| {
            let dbc_handle = alloc_tracked_dbc(env_id);
            let dbc_id = HandleId::from(dbc_handle);
            free_connection(dbc_handle).expect("free_connection");
            let connections = env_connections(env_id);
            assert!(
                !connections.contains(&dbc_id),
                "env.connections must drop freed dbc {dbc_id:?}; got {connections:?}"
            );
        });
    }

    #[test]
    fn alloc_statement_registers_in_parent_conn_child_statements() {
        with_env(|_env_handle, env_id| {
            let dbc_handle = alloc_tracked_dbc(env_id);
            mark_dbc_connected(dbc_handle);

            let stmt_handle = alloc_statement(dbc_handle).expect("alloc_statement");
            let stmt_id = HandleId::from(stmt_handle);
            let stmts = child_statements(dbc_handle);
            assert!(
                stmts.contains(&stmt_id),
                "child_statements must track allocated stmt {stmt_id:?}; got {stmts:?}"
            );

            free_statement(stmt_handle).expect("free_statement");
            mark_dbc_disconnected(dbc_handle);
            free_connection(dbc_handle).expect("free_connection");
        });
    }

    #[test]
    fn free_statement_removes_handle_from_conn_child_statements() {
        with_env(|_env_handle, env_id| {
            let dbc_handle = alloc_tracked_dbc(env_id);
            mark_dbc_connected(dbc_handle);

            let stmt_handle = alloc_statement(dbc_handle).expect("alloc_statement");
            let stmt_id = HandleId::from(stmt_handle);
            free_statement(stmt_handle).expect("free_statement");

            let stmts = child_statements(dbc_handle);
            assert!(
                !stmts.contains(&stmt_id),
                "child_statements must drop freed stmt {stmt_id:?}; got {stmts:?}"
            );

            mark_dbc_disconnected(dbc_handle);
            free_connection(dbc_handle).expect("free_connection");
        });
    }

    #[test]
    fn parent_child_invariants_hold_across_repeated_alloc_free_cycles() {
        with_env(|_env_handle, env_id| {
            for round in 0..2 {
                let dbc_handle = alloc_tracked_dbc(env_id);
                let dbc_id = HandleId::from(dbc_handle);
                assert!(
                    env_connections(env_id).contains(&dbc_id),
                    "round {round}: env must track dbc {dbc_id:?}"
                );

                mark_dbc_connected(dbc_handle);
                let mut stmt_ids = Vec::new();
                for _ in 0..2 {
                    let stmt_handle = alloc_statement(dbc_handle).expect("alloc_statement");
                    let stmt_id = HandleId::from(stmt_handle);
                    assert!(
                        child_statements(dbc_handle).contains(&stmt_id),
                        "round {round}: conn must track stmt {stmt_id:?}"
                    );
                    stmt_ids.push(stmt_handle);
                }

                for stmt_handle in stmt_ids {
                    let stmt_id = HandleId::from(stmt_handle);
                    free_statement(stmt_handle).expect("free_statement");
                    assert!(
                        !child_statements(dbc_handle).contains(&stmt_id),
                        "round {round}: conn must drop stmt {stmt_id:?}"
                    );
                }
                assert!(
                    child_statements(dbc_handle).is_empty(),
                    "round {round}: child_statements must be empty after freeing all stmts"
                );

                mark_dbc_disconnected(dbc_handle);
                free_connection(dbc_handle).expect("free_connection");
                assert!(
                    !env_connections(env_id).contains(&dbc_id),
                    "round {round}: env must drop dbc {dbc_id:?}"
                );
            }

            assert!(
                env_connections(env_id).is_empty(),
                "env.connections must be empty after all cycles"
            );
        });
    }

    #[test]
    fn free_connection_releases_orphaned_child_statements() {
        with_env(|_env_handle, env_id| {
            let dbc_handle = alloc_tracked_dbc(env_id);
            mark_dbc_connected(dbc_handle);

            let stmt_a = alloc_statement(dbc_handle).expect("alloc_statement a");
            let stmt_b = alloc_statement(dbc_handle).expect("alloc_statement b");
            let stmt_ids = [HandleId::from(stmt_a), HandleId::from(stmt_b)];
            let mut desc_ids = Vec::new();
            desc_ids.extend(stmt_desc_handles(stmt_a));
            desc_ids.extend(stmt_desc_handles(stmt_b));

            assert_eq!(
                child_statements(dbc_handle).len(),
                2,
                "both statements must be tracked before free_connection"
            );

            // Intentionally skip free_statement — free_connection must drain orphans.
            mark_dbc_disconnected(dbc_handle);
            free_connection(dbc_handle).expect("free_connection with orphaned statements");

            assert_handles_gone_from_registries(&stmt_ids, &desc_ids);
        });
    }

    /// SNOW-3240577: SQLDisconnect must free child statements and explicit
    /// descriptors so a follow-up SQLFreeHandle returns SQL_INVALID_HANDLE.
    #[test]
    fn disconnect_releases_orphaned_child_statements_and_explicit_descriptors() {
        with_env(|_env_handle, env_id| {
            let dbc_handle = alloc_tracked_dbc(env_id);
            mark_dbc_connected(dbc_handle);

            let stmt_a = alloc_statement(dbc_handle).expect("alloc_statement a");
            let stmt_b = alloc_statement(dbc_handle).expect("alloc_statement b");
            let explicit_desc = alloc_descriptor(dbc_handle).expect("alloc_descriptor");
            let stmt_ids = [HandleId::from(stmt_a), HandleId::from(stmt_b)];
            let explicit_desc_id = HandleId::from(explicit_desc);
            let mut desc_ids = Vec::new();
            desc_ids.extend(stmt_desc_handles(stmt_a));
            desc_ids.extend(stmt_desc_handles(stmt_b));
            desc_ids.push(explicit_desc_id);

            assert_eq!(child_statements(dbc_handle).len(), 2);
            assert_eq!(child_descriptor_ids(dbc_handle), vec![explicit_desc_id]);

            crate::api::connection::disconnect(dbc_handle, &mut Vec::new()).expect("disconnect");

            assert!(
                !connection_is_connected(dbc_handle),
                "disconnect must leave the connection Disconnected"
            );
            assert!(
                child_statements(dbc_handle).is_empty(),
                "child_statements must be drained after disconnect"
            );
            assert!(
                child_descriptor_ids(dbc_handle).is_empty(),
                "child_descriptors must be drained after disconnect"
            );
            assert_handles_gone_from_registries(&stmt_ids, &desc_ids);

            free_connection(dbc_handle).expect("free_connection after disconnect");
        });
    }

    /// SNOW-3240577: HY010 before any teardown when a child is mid data-at-execution.
    #[test]
    fn disconnect_rejects_when_child_statement_in_need_data() {
        with_env(|_env_handle, env_id| {
            let dbc_handle = alloc_tracked_dbc(env_id);
            mark_dbc_connected(dbc_handle);

            let stmt_handle = alloc_statement(dbc_handle).expect("alloc_statement");
            let stmt_id = HandleId::from(stmt_handle);

            {
                let g = global().expect("globals");
                let stmt = g.stmt_registry.get(stmt_id).expect("stmt in registry");
                let mut inner = stmt.inner.lock();
                inner.state.set(StatementState::AwaitingParamData {
                    dae_context: Box::new(DaeContext {
                        dae_params: vec![1],
                        current_index: 0,
                        pushed_data: HashMap::new(),
                        deferred_query: None,
                    }),
                    origin: ExecutionOrigin::Direct,
                });
            }

            let err = crate::api::connection::disconnect(dbc_handle, &mut Vec::new())
                .expect_err("disconnect must reject NeedData with HY010");
            assert!(
                matches!(err, crate::api::OdbcError::InvalidDuringDae { .. }),
                "expected InvalidDuringDae, got {err:?}"
            );
            assert!(
                connection_is_connected(dbc_handle),
                "failed disconnect must leave the connection Connected"
            );
            assert_eq!(
                child_statements(dbc_handle),
                vec![stmt_id],
                "failed disconnect must not free child statements"
            );
            assert!(
                global()
                    .expect("globals")
                    .stmt_registry
                    .get(stmt_id)
                    .is_ok(),
                "stmt must still be registered after rejected disconnect"
            );

            // Reset state so free_statement / disconnect can complete cleanup.
            {
                let g = global().expect("globals");
                let stmt = g.stmt_registry.get(stmt_id).expect("stmt in registry");
                stmt.inner.lock().state.set(StatementState::Created);
            }
            free_statement(stmt_handle).expect("free_statement");
            crate::api::connection::disconnect(dbc_handle, &mut Vec::new())
                .expect("disconnect after reset");
            free_connection(dbc_handle).expect("free_connection");
        });
    }

    /// SNOW-3240576: after `connection_close` succeeds, a soft failure in
    /// `database_release` still leaves the connection Disconnected and records
    /// `Warning::DisconnectError` (SQLSTATE 01002 / SQL_SUCCESS_WITH_INFO).
    #[test]
    fn disconnect_soft_database_release_failure_records_disconnect_error_warning() {
        with_env(|_env_handle, env_id| {
            let dbc_handle = alloc_tracked_dbc(env_id);
            mark_dbc_connected(dbc_handle);

            // Pre-release the database handle so disconnect's database_release
            // soft-fails after a successful close + connection_release.
            let g = global().expect("globals");
            let db_handle = {
                let dbc = g
                    .dbc_registry
                    .get(HandleId::from(dbc_handle))
                    .expect("dbc in registry");
                match &dbc.connection.lock().state {
                    ConnectionState::Connected { db_handle, .. } => *db_handle,
                    ConnectionState::Disconnected => panic!("expected Connected"),
                }
            };
            g.block_on(async |c| {
                c.database_release(DatabaseReleaseRequest {
                    db_handle: Some(db_handle),
                })
                .await
                .expect("pre-release database");
            });

            let mut warnings = Vec::new();
            crate::api::connection::disconnect(dbc_handle, &mut warnings)
                .expect("disconnect must succeed when only post-close release soft-fails");

            assert!(
                !connection_is_connected(dbc_handle),
                "soft cleanup failure must still leave the connection Disconnected"
            );
            assert_eq!(
                warnings,
                vec![Warning::DisconnectError],
                "expected a single DisconnectError warning for soft database_release failure"
            );

            let rec = crate::api::diagnostic::from_warning(&Warning::DisconnectError);
            assert_eq!(rec.sql_state.as_str(), "01002");
            assert_eq!(rec.message_text, "Disconnect error");

            free_connection(dbc_handle).expect("free_connection after soft-fail disconnect");
        });
    }

    #[test]
    fn free_connection_fails_when_still_connected() {
        with_env(|_env_handle, env_id| {
            let dbc_handle = alloc_tracked_dbc(env_id);
            mark_dbc_connected(dbc_handle);

            let err = free_connection(dbc_handle).expect_err("must reject Connected dbc");
            assert!(
                matches!(err, crate::api::OdbcError::ConnectionStillConnected { .. }),
                "expected ConnectionStillConnected, got {err:?}"
            );

            mark_dbc_disconnected(dbc_handle);
            free_connection(dbc_handle).expect("free_connection after disconnect");
        });
    }

    #[test]
    fn free_environment_fails_when_connections_non_empty() {
        let env_handle = alloc_environment().expect("alloc_environment");
        let env_id = HandleId::from(env_handle);
        let dbc_handle = alloc_tracked_dbc(env_id);

        let err = free_environment(env_handle).expect_err("must reject env with connections");
        assert!(
            matches!(err, crate::api::OdbcError::EnvironmentHasConnections { .. }),
            "expected EnvironmentHasConnections, got {err:?}"
        );

        free_connection(dbc_handle).expect("free_connection");
        free_environment(env_handle).expect("free_environment after last connection freed");
    }

    #[test]
    fn alloc_statement_registers_four_implicit_descriptors() {
        with_env(|_env_handle, env_id| {
            let dbc_handle = alloc_tracked_dbc(env_id);
            mark_dbc_connected(dbc_handle);

            let stmt_handle = alloc_statement(dbc_handle).expect("alloc_statement");
            let stmt_id = HandleId::from(stmt_handle);
            let descs = stmt_desc_handles(stmt_handle);
            assert!(
                descs.iter().all(|id| *id != HandleId::default()),
                "implicit desc handles must be non-default; got {descs:?}"
            );
            assert_implicit_descs_registered(stmt_id, descs);

            free_statement(stmt_handle).expect("free_statement");
            mark_dbc_disconnected(dbc_handle);
            free_connection(dbc_handle).expect("free_connection");
        });
    }

    #[test]
    fn free_statement_tears_down_implicit_descriptors() {
        with_env(|_env_handle, env_id| {
            let dbc_handle = alloc_tracked_dbc(env_id);
            mark_dbc_connected(dbc_handle);

            let stmt_handle = alloc_statement(dbc_handle).expect("alloc_statement");
            let stmt_id = HandleId::from(stmt_handle);
            let descs = stmt_desc_handles(stmt_handle);

            free_statement(stmt_handle).expect("free_statement");
            assert_handles_gone_from_registries(&[stmt_id], &descs);

            mark_dbc_disconnected(dbc_handle);
            free_connection(dbc_handle).expect("free_connection");
        });
    }

    /// Both unixODBC and iODBC answer `SQLFreeHandle(SQL_HANDLE_DESC, <implicit>)` from their
    /// own descriptor bookkeeping without dispatching to the driver, so this unit test is the
    /// only coverage of the driver-side contract for direct (DM-less) callers.
    #[test]
    fn free_descriptor_rejects_implicit_descriptor_and_leaves_it_usable() {
        with_env(|_env_handle, env_id| {
            let dbc_handle = alloc_tracked_dbc(env_id);
            mark_dbc_connected(dbc_handle);

            let stmt_handle = alloc_statement(dbc_handle).expect("alloc_statement");
            let stmt_id = HandleId::from(stmt_handle);
            let descs = stmt_desc_handles(stmt_handle);
            let ard_handle: sql::Handle = descs[0].into();

            let result = free_descriptor(ard_handle);
            assert!(
                matches!(
                    result,
                    Err(crate::api::OdbcError::InvalidUseOfImplicitDescriptor { .. })
                ),
                "expected InvalidUseOfImplicitDescriptor (HY017), got {result:?}"
            );

            // HY017 must leave the handle valid: all four implicit descriptors stay registered
            // and bound to the statement, and the statement itself is untouched.
            assert_implicit_descs_registered(stmt_id, descs);

            free_statement(stmt_handle).expect("free_statement");
            mark_dbc_disconnected(dbc_handle);
            free_connection(dbc_handle).expect("free_connection");
        });
    }

    fn assert_invalid_handle<T: std::fmt::Debug>(result: OdbcResult<T>) {
        assert!(
            matches!(result, Err(crate::api::OdbcError::InvalidHandle { .. })),
            "expected InvalidHandle, got {result:?}"
        );
    }

    /// Slot indexes collide across registries (all start at 1); tagged kinds
    /// must make cross-type free/disconnect fail before touching the peer.
    #[test]
    fn mismatched_handle_kinds_return_invalid_handle() {
        with_env(|env_handle, env_id| {
            let dbc_handle = alloc_tracked_dbc(env_id);
            mark_dbc_connected(dbc_handle);
            let stmt_handle = alloc_statement(dbc_handle).expect("alloc_statement");

            assert_eq!(HandleId::from(env_handle).slot(), 1);
            assert_eq!(HandleId::from(dbc_handle).slot(), 1);
            assert_eq!(HandleId::from(stmt_handle).slot(), 1);

            assert_invalid_handle(free_connection(stmt_handle));
            assert_invalid_handle(free_statement(dbc_handle));
            assert_invalid_handle(free_environment(dbc_handle));
            assert_invalid_handle(free_environment(stmt_handle));
            assert_invalid_handle(crate::api::connection::disconnect(
                env_handle,
                &mut Vec::new(),
            ));
            assert_invalid_handle(sql_free_handle(sql::HandleType::Stmt, dbc_handle));
            assert_invalid_handle(sql_free_handle(sql::HandleType::Dbc, stmt_handle));
            assert_invalid_handle(alloc_statement(env_handle));

            // Peers still usable after rejected cross-type calls.
            free_statement(stmt_handle).expect("free_statement");
            mark_dbc_disconnected(dbc_handle);
            free_connection(dbc_handle).expect("free_connection");
        });
    }

    #[test]
    fn double_free_statement_returns_invalid_handle() {
        with_env(|_env_handle, env_id| {
            let dbc_handle = alloc_tracked_dbc(env_id);
            mark_dbc_connected(dbc_handle);
            let stmt_handle = alloc_statement(dbc_handle).expect("alloc_statement");
            free_statement(stmt_handle).expect("first free");
            assert_invalid_handle(free_statement(stmt_handle));
            mark_dbc_disconnected(dbc_handle);
            free_connection(dbc_handle).expect("free_connection");
        });
    }

    #[test]
    fn app_row_desc_sqlpointer_round_trips_tagged_handle() {
        use crate::api::Narrow;
        use crate::api::StmtAttr;
        use crate::api::statement::{get_stmt_attr, set_stmt_attr};
        use crate::conversion::warning::Warnings;

        with_env(|_env_handle, env_id| {
            let dbc_handle = alloc_tracked_dbc(env_id);
            mark_dbc_connected(dbc_handle);
            let stmt_handle = alloc_statement(dbc_handle).expect("alloc_statement");
            let desc_handle = alloc_descriptor(dbc_handle).expect("alloc_descriptor");

            let mut warnings = Warnings::default();
            set_stmt_attr(
                stmt_handle,
                StmtAttr::AppRowDesc as sql::Integer,
                desc_handle as sql::Pointer,
                0,
                &mut warnings,
            )
            .expect("SQLSetStmtAttr APP_ROW_DESC");

            let mut out: sql::Handle = std::ptr::null_mut();
            get_stmt_attr::<Narrow>(
                stmt_handle,
                StmtAttr::AppRowDesc as sql::Integer,
                &mut out as *mut _ as sql::Pointer,
                0,
                std::ptr::null_mut(),
                &mut warnings,
            )
            .expect("SQLGetStmtAttr APP_ROW_DESC");
            assert_eq!(
                out, desc_handle,
                "APP_ROW_DESC must round-trip the tagged SQLHANDLE bit-identically"
            );

            // Revert to implicit ARD so the explicit desc can be freed.
            set_stmt_attr(
                stmt_handle,
                StmtAttr::AppRowDesc as sql::Integer,
                std::ptr::null_mut(),
                0,
                &mut warnings,
            )
            .expect("revert APP_ROW_DESC");
            free_descriptor(desc_handle).expect("free_descriptor");
            free_statement(stmt_handle).expect("free_statement");
            mark_dbc_disconnected(dbc_handle);
            free_connection(dbc_handle).expect("free_connection");
        });
    }
}
