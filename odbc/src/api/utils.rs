use crate::api::encoding::{OdbcEncoding, write_string_bytes, write_string_chars};
use crate::api::error::{
    ConversionSnafu, InvalidBufferLengthSnafu, InvalidDescriptorIndexSnafu,
    StatementNotExecutedSnafu,
};
use crate::api::{DescField, OdbcResult, StatementState, stmt_from_handle};
use crate::conversion::warning::Warnings;
use crate::conversion::{column_size_from_field, decimal_digits_from_field, sql_type_from_field};
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
        StatementState::Prepared { schema } => schema.fields().len() as sql::SmallInt,
        StatementState::QueryExecuted { reader, .. } => {
            reader.schema().fields().len() as sql::SmallInt
        }
        StatementState::Fetching { record_batch, .. } => {
            record_batch.schema().fields().len() as sql::SmallInt
        }
        StatementState::DdlExecuted { .. } | StatementState::DmlExecuted { .. } => 0,
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
        StatementState::QueryExecuted { rows_affected, .. }
        | StatementState::Fetching { rows_affected, .. } => rows_affected.unwrap_or(0) as sql::Len,
        StatementState::DmlExecuted { rows_affected, .. } => *rows_affected as sql::Len,
        StatementState::DdlExecuted { .. } => -1,
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
#[allow(clippy::too_many_arguments)]
pub fn col_attribute<E: OdbcEncoding>(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    field_identifier: sql::USmallInt,
    character_attribute_ptr: *mut E::Char,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    numeric_attribute_ptr: *mut sql::Len,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    tracing::debug!(
        "col_attribute: column_number={}, field_identifier={}",
        column_number,
        field_identifier
    );
    let stmt = stmt_from_handle(statement_handle);

    let schema = match stmt.state.as_ref() {
        StatementState::QueryExecuted { reader, .. } => reader.schema(),
        StatementState::Fetching { record_batch, .. } => record_batch.schema(),
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
        DescField::TypeName => {
            let logical_type = field
                .metadata()
                .get("logicalType")
                .map(|s| s.as_str())
                .unwrap_or("");
            let type_name = match logical_type {
                "OBJECT" => "STRUCT",
                "ARRAY" => "ARRAY",
                "VARIANT" => "VARIANT",
                _ => "",
            };
            match logical_type {
                "OBJECT" | "ARRAY" | "VARIANT" => {
                    write_string_bytes::<E>(
                        type_name,
                        character_attribute_ptr,
                        buffer_length,
                        string_length_ptr,
                        Some(warnings),
                    );
                    Ok(())
                }
                _ => {
                    tracing::warn!(
                        "col_attribute: SQL_DESC_TYPE_NAME not yet implemented for logicalType={logical_type}"
                    );
                    write_string_bytes::<E>(
                        "",
                        character_attribute_ptr,
                        buffer_length,
                        string_length_ptr,
                        None,
                    );
                    Ok(())
                }
            }
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

/// Describe a column in the result set (SQLDescribeCol / SQLDescribeColW).
#[allow(clippy::too_many_arguments)]
pub fn describe_col<E: OdbcEncoding>(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    column_name: *mut E::Char,
    buffer_length: sql::SmallInt,
    name_length_ptr: *mut sql::SmallInt,
    data_type_ptr: *mut sql::SmallInt,
    column_size_ptr: *mut sql::ULen,
    decimal_digits_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    tracing::debug!("describe_col: column_number={column_number}");
    let stmt = stmt_from_handle(statement_handle);

    let schema = match stmt.state.as_ref() {
        StatementState::QueryExecuted { reader, .. } => reader.schema(),
        StatementState::Fetching { record_batch, .. } => record_batch.schema(),
        StatementState::Prepared { schema } => schema.clone(),
        _ => return StatementNotExecutedSnafu.fail(),
    };

    if column_number < 1 || (column_number as usize - 1) >= schema.fields().len() {
        return InvalidDescriptorIndexSnafu {
            number: column_number as sql::SmallInt,
        }
        .fail();
    }
    let col_idx = (column_number - 1) as usize;

    if buffer_length < 0 {
        return InvalidBufferLengthSnafu {
            length: buffer_length as i64,
        }
        .fail();
    }

    let field = schema.field(col_idx);

    let name = field.name();
    write_string_chars::<E>(
        name,
        column_name,
        buffer_length,
        name_length_ptr,
        Some(warnings),
    );

    if !data_type_ptr.is_null() {
        let sql_type =
            sql_type_from_field(field, &stmt.conn.numeric_settings).context(ConversionSnafu)?;
        unsafe { std::ptr::write(data_type_ptr, sql_type.0 as sql::SmallInt) };
    }

    if !column_size_ptr.is_null() {
        let col_size =
            column_size_from_field(field, &stmt.conn.numeric_settings).context(ConversionSnafu)?;
        unsafe { std::ptr::write(column_size_ptr, col_size) };
    }

    if !decimal_digits_ptr.is_null() {
        let digits = decimal_digits_from_field(field, &stmt.conn.numeric_settings)
            .context(ConversionSnafu)?;
        unsafe { std::ptr::write(decimal_digits_ptr, digits) };
    }

    if !nullable_ptr.is_null() {
        let nullable = if field.is_nullable() {
            sql::Nullability::NULLABLE.0
        } else {
            sql::Nullability::NO_NULLS.0
        };
        unsafe { std::ptr::write(nullable_ptr, nullable) };
    }

    Ok(())
}
