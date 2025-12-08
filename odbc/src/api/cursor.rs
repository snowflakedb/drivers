use crate::api::error::InvalidCursorStateSnafu;
///! Cursor Management Functions
///!
///! Implements SQLCloseCursor, SQLFreeStmt, SQLMoreResults, and SQLCancel for ODBC.
use crate::api::{OdbcResult, StatementState, stmt_from_handle};
use odbc_sys as sql;
use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf_gen::database_driver_v1::StatementCancelRequest;
use std::fs::OpenOptions;
use std::io::Write;
use tracing;

/// SQLCloseCursor - Close cursor and discard pending results
pub fn close_cursor(statement_handle: sql::Handle) -> OdbcResult<()> {
    tracing::debug!("close_cursor");

    let stmt = stmt_from_handle(statement_handle);

    // Check if there's actually a cursor open using the has_cursor flag
    // This flag is set to true for SELECT queries and false for DDL statements
    let has_cursor = stmt.has_cursor;

    if !has_cursor {
        // No cursor to close - return error like the official driver does
        // This prevents iODBC from transitioning its state machine,
        // allowing SQLRowCount to still work after SQLCloseCursor on DDL statements
        tracing::debug!("close_cursor: no cursor open, returning InvalidCursorState");
        return Err(InvalidCursorStateSnafu.build());
    }

    // Emit telemetry for rows consumed before resetting state
    let consumed = stmt.current_row;
    if consumed > 0 {
        let message = format!(
            "Telemetry for number of consumed rows, consumed {} rows in total {} rows",
            consumed, consumed
        );
        tracing::info!("{message}");

        if let Some(settings) = &stmt.conn.log_settings {
            let _ = OpenOptions::new()
                .create(true)
                .append(true)
                .open(settings.generic_log_file())
                .and_then(|mut file| writeln!(file, "{message}"));
        }
    }

    // Reset cursor position but DON'T change state
    // This allows SQLRowCount to still work after SQLCloseCursor
    stmt.current_row = 0;
    stmt.has_cursor = false; // Cursor is now closed

    tracing::debug!("close_cursor: cursor closed, state unchanged");
    Ok(())
}

/// SQLFreeStmt - Free statement resources with options
pub fn free_stmt(statement_handle: sql::Handle, option: sql::USmallInt) -> OdbcResult<()> {
    tracing::debug!("free_stmt: option={}", option);

    let stmt = stmt_from_handle(statement_handle);

    const SQL_CLOSE: u16 = 0;
    const SQL_DROP: u16 = 1;
    const SQL_UNBIND: u16 = 2;
    const SQL_RESET_PARAMS: u16 = 3;

    match option {
        SQL_CLOSE => {
            // Close cursor
            stmt.state = StatementState::Created.into();
            stmt.current_row = 0;
        }
        SQL_DROP => {
            // This should free the handle, but that's done by SQLFreeHandle
            // Just close the cursor here
            stmt.state = StatementState::Created.into();
            stmt.current_row = 0;
        }
        SQL_UNBIND => {
            // Unbind all columns
            stmt.column_bindings.clear();
        }
        SQL_RESET_PARAMS => {
            // Reset all parameters
            stmt.parameter_bindings.clear();
        }
        _ => {
            tracing::warn!("free_stmt: unsupported option={}", option);
        }
    }

    Ok(())
}

/// SQLMoreResults - Move to next result set
pub fn more_results(statement_handle: sql::Handle) -> OdbcResult<()> {
    tracing::debug!("more_results");

    let stmt = stmt_from_handle(statement_handle);

    // Check if we have more child results to process
    stmt.current_result_index += 1;

    if stmt.current_result_index < stmt.child_result_ids.len() {
        let child_id = stmt.child_result_ids[stmt.current_result_index].clone();
        tracing::info!(
            "more_results: fetching child result {} (index {})",
            child_id,
            stmt.current_result_index
        );

        // Fetch the next child result
        let handle = sf_core::handle_manager::Handle {
            id: stmt.stmt_handle.id as u64,
            magic: stmt.stmt_handle.magic as u64,
        };

        match sf_core::apis::database_driver_v1::fetch_child_result(handle, &child_id) {
            Ok(result) => {
                stmt.last_rows_affected = result.rows_affected;
                stmt.last_query_id = result.query_id.clone();
                // Reset cursor position for new result set
                stmt.current_row = 0;
                stmt.state = StatementState::Done.into(); // Will be updated when fetching
                Ok(())
            }
            Err(e) => {
                tracing::error!("more_results: failed to fetch child result: {e}");
                Err(crate::api::error::NoMoreDataSnafu.build())
            }
        }
    } else {
        // No more results
        tracing::debug!("more_results: no more child results");
        Err(crate::api::error::NoMoreDataSnafu.build())
    }
}

/// SQLCancel - Cancel a running statement
pub fn cancel(statement_handle: sql::Handle) -> OdbcResult<()> {
    tracing::debug!("cancel");

    let stmt = stmt_from_handle(statement_handle);

    DatabaseDriverClient::statement_cancel(StatementCancelRequest {
        stmt_handle: Some(stmt.stmt_handle),
    })?;

    stmt.state = StatementState::Created.into();
    stmt.current_row = 0;

    tracing::info!("cancel: cancel request submitted");
    Ok(())
}
