use crate::api::utils::resolve_schema;
///! Column Metadata Functions
///!
///! Implements SQLDescribeCol and SQLColAttribute for ODBC.
use crate::api::{
    OdbcResult, StatementState, error::InvalidColumnNumberSnafu, stmt_from_handle,
    types::LargeObjectSettings,
};
use arrow::datatypes::{DataType, Field};
use odbc_sys as sql;
use tracing;

// ODBC descriptor field identifiers
const DESC_AUTO_UNIQUE_VALUE: u32 = 11;
const DESC_BASE_COLUMN_NAME: u32 = 22;
const DESC_BASE_TABLE_NAME: u32 = 23;
const DESC_CASE_SENSITIVE: u32 = 12;
const DESC_CATALOG_NAME: u32 = 17;
const DESC_CONCISE_TYPE: u32 = 2;
const DESC_COUNT: u32 = 1001;
const DESC_DISPLAY_SIZE: u32 = 6;
const DESC_FIXED_PREC_SCALE: u32 = 9;
const DESC_LABEL: u32 = 18;
const DESC_LENGTH: u32 = 1003;
const DESC_LITERAL_PREFIX: u32 = 27;
const DESC_LITERAL_SUFFIX: u32 = 28;
const DESC_LOCAL_TYPE_NAME: u32 = 29;
const DESC_NAME: u32 = 1011;
const DESC_NULLABLE: u32 = 1008;
const DESC_NUM_PREC_RADIX: u32 = 32;
const DESC_OCTET_LENGTH: u32 = 1013;
const DESC_PRECISION: u32 = 1005;
const DESC_SCALE: u32 = 1006;
const DESC_SCHEMA_NAME: u32 = 16;
const DESC_SEARCHABLE: u32 = 13;
const DESC_TABLE_NAME: u32 = 15;
const DESC_TYPE: u32 = 1002;
const DESC_TYPE_NAME: u32 = 14;
const DESC_UNNAMED: u32 = 1012;
const DESC_UNSIGNED: u32 = 8;
const DESC_UPDATABLE: u32 = 10;

// ODBC constants
const SQL_TRUE: i32 = 1;
const SQL_FALSE: i32 = 0;
const SQL_NAMED: i16 = 0;
const SQL_UNNAMED: i16 = 1;
const SQL_PRED_BASIC: i32 = 2;
const SQL_PRED_SEARCHABLE: i32 = 3;
const SQL_ATTR_READONLY: i32 = 0;
const SQL_ATTR_READWRITE_UNKNOWN: i32 = 2;

// Nullable values
const SQL_NO_NULLS: i16 = 0;
const SQL_NULLABLE: i16 = 1;

// SQL data type constants (from sql.h)
const SQL_BIT: i16 = -7;
const SQL_TINYINT: i16 = -6;
const SQL_SMALLINT: i16 = 5;
const SQL_INTEGER: i16 = 4;
const SQL_BIGINT: i16 = -5;
const SQL_REAL: i16 = 7;
const SQL_DOUBLE: i16 = 8;
const SQL_NUMERIC: i16 = 2;
const SQL_DECIMAL: i16 = 3;
const SQL_VARCHAR: i16 = 12;
const SQL_VARBINARY: i16 = -3;
const SQL_BINARY: i16 = -2;
const SQL_DATETIME: i16 = 9;
const SQL_TYPE_DATE: i16 = 91;
const SQL_TYPE_TIME: i16 = 92;
const SQL_TYPE_TIMESTAMP: i16 = 93;
const DISPLAY_SIZE_DECIMAL: sql::Integer = 136;
const OCTET_LENGTH_DECIMAL: sql::Integer = 136;
const MAX_BINARY_LENGTH: i64 = 8_388_608; // 8 MB default when metadata is missing
const UTF8_MAX_BYTES_PER_CHAR: sql::Integer = 4;
const LARGE_VARCHAR_THRESHOLD: sql::Integer = 16_777_216;

fn snowflake_type(field: &Field) -> Option<&str> {
    field.metadata().get("snowflakeType").map(|s| s.as_str())
}

fn logical_type(field: &Field) -> Option<&str> {
    field.metadata().get("logicalType").map(|s| s.as_str())
}

fn logical_type_matches(field: &Field, ty: &str) -> bool {
    logical_type(field)
        .map(|value| value.eq_ignore_ascii_case(ty))
        .unwrap_or(false)
}

