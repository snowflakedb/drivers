use arrow::array::{Array, Int64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use arrow_ipc::writer::StreamWriter;

use crate::rest::RestError;
use crate::rest::snowflake::query_response::RowType;

/// Maps Snowflake data types to Arrow data types
/// Only supports TEXT and FIXED (with scale 0) types
fn snowflake_type_to_arrow_type(row_type: &RowType) -> Result<DataType, RestError> {
    let type_name = row_type.type_.to_uppercase();

    match type_name.as_str() {
        "TEXT" => Ok(DataType::Utf8),
        "FIXED" if row_type.scale == Some(0) => Ok(DataType::Int64),
        _ => Err(RestError::InvalidSnowflakeResponse(format!(
            "Unsupported Snowflake type '{type_name}', only TEXT and FIXED with scale 0 types are supported"
        ))),
    }
}

/// Converts a rowset with RowType metadata to Arrow format
/// Only supports TEXT and FIXED (with scale 0) types
pub fn convert_result_to_arrow(
    rowset: &[Vec<Option<String>>],
    row_types: &[RowType],
) -> Result<Vec<u8>, RestError> {
    if rowset.is_empty() {
        return Ok(Vec::new());
    }

    let num_columns = row_types.len();
    // Validate that row_types matches the number of columns
    if num_columns != rowset[0].len() {
        return Err(RestError::InvalidSnowflakeResponse(format!(
            "RowType count ({}) doesn't match column count ({})",
            num_columns,
            rowset[0].len()
        )));
    }

    // Create Arrow schema from RowType metadata
    let fields: Result<Vec<Field>, RestError> = row_types
        .iter()
        .map(|row_type| {
            let arrow_type = snowflake_type_to_arrow_type(row_type)?;
            Ok(Field::new(&row_type.name, arrow_type, row_type.nullable))
        })
        .collect();
    let fields = fields?;
    let schema = Schema::new(fields.clone());

    // Create Arrow arrays for each column
    let arrow_arrays: Result<Vec<std::sync::Arc<dyn Array>>, RestError> = fields
        .iter()
        .enumerate()
        .map(|(col_idx, field)| {
            let row_type = &row_types[col_idx];
            let arrow_type = field.data_type();

            // Collect values from all rows for this column
            let values: Vec<Option<String>> =
                rowset.iter().map(|row| row[col_idx].clone()).collect();

            // Create the appropriate Arrow array based on the data type
            let array: std::sync::Arc<dyn Array> = match arrow_type {
                DataType::Utf8 => std::sync::Arc::new(StringArray::from(values)),
                DataType::Int64 => {
                    let int_values: Result<Vec<Option<i64>>, RestError> = values
                        .iter()
                        .map(|v| match v {
                            None => Ok(None),
                            Some(s) if s.is_empty() => Ok(None),
                            Some(s) => s.parse::<i64>().map(Some).map_err(|_| {
                                RestError::Internal(format!(
                                    "Invalid integer value '{}' for FIXED column '{}'",
                                    s, row_type.name
                                ))
                            }),
                        })
                        .collect();
                    std::sync::Arc::new(Int64Array::from(int_values?))
                }
                _ => {
                    return Err(RestError::InvalidSnowflakeResponse(format!(
                        "Unsupported Arrow data type for column '{}'",
                        row_type.name
                    )));
                }
            };

            Ok(array)
        })
        .collect();
    let arrow_arrays = arrow_arrays?;

    // Create RecordBatch
    let batch = RecordBatch::try_new(std::sync::Arc::new(schema), arrow_arrays).map_err(|e| {
        RestError::Internal(format!("Failed to create RecordBatch from rowset: {e}"))
    })?;

    // Serialize to Arrow IPC format
    let mut bytes = Vec::new();
    let mut writer = StreamWriter::try_new(&mut bytes, &batch.schema())
        .map_err(|e| RestError::Internal(format!("Failed to create Arrow StreamWriter: {e}")))?;

    writer
        .write(&batch)
        .map_err(|e| RestError::Internal(format!("Failed to write Arrow batch: {e}")))?;

    writer
        .finish()
        .map_err(|e| RestError::Internal(format!("Failed to finish Arrow writing: {e}")))?;

    Ok(bytes)
}
