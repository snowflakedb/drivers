use crate::common::snowflake_test_client::SnowflakeTestClient;
use arrow::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use sf_core::protobuf::generated::database_driver_v1::*;

fn read_batches_from_response(
    response: DatabaseFetchChunkResponse,
) -> Vec<arrow::record_batch::RecordBatch> {
    let stream_ptr: *mut FFI_ArrowArrayStream = response.stream.unwrap().into();
    let stream = unsafe { FFI_ArrowArrayStream::from_raw(stream_ptr) };
    let reader = ArrowArrayStreamReader::try_new(stream).unwrap();
    reader.collect::<Result<Vec<_>, _>>().unwrap()
}

#[test]
fn distributed_fetch_simple_query() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When Query "SELECT 42 AS answer, 'hello' AS greeting" is executed
    let stmt = client.new_statement();
    client.set_sql_query(&stmt, "SELECT 42 AS answer, 'hello' AS greeting");
    let _execute_result = client.execute_statement_query(&stmt);

    // Then result chunks should contain at least one inline chunk
    let chunks_result = client.result_chunks(&stmt);
    assert!(
        !chunks_result.chunks.is_empty(),
        "Should have at least one chunk"
    );
    let first_chunk = &chunks_result.chunks[0];
    assert_eq!(first_chunk.format, ChunkFormat::ArrowIpc as i32);
    assert!(
        matches!(&first_chunk.data, Some(result_chunk::Data::Inline(_))),
        "First chunk should be inline"
    );

    // And fetching the inline chunk should return 1 row with 2 columns
    let response = client.fetch_chunk(first_chunk.clone());
    let batches = read_batches_from_response(response);
    assert!(!batches.is_empty());
    let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    assert_eq!(total_rows, 1);
    assert_eq!(batches[0].num_columns(), 2);

    // And Statement should be released
    client.release_statement(&stmt);
}

#[test]
fn distributed_fetch_large_result_produces_multiple_chunks() {
    // Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When Large query generating 500000 rows is executed
    let stmt = client.new_statement();
    client.set_sql_query(
        &stmt,
        "SELECT seq8() AS id, RANDSTR(100, RANDOM()) AS payload \
         FROM TABLE(GENERATOR(ROWCOUNT => 500000)) v ORDER BY id",
    );
    let _execute_result = client.execute_statement_query(&stmt);

    // Then result chunks should contain at least 2 chunks
    let chunks_result = client.result_chunks(&stmt);
    assert!(
        chunks_result.chunks.len() >= 2,
        "Large result should produce at least 2 chunks, got {}",
        chunks_result.chunks.len()
    );

    // And result chunks should contain at least one remote chunk
    let has_remote = chunks_result
        .chunks
        .iter()
        .any(|c| matches!(&c.data, Some(result_chunk::Data::Remote(_))));
    assert!(
        has_remote,
        "Large result should contain at least one remote chunk"
    );

    // And fetching all chunks should return 500000 total rows
    let mut total_rows = 0;
    for chunk in &chunks_result.chunks {
        assert_eq!(chunk.format, ChunkFormat::ArrowIpc as i32);
        let response = client.fetch_chunk(chunk.clone());
        let batches = read_batches_from_response(response);
        for batch in &batches {
            total_rows += batch.num_rows();
        }
    }
    assert_eq!(total_rows, 500000);

    // And Statement should be released
    client.release_statement(&stmt);
}
