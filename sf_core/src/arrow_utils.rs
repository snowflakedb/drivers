use crate::rest::snowflake::query_response::RowType;
use arrow::array::{Array, Int64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use std::sync::Arc;
use thiserror::Error;

/// Custom error type for Arrow operations
#[derive(Error, Debug)]
pub enum ArrowUtilsError {
    #[error("Arrow error: {0}")]
    Arrow(#[from] ArrowError),
    #[error("Invalid integer value")]
    InvalidInteger(#[from] std::num::ParseIntError),
    #[error("Unsupported Snowflake type, only TEXT and FIXED with scale 0 types are supported")]
    UnsupportedType,
}

/// Maps Snowflake data types to Arrow data types
/// Only supports TEXT and FIXED (with scale 0) types
fn snowflake_type_to_arrow_type(row_type: &RowType) -> Result<DataType, ArrowUtilsError> {
    match row_type.type_.to_uppercase().as_str() {
        "TEXT" => Ok(DataType::Utf8),
        "FIXED" if row_type.scale == Some(0) => Ok(DataType::Int64),
        _ => Err(ArrowUtilsError::UnsupportedType),
    }
}

/// Creates an Arrow Field from a RowType
fn create_field(row_type: &RowType) -> Result<Field, ArrowUtilsError> {
    let arrow_type = snowflake_type_to_arrow_type(row_type)?;
    Ok(Field::new(&row_type.name, arrow_type, row_type.nullable))
}

/// Creates an Arrow array from column values and data type
fn create_column_array(
    values: Vec<&str>,
    data_type: &DataType,
) -> Result<Arc<dyn Array>, ArrowUtilsError> {
    match data_type {
        DataType::Utf8 => Ok(Arc::new(StringArray::from(values))),
        DataType::Int64 => {
            // Convert string values to i64 and return parsing error if any value is invalid
            let int_values: Result<Vec<i64>, ArrowUtilsError> = values
                .into_iter()
                .map(|v| v.parse::<i64>().map_err(ArrowUtilsError::InvalidInteger))
                .collect();
            Ok(Arc::new(Int64Array::from(int_values?)))
        }
        _ => Err(ArrowUtilsError::UnsupportedType),
    }
}

/// Converts a string rowset with RowType metadata to Arrow format
/// Supports TEXT and FIXED (with scale 0) types, converting strings to appropriate Arrow types
/// Assumes rowset and row_types have been validated to have matching column counts
pub fn convert_string_rowset_to_arrow_reader(
    rowset: &[Vec<String>],
    row_types: &[RowType],
) -> Result<Box<dyn arrow::record_batch::RecordBatchReader + Send>, ArrowUtilsError> {
    // Create Arrow schema from RowType metadata
    let fields: Result<Vec<Field>, ArrowUtilsError> = row_types.iter().map(create_field).collect();
    let fields = fields?;
    let schema = Arc::new(Schema::new(fields));

    // Create Arrow arrays for each column
    let columns: Result<Vec<Arc<dyn Array>>, ArrowUtilsError> = row_types
        .iter()
        .enumerate()
        .map(|(col_idx, _row_type)| {
            let values: Vec<&str> = rowset.iter().map(|row| row[col_idx].as_str()).collect();
            let data_type = schema.field(col_idx).data_type();
            create_column_array(values, data_type)
        })
        .collect();

    let columns = columns?;

    boxed_arrow_reader(schema, columns).map_err(|e| e.into())
}

pub fn boxed_arrow_reader(
    schema: Arc<Schema>,
    columns: Vec<Arc<dyn Array>>,
) -> Result<Box<dyn arrow::record_batch::RecordBatchReader + Send>, ArrowError> {
    let batch = RecordBatch::try_new(schema.clone(), columns)?;
    Ok(Box::new(arrow::record_batch::RecordBatchIterator::new(
        vec![Ok(batch)],
        schema,
    )))
}