fn is_timestamp_logical(field: &Field) -> bool {
    ["TIMESTAMP_NTZ", "TIMESTAMP_LTZ", "TIMESTAMP_TZ"]
        .iter()
        .any(|ty| logical_type_matches(field, ty))
}

fn timestamp_column_size(field: &Field) -> sql::ULen {
    field
        .metadata()
        .get("precision")
        .and_then(|s| s.parse::<sql::ULen>().ok())
        .unwrap_or(29)
}

fn timestamp_scale(field: &Field) -> sql::SmallInt {
    field
        .metadata()
        .get("scale")
        .and_then(|s| s.parse::<sql::SmallInt>().ok())
        .unwrap_or(9)
}

fn timestamp_byte_length(field: &Field) -> sql::Integer {
    field
        .metadata()
        .get("byteLength")
        .and_then(|s| s.parse::<sql::Integer>().ok())
        .unwrap_or(16)
}

fn is_decfloat(field: &Field) -> bool {
    snowflake_type(field)
        .map(|t| t.eq_ignore_ascii_case("DECFLOAT"))
        .unwrap_or(false)
}

fn binary_length_metadata(field: &Field) -> Option<i64> {
    let metadata = field.metadata();
    for key in ["byteLength", "length", "maxLength"] {
        if let Some(value) = metadata.get(key) {
            if let Ok(parsed) = value.parse::<i64>() {
                return Some(parsed);
            }
        }
    }
    tracing::debug!(
        field = field.name(),
        metadata = ?metadata,
        "Binary length metadata missing expected keys"
    );
    None
}

fn clamp_to_limit(value: sql::ULen, limit: Option<i64>) -> sql::ULen {
    if let Some(limit) = limit {
        if limit > 0 {
            return value.min(limit as sql::ULen);
        }
    }
    value
}

fn varchar_length_from_metadata(field: &Field) -> Option<sql::ULen> {
    field
        .metadata()
        .get("charLength")
        .and_then(|s| s.parse::<sql::ULen>().ok())
}

fn fallback_varchar_length(settings: &LargeObjectSettings) -> Option<sql::ULen> {
    if settings.enable_large_varchar_binary.unwrap_or(false) {
        settings
            .max_lob_size_in_memory
            .map(|value| value.max(0) as sql::ULen)
    } else {
        None
    }
}

fn effective_varchar_length(field: &Field, settings: &LargeObjectSettings) -> sql::ULen {
    let base = varchar_length_from_metadata(field)
        .or_else(|| fallback_varchar_length(settings))
        .unwrap_or(16_777_216);
    clamp_to_limit(base, settings.default_varchar_size)
}

fn binary_length_from_metadata(field: &Field) -> Option<sql::ULen> {
    binary_length_metadata(field).map(|len| len.max(0) as sql::ULen)
}

fn fallback_binary_length(settings: &LargeObjectSettings) -> Option<sql::ULen> {
    if settings.enable_large_varchar_binary.unwrap_or(false) {
        settings
            .max_lob_size_in_memory
            .map(|value| (value / 2).max(0) as sql::ULen)
    } else {
        None
    }
}

fn effective_binary_length(field: &Field, settings: &LargeObjectSettings) -> sql::ULen {
    let base = binary_length_from_metadata(field)
        .or_else(|| fallback_binary_length(settings))
        .unwrap_or(MAX_BINARY_LENGTH as sql::ULen);
    clamp_to_limit(base, settings.default_binary_size)
}

fn clamp_to_sql_integer(value: sql::ULen) -> sql::Integer {
    value.min(sql::Integer::MAX as sql::ULen) as sql::Integer
}

fn fixed_precision(field: &Field) -> Option<sql::ULen> {
    field
        .metadata()
        .get("precision")
        .and_then(|s| s.parse::<sql::ULen>().ok())
}

fn fixed_scale(field: &Field) -> Option<sql::SmallInt> {
    field
        .metadata()
        .get("scale")
        .and_then(|s| s.parse::<sql::SmallInt>().ok())
}

