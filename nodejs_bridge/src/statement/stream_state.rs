use arrow::array::{Int64Array, RecordBatch, RecordBatchReader};
use arrow::datatypes::SchemaRef;
use arrow::error::ArrowError;
use std::collections::HashMap;

pub(super) struct StreamState {
    batch_reader: Box<dyn RecordBatchReader + Send>,
    current_batch: Option<RecordBatch>,
    current_batch_row_index: usize,
}

impl StreamState {
    pub(super) fn new(batch_reader: Box<dyn RecordBatchReader + Send>) -> Self {
        Self {
            batch_reader,
            current_batch: None,
            current_batch_row_index: 0,
        }
    }

    pub(super) fn next_row(&mut self) -> Result<Option<HashMap<String, i64>>, ArrowError> {
        loop {
            if let Some(batch) = &self.current_batch {
                if self.current_batch_row_index < batch.num_rows() {
                    let row = convert_row(
                        &self.batch_reader.schema(),
                        batch,
                        self.current_batch_row_index,
                    );
                    self.current_batch_row_index += 1;
                    return Ok(Some(row));
                }
                self.current_batch = None;
            }

            match self.batch_reader.next() {
                Some(batch) => {
                    self.current_batch = Some(batch?);
                    self.current_batch_row_index = 0;
                }
                None => {
                    return Ok(None);
                }
            }
        }
    }
}

/// Placeholder Arrow-to-JS row conversion.
///
/// TODO: This intentionally handles only `Int64` columns (any other type
/// yields `-1`). Proper per-`DataType` conversion mirroring the ODBC
/// converters in `odbc/src/conversion/` is a separate follow-up task. Keeping
/// the logic in this single function makes that swap a one-site change.
fn convert_row(schema: &SchemaRef, batch: &RecordBatch, row: usize) -> HashMap<String, i64> {
    let mut out = HashMap::new();
    for (i, field) in schema.fields().iter().enumerate() {
        let value = batch
            .column(i)
            .as_any()
            .downcast_ref::<Int64Array>()
            .map(|arr| arr.value(row))
            .unwrap_or(-1);
        out.insert(field.name().clone(), value);
    }
    out
}
