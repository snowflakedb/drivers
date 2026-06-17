use crate::api::encoding::{OdbcEncoding, write_string_bytes, write_string_chars};
use crate::api::error::{
    ConversionSnafu, InvalidBufferLengthSnafu, InvalidDescriptorIndexSnafu, NullPointerSnafu,
    StatementNotExecutedSnafu,
};
use crate::api::{DescField, OdbcResult, StatementState, stmt_from_handle};
use crate::conversion::warning::Warnings;
use crate::conversion::{
    column_size_from_field, decimal_digits_from_field, display_size_from_field,
    is_case_sensitive_from_field, is_unsigned_from_field, literal_prefix_from_field,
    literal_suffix_from_field, num_prec_radix_from_field, octet_length_from_field,
    precision_from_field, searchable_from_field, sql_type_from_field, type_name_from_field,
    verbose_sql_type_from_field,
};
use arrow::array::RecordBatchReader;
use odbc_sys as sql;
use snafu::ResultExt;
use tracing;

/// Process a catalog function string argument according to SQL_ATTR_METADATA_ID rules.
///
/// When `metadata_id` is `true`, the argument is treated as a case-insensitive identifier:
/// - `None` (NULL pointer) → `HY009` (`InvalidUseOfNullPointer`): identifier is required.
/// - Trailing spaces are stripped.
/// - The string is folded to uppercase.
///
/// When `metadata_id` is `false` (default), the argument is treated as an ordinary search
/// pattern and returned unchanged; `None` means "match all" (no filter applied).
///
/// Catalog functions must call this for every string argument except `TableType` in
/// `SQLTables` (which is always treated as an ordinary argument per the ODBC spec).
#[allow(dead_code)] // Used by catalog functions (SQLTables, SQLColumns, …) not yet implemented
pub(crate) fn process_catalog_arg(
    arg: Option<&str>,
    metadata_id: bool,
) -> OdbcResult<Option<String>> {
    match (arg, metadata_id) {
        (None, true) => NullPointerSnafu.fail(),
        (None, false) => Ok(None),
        (Some(s), true) => Ok(Some(s.trim_end().to_uppercase())),
        (Some(s), false) => Ok(Some(s.to_string())),
    }
}