fn map_sql_type(field: &Field) -> i16 {
    if is_timestamp_logical(field) {
        return SQL_TYPE_TIMESTAMP;
    }
    if logical_type_matches(field, "TIME") {
        return SQL_TYPE_TIME;
    }
    if logical_type_matches(field, "DATE") {
        return SQL_TYPE_DATE;
    }

    match field.data_type() {
        DataType::Boolean => SQL_BIT,
        DataType::Int8 => SQL_TINYINT,
        DataType::Int16 => SQL_SMALLINT,
        DataType::Int32 => SQL_INTEGER,
        DataType::Int64 => {
            if is_decfloat(field) {
                SQL_NUMERIC
            } else {
                SQL_BIGINT
            }
        }
        DataType::Float32 => SQL_REAL,
        DataType::Float64 => SQL_DOUBLE,
        DataType::Decimal128(_, _) => SQL_DECIMAL,
        DataType::Utf8 | DataType::LargeUtf8 => SQL_VARCHAR,
        DataType::Binary => SQL_BINARY,
        DataType::LargeBinary => SQL_VARBINARY,
        DataType::Date32 | DataType::Date64 => SQL_TYPE_DATE,
        DataType::Time32(_) | DataType::Time64(_) => SQL_TYPE_TIME,
        DataType::Timestamp(_, _) => SQL_TYPE_TIMESTAMP,
        _ => SQL_VARCHAR,
    }
}

fn map_sql_general_type(field: &Field) -> i16 {
    if is_timestamp_logical(field)
        || logical_type_matches(field, "TIME")
        || logical_type_matches(field, "DATE")
    {
        return SQL_DATETIME;
    }

    match field.data_type() {
        DataType::Timestamp(_, _)
        | DataType::Time32(_)
        | DataType::Time64(_)
        | DataType::Date32
        | DataType::Date64 => SQL_DATETIME,
        _ => map_sql_type(field),
    }
}

/// SQLDescribeCol - Get column metadata
pub fn describe_col(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    column_name: *mut sql::Char,
    buffer_length: sql::SmallInt,
    name_length_ptr: *mut sql::SmallInt,
    data_type_ptr: *mut sql::SmallInt,
    column_size_ptr: *mut sql::ULen,
    decimal_digits_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
) -> OdbcResult<()> {
    tracing::debug!("describe_col: column_number={}", column_number);

    let stmt = stmt_from_handle(statement_handle);
    let lob_settings = &stmt.conn.lob_settings;

    let schema = match resolve_schema(stmt) {
        Some(schema) => schema,
        None => {
            tracing::error!("describe_col: no schema available for statement");
            return InvalidColumnNumberSnafu.fail();
        }
    };
    let fields = schema.fields().clone();

    // ODBC uses 1-based indexing
    if column_number < 1 || column_number as usize > fields.len() {
        return InvalidColumnNumberSnafu.fail();
    }

    let field = &fields[column_number as usize - 1];
    tracing::debug!("col_attribute: field metadata {:?}", field.metadata());

    // Write column name
    if !column_name.is_null() && buffer_length > 0 {
        let name_bytes = field.name().as_bytes();
        let copy_len = (buffer_length as usize - 1).min(name_bytes.len());

        unsafe {
            std::ptr::copy_nonoverlapping(name_bytes.as_ptr(), column_name, copy_len);
            *column_name.add(copy_len) = 0; // Null terminator
        }

        if !name_length_ptr.is_null() {
            unsafe {
                *name_length_ptr = name_bytes.len() as sql::SmallInt;
            }
        }
    }

    // Map Arrow data type to SQL data type
    if !data_type_ptr.is_null() {
        let sql_type = map_sql_type(field);

        unsafe {
            *data_type_ptr = sql_type as sql::SmallInt;
        }
    }

    // Set column size
    if !column_size_ptr.is_null() {
        let size = if is_timestamp_logical(field) {
            timestamp_column_size(field)
        } else if let Some(precision) = fixed_precision(field) {
            precision
        } else {
            match field.data_type() {
                arrow::datatypes::DataType::Decimal128(precision, _) => *precision as sql::ULen,
                arrow::datatypes::DataType::Utf8 | arrow::datatypes::DataType::LargeUtf8 => {
                    effective_varchar_length(field, lob_settings)
                }
                arrow::datatypes::DataType::Binary | arrow::datatypes::DataType::LargeBinary => {
                    effective_binary_length(field, lob_settings)
                }
                arrow::datatypes::DataType::Int8 => 3,
                arrow::datatypes::DataType::Int16 => 5,
                arrow::datatypes::DataType::Int32 => 10,
                arrow::datatypes::DataType::Int64 => {
                    if is_decfloat(field) {
                        38
                    } else {
                        19
                    }
                }
                arrow::datatypes::DataType::Float32 => 7,
                arrow::datatypes::DataType::Float64 => 15,
                arrow::datatypes::DataType::Date32 | arrow::datatypes::DataType::Date64 => 10,
                arrow::datatypes::DataType::Time32(_) | arrow::datatypes::DataType::Time64(_) => {
                    let scale = field
                        .metadata()
                        .get("scale")
                        .and_then(|s| s.parse::<sql::ULen>().ok())
                        .unwrap_or(0);
                    if scale > 0 { 8 + 1 + scale } else { 8 }
                }
                arrow::datatypes::DataType::Timestamp(_, _) => 29,
                _ => 255,
            }
        };

        unsafe {
            *column_size_ptr = size;
        }
    }

    // Set decimal digits (scale)
    if !decimal_digits_ptr.is_null() {
        let scale = if is_timestamp_logical(field) {
            timestamp_scale(field)
        } else if let Some(scale) = fixed_scale(field) {
            scale
        } else {
            match field.data_type() {
                arrow::datatypes::DataType::Decimal128(_, scale) => *scale as sql::SmallInt,
                arrow::datatypes::DataType::Float32 | arrow::datatypes::DataType::Float64 => field
                    .metadata()
                    .get("scale")
                    .and_then(|s| s.parse::<sql::SmallInt>().ok())
                    .unwrap_or(0),
                arrow::datatypes::DataType::Time32(_) | arrow::datatypes::DataType::Time64(_) => {
                    field
                        .metadata()
                        .get("scale")
                        .and_then(|s| s.parse::<sql::SmallInt>().ok())
                        .unwrap_or(0)
                }
                arrow::datatypes::DataType::Timestamp(unit, _) => match unit {
                    arrow::datatypes::TimeUnit::Second => 0,
                    arrow::datatypes::TimeUnit::Millisecond => 3,
                    arrow::datatypes::TimeUnit::Microsecond => 6,
                    arrow::datatypes::TimeUnit::Nanosecond => 9,
                },
                _ => 0,
            }
        };

        unsafe {
            *decimal_digits_ptr = scale;
        }
    }

    // Set nullable
    if !nullable_ptr.is_null() {
        let nullable = if field.is_nullable() {
            SQL_NULLABLE
        } else {
            SQL_NO_NULLS
        };

        unsafe {
            *nullable_ptr = nullable;
        }
    }

    Ok(())
}

