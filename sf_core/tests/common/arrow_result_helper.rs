extern crate sf_core;
extern crate tracing;
extern crate tracing_subscriber;

use super::arrow_deserialize::ArrowDeserialize;
use super::arrow_extract_value::{ArrowExtractError, ArrowExtractValue, extract_arrow_value};
use arrow::ffi_stream::ArrowArrayStreamReader;
use arrow::ffi_stream::FFI_ArrowArrayStream;
use arrow::record_batch::RecordBatchReader;
use std::fmt::Debug;
use std::sync::Arc;

/// Helper for processing Arrow stream results
pub struct ArrowResultHelper {
    reader: ArrowArrayStreamReader,
}

impl ArrowResultHelper {
    /// Creates a new Arrow result helper from an ExecuteResult
    pub fn from_result(result: sf_core::thrift_gen::database_driver_v1::ExecuteResult) -> Self {
        let stream_ptr: *mut FFI_ArrowArrayStream = result.stream.into();
        let stream: FFI_ArrowArrayStream = unsafe { FFI_ArrowArrayStream::from_raw(stream_ptr) };
        let reader = ArrowArrayStreamReader::try_new(stream).unwrap();
        Self { reader }
    }

    /// Gets the next record batch
    pub fn next_batch(&mut self) -> Option<arrow::record_batch::RecordBatch> {
        match self.reader.next() {
            Some(Ok(batch)) => Some(batch),
            Some(Err(e)) => {
                tracing::error!("Error reading record batch: {e}");
                None
            }
            None => None,
        }
    }

    /// Returns the Arrow schema without consuming the stream
    pub fn schema(&self) -> Arc<arrow::datatypes::Schema> {
        self.reader.schema()
    }

    /// Converts all result data to a 2D array of strings for easy comparison
    pub fn transform_into_array<T: ArrowExtractValue>(
        &mut self,
    ) -> Result<Vec<Vec<T>>, ArrowExtractError> {
        let mut all_rows = Vec::new();
        while let Some(batch) = self.next_batch() {
            for row_idx in 0..batch.num_rows() {
                let mut row = Vec::new();
                for col_idx in 0..batch.num_columns() {
                    let column = batch.column(col_idx);
                    let value = extract_arrow_value::<T>(column, row_idx)?;
                    row.push(value);
                }
                all_rows.push(row);
            }
        }
        Ok(all_rows)
    }

    /// Asserts that the result equals the expected 2D array
    pub fn assert_equals_array<T: ArrowExtractValue + PartialEq + Debug>(
        &mut self,
        expected: Vec<Vec<T>>,
    ) {
        let actual = self.transform_into_array::<T>().unwrap();

        assert_eq!(
            actual, expected,
            "Arrow result does not match expected array"
        );
    }

    /// Convenience method for single row assertions
    pub fn assert_equals_single_row<T: ArrowExtractValue + PartialEq + Debug>(
        &mut self,
        expected: Vec<T>,
    ) {
        self.assert_equals_array(vec![expected]);
    }

    /// Convenience method for single value assertions
    pub fn assert_equals_single_value<T: ArrowExtractValue + PartialEq + Debug>(
        &mut self,
        expected: T,
    ) {
        self.assert_equals_array(vec![vec![expected]]);
    }

    /// Fetches all batches, converts them all to vectors and returns one big merged vector
    pub fn fetch_all<T: ArrowDeserialize>(&mut self) -> Result<Vec<T>, String> {
        let mut all_rows = Vec::new();

        // Then read all remaining batches
        while let Some(batch) = self.next_batch() {
            let batch_rows = T::deserialize_all(&batch)?;
            all_rows.extend(batch_rows);
        }

        Ok(all_rows)
    }

    /// Reads one row from the current batch and returns T
    pub fn fetch_one<T: ArrowDeserialize>(&mut self) -> Result<T, String> {
        if let Some(batch) = self.next_batch()
            && batch.num_rows() == 1
        {
            return T::deserialize_one(&batch, 0);
        }
        Err("Expected exactly one row in the batch".to_string())
    }
}
