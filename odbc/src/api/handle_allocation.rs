use crate::{
    api::{
        Connection, ConnectionState, Environment, LargeObjectSettings, OdbcResult, ParamBindType,
        Statement, StatementState, TimestampLtzFormat, conn_from_handle,
        diagnostic::DiagnosticInfo,
        error::{DisconnectedSnafu, InvalidHandleSnafu, Required},
    },
    timezone::normalize_timezone_name,
};
use odbc_sys as sql;
use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf_gen::database_driver_v1::StatementNewRequest;
use std::sync::OnceLock;
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
        timestamp_ltz_format: TimestampLtzFormat::new(true, false),
        timestamp_ntz_format: TimestampLtzFormat::new(true, false),
        timestamp_tz_format: TimestampLtzFormat::new(true, true),
        timestamp_type_mapping: crate::api::types::TimestampType::Ltz,
        log_settings: None,
        session_timezone: None,
        lob_settings: LargeObjectSettings::default(),
        use_custom_sql_types: false,
        current_catalog: None,
        use_current_catalog: false,
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
            })
            .map_err(|err| {
                eprintln!("statement_new failed: {err:?}");
                err
            })?;
            let stmt_handle = response
                .stmt_handle
                .required("Statement handle is required")?;

            // Retrieve session timezone from the connection cache; fall back to core if needed
            let mut session_timezone = conn.session_timezone.clone();
            if session_timezone.is_none() {
                let core_handle = sf_core::apis::database_driver_v1::Handle {
                    id: conn_handle.id as u64,
                    magic: conn_handle.magic as u64,
                };
                session_timezone =
                    sf_core::apis::database_driver_v1::connection_get_timezone(core_handle)
                        .ok()
                        .flatten();
                if let Some(value) = &session_timezone {
                    let normalized = normalize_timezone_name(value);
                    conn.session_timezone = Some(normalized.clone());
                    session_timezone = Some(normalized);
                }
            } else if let Some(value) = &session_timezone {
                let normalized = normalize_timezone_name(value);
                if normalized != *value {
                    conn.session_timezone = Some(normalized.clone());
                    session_timezone = Some(normalized);
                }
            }

            let stmt = Box::new(Statement {
                conn,
                stmt_handle,
                state: StatementState::Created.into(),
                cached_schema: None,
                is_prepared: false,
                parameter_bindings: std::collections::HashMap::new(),
                column_bindings: std::collections::HashMap::new(),
                diagnostic_info: DiagnosticInfo::default(),
                query_timeout: 0,
                max_rows: 0,
                current_row: 0,
                row_bind_type: 0,
                row_array_size: 1,
                last_rows_affected: 0,
                multi_statement_count: 1, // Default to single statement
                paramset_size: 1,
                param_status_ptr: None,
                params_processed_ptr: None,
                param_bind_type: ParamBindType::Column,
                rows_fetched_ptr: None,
                row_status_ptr: None,
                session_timezone,
                prepared_query: None,
                child_result_ids: Vec::new(),
                current_result_index: 0,
                has_cursor: false, // No cursor until a SELECT is executed
                metadata_id: false,
                last_query_id: None,
                data_at_exec_state: None,
            });
            Ok(Box::into_raw(stmt))
        }
        ConnectionState::Disconnected => {
            tracing::error!("Cannot allocate statement: connection is disconnected");
            eprintln!("alloc_statement: connection is disconnected");
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
    // Safety: We need to be careful here because the Statement contains
    // a reference to the Connection. If the Connection has been freed,
    // dropping the Statement could cause a use-after-free.
    // For now, we just drop the Box and hope the caller frees things in order.
    unsafe {
        let stmt = Box::from_raw(handle as *mut Statement);
        // Explicitly drop the statement
        drop(stmt);
    }
    Ok(())
}

/// Initialize logging (helper function for allocation)
fn init_logging() {
    static LOGGING_INIT: OnceLock<Result<(), sf_core::logging::LogError>> = OnceLock::new();
    let result = LOGGING_INIT.get_or_init(|| {
        sf_core::logging::init(sf_core::logging::LoggingConfig::new(None, true, false))
    });
    if let Err(err) = result {
        eprintln!("Failed to initialize logging: {err:?}");
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
