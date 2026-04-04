use crate::api::error::{DisconnectedSnafu, InvalidHandleSnafu, OdbcRuntimeSnafu, Required};
use crate::api::{
    ApdDescriptor, ArdDescriptor, Connection, ConnectionState, Environment, IpdDescriptor,
    IrdDescriptor, OdbcResult, Statement, StatementState, conn_from_handle,
    diagnostic::DiagnosticInfo,
    runtime::{env_allocated, env_freed, global},
};
use odbc_sys as sql;
use sf_core::protobuf::generated::database_driver_v1::{
    StatementNewRequest, StatementReleaseRequest,
};
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;
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
            let response = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
                c.statement_new(StatementNewRequest {
                    conn_handle: Some(*conn_handle),
                })
                .await
            })?;
            let stmt_handle = response
                .stmt_handle
                .required("Statement handle is required")?;

            let stmt = Box::new(Statement {
                conn,
                stmt_handle,
                state: StatementState::Created.into(),
                ard: ArdDescriptor::new(),
                ird: IrdDescriptor::new(),
                apd: ApdDescriptor::new(),
                ipd: IpdDescriptor::new(),
                diagnostic_info: DiagnosticInfo::default(),
                get_data_state: None,
                cursor_type: crate::api::CursorType::ForwardOnly,
                max_length: 0,
                used_extended_fetch: false,
                last_query_id: None,
                cancel_token: CancellationToken::new(),
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
    env_freed().context(OdbcRuntimeSnafu)?;
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
    let stmt = unsafe { Box::from_raw(handle as *mut Statement) };
    global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
        c.statement_release(StatementReleaseRequest {
            stmt_handle: Some(stmt.stmt_handle),
        })
        .await
    })?;
    Ok(())
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
