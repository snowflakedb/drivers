# JSON Parameter Binding Implementation

## Overview

This document describes the implementation of JSON parameter binding for the Snowflake Universal Driver, as specified in `bindingsdesign.md`.

## Changes Made

### 1. Protobuf Definition Updates (`protobuf/database_driver_v1.proto`)

Added new message types for passing data pointers from wrappers to Rust core:

```protobuf
// Pointer to an UTF-8 string in memory
message StringPtr {
  bytes value = 1;  // 8-byte pointer
  int64 length = 2;  // Length of data in bytes
}

// Pointer to binary data with explicit length
message BinaryDataPtr {
  bytes value = 1;   // 8-byte pointer
  int64 length = 2;  // Length of data in bytes
}

// Arrow bindings for ODBC (mirrors current StatementBind)
message ArrowBindings {
  ArrowSchemaPtr schema = 1;
  ArrowArrayPtr array = 2;
}

// Union of all binding types
message QueryBindings {
  oneof binding_type {
    StringPtr json = 1;
    BinaryDataPtr csv = 2;
    ArrowBindings arrow = 3;
  }
}

// Extend current StatementExecute
message StatementExecuteQueryRequest {
  StatementHandle stmt_handle = 1;
  optional QueryBindings bindings = 2;  // None = no bindings
}
```

### 2. Python Wrapper Implementation

#### New Module: `binding_serializer.py`

Created `python/src/snowflake/connector/_internal/binding_serializer.py` with the `BindingSerializer` class that:

- Converts Python parameters (sequence or dict) to Snowflake JSON binding format
- Handles both single-row and multi-row (array) bindings
- Maps Python types to Snowflake types (int → FIXED, str → TEXT, etc.)
- Provides a method to determine if stage binding should be used based on threshold

**JSON Format:**
```json
{
  "1": {"type": "FIXED", "value": "123"},
  "2": {"type": "TEXT", "value": "hello"}
}
```

For arrays (multi-row):
```json
{
  "1": {"type": "FIXED", "value": ["1", "2", "3"]},
  "2": {"type": "TEXT", "value": ["hello", "world", "foo"]}
}
```

#### Updated: `cursor.py`

Modified the `execute()` method to:
1. Serialize parameters using `BindingSerializer`
2. Create `StringPtr` protobuf message with JSON data
3. Wrap in `QueryBindings` message
4. Pass to `StatementExecuteQueryRequest`

**Code:**
```python
# Serialize parameters if provided
bindings = None
if parameters is not None:
    json_str, length = BindingSerializer.serialize_parameters(parameters)
    if json_str is not None:
        json_bytes = json_str.encode('utf-8')
        string_ptr = StringPtr(value=json_bytes, length=length)
        bindings = QueryBindings(json=string_ptr)

# Execute query with optional bindings
request = StatementExecuteQueryRequest(stmt_handle=stmt_handle)
if bindings is not None:
    request.bindings.CopyFrom(bindings)
```

### 3. Rust Core Implementation

#### Updated: `protobuf_apis/database_driver_v1.rs`

Modified the `statement_execute_query` protobuf handler to:
- Extract optional `bindings` field from request
- Pass bindings to core API function

```rust
let bindings_opt = input.bindings.and_then(|b| b.binding_type);
let result = statement_execute_query(stmt_handle.into(), bindings_opt).to_protobuf()?;
```

#### Updated: `apis/database_driver_v1/statement.rs`

1. **Modified function signature:**
```rust
pub fn statement_execute_query(
    stmt_handle: Handle,
    proto_bindings: Option<query_bindings::BindingType>,
) -> Result<ExecuteResult, ApiError>
```

2. **Added binding type handling:**
- JSON bindings: Parse JSON and convert to `HashMap<String, BindParameter>`
- CSV bindings: Placeholder for future stage upload implementation
- Arrow bindings: Placeholder for future Arrow binding support
- Falls back to existing Arrow bindings from `StatementBind` for backwards compatibility

3. **New function `parse_json_bindings()`:**
- Parses JSON from `StringPtr`
- Converts to `HashMap<String, query_request::BindParameter>`
- Validates JSON structure
- Handles both scalar and array values

```rust
fn parse_json_bindings(
    string_ptr: &crate::protobuf_gen::database_driver_v1::StringPtr,
) -> Result<Option<HashMap<String, query_request::BindParameter>>, StatementError>
```

4. **Added tests:**
- `test_parse_json_bindings()`: Tests simple parameter binding
- `test_parse_json_bindings_with_array()`: Tests array (multi-row) binding

## Backwards Compatibility

### ✅ Maintains Full Backwards Compatibility

1. **Optional bindings field:**
   - The `bindings` field in `StatementExecuteQueryRequest` is optional
   - Existing code that doesn't provide bindings continues to work

2. **Fallback to Arrow bindings:**
   - If no bindings are provided via protobuf, the code falls back to using the existing `StatementBind` mechanism
   - This ensures ODBC and other existing implementations continue working

