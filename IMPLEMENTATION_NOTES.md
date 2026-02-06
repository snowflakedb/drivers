# JSON Parameter Binding Implementation

## Overview

This document describes the implementation of JSON parameter binding for the Snowflake Universal Driver, as specified in `bindingsdesign.md`.

## Changes Made

### 1. Protobuf Definition Updates (`protobuf/database_driver_v1.proto`)

Added new message types for passing data pointers from wrappers to Rust core:

```protobuf
// Pointer to an UTF-8 string in memory
message StringPtr {
  bytes value = 1;  // 8-byte pointer (memory address in little-endian)
  int64 length = 2;  // Length of data in bytes
}

// Pointer to binary data with explicit length
message BinaryDataPtr {
  bytes value = 1;   // 8-byte pointer (memory address in little-endian)
  int64 length = 2;  // Length of data in bytes
}

// Union of all binding types
message QueryBindings {
  oneof binding_type {
    StringPtr json = 1;
    BinaryDataPtr csv = 2;
    // Note: Arrow bindings removed - backwards compatibility via StatementBind fallback
  }
}

// Extend current StatementExecute
message StatementExecuteQueryRequest {
  StatementHandle stmt_handle = 1;
  optional QueryBindings bindings = 2;  // None = no bindings
}
```

**Key Design Decision**: Removed `ArrowBindings` from protobuf. Backwards compatibility
is achieved by falling back to the existing `StatementBind` mechanism when no bindings
are provided via protobuf. This is cleaner than passing Arrow pointers through protobuf.

### 2. Python Wrapper Implementation

#### New Module: `binding_serializer.py`

Created `python/src/snowflake/connector/_internal/binding_serializer.py` with the `BindingSerializer` class that:

- Converts Python parameters (sequence or dict) to Snowflake JSON binding format
- Handles both single-row and multi-row (array) bindings
- Maps Python types to Snowflake types (int → FIXED, str → TEXT, etc.)
- **Stage binding decision logic deferred to follow-up** (see TODO comment in file)
- **Removed `should_use_stage_binding` method** - will be implemented when CSV stage binding is added

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

**Code (No-Copy Scheme):**
```python
# Serialize parameters if provided
if parameters is not None:
    json_str, length = BindingSerializer.serialize_parameters(parameters)
    if json_str is not None:
        # Convert to bytes and keep reference (prevent GC)
        json_bytes = json_str.encode('utf-8')
        self._binding_data = json_bytes  # Keep alive during RPC

        # Get memory pointer (no-copy scheme)
        import ctypes
        ptr_value = ctypes.cast(ctypes.c_char_p(json_bytes), ctypes.c_void_p).value
        ptr_bytes = ptr_value.to_bytes(8, byteorder='little', signed=False)

        # Pass pointer, not data
        string_ptr = StringPtr(value=ptr_bytes, length=length)
        request = StatementExecuteQueryRequest(
            stmt_handle=stmt_handle,
            bindings=QueryBindings(json=string_ptr)
        )
    else:
        request = StatementExecuteQueryRequest(stmt_handle=stmt_handle)
else:
    request = StatementExecuteQueryRequest(stmt_handle=stmt_handle)
```

**No-Copy Guarantee**: Data exists once in Python memory. Only the 8-byte pointer
value is passed through protobuf, not the actual JSON bytes. Rust dereferences the
pointer to access the data.

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
- JSON bindings: Dereference pointer and parse as raw `serde_json::Value` - passed directly to HTTP layer
- CSV bindings: Placeholder for future stage upload implementation
- Falls back to existing Arrow bindings from `StatementBind` for backwards compatibility

3. **New function `parse_json_bindings()`:**
- Dereferences pointer from `StringPtr` (8-byte memory address)
- Parses JSON as raw `serde_json::Value` - **no intermediate deserialization**
- **Zero validation** - server is responsible for validating binding format
- **Single parse** - no conversion to HashMap, passed directly to HTTP serialization

```rust
fn parse_json_bindings(
    string_ptr: &crate::protobuf_gen::database_driver_v1::StringPtr,
) -> Result<Option<serde_json::Value>, StatementError>
```

**Safety**: Uses `unsafe` to dereference the raw pointer. Python wrapper guarantees
the pointer is valid and the data won't be deallocated during the RPC call.

**Performance**: Optimal path - Python serializes → Rust dereferences → HTTP serializes.
No intermediate conversions or re-serialization.

4. **Updated HTTP layer** (`sf_core/src/rest/snowflake/query_request.rs`):
- Changed `bindings` field from `HashMap<String, BindParameter>` to `serde_json::Value`
- Allows passing raw JSON through without intermediate deserialization
- Server receives exactly what Python sent

5. **Updated backwards compatibility** (`parameters_from_record_batch`):
- Converts Arrow RecordBatch to HashMap<String, BindParameter>
- Serializes HashMap to `serde_json::Value` using `serde_json::to_value()`
- All existing validation logic preserved in backwards compatibility path

6. **Added tests:**
- `test_parse_json_bindings()`: Tests simple parameter binding
- `test_parse_json_bindings_with_array()`: Tests array (multi-row) binding
- Tests verify JSON structure without deserializing to HashMap

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

## Performance Characteristics

### Data Flow (Optimized):
1. **Python**: Serialize params → JSON string (once)
2. **Python**: Get pointer to JSON bytes (zero-copy)
3. **Protobuf**: Pass 8-byte pointer (not data)
4. **Rust**: Dereference pointer → Parse as `serde_json::Value` (once)
5. **HTTP**: Serialize `serde_json::Value` to request body (once)

**Total**: 1 serialize in Python + 1 parse in Rust + 1 serialize to HTTP = 3 operations
**No intermediate HashMap conversions or re-serialization**

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

## Memory Management (No-Copy Scheme)

Implemented as specified in the design document:
- **Wrapper responsibility:** Python manages the memory for the JSON bytes
- **Pointer passing:** Only the 8-byte pointer value is passed through protobuf, not the data
- **Lifetime guarantee:** Python keeps a reference (`self._binding_data`) to prevent GC
- **Rust side:** Dereferences the pointer using `unsafe` to access data
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

2. **Why raw `serde_json::Value` instead of HashMap:**
   - Eliminates intermediate deserialization/re-serialization
   - Rust core just passes JSON through to HTTP layer
   - Server is responsible for validation (single source of truth)
   - Significantly simpler code with better performance

3. **Why removed Arrow from protobuf:**
   - Better backwards compatibility via StatementBind fallback
   - Cleaner separation: protobuf for new wrappers, StatementBind for ODBC
   - No need to pass Arrow pointers through protobuf

4. **Why optional in protobuf:**
   - Backwards compatibility with existing `StatementBind` mechanism
   - Allows gradual migration
   - ODBC can continue using Arrow format

5. **Memory safety:**
   - Python keeps reference to prevent GC (`self._binding_data`)
   - Only pointer value (8 bytes) passed through protobuf
   - Rust uses `unsafe` with documented safety guarantees
   - Data lifetime managed by Python GC after RPC completes

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