/// SQLColAttribute - Get column attribute
fn write_numeric_attr(numeric_attribute_ptr: *mut sql::Len, value: sql::Len) {
    if numeric_attribute_ptr.is_null() {
        return;
    }
    unsafe {
        *numeric_attribute_ptr = value;
    }
}

fn write_char_attr(
    character_attribute_ptr: *mut sql::Char,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    value: &str,
) {
    if !string_length_ptr.is_null() {
        unsafe {
            *string_length_ptr = value.len() as sql::SmallInt;
        }
    }

    if character_attribute_ptr.is_null() || buffer_length <= 0 {
        return;
    }

    let max_copy = buffer_length.saturating_sub(1) as usize;
    let bytes = value.as_bytes();
    let copy_len = bytes.len().min(max_copy);

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), character_attribute_ptr, copy_len);
        *character_attribute_ptr.add(copy_len) = 0;
    }
}

fn literal_prefix(field: &Field) -> &'static str {
    match field.data_type() {
        DataType::Utf8 | DataType::LargeUtf8 => "'",
        DataType::Timestamp(_, _)
        | DataType::Time32(_)
        | DataType::Time64(_)
        | DataType::Date32
        | DataType::Date64 => "'",
        DataType::Binary | DataType::LargeBinary => "0x",
        _ if is_timestamp_logical(field)
            || logical_type_matches(field, "TIME")
            || logical_type_matches(field, "DATE") =>
        {
            "'"
        }
        _ => "",
    }
}

fn literal_suffix(field: &Field) -> &'static str {
    match field.data_type() {
        DataType::Utf8 | DataType::LargeUtf8 => "'",
        DataType::Timestamp(_, _)
        | DataType::Time32(_)
        | DataType::Time64(_)
        | DataType::Date32
        | DataType::Date64 => "'",
        _ if is_timestamp_logical(field)
            || logical_type_matches(field, "TIME")
            || logical_type_matches(field, "DATE") =>
        {
            "'"
        }
        _ => "",
    }
}