3. **Compatible with old Python connector:**
   - The JSON format exactly matches the format used by the old `snowflake-connector-python`
   - See comparison in next section

### Comparison with Old Python Connector

The old connector (`/home/repo/snowflake-connector-python`) uses the exact same JSON format:

**Old Connector (`connection.py:_process_params_qmarks()`):**
```python
processed_params[str(idx + 1)] = {
    "type": snowflake_type,
    "value": snowflake_binding,
}
```

**New Connector (`binding_serializer.py:_process_params()`):**
```python
bindings[str(idx + 1)] = {
    "type": snowflake_type,
    "value": snowflake_value
}
```

Both produce identical JSON structure sent to Snowflake server.

## Memory Management

As specified in the design document:
- **Wrapper responsibility:** Python manages the memory for the JSON bytes
- **No deallocation needed:** The bytes are passed by value in the protobuf message, not by raw pointer
- **Safe:** Python's garbage collector handles cleanup after the RPC completes

## Next Steps

### Required Actions:

1. **Regenerate Protobuf Files:**
   ```bash
   # Generate Python bindings
   cd /home/repo/universal-driver
   protoc --python_out=python/src/snowflake/connector/_internal/protobuf_gen \
          --proto_path=protobuf protobuf/database_driver_v1.proto

   # Generate Rust bindings (using tonic or prost)
   # This is typically done via build.rs
   cargo build
   ```

2. **Implement CSV Stage Binding:**
   - Add CSV serialization in Python wrapper
   - Implement stage upload in Rust core
   - Use `CLIENT_STAGE_ARRAY_BINDING_THRESHOLD` to decide JSON vs CSV

3. **Add Integration Tests:**
   - Test single-row bindings
   - Test multi-row (array) bindings
   - Test mixed types
   - Test backwards compatibility with existing `StatementBind`

4. **Performance Testing:**
   - Compare with old connector
   - Verify no regression in binding performance
   - Test with large datasets

### Optional Enhancements:

1. **Stream-based CSV upload:**
   - Instead of keeping all CSV data in memory
   - Stream chunks to Rust core
   - Implement as suggested in design doc

2. **Arrow format support:**
   - Future optimization for direct Arrow data transfer
   - Requires server-side changes

3. **Stage naming improvements:**
   - Implement database/schema inference from query
   - Handle cases where default database/schema not set
   - As discussed in design doc Note 2

## Testing

### Unit Tests Added:

1. **Rust Core:**
   - `test_parse_json_bindings()`: Validates JSON parsing
   - `test_parse_json_bindings_with_array()`: Validates array handling

### Integration Tests Needed:

1. **Python → Rust → Snowflake:**
   ```python
   cursor.execute("SELECT ?, ?", [123, "hello"])
   ```

2. **Array Bindings:**
   ```python
   cursor.execute("INSERT INTO t VALUES (?, ?)", [[1, 2, 3], ["a", "b", "c"]])
   ```

3. **Named Parameters:**
   ```python
   cursor.execute("SELECT :id, :name", {"id": 123, "name": "hello"})
   ```

## API Contract

### Python API (PEP 249 compliant):

```python
cursor.execute(operation: str, parameters: Sequence[Any] | dict[str, Any] | None = None)
```

- `parameters` as `Sequence`: Positional binding (? or :1 style)
- `parameters` as `dict`: Named binding (:name style)

### Protobuf API:

```protobuf
message StatementExecuteQueryRequest {
  StatementHandle stmt_handle = 1;
  optional QueryBindings bindings = 2;
}
```

### Rust API:

```rust
pub fn statement_execute_query(
    stmt_handle: Handle,
    proto_bindings: Option<query_bindings::BindingType>,
) -> Result<ExecuteResult, ApiError>
```

## Design Decisions

1. **Why JSON first, not CSV:**
   - Simpler implementation for PuPr target
   - Most queries use small parameter sets
   - CSV stage upload can be added later for large datasets

2. **Why not Arrow for all wrappers:**
   - Type conversion complexity
   - Risk of backwards compatibility breaks
   - Existing drivers use string-based serialization

3. **Why optional in protobuf:**
   - Backwards compatibility with existing `StatementBind` mechanism
   - Allows gradual migration
   - ODBC can continue using Arrow format

4. **Memory safety:**
   - Protobuf bytes fields handle memory correctly
   - No raw pointers across FFI boundary
   - Python GC manages lifecycle

## File Changes Summary

### New Files:
- `python/src/snowflake/connector/_internal/binding_serializer.py`
- `IMPLEMENTATION_NOTES.md` (this file)

### Modified Files:
- `protobuf/database_driver_v1.proto`
- `python/src/snowflake/connector/cursor.py`
- `sf_core/src/protobuf_apis/database_driver_v1.rs`
- `sf_core/src/apis/database_driver_v1/statement.rs`

### Files to Regenerate:
- `python/src/snowflake/connector/_internal/protobuf_gen/database_driver_v1_pb2.py`
- `sf_core/src/protobuf_gen/database_driver_v1.rs`
- Other language bindings (Java, .NET, Go, etc.)
