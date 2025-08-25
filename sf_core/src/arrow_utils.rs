use crate::rest::snowflake::query_response::RowType;
use arrow::array::{Array, Int64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use snafu::{Location, ResultExt, Snafu};
use std::sync::Arc;

/// Maps Snowflake data types to Arrow data types
/// Only supports TEXT and FIXED (with scale 0) types
fn snowflake_type_to_arrow_type(row_type: &RowType) -> Result<DataType, ArrowUtilsError> {
    match row_type.type_.to_uppercase().as_str() {
        "TEXT" => Ok(DataType::Utf8),
        "FIXED" if row_type.scale == Some(0) => Ok(DataType::Int64),
        _ => UnsupportedTypeSnafu {
            snowflake_type: format!("{} (scale: {:?})", row_type.type_, row_type.scale),
        }
        .fail(),
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
                .map(|v| {
                    v.parse::<i64>().context(InvalidIntegerSnafu {
                        value: v.to_string(),
                    })
                })
                .collect();
            Ok(Arc::new(Int64Array::from(int_values?)))
        }
        _ => UnsupportedTypeSnafu {
            snowflake_type: format!("{data_type:?}"),
        }
        .fail(),
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

    boxed_arrow_reader(schema, columns).context(ArrowSnafu)
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

#[derive(Snafu, Debug)]
pub enum ArrowUtilsError {
    #[snafu(display("Arrow operation failed"))]
    Arrow {
        source: ArrowError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse integer value: {value}"))]
    InvalidInteger {
        value: String,
        source: std::num::ParseIntError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Unsupported Snowflake type: {snowflake_type}. Only TEXT and FIXED with scale 0 are supported"
    ))]
    UnsupportedType {
        snowflake_type: String,
        #[snafu(implicit)]
        location: Location,
    },
}