fn odbc_type_name(field: &Field) -> String {
    let logical_type = field
        .metadata()
        .get("logicalType")
        .map(|s| s.to_ascii_uppercase());
    let ext_type_name = field
        .metadata()
        .get("extTypeName")
        .map(|s| s.to_ascii_uppercase());
    let snowflake_type = snowflake_type(field).map(|s| s.to_ascii_uppercase());

    if let Some(sf_type) = snowflake_type.as_deref() {
        if sf_type == "DECFLOAT" {
            return "NUMERIC".to_string();
        }
    }

    match ext_type_name.as_deref() {
        Some("GEOGRAPHY") => "GEOGRAPHY".to_string(),
        Some("GEOMETRY") => "GEOMETRY".to_string(),
        _ => match logical_type.as_deref() {
            Some("GEOGRAPHY") => "GEOGRAPHY".to_string(),
            Some("GEOMETRY") => "GEOMETRY".to_string(),
            Some("VARIANT") => "VARIANT".to_string(),
            Some("OBJECT") => "OBJECT".to_string(),
            Some("ARRAY") => "ARRAY".to_string(),
            Some("TIMESTAMP_NTZ") | Some("TIMESTAMP_LTZ") | Some("TIMESTAMP_TZ") => {
                "TYPE_TIMESTAMP".to_string()
            }
            Some("TIME") => "TYPE_TIME".to_string(),
            Some("DATE") => "DATE".to_string(),
            _ => match field.data_type() {
                DataType::Boolean => "BIT",
                DataType::Int8 => "TINYINT",
                DataType::Int16 => "SMALLINT",
                DataType::Int32 => "INTEGER",
                DataType::Int64 => "BIGINT",
                DataType::Float32 => "REAL",
                DataType::Float64 => "DOUBLE",
                DataType::Decimal128(_, _) => "DECIMAL",
                DataType::Utf8 | DataType::LargeUtf8 => "VARCHAR",
                DataType::Binary | DataType::LargeBinary => "BINARY",
                DataType::Date32 | DataType::Date64 => "DATE",
                DataType::Time32(_) | DataType::Time64(_) => "TYPE_TIME",
                DataType::Timestamp(_, _) => "TYPE_TIMESTAMP",
                _ => "VARCHAR",
            }
            .to_string(),
        },
    }
}

fn local_type_name(field: &Field) -> String {
    if is_timestamp_logical(field) {
        return "TYPE_TIMESTAMP".to_string();
    }

    match field.data_type() {
        DataType::Timestamp(_, _) => "TYPE_TIMESTAMP".to_string(),
        DataType::Time32(_) | DataType::Time64(_) => "TYPE_TIME".to_string(),
        _ if snowflake_type(field)
            .map(|t| t.eq_ignore_ascii_case("DECFLOAT"))
            .unwrap_or(false) =>
        {
            "NUMERIC".to_string()
        }
        _ => odbc_type_name(field),
    }
}

