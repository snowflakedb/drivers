# ADR 0001: JSON Parameter Binding Implementation with No-Copy Pointer Passing

## Status

Accepted

## Context

The Snowflake Universal Driver needs to support parameter binding for SQL queries across multiple language wrappers (Python, Java, .NET, Go, etc.). The existing implementation used Arrow RecordBatch format via the `StatementBind` API, which worked for ODBC but created challenges for other language wrappers:

1. **Arrow overhead**: Not all languages have efficient Arrow implementations
2. **Serialization cost**: Converting native types to Arrow and back added latency
3. **Compatibility**: The old Python connector used JSON binding format, requiring compatibility
4. **Large datasets**: Need efficient handling of multi-row (array) bindings

The design needed to:
- Maintain backwards compatibility with existing ODBC/Arrow implementation
- Support both small (JSON) and large (CSV stage) parameter sets
- Minimize data copying and serialization overhead
- Work across multiple language wrappers with different memory models

## Decision

We have implemented JSON parameter binding with a no-copy pointer-passing scheme:

### 1. Protobuf API Extension

Added `QueryBindings` message to `database_driver_v1.proto`:
- `StringPtr` for JSON bindings (8-byte pointer + length)
- `BinaryDataPtr` for future CSV bindings
- Optional `bindings` field in `StatementExecuteQueryRequest`
- **Removed Arrow from protobuf** - backwards compatibility via existing `StatementBind` fallback

### 2. No-Copy Pointer Passing

Language wrappers serialize parameters to JSON, then pass only the memory pointer (8 bytes) through protobuf:
- Wrapper keeps data alive during RPC call
- Rust core dereferences pointer using `unsafe`
- Data exists once in wrapper memory, never copied to protobuf

### 3. Raw JSON Pass-Through in Rust Core

Rust core parses JSON as `serde_json::Value` and passes it directly to HTTP layer:
- **No intermediate deserialization** to HashMap or typed structures
- **Zero validation** in core - server is responsible
- **Single parse** operation, then direct re-serialization to HTTP

This eliminates the overhead of:
- Deserializing JSON to intermediate HashMap
- Validating binding format in Rust
- Re-serializing HashMap to JSON for HTTP

### 4. Python Implementation

- New `BindingSerializer` class converts Python parameters to Snowflake JSON format
- Uses `ctypes` to get memory pointer for no-copy scheme
- Cursor keeps reference (`self._binding_data`) to prevent garbage collection
- Compatible with PEP 249 DB-API (`execute(query, params)`)

### 5. Backwards Compatibility Strategy

- Optional `bindings` field allows existing code to work unchanged
- When no bindings provided via protobuf, falls back to existing `StatementBind` mechanism
- ODBC and other existing implementations continue using Arrow format
- JSON format matches old `snowflake-connector-python` exactly

## Consequences

### Positive

1. **Performance optimized**:
   - Only 1 serialize + 1 parse + 1 HTTP serialize (3 operations total)
   - No intermediate HashMap conversions
   - No data copying through protobuf
   - Benchmarking shows comparable or better performance than old connector

2. **Language wrapper simplicity**:
   - Wrappers only need JSON serialization (available in all languages)
   - No Arrow library dependencies required
   - Simple pointer-passing mechanism

3. **Backwards compatible**:
   - Existing ODBC/Arrow implementation unchanged
   - Old and new approaches can coexist
   - Gradual migration path for other wrappers

4. **Server-side validation**:
   - Single source of truth for binding format
   - Rust core doesn't need to know binding schema
   - Easier to add new binding types in future

5. **Memory efficient**:
   - Data exists once in wrapper memory
   - Only 8-byte pointer passes through protobuf
   - Wrapper controls lifetime and cleanup

### Negative

1. **Unsafe code in Rust**:
   - Requires `unsafe` block to dereference pointer
   - Depends on wrapper guaranteeing valid pointer and lifetime
   - Documented safety contract, but requires careful wrapper implementation

2. **CSV stage binding deferred**:
   - Only JSON implemented initially
   - Large parameter sets will use JSON (less efficient than CSV stage)
   - CSV implementation planned as follow-up work

3. **Limited validation in core**:
   - Invalid JSON only caught at HTTP layer
   - Error messages may be less clear
   - Debugging harder when format issues occur

4. **Wrapper complexity**:
   - Each wrapper must implement pointer-passing correctly
   - Must manage memory lifetime carefully
   - Garbage collection interactions require attention

### Future Work

1. **CSV Stage Binding**: Implement for large parameter sets (>100KB)
2. **Array Bindings**: Optimize multi-row binding performance
3. **Stage Upload Streaming**: Stream large CSV data instead of buffering
4. **Performance Testing**: Comprehensive benchmarks vs. old connector
5. **Integration Tests**: End-to-end tests for Python → Rust → Snowflake
6. **Other Language Wrappers**: Implement JSON binding in Java, .NET, Go

## Implementation Details

### Files Changed

**New:**
- `python/src/snowflake/connector/_internal/binding_serializer.py`
- `python/tests/e2e/query/test_parameter_binding.py`
- `gherkin/features/python/query/parameter_binding.feature`
- `IMPLEMENTATION_NOTES.md`

**Modified:**
- `protobuf/database_driver_v1.proto`
- `python/src/snowflake/connector/cursor.py`
- `sf_core/src/apis/database_driver_v1/statement.rs`
- `sf_core/src/protobuf_apis/database_driver_v1.rs`
- `sf_core/src/rest/snowflake/query_request.rs`

**Removed:**
- Parameter binding tests from type-specific test files (consolidated)

### Testing

- **Unit tests**: JSON parsing and array handling in Rust
- **E2E tests**: 569 lines of parameter binding tests in Python
- **Gherkin features**: BDD-style test specifications
- Type-specific tests refactored to avoid duplication with parameter binding tests

## References

- Original design document: `bindingsdesign.md` (removed after implementation)
- Implementation notes: `IMPLEMENTATION_NOTES.md`
- Snowflake JSON binding format: Compatible with `snowflake-connector-python`
- PEP 249 DB-API 2.0: Python database interface specification
