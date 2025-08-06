use arrow::array::{Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use arrow_ipc::writer::StreamWriter;

use crate::rest::RestError;

// This function assumes that the rowset contains only string values
pub fn convert_result_to_arrow(rowset: &[Vec<Option<String>>]) -> Result<Vec<u8>, RestError> {
    if rowset.is_empty() {
        return Ok(Vec::new());
    }

    // Get the number of columns from the first row
    let num_columns = rowset[0].len();

    // Create simple string schema with generic column names
    let fields: Vec<Field> = (0..num_columns)
        .map(|i| Field::new(format!("column_{i}"), DataType::Utf8, true))
        .collect();
    let schema = Schema::new(fields);

    // Create string arrays from all rows
    let arrow_arrays: Vec<std::sync::Arc<dyn Array>> = (0..num_columns)
        .map(|col_idx| {
            // Collect values from all rows for this column
            let values: Vec<Option<String>> =
                rowset.iter().map(|row| row[col_idx].clone()).collect();
            std::sync::Arc::new(StringArray::from(values)) as std::sync::Arc<dyn Array>
        })
        .collect();

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
