use crate::api::{OdbcResult, StatementState, stmt_from_handle};
use odbc_sys as sql;
use tracing;

/// Get the number of result columns
pub fn num_result_cols(
    _statement_handle: sql::Handle,
    column_count_ptr: *mut sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("num_result_cols called");
    let int_ptr = column_count_ptr as *mut i32;
    unsafe {
        std::ptr::write(int_ptr, 1);
    }
    Ok(())
}

/// Get the number of affected rows
pub fn row_count(statement_handle: sql::Handle, row_count_ptr: *mut sql::Len) -> OdbcResult<()> {
    tracing::debug!("row_count called");
    let stmt = stmt_from_handle(statement_handle);
    let row_count_ptr = row_count_ptr as *mut i32;

    match &mut stmt.state {
        StatementState::Executed { result } => unsafe {
            std::ptr::write(row_count_ptr, result.rows_affected as i32);
        },
        _ => unsafe {
            std::ptr::write(row_count_ptr, 0);
        },
    }
    Ok(())
}
