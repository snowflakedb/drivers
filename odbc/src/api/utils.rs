use crate::api::{OdbcResult, Statement, StatementState, stmt_from_handle};
use odbc_sys as sql;
use tracing;

pub fn resolve_schema(stmt: &Statement) -> Option<arrow::datatypes::SchemaRef> {
    match stmt.state.as_ref() {
        StatementState::Fetching { record_batch, .. } => Some(record_batch.schema()),
        StatementState::Executed { schema, .. } => Some(schema.clone()),
        _ => stmt.cached_schema.clone(),
    }
}

/// Get the number of result columns
pub fn num_result_cols(
    statement_handle: sql::Handle,
    column_count_ptr: *mut sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("num_result_cols called");
    let stmt = stmt_from_handle(statement_handle);

    let schema = match resolve_schema(stmt) {
        Some(schema) => schema,
        None => {
            tracing::warn!("num_result_cols: no schema available for statement");
            unsafe {
                std::ptr::write(column_count_ptr, 0);
            }
            return Ok(());
        }
    };

    let column_count = schema.fields().len() as sql::SmallInt;

    unsafe {
        std::ptr::write(column_count_ptr, column_count);
    }
    Ok(())
}

/// Get the number of affected rows
pub fn row_count(statement_handle: sql::Handle, row_count_ptr: *mut sql::Len) -> OdbcResult<()> {
    eprintln!("DEBUG row_count called");
    tracing::debug!("row_count called");
    let stmt = stmt_from_handle(statement_handle);
    eprintln!(
        "DEBUG row_count: last_rows_affected = {}",
        stmt.last_rows_affected
    );

    match stmt.state.as_ref() {
        StatementState::Executed { rows_affected, .. } => unsafe {
            std::ptr::write(row_count_ptr, *rows_affected as sql::Len);
        },
        _ => unsafe {
            // Use last_rows_affected even after cursor is closed
            std::ptr::write(row_count_ptr, stmt.last_rows_affected as sql::Len);
        },
    }
    Ok(())
}
