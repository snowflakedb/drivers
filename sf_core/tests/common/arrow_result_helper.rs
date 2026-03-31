extern crate sf_core;
extern crate tracing;
extern crate tracing_subscriber;

use super::arrow_deserialize::ArrowDeserialize;
use super::arrow_extract_value::{ArrowExtractError, ArrowExtractValue, extract_arrow_value};
use arrow::array::{Array, ArrayRef, AsArray};
use arrow::buffer::NullBuffer;
use arrow::compute::kernels::cmp::not_distinct;
use arrow::datatypes::{DataType, FieldRef, Schema};
use arrow::ffi_stream::ArrowArrayStreamReader;
use arrow::ffi_stream::FFI_ArrowArrayStream;
use arrow::record_batch::{RecordBatch, RecordBatchReader};
use sf_core::protobuf::generated::database_driver_v1::ExecuteResult;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::mem::discriminant;
use std::sync::Arc;

/// Helper for processing Arrow stream results
pub struct ArrowResultHelper {
    reader: ArrowArrayStreamReader,
}

impl ArrowResultHelper {
    /// Creates a new Arrow result helper from an ExecuteResult
    pub fn from_result(result: ExecuteResult) -> Self {
        let stream_ptr: *mut FFI_ArrowArrayStream = result.stream.unwrap().into();
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

/// Returns metadata keys to exclude from comparison for a given Arrow DataType.
fn metadata_keys_to_exclude(logical_type: &str) -> &'static [&'static str] {
    match logical_type {
        "TEXT" => &["finalType", "precision", "scale"],
        "FIXED" => &["finalType", "charLength", "byteLength"],
        "BOOLEAN" => &[
            "byteLength",
            "charLength",
            "finalType",
            "precision",
            "scale",
        ],
        "REAL" => &[
            "byteLength",
            "charLength",
            "finalType",
            "precision",
            "scale",
        ],
        "TIMESTAMP_NTZ" | "TIMESTAMP_LTZ" | "TIMESTAMP_TZ" | "TIME" => &[
            "finalType",
            "charLength",
            "byteLength",
            "precision",
            "physicalType",
        ],
        "DATE" => &[
            "finalType",
            "charLength",
            "byteLength",
            "precision",
            "scale",
            "physicalType",
        ],
        "BINARY" => &["finalType", "precision", "scale", "physicalType"],
        "DECFLOAT" => &[
            "finalType",
            "precision",
            "scale",
            "physicalType",
            "byteLength",
            "charLength",
        ],
        "ARRAY" | "OBJECT" | "VARIANT" => &[
            "finalType",
            "byteLength",
            "charLength",
            "precision",
            "scale",
            "physicalType",
        ],
        _ => &[],
    }
}

/// Asserts that two Arrow schemas match in field name, data type, nullability,
/// and relevant metadata keys (excluding keys that are known to differ between
/// Arrow-native and JSON-converted-to-Arrow results).
pub fn assert_schemas_match(left: &Schema, right: &Schema) {
    assert_eq!(
        left.fields().len(),
        right.fields().len(),
        "Schema field count mismatch: left has {}, right has {}",
        left.fields().len(),
        right.fields().len()
    );

    for (arrow_field, json_field) in left.fields().iter().zip(right.fields().iter()) {
        assert_fields_match(arrow_field, json_field);
    }
}

fn assert_fields_match(left: &FieldRef, right: &FieldRef) {
    assert_eq!(left.name(), right.name(), "Field name mismatch");
    assert_eq!(
        left.is_nullable(),
        right.is_nullable(),
        "Nullability mismatch for field '{}'",
        left.name()
    );
    assert_eq!(
        discriminant(left.data_type()),
        discriminant(right.data_type()),
        "Data type variant mismatch for field '{}', left type: {:?}, right type: {:?}",
        left.name(),
        left.data_type(),
        right.data_type()
    );
    let logical_type = left
        .metadata()
        .get("logicalType")
        .unwrap_or_else(|| panic!("logicalType metadata key missing for field {}", left.name()));
    let excluded = metadata_keys_to_exclude(logical_type);

    let filter_metadata = |metadata: &BTreeMap<String, String>| -> BTreeMap<String, String> {
        metadata
            .iter()
            .filter(|(k, _)| !excluded.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    };

    let left_meta: BTreeMap<String, String> = left
        .metadata()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let right_meta: BTreeMap<String, String> = right
        .metadata()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let filtered_left_meta = filter_metadata(&left_meta);
    let filtered_right_meta = filter_metadata(&right_meta);

    assert_eq!(
        filtered_left_meta,
        filtered_right_meta,
        "Metadata mismatch for field '{}'\n  left: {:?}\n  right:  {:?}",
        left.name(),
        filtered_left_meta,
        filtered_right_meta
    );
    match (left.data_type(), right.data_type()) {
        (DataType::Struct(left_fields), DataType::Struct(right_fields)) => {
            left_fields
                .iter()
                .zip(right_fields.iter())
                .for_each(|(a, j)| assert_fields_match(a, j));
        }
        (left_data_type, right_data_type) => {
            assert_eq!(
                left_data_type,
                right_data_type,
                "Data type mismatch for field '{}'",
                left.name()
            );
        }
    }
}

/// Asserts that two RecordBatches match in schema (using relaxed metadata comparison)
/// and column data.
pub fn assert_record_batches_match(left: &RecordBatch, right: &RecordBatch) {
    assert_schemas_match(left.schema().as_ref(), right.schema().as_ref());

    assert_eq!(
        left.num_columns(),
        right.num_columns(),
        "Column count mismatch"
    );
    assert_eq!(left.num_rows(), right.num_rows(), "Row count mismatch");

    for col_idx in 0..left.num_columns() {
        let left_col = left.column(col_idx);
        let right_col = right.column(col_idx);
        let schema = left.schema();
        let field_name = schema.field(col_idx).name();
        assert_arrays_match(left_col, right_col, field_name);
    }
}

fn assert_arrays_match(left: &ArrayRef, right: &ArrayRef, field_path: &str) {
    match (left.data_type(), right.data_type()) {
        (DataType::Struct(_), DataType::Struct(_)) => {
            let left_struct = left.as_struct();
            let right_struct = right.as_struct();
            assert_eq!(
                left_struct.num_columns(),
                right_struct.num_columns(),
                "Struct '{field_path}' child count mismatch"
            );
            for (i, name) in left_struct.column_names().iter().enumerate() {
                let child_name = format!("{field_path}.{name}");
                // Propagate parent null bitmap into children so we don't compare
                // filler values at positions where the parent struct is null
                let left_child = nullify_child(left_struct.nulls(), left_struct.column(i));
                let right_child = nullify_child(right_struct.nulls(), right_struct.column(i));
                assert_arrays_match(&left_child, &right_child, &child_name);
            }
        }
        _ => {
            let result = not_distinct(left, right)
                .unwrap_or_else(|e| panic!("Failed to compare '{field_path}': {e}"));

            let mismatches: Vec<usize> = result
                .iter()
                .enumerate()
                .filter(|(_, v)| v != &Some(true))
                .map(|(i, _)| i)
                .collect();

            for idx in mismatches.iter().take(5) {
                println!(
                    "Mismatch in {:?} at idx {:?}, left: {:?}, right: {:?}",
                    field_path,
                    idx,
                    extract_arrow_value::<String>(left, *idx),
                    extract_arrow_value::<String>(right, *idx)
                );
            }

            assert!(
                mismatches.is_empty(),
                "'{field_path}' has mismatched values at rows: {mismatches:?}",
            );
        }
    }
}

/// Returns a new array with the parent's null bitmap merged into the child array,
/// so that positions where the parent struct is null also appear null in the child.
fn nullify_child(parent_nulls: Option<&NullBuffer>, child: &ArrayRef) -> ArrayRef {
    let Some(parent_nulls) = parent_nulls else {
        return Arc::clone(child);
    };
    let child_data = child.to_data();
    let merged_nulls = match child_data.nulls() {
        Some(child_nulls) => NullBuffer::new(parent_nulls.inner() & child_nulls.inner()),
        None => parent_nulls.clone(),
    };
    let new_data = child_data
        .into_builder()
        .null_bit_buffer(Some(merged_nulls.into_inner().into_inner()))
        .build()
        .unwrap();
    arrow::array::make_array(new_data)
}
