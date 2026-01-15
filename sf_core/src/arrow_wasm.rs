//! Zero-copy Arrow export for WASM.
//!
//! This module provides functionality to export Arrow data as buffer offsets
//! into WASM linear memory, allowing hosts to read Arrow data directly without
//! serialization overhead.

use arrow::array::{Array, ArrayRef};
use arrow::buffer::Buffer;
use arrow::datatypes::Schema;
use arrow::ipc::writer::StreamWriter;
use arrow::record_batch::{RecordBatch, RecordBatchReader};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::protobuf_gen::database_driver_v1::{
    WasmArrowColumn, WasmArrowResult, WasmBuffer, WasmRecordBatch,
};

lazy_static::lazy_static! {
    /// Global storage for Arrow data that has been exported to WASM.
    /// The host must call release_arrow_result to free this memory.
    static ref EXPORTED_RESULTS: Mutex<HashMap<u64, ExportedArrowData>> = Mutex::new(HashMap::new());
    static ref NEXT_HANDLE: Mutex<u64> = Mutex::new(1);
}

/// Holds Arrow data that has been exported to WASM memory.
/// This keeps the data alive until the host calls release.
struct ExportedArrowData {
    #[allow(dead_code)]
    schema: Arc<Schema>,
    #[allow(dead_code)]
    batches: Vec<RecordBatch>,
}

/// Export a RecordBatchReader as a WasmArrowResult with buffer offsets.
///
/// The returned result contains offsets into WASM linear memory where the
/// Arrow buffers reside. The host can read directly from these offsets.
///
/// IMPORTANT: The caller must eventually call `release_arrow_result` with the
/// returned `release_handle` to free the memory.
pub fn export_reader_to_wasm(
    reader: Box<dyn RecordBatchReader + Send>,
) -> Result<WasmArrowResult, String> {
    let schema = reader.schema();

    // Serialize schema as IPC (small overhead)
    let schema_ipc = serialize_schema_ipc(&schema)?;

    // Collect all batches
    let batches: Result<Vec<RecordBatch>, _> = reader.collect();
    let batches = batches.map_err(|e| format!("Failed to read batches: {}", e))?;

    // Calculate total rows
    let total_rows: i64 = batches.iter().map(|b| b.num_rows() as i64).sum();

    // Export each batch
    let wasm_batches: Vec<WasmRecordBatch> = batches.iter().map(export_batch_to_wasm).collect();

    // Store the data to keep it alive
    let handle = {
        let mut next = NEXT_HANDLE.lock().unwrap();
        let h = *next;
        *next += 1;
        h
    };

    {
        let mut results = EXPORTED_RESULTS.lock().unwrap();
        results.insert(
            handle,
            ExportedArrowData {
                schema: schema.clone(),
                batches,
            },
        );
    }

    Ok(WasmArrowResult {
        schema_ipc,
        batches: wasm_batches,
        total_rows,
        release_handle: handle,
    })
}

/// Release Arrow data that was exported to WASM.
/// This should be called by the host when it's done reading the data.
pub fn release_arrow_result(handle: u64) {
    let mut results = EXPORTED_RESULTS.lock().unwrap();
    results.remove(&handle);
}

/// Serialize schema to Arrow IPC format.
fn serialize_schema_ipc(schema: &Arc<Schema>) -> Result<Vec<u8>, String> {
    let mut buffer = Vec::new();
    {
        let mut writer = StreamWriter::try_new(&mut buffer, schema)
            .map_err(|e| format!("Failed to create IPC writer: {}", e))?;
        writer
            .finish()
            .map_err(|e| format!("Failed to finish IPC writer: {}", e))?;
    }
    Ok(buffer)
}

/// Serialize full Arrow data (schema + batches) to IPC format.
/// This is used when the host doesn't support zero-copy access.
pub fn serialize_reader_to_full_ipc(
    mut reader: Box<dyn RecordBatchReader + Send>,
) -> Result<Vec<u8>, String> {
    let schema = reader.schema();
    let mut buffer = Vec::new();
    {
        let mut writer = StreamWriter::try_new(&mut buffer, &schema)
            .map_err(|e| format!("Failed to create IPC writer: {}", e))?;

        for batch in reader.by_ref() {
            let batch = batch.map_err(|e| format!("Failed to read batch: {e}"))?;
            writer
                .write(&batch)
                .map_err(|e| format!("Failed to write batch: {e}"))?;
        }

        writer
            .finish()
            .map_err(|e| format!("Failed to finish IPC writer: {e}"))?;
    }
    Ok(buffer)
}

/// Export a single RecordBatch as WasmRecordBatch with buffer offsets.
fn export_batch_to_wasm(batch: &RecordBatch) -> WasmRecordBatch {
    let columns: Vec<WasmArrowColumn> = (0..batch.num_columns())
        .map(|i| export_column_to_wasm(batch.column(i)))
        .collect();

    WasmRecordBatch {
        num_rows: batch.num_rows() as i64,
        columns,
    }
}

/// Export a single column's buffers as WasmArrowColumn.
fn export_column_to_wasm(array: &ArrayRef) -> WasmArrowColumn {
    let data = array.to_data();

    // Get validity buffer (null bitmap)
    let validity = data.nulls().map(|nulls| {
        let buffer = nulls.buffer();
        buffer_to_wasm(buffer)
    });

    // Get data buffers
    let data_buffers: Vec<WasmBuffer> = data.buffers().iter().map(buffer_to_wasm).collect();

    // For variable-length types, the offsets are typically in the first buffer
    // and data in the second. We'll put offsets separately for clarity.
    let (offsets, data) = if data_buffers.len() >= 2 {
        // Variable-length type (e.g., String, Binary)
        (Some(data_buffers[0]), vec![data_buffers[1]])
    } else {
        // Fixed-length type
        (None, data_buffers)
    };

    WasmArrowColumn {
        validity,
        data,
        offsets,
    }
}

/// Convert an Arrow Buffer to a WasmBuffer with offset and length.
fn buffer_to_wasm(buffer: &Buffer) -> WasmBuffer {
    // The buffer's ptr() gives us the address in WASM linear memory
    let ptr = buffer.as_ptr() as usize;
    let len = buffer.len();

    WasmBuffer {
        offset: ptr as u32,
        length: len as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Int64Array, StringArray};
    use arrow::datatypes::{DataType, Field};

    #[test]
    fn test_export_simple_batch() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, false),
        ]));

        let id_array = Int64Array::from(vec![1, 2, 3]);
        let name_array = StringArray::from(vec!["a", "b", "c"]);

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(id_array), Arc::new(name_array)],
        )
        .unwrap();

        let wasm_batch = export_batch_to_wasm(&batch);

        assert_eq!(wasm_batch.num_rows, 3);
        assert_eq!(wasm_batch.columns.len(), 2);

        // Int64 column should have data buffer
        assert!(!wasm_batch.columns[0].data.is_empty());

        // String column should have offsets and data
        assert!(wasm_batch.columns[1].offsets.is_some());
        assert!(!wasm_batch.columns[1].data.is_empty());
    }
}