pub fn col_attribute(
    statement_handle: sql::Handle,
    column_number: sql::USmallInt,
    field_identifier: sql::USmallInt,
    character_attribute_ptr: *mut sql::Char,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    numeric_attribute_ptr: *mut sql::Len,
) -> OdbcResult<()> {
    let stmt = stmt_from_handle(statement_handle);
    let lob_settings = &stmt.conn.lob_settings;
    let state_name = match stmt.state.as_ref() {
        StatementState::Fetching { .. } => "Fetching",
        StatementState::Executed { .. } => "Executed",
        StatementState::Done => "Done",
        StatementState::Created => "Created",
        StatementState::Error => "Error",
    };
    tracing::debug!(
        "col_attribute: state={}, column_number={}, field_identifier={}",
        state_name,
        column_number,
        field_identifier
    );

    let schema = match resolve_schema(stmt) {
        Some(schema) => schema,
        None => {
            tracing::error!("col_attribute: no schema available for statement");
            return InvalidColumnNumberSnafu.fail();
        }
    };
    let fields = schema.fields().clone();

    // ODBC uses 1-based indexing
    if column_number < 1 || column_number as usize > fields.len() {
        return InvalidColumnNumberSnafu.fail();
    }

    let field = &fields[column_number as usize - 1];

    // Handle different field identifiers
    match field_identifier as u32 {
        DESC_AUTO_UNIQUE_VALUE => {
            // Snowflake doesn't have auto-increment
            write_numeric_attr(numeric_attribute_ptr, SQL_FALSE as sql::Len);
        }
        DESC_BASE_COLUMN_NAME | DESC_NAME | DESC_LABEL => {
            write_char_attr(
                character_attribute_ptr,
                buffer_length,
                string_length_ptr,
                field.name(),
            );
        }
        DESC_BASE_TABLE_NAME | DESC_TABLE_NAME | DESC_CATALOG_NAME | DESC_SCHEMA_NAME => {
            write_char_attr(
                character_attribute_ptr,
                buffer_length,
                string_length_ptr,
                "",
            );
        }
        DESC_LITERAL_PREFIX => {
            write_char_attr(
                character_attribute_ptr,
                buffer_length,
                string_length_ptr,
                literal_prefix(field),
            );
        }
        DESC_LITERAL_SUFFIX => {
            write_char_attr(
                character_attribute_ptr,
                buffer_length,
                string_length_ptr,
                literal_suffix(field),
            );
        }
        DESC_LOCAL_TYPE_NAME => {
            let type_name = local_type_name(field);
            write_char_attr(
                character_attribute_ptr,
                buffer_length,
                string_length_ptr,
                &type_name,
            );
        }
        DESC_CASE_SENSITIVE => {
            // Strings are case-sensitive in Snowflake
            let is_case_sensitive = matches!(
                field.data_type(),
                arrow::datatypes::DataType::Utf8 | arrow::datatypes::DataType::LargeUtf8
            );
            write_numeric_attr(
                numeric_attribute_ptr,
                if is_case_sensitive {
                    SQL_TRUE as sql::Len
                } else {
                    SQL_FALSE as sql::Len
                },
            );
        }
        DESC_CONCISE_TYPE | DESC_TYPE => {
            // SQL data type
            let sql_type = if field_identifier as u32 == DESC_TYPE {
                map_sql_general_type(field) as sql::Len
            } else {
                map_sql_type(field) as sql::Len
            };
            write_numeric_attr(numeric_attribute_ptr, sql_type);
        }
        DESC_COUNT => {
            // Number of columns
            write_numeric_attr(numeric_attribute_ptr, fields.len() as sql::Len);
        }
        DESC_DISPLAY_SIZE => {
            // Display size
            let size = if is_timestamp_logical(field) {
                timestamp_column_size(field) as sql::Integer
            } else if fixed_precision(field).is_some() {
                DISPLAY_SIZE_DECIMAL
            } else {
                match field.data_type() {
                    arrow::datatypes::DataType::Decimal128(precision, scale) => {
                        *precision as sql::Integer + if *scale > 0 { 2 } else { 0 }
                    }
                    arrow::datatypes::DataType::Utf8 | arrow::datatypes::DataType::LargeUtf8 => {
                        clamp_to_sql_integer(effective_varchar_length(field, lob_settings))
                    }
                    arrow::datatypes::DataType::Binary
                    | arrow::datatypes::DataType::LargeBinary => clamp_to_sql_integer(
                        effective_binary_length(field, lob_settings).saturating_mul(2),
                    ),
                    arrow::datatypes::DataType::Int8 => 4, // -128
                    arrow::datatypes::DataType::Int16 => 6, // -32768
                    arrow::datatypes::DataType::Int32 => 11, // -2147483648
                    arrow::datatypes::DataType::Int64 => 20, // -9223372036854775808
                    arrow::datatypes::DataType::Boolean => 1,
                    arrow::datatypes::DataType::Float32 => 14,
                    arrow::datatypes::DataType::Float64 => 24,
                    arrow::datatypes::DataType::Date32 | arrow::datatypes::DataType::Date64 => 10, // YYYY-MM-DD
                    arrow::datatypes::DataType::Time32(_)
                    | arrow::datatypes::DataType::Time64(_) => {
                        let scale = field
                            .metadata()
                            .get("scale")
                            .and_then(|s| s.parse::<sql::Integer>().ok())
                            .unwrap_or(0);
                        if scale > 0 { 8 + 1 + scale } else { 8 }
                    }
                    arrow::datatypes::DataType::Timestamp(_, _) => 29, // YYYY-MM-DD HH:MM:SS.NNNNNNNNN
                    _ => 255,
                }
            };
            write_numeric_attr(numeric_attribute_ptr, size as sql::Len);
        }
        DESC_FIXED_PREC_SCALE => {
            write_numeric_attr(numeric_attribute_ptr, SQL_FALSE as sql::Len);
        }
        DESC_NUM_PREC_RADIX => {
            let radix = if is_timestamp_logical(field) {
                0
            } else if fixed_precision(field).is_some()
                || matches!(
                    field.data_type(),
                    DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64
                )
            {
                10
            } else if matches!(field.data_type(), DataType::Float32 | DataType::Float64) {
                2
            } else {
                0
            };
            write_numeric_attr(numeric_attribute_ptr, radix as sql::Len);
        }
        DESC_LENGTH | DESC_OCTET_LENGTH => {
            // Column length
            let is_octet = field_identifier as u32 == DESC_OCTET_LENGTH;
            let length = if is_timestamp_logical(field) {
                if is_octet {
                    timestamp_byte_length(field)
                } else {
                    timestamp_column_size(field) as sql::Integer
                }
            } else if let Some(precision) = fixed_precision(field) {
                if is_octet {
                    OCTET_LENGTH_DECIMAL
                } else {
                    precision as sql::Integer
                }
            } else {
                match field.data_type() {
                    arrow::datatypes::DataType::Utf8 | arrow::datatypes::DataType::LargeUtf8 => {
                        let char_len = effective_varchar_length(field, lob_settings);
                        let len = if is_octet {
                            char_len.saturating_mul(UTF8_MAX_BYTES_PER_CHAR as sql::ULen)
                        } else {
                            char_len
                        };
                        clamp_to_sql_integer(len)
                    }
                    arrow::datatypes::DataType::Binary
                    | arrow::datatypes::DataType::LargeBinary => {
                        clamp_to_sql_integer(effective_binary_length(field, lob_settings))
                    }
                    arrow::datatypes::DataType::Boolean => 1,
                    arrow::datatypes::DataType::Date32 | arrow::datatypes::DataType::Date64 => {
                        if is_octet {
                            8
                        } else {
                            10
                        }
                    }
                    arrow::datatypes::DataType::Time32(_)
                    | arrow::datatypes::DataType::Time64(_) => {
                        let scale = field
                            .metadata()
                            .get("scale")
                            .and_then(|s| s.parse::<sql::Integer>().ok())
                            .unwrap_or(0);
                        if is_octet {
                            6
                        } else if scale > 0 {
                            8 + 1 + scale
                        } else {
                            8
                        }
                    }
                    arrow::datatypes::DataType::Timestamp(_, _) => {
                        if is_octet {
                            16
                        } else {
                            29
                        }
                    }
                    _ => 0,
                }
            };
            write_numeric_attr(numeric_attribute_ptr, length as sql::Len);
        }
        DESC_NULLABLE => {
            // Nullable
            let nullable = if field.is_nullable() {
                SQL_NULLABLE
            } else {
                SQL_NO_NULLS
            };
            write_numeric_attr(numeric_attribute_ptr, nullable as sql::Len);
        }
        DESC_PRECISION => {
            // Precision
            let precision = if is_timestamp_logical(field) {
                timestamp_scale(field)
            } else {
                match field.data_type() {
                    arrow::datatypes::DataType::Decimal128(precision, _) => {
                        *precision as sql::SmallInt
                    }
                    arrow::datatypes::DataType::Int8 => 3,
                    arrow::datatypes::DataType::Int16 => 5,
                    arrow::datatypes::DataType::Int32 => 10,
                    arrow::datatypes::DataType::Int64 => {
                        if is_decfloat(field) {
                            38
                        } else {
                            19
                        }
                    }
                    arrow::datatypes::DataType::Float32 => 7,
                    arrow::datatypes::DataType::Float64 => 15,
                    arrow::datatypes::DataType::Utf8 | arrow::datatypes::DataType::LargeUtf8 => {
                        field
                            .metadata()
                            .get("charLength")
                            .and_then(|s| s.parse::<sql::SmallInt>().ok())
                            .unwrap_or(0)
                    }
                    arrow::datatypes::DataType::Binary
                    | arrow::datatypes::DataType::LargeBinary => binary_length_metadata(field)
                        .map(|len| len as sql::SmallInt)
                        .unwrap_or(0),
                    arrow::datatypes::DataType::Timestamp(unit, _) => field
                        .metadata()
                        .get("scale")
                        .and_then(|s| s.parse::<sql::SmallInt>().ok())
                        .unwrap_or_else(|| match unit {
                            arrow::datatypes::TimeUnit::Second => 0,
                            arrow::datatypes::TimeUnit::Millisecond => 3,
                            arrow::datatypes::TimeUnit::Microsecond => 6,
                            arrow::datatypes::TimeUnit::Nanosecond => 9,
                        }),
                    arrow::datatypes::DataType::Time32(unit)
                    | arrow::datatypes::DataType::Time64(unit) => field
                        .metadata()
                        .get("scale")
                        .and_then(|s| s.parse::<sql::SmallInt>().ok())
                        .unwrap_or_else(|| match unit {
                            arrow::datatypes::TimeUnit::Second => 0,
                            arrow::datatypes::TimeUnit::Millisecond => 3,
                            arrow::datatypes::TimeUnit::Microsecond => 6,
                            arrow::datatypes::TimeUnit::Nanosecond => 9,
                        }),
                    arrow::datatypes::DataType::Boolean => 1,
                    _ => 0,
                }
            };
            write_numeric_attr(numeric_attribute_ptr, precision as sql::Len);
        }
        DESC_SCALE => {
            // Scale
            let scale = match field.data_type() {
                arrow::datatypes::DataType::Decimal128(_, scale) => *scale as sql::SmallInt,
                arrow::datatypes::DataType::Timestamp(unit, _) => field
                    .metadata()
                    .get("scale")
                    .and_then(|s| s.parse::<sql::SmallInt>().ok())
                    .unwrap_or_else(|| match unit {
                        arrow::datatypes::TimeUnit::Second => 0,
                        arrow::datatypes::TimeUnit::Millisecond => 3,
                        arrow::datatypes::TimeUnit::Microsecond => 6,
                        arrow::datatypes::TimeUnit::Nanosecond => 9,
                    }),
                arrow::datatypes::DataType::Time32(unit)
                | arrow::datatypes::DataType::Time64(unit) => field
                    .metadata()
                    .get("scale")
                    .and_then(|s| s.parse::<sql::SmallInt>().ok())
                    .unwrap_or_else(|| match unit {
                        arrow::datatypes::TimeUnit::Second => 0,
                        arrow::datatypes::TimeUnit::Millisecond => 3,
                        arrow::datatypes::TimeUnit::Microsecond => 6,
                        arrow::datatypes::TimeUnit::Nanosecond => 9,
                    }),
                _ => field
                    .metadata()
                    .get("scale")
                    .and_then(|s| s.parse::<sql::SmallInt>().ok())
                    .unwrap_or(0),
            };
            write_numeric_attr(numeric_attribute_ptr, scale as sql::Len);
        }
        DESC_SEARCHABLE => {
            // Match official driver: VARCHAR columns are fully searchable, others basic
            let predicate = match field.data_type() {
                arrow::datatypes::DataType::Utf8 | arrow::datatypes::DataType::LargeUtf8 => {
                    SQL_PRED_SEARCHABLE
                }
                _ => SQL_PRED_BASIC,
            };
            write_numeric_attr(numeric_attribute_ptr, predicate as sql::Len);
        }
        DESC_TYPE_NAME => {
            // Type name - check metadata for logical type and extTypeName first
            let type_name = odbc_type_name(field);
            write_char_attr(
                character_attribute_ptr,
                buffer_length,
                string_length_ptr,
                &type_name,
            );
        }
        DESC_UNNAMED => {
            // Column is named
            write_numeric_attr(numeric_attribute_ptr, SQL_NAMED as sql::Len);
        }
        DESC_UNSIGNED => {
            // Match official driver behavior: numeric types report false, character/binary/time types true
            let is_unsigned = matches!(
                field.data_type(),
                DataType::Utf8
                    | DataType::LargeUtf8
                    | DataType::Binary
                    | DataType::LargeBinary
                    | DataType::Timestamp(_, _)
                    | DataType::Time32(_)
                    | DataType::Time64(_)
                    | DataType::Boolean
            ) || is_timestamp_logical(field);
            let value = if is_unsigned { SQL_TRUE } else { SQL_FALSE };
            write_numeric_attr(numeric_attribute_ptr, value as sql::Len);
        }
        DESC_UPDATABLE => {
            // Match official driver: report unknown read/write capability
            write_numeric_attr(
                numeric_attribute_ptr,
                SQL_ATTR_READWRITE_UNKNOWN as sql::Len,
            );
        }
        _ => {
            tracing::warn!(
                "col_attribute: unsupported field_identifier={}",
                field_identifier
            );
            // Return success for unsupported attributes
        }
    }

    Ok(())
}