/// Get the number of result columns
pub fn num_result_cols(
    statement_handle: sql::Handle,
    column_count_ptr: *mut sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("num_result_cols called");
    let guard = stmt_from_handle(statement_handle)?;
    let inner = guard.inner.lock();

    if inner.state.as_ref().is_async_executing() {
        return crate::api::error::AsyncInProgressSnafu.fail();
    }

    if inner.state.as_ref().is_need_data() {
        return crate::api::error::InvalidDuringDaeSnafu.fail();
    }

    let num_cols = match inner.state.as_ref() {
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
    let guard = stmt_from_handle(statement_handle)?;
    let inner = guard.inner.lock();

    if inner.state.as_ref().is_need_data() {
        return crate::api::error::InvalidDuringDaeSnafu.fail();
    }

    let row_count = match inner.state.as_ref() {
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

/// Get a column attribute (SQLColAttribute / SQLColAttributes)
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
    let guard = stmt_from_handle(statement_handle)?;
    let inner = guard.inner.lock();

    if inner.state.as_ref().is_need_data() {
        return crate::api::error::InvalidDuringDaeSnafu.fail();
    }

    let schema = match inner.state.as_ref() {
        StatementState::Prepared { schema } => schema.clone(),
        StatementState::QueryExecuted { reader, .. } => reader.schema(),
        StatementState::Fetching { record_batch, .. } => record_batch.schema(),
        _ => return StatementNotExecutedSnafu.fail(),
    };

    let desc_field = DescField::try_from(field_identifier as i16)?;

    // SQL_DESC_COUNT / SQL_COLUMN_COUNT don't require a valid column number
    if matches!(desc_field, DescField::Count | DescField::ColumnCount) {
        write_numeric(numeric_attribute_ptr, schema.fields().len() as sql::Len);
        return Ok(());
    }

    // Validate column number (1-based)
    if column_number < 1 || (column_number as usize - 1) >= schema.fields().len() {
        return InvalidDescriptorIndexSnafu {
            number: column_number as sql::SmallInt,
        }
        .fail();
    }
    let column_index = (column_number - 1) as usize;
    let field = schema.field(column_index);
    let dbc = guard.conn()?;
    let numeric_settings = dbc.connection.lock().numeric_settings;

    match compute_ird_field(field, desc_field, &numeric_settings)? {
        IrdFieldValue::SmallInt(v) => write_numeric(numeric_attribute_ptr, v as sql::Len),
        IrdFieldValue::Integer(v) => write_numeric(numeric_attribute_ptr, v as sql::Len),
        IrdFieldValue::Len(v) => write_numeric(numeric_attribute_ptr, v),
        IrdFieldValue::Str(s) => {
            write_string_bytes::<E>(
                s,
                character_attribute_ptr,
                buffer_length,
                string_length_ptr,
                Some(warnings),
            );
        }
    }

    Ok(())
}

fn write_numeric(ptr: *mut sql::Len, value: sql::Len) {
    if !ptr.is_null() {
        unsafe { std::ptr::write(ptr, value) };
    }
}

/// Result of computing an IRD record field value from Arrow metadata.
///
/// Variants match the C type that `SQLGetDescField` writes for each IRD field
/// (per the ODBC spec table), so the descriptor path writes exactly the right
/// number of bytes.  `SQLColAttribute` widens everything to `SQLLEN`.
pub(crate) enum IrdFieldValue<'a> {
    SmallInt(sql::SmallInt),
    Integer(sql::Integer),
    Len(sql::Len),
    Str(&'a str),
}

/// Compute the value of an IRD record field from Arrow metadata.
/// Shared logic between `SQLColAttribute` and `SQLGetDescField(IRD)`.
pub(crate) fn compute_ird_field<'a>(
    field: &'a arrow::datatypes::Field,
    desc_field: DescField,
    numeric_settings: &crate::conversion::NumericSettings,
) -> OdbcResult<IrdFieldValue<'a>> {
    use IrdFieldValue::*;
    match desc_field {
        DescField::Type => {
            let t =
                verbose_sql_type_from_field(field, numeric_settings).context(ConversionSnafu)?;
            Ok(SmallInt(t.0))
        }
        DescField::ConciseType => {
            let t = sql_type_from_field(field, numeric_settings).context(ConversionSnafu)?;
            Ok(SmallInt(t.0))
        }
        DescField::Nullable | DescField::ColumnNullable => {
            let val = if field.is_nullable() {
                sql::Nullability::NULLABLE.0
            } else {
                sql::Nullability::NO_NULLS.0
            };
            Ok(SmallInt(val))
        }
        DescField::Precision | DescField::ColumnPrecision => {
            let v = precision_from_field(field, numeric_settings).context(ConversionSnafu)?;
            Ok(SmallInt(v as sql::SmallInt))
        }
        DescField::Scale | DescField::ColumnScale => {
            let v = decimal_digits_from_field(field, numeric_settings).context(ConversionSnafu)?;
            Ok(SmallInt(v))
        }
        DescField::Length => {
            let v = column_size_from_field(field, numeric_settings).context(ConversionSnafu)?;
            Ok(Len(v as sql::Len))
        }
        DescField::OctetLength | DescField::ColumnLength => {
            let v = octet_length_from_field(field, numeric_settings).context(ConversionSnafu)?;
            Ok(Len(v))
        }
        DescField::DisplaySize => {
            let v = display_size_from_field(field, numeric_settings).context(ConversionSnafu)?;
            Ok(Len(v))
        }
        DescField::NumPrecRadix => {
            let v = num_prec_radix_from_field(field, numeric_settings).context(ConversionSnafu)?;
            Ok(Integer(v as sql::Integer))
        }
        DescField::Unsigned => {
            let v = is_unsigned_from_field(field, numeric_settings).context(ConversionSnafu)?;
            Ok(SmallInt(if v { 1 } else { 0 }))
        }
        DescField::CaseSensitive => {
            let v =
                is_case_sensitive_from_field(field, numeric_settings).context(ConversionSnafu)?;
            Ok(Integer(if v { 1 } else { 0 }))
        }
        DescField::Searchable => {
            let v = searchable_from_field(field, numeric_settings).context(ConversionSnafu)?;
            Ok(SmallInt(v as sql::SmallInt))
        }
        DescField::Updatable => Ok(SmallInt(2)), // SQL_ATTR_READWRITE_UNKNOWN
        DescField::AutoUniqueValue => Ok(Integer(0)), // SQL_FALSE
        DescField::FixedPrecScale => Ok(SmallInt(0)), // SQL_FALSE
        DescField::Unnamed => Ok(SmallInt(0)),   // SQL_NAMED
        DescField::Name | DescField::ColumnName | DescField::Label | DescField::BaseColumnName => {
            Ok(Str(field.name()))
        }
        DescField::TableName
        | DescField::BaseTableName
        | DescField::CatalogName
        | DescField::SchemaName => Ok(Str("")),
        DescField::TypeName | DescField::LocalTypeName => {
            let name = type_name_from_field(field, numeric_settings).context(ConversionSnafu)?;
            Ok(Str(name))
        }
        DescField::LiteralPrefix => {
            let v = literal_prefix_from_field(field, numeric_settings).context(ConversionSnafu)?;
            Ok(Str(v))
        }
        DescField::LiteralSuffix => {
            let v = literal_suffix_from_field(field, numeric_settings).context(ConversionSnafu)?;
            Ok(Str(v))
        }
        _ => crate::api::error::UnknownAttributeSnafu {
            attribute: desc_field as i32,
        }
        .fail(),
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
    let guard = stmt_from_handle(statement_handle)?;
    let inner = guard.inner.lock();

    if inner.state.as_ref().is_need_data() {
        return crate::api::error::InvalidDuringDaeSnafu.fail();
    }

    let schema = match inner.state.as_ref() {
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
    let dbc = guard.conn()?;
    let numeric_settings = dbc.connection.lock().numeric_settings;

    let name = field.name();
    write_string_chars::<E>(
        name,
        column_name,
        buffer_length,
        name_length_ptr,
        Some(warnings),
    );

    if !data_type_ptr.is_null() {
        let sql_type = sql_type_from_field(field, &numeric_settings).context(ConversionSnafu)?;
        unsafe { std::ptr::write(data_type_ptr, sql_type.0 as sql::SmallInt) };
    }

    if !column_size_ptr.is_null() {
        let col_size = column_size_from_field(field, &numeric_settings).context(ConversionSnafu)?;
        unsafe { std::ptr::write(column_size_ptr, col_size) };
    }

    if !decimal_digits_ptr.is_null() {
        let digits =
            decimal_digits_from_field(field, &numeric_settings).context(ConversionSnafu)?;
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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::error::OdbcError;

    // ---- process_catalog_arg: SQL_FALSE (pattern mode) ----

    #[test]
    fn catalog_arg_none_pattern_mode_returns_none() {
        assert_eq!(process_catalog_arg(None, false).unwrap(), None);
    }

    #[test]
    fn catalog_arg_some_pattern_mode_returns_unchanged() {
        assert_eq!(
            process_catalog_arg(Some("Hello World"), false).unwrap(),
            Some("Hello World".to_string())
        );
    }

    #[test]
    fn catalog_arg_pattern_mode_preserves_trailing_spaces() {
        assert_eq!(
            process_catalog_arg(Some("hello  "), false).unwrap(),
            Some("hello  ".to_string())
        );
    }

    #[test]
    fn catalog_arg_pattern_mode_preserves_leading_spaces() {
        assert_eq!(
            process_catalog_arg(Some("  hello"), false).unwrap(),
            Some("  hello".to_string())
        );
    }

    #[test]
    fn catalog_arg_empty_string_pattern_mode() {
        assert_eq!(
            process_catalog_arg(Some(""), false).unwrap(),
            Some("".to_string())
        );
    }

    // ---- process_catalog_arg: SQL_TRUE (identifier mode) ----

    #[test]
    fn catalog_arg_none_identifier_mode_returns_hy009() {
        let result = process_catalog_arg(None, true);
        assert!(matches!(result, Err(OdbcError::NullPointer { .. })));
    }

    #[test]
    fn catalog_arg_identifier_mode_uppercases() {
        assert_eq!(
            process_catalog_arg(Some("hello"), true).unwrap(),
            Some("HELLO".to_string())
        );
    }

    #[test]
    fn catalog_arg_identifier_mode_strips_trailing_spaces() {
        assert_eq!(
            process_catalog_arg(Some("  foo  "), true).unwrap(),
            Some("  FOO".to_string())
        );
    }

    #[test]
    fn catalog_arg_identifier_mode_preserves_leading_spaces() {
        assert_eq!(
            process_catalog_arg(Some("  hello"), true).unwrap(),
            Some("  HELLO".to_string())
        );
    }

    #[test]
    fn catalog_arg_empty_string_identifier_mode() {
        assert_eq!(
            process_catalog_arg(Some(""), true).unwrap(),
            Some("".to_string())
        );
    }

    #[test]
    fn catalog_arg_only_spaces_identifier_mode_strips_all() {
        assert_eq!(
            process_catalog_arg(Some("   "), true).unwrap(),
            Some("".to_string())
        );
    }

    #[test]
    fn catalog_arg_mixed_case_identifier_mode() {
        assert_eq!(
            process_catalog_arg(Some("MyTable"), true).unwrap(),
            Some("MYTABLE".to_string())
        );
    }

    // ---- process_catalog_arg: Unicode uppercasing ----

    #[test]
    fn catalog_arg_identifier_mode_uppercases_accented_latin() {
        // Basic Latin accented characters: é → É, ñ → Ñ
        assert_eq!(
            process_catalog_arg(Some("résumé"), true).unwrap(),
            Some("RÉSUMÉ".to_string())
        );
    }

    #[test]
    fn catalog_arg_identifier_mode_uppercases_german_sharp_s() {
        // ß has no single-char uppercase in Unicode — it maps to the two-char sequence "SS"
        assert_eq!(
            process_catalog_arg(Some("straße"), true).unwrap(),
            Some("STRASSE".to_string())
        );
    }

    #[test]
    fn catalog_arg_identifier_mode_uppercases_greek() {
        assert_eq!(
            process_catalog_arg(Some("ελληνικά"), true).unwrap(),
            Some("ΕΛΛΗΝΙΚΆ".to_string())
        );
    }

    #[test]
    fn catalog_arg_pattern_mode_preserves_unicode_unchanged() {
        // In pattern mode nothing should be transformed regardless of script
        assert_eq!(
            process_catalog_arg(Some("straße"), false).unwrap(),
            Some("straße".to_string())
        );
    }
}
