use crate::api::error::{ConversionSnafu, StatementNotExecutedSnafu};
use crate::api::{DescField, OdbcResult, StatementState, stmt_from_handle};
use crate::conversion::sql_type_from_field;
use arrow::array::RecordBatchReader;
use odbc_sys as sql;
use snafu::ResultExt;
use tracing;

/// Get the number of result columns
pub fn num_result_cols(
    statement_handle: sql::Handle,
    column_count_ptr: *mut sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("num_result_cols called");
    let stmt = stmt_from_handle(statement_handle);

    let num_cols = match stmt.state.as_ref() {
        StatementState::Executed { reader, .. } => reader.schema().fields().len() as sql::SmallInt,
        StatementState::Fetching { record_batch, .. } => {
            record_batch.schema().fields().len() as sql::SmallInt
        }
        StatementState::NoResultSet => 0,
        _ => return StatementNotExecutedSnafu.fail(),
    };

    if column_count_ptr.is_null() {
        tracing::warn!("num_result_cols: null column_count_ptr");
        return crate::api::error::NullPointerSnafu.fail();
    }
    unsafe {
        std::ptr::write(column_count_ptr, num_cols);
    }
    Ok(())
}

/// Get the number of affected rows
pub fn row_count(statement_handle: sql::Handle, row_count_ptr: *mut sql::Len) -> OdbcResult<()> {
    tracing::debug!("row_count called");
    let stmt = stmt_from_handle(statement_handle);
    let row_count = match stmt.state.as_ref() {
        StatementState::Executed { rows_affected, .. }
        | StatementState::Fetching { rows_affected, .. } => rows_affected.unwrap_or(0) as sql::Len,
        StatementState::NoResultSet => -1,
        _ => return StatementNotExecutedSnafu.fail(),
    };

    if row_count_ptr.is_null() {
        tracing::warn!("row_count: null row_count_ptr");
        return crate::api::error::NullPointerSnafu.fail();
    }
    unsafe {
        std::ptr::write(row_count_ptr, row_count);
    }
    Ok(())
}

/// Get a column attribute (SQLColAttribute)
pub fn col_attribute(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    field_identifier: sql::USmallInt,
    _character_attribute_ptr: sql::Pointer,
    _buffer_length: sql::SmallInt,
    _string_length_ptr: *mut sql::SmallInt,
    numeric_attribute_ptr: *mut sql::Len,
) -> OdbcResult<()> {
    tracing::debug!(
        "col_attribute: column_number={}, field_identifier={}",
        column_number,
        field_identifier
    );
    let stmt = stmt_from_handle(statement_handle);

    let schema = match stmt.state.as_ref() {
        StatementState::Fetching { record_batch, .. } => record_batch.schema(),
        StatementState::Executed { reader, .. } => reader.schema(),
        _ => return StatementNotExecutedSnafu.fail(),
    };

    // ODBC column numbers are 1-based; validate before indexing into the schema
    if column_number == 0 {
        tracing::warn!("col_attribute: invalid column_number=0");
        return StatementNotExecutedSnafu.fail();
    }
    let column_index = (column_number - 1) as usize;
    if column_index >= schema.fields().len() {
        tracing::warn!(
            "col_attribute: column_number={} out of range (num_fields={})",
            column_number,
            schema.fields().len()
        );
        return StatementNotExecutedSnafu.fail();
    }

    let field = schema.field(column_index);
    let desc_field = DescField::try_from(field_identifier as i16)?;

    match desc_field {
        DescField::Type | DescField::ConciseType => {
            let sql_type =
                sql_type_from_field(field, &stmt.conn.numeric_settings).context(ConversionSnafu)?;
            if !numeric_attribute_ptr.is_null() {
                unsafe {
                    std::ptr::write(numeric_attribute_ptr, sql_type.0 as sql::Len);
                }
            }
            Ok(())
        }
        _ => {
            tracing::warn!(
                "col_attribute: unsupported field_identifier={:?}",
                desc_field
            );
            Ok(())
        }
    }
}
