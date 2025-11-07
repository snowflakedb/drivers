//! Arrow stream processing utilities

type Result<T> = std::result::Result<T, String>;
use arrow::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use sf_core::protobuf_gen::database_driver_v1::ExecuteResult;

/// Creates an Arrow reader from a protobuf ExecuteResult
pub fn create_arrow_reader(result: ExecuteResult) -> Result<ArrowArrayStreamReader> {
    let stream_ptr: *mut FFI_ArrowArrayStream = result
        .stream
        .ok_or_else(|| "No stream in result".to_string())?
        .into();

    let stream: FFI_ArrowArrayStream = unsafe { FFI_ArrowArrayStream::from_raw(stream_ptr) };

    ArrowArrayStreamReader::try_new(stream)
        .map_err(|e| format!("Failed to create Arrow stream reader: {}", e))
}

/// Fetches all rows from an ExecuteResult and returns the count
pub fn fetch_result_rows(result: ExecuteResult) -> Result<usize> {
    let mut reader = create_arrow_reader(result)?;
    let mut total_rows = 0;

    while let Some(batch_result) = reader.next() {
        let batch = batch_result.map_err(|e| format!("Failed to read batch: {:?}", e))?;
        total_rows += batch.num_rows();
    }

    Ok(total_rows)
}
