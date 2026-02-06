# ADR: Async Execution Support

## Status
Implemented

## Context
The legacy snowflake-connector-python driver provides async execution capabilities through `execute_async()` and `get_results_from_sfqid()` methods. Users rely on these features for long-running queries where they don't want to block waiting for results. The new Python driver wrapper must maintain compatibility with this functionality.

## Decision
We implemented async execution support by extending both the sf_core Rust library and the Python wrapper:

### 1. Protobuf API Extensions
Added new RPC methods to `database_driver_v1.proto`:
- `ConnectionGetQueryStatus` - Check query status by query ID
- `ConnectionGetResultsFromQueryId` - Retrieve results by query ID
- `StatementExecuteQueryRequest.async_exec` - Optional flag for async execution

Added `QueryStatus` enum matching Snowflake's query status values:
- RUNNING, SUCCESS, FAILED_WITH_ERROR, QUEUED, etc.

### 2. Rust Core Implementation
**Query Status Checking** (`sf_core/src/apis/database_driver_v1/connection.rs`):
- `connection_get_query_status()` - Calls Snowflake's `/monitoring/queries/{sfqid}` REST endpoint
- `QueryStatus` enum with `from_string()` conversion from Snowflake status strings
- Error handling for invalid query IDs and network failures

**Result Retrieval** (`sf_core/src/apis/database_driver_v1/connection.rs`):
- `connection_get_results_from_query_id()` - Uses SQL `RESULT_SCAN` function
- Creates temporary statement to execute: `SELECT * FROM TABLE(RESULT_SCAN('{query_id}'))`
- Returns `ExecuteResult` with Arrow stream, metadata, and column information

**Async Execution** (`sf_core/src/apis/database_driver_v1/statement.rs`):
- Modified `statement_execute_query()` to accept `async_exec` parameter
- Sets `async_execution` option on statement before execution
- Leverages existing async execution infrastructure in `sf_core/src/rest/snowflake/async_exec.rs`

### 3. Python Wrapper Implementation
**QueryStatus Enum** (`python/src/snowflake/connector/constants.py`):
- Python enum matching protobuf QueryStatus values
- Maintains compatibility with legacy driver enum names

**Cursor Methods** (`python/src/snowflake/connector/cursor.py`):
- `execute_async()` - Wrapper around `execute()` with `_exec_async=True`
- Returns dict with `queryId` key
- Validates that PUT/GET commands are not used (not supported in async mode)
- `get_results_from_sfqid()` - Polls query status and retrieves results
  - Exponential backoff: [1, 1, 2, 3, 4, 8, 10] seconds
  - Maximum 24 retries for NO_DATA status
  - Calls protobuf API to get results via RESULT_SCAN
  - Populates cursor description and rowcount

**Connection Methods** (`python/src/snowflake/connector/connection.py`):
- `get_query_status(sfqid)` - Returns QueryStatus enum
- `get_query_status_throw_if_error(sfqid)` - Throws DatabaseError if query failed
- `is_still_running(status)` - Helper to check if query is running
- `is_an_error(status)` - Helper to check if query failed

## Architecture

### Query Execution Flow
```
Python execute_async()
  → Protobuf: StatementExecuteQueryRequest(async_exec=True)
    → Rust: statement_execute_query()
      → Sets "async_execution" option
      → Snowflake REST API: POST /queries/v1/query-request (asyncExec=true)
      → Returns immediately with query_id
  ← Returns dict{"queryId": "..."}
```

### Query Status Flow
```
Python get_query_status(sfqid)
  → Protobuf: ConnectionGetQueryStatusRequest(query_id)
    → Rust: connection_get_query_status()
      → Snowflake REST API: GET /monitoring/queries/{sfqid}
      → Parses status string to QueryStatus enum
  ← Returns QueryStatus enum
```

### Result Retrieval Flow
```
Python get_results_from_sfqid(sfqid)
  → Polls query status until completion
  → Protobuf: ConnectionGetResultsFromQueryIdRequest(query_id)
    → Rust: connection_get_results_from_query_id()
      → Executes: SELECT * FROM TABLE(RESULT_SCAN('{sfqid}'))
      → Returns ExecuteResult with Arrow stream
  ← Populates cursor with results
```

## Compatibility with Legacy Driver

### Matching Behavior
1. **execute_async()** - Same signature and return value
2. **get_results_from_sfqid()** - Same polling strategy with exponential backoff
3. **Query status methods** - Identical helper methods on connection
4. **QueryStatus enum** - Same values and names (including typo: QUEUED_REPARING_WAREHOUSE)
5. **Error handling** - PUT/GET rejection, invalid UUID validation

### Differences
1. **Implementation** - New driver uses RESULT_SCAN instead of direct result endpoint
   - Legacy: GET `/queries/{qid}/result`
   - New: SQL `RESULT_SCAN('{qid}')`
   - Both approaches retrieve the same results

2. **Architecture** - New driver routes through Rust core for consistency
   - Enables reuse across all language wrappers (Python, JDBC, ODBC)
   - Maintains universal driver architecture

## Limitations

1. **PUT/GET Not Supported** - File transfer commands cannot be executed asynchronously
   - Explicitly rejected with NotSupportedError
   - Matches legacy driver behavior

2. **Multi-Statement Queries** - Not yet implemented
   - Would require tracking multiple query IDs
   - Legacy driver supports via MULTI_STATEMENT_COUNT parameter

3. **Connection Pooling** - Async query tracking not yet implemented
   - Legacy driver tracks `_async_sfqids` and `_done_async_sfqids`
   - Not critical for Phase 7 implementation

## Testing

Integration tests cover:
- Basic async execution and result retrieval
- Query status checking and polling
- Long-running queries with exponential backoff
- Error cases (PUT/GET rejection, invalid query IDs)
- Helper methods (is_still_running, is_an_error)
- Multiple rows and cursor iteration

All tests must pass for both new and legacy drivers to ensure compatibility.

## Future Enhancements

1. **Multi-Statement Support** - Track and retrieve results from multiple statements
2. **Async Query Tracking** - Connection-level tracking of pending async queries
3. **Batch Status Checking** - Check status of multiple queries in one call
4. **Result Caching** - Cache result URLs to avoid RESULT_SCAN overhead
5. **Async Cancellation** - Support `SYSTEM$CANCEL_QUERY` for async queries

## References

- Legacy driver implementation: `/home/repo/snowflake-connector-python/src/snowflake/connector/cursor.py`
- Snowflake REST API: `/monitoring/queries/{sfqid}` endpoint
- Snowflake SQL: `RESULT_SCAN` table function
- sf_core async implementation: `sf_core/src/rest/snowflake/async_exec.rs`
