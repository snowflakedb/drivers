//! Verifies the public Arrow reader API: `process_query_response` (re-exported
//! with `QueryResult`) returns a `Box<dyn RecordBatchReader + Send + 'static>`
//! directly, with no `FFI_ArrowArrayStream` wrapping. It uses only the crate's
//! public surface, so it fails to build if the re-export is removed.

use arrow::array::{Array, Int64Array, RecordBatchReader, StringArray};
use arrow::datatypes::DataType;
use sf_core::apis::database_driver_v1::{QueryResult, process_query_response};
use sf_core::rest::snowflake::query_response::Data;

// JSON format with an inline rowset and no `chunks` resolves to an in-memory
// `JsonRowset`, so `process_query_response` performs no network I/O.
const JSON_ROWSET: &str = r#"{
    "queryResultFormat": "json",
    "rowset": [["1", "alice"], ["2", "bob"]],
    "rowtype": [
        {"name": "ID", "type": "FIXED", "nullable": false, "precision": 38, "scale": 0},
        {"name": "NAME", "type": "TEXT", "nullable": true, "length": 16777216, "byteLength": 16777216}
    ]
}"#;

#[test]
fn process_query_response_returns_record_batch_reader_without_ffi() {
    let data: Data = serde_json::from_str(JSON_ROWSET)
        .expect("fixture must deserialize into query_response::Data");
    let client = reqwest::Client::new();

    let runtime = tokio::runtime::Runtime::new().expect("failed to build tokio runtime");
    let result: QueryResult = runtime
        .block_on(process_query_response(&data, &client))
        .expect("process_query_response should succeed for an inline JSON rowset");

    // The explicit type is the contract under test: the field is exactly
    // `Box<dyn RecordBatchReader + Send + 'static>`, not an FFI stream. Routing
    // the real value through a `Send + 'static` bound ties the guarantee to
    // `QueryResult.reader` itself.
    fn assert_send_static<T: Send + 'static>(value: T) -> T {
        value
    }
    let reader: Box<dyn RecordBatchReader + Send + 'static> = assert_send_static(result.reader);
    assert!(result.columns.is_none());

    let schema = reader.schema();
    assert_eq!(schema.fields().len(), 2);
    assert_eq!(schema.field(0).name(), "ID");
    assert_eq!(schema.field(0).data_type(), &DataType::Int64);
    assert_eq!(schema.field(1).name(), "NAME");
    assert_eq!(schema.field(1).data_type(), &DataType::Utf8);

    // The chunked reader uses `blocking_recv` internally, so it must be drained
    // from a synchronous context, outside the runtime that produced it.
    let batches = reader
        .collect::<Result<Vec<_>, _>>()
        .expect("draining the reader should not error");
    drop(runtime);

    assert_eq!(batches.len(), 1);
    let batch = &batches[0];
    assert_eq!(batch.num_rows(), 2);

    let ids = batch
        .column(0)
        .as_any()
        .downcast_ref::<Int64Array>()
        .expect("column 0 should be Int64");
    assert_eq!(ids.value(0), 1);
    assert_eq!(ids.value(1), 2);

    let names = batch
        .column(1)
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("column 1 should be Utf8");
    assert_eq!(names.value(0), "alice");
    assert_eq!(names.value(1), "bob");
}
