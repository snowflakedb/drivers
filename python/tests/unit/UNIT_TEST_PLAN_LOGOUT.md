# Unit Test Plan: Python Wrapper Logout Call to Core

## Objective
Create unit tests to verify that the Python wrapper's `Connection.close()` method correctly calls the underlying core's `connection_close` method with appropriate arguments.

## Architecture Understanding

### Current Implementation Flow
1. **Python Layer**: `Connection.close()` in `python/src/snowflake/connector/connection.py`
2. **Client API Layer**: `DatabaseDriverClient.connection_close()` in `python/src/snowflake/connector/_internal/protobuf_gen/database_driver_v1_services.py`
3. **Transport Layer**: `ProtoTransport.handle_message()` in `python/src/snowflake/connector/_internal/api_client/client_api.py`
4. **C API Layer**: `sf_core_api_call_proto()` in `python/src/snowflake/connector/_internal/api_client/c_api.py`
5. **Core Layer**: Rust implementation via ctypes CDLL

### Key Connection.close() Logic (lines 151-159)
```python
self.db_api.connection_close(
    ConnectionCloseRequest(
        conn_handle=self.conn_handle,
        server_session_keep_alive=self.server_session_keep_alive,
        enable_auto_detection=effective_enable_auto,
        error_strategy="BestEffort",
        timeout_seconds=5,
    )
)
```

### Parameters to Test
1. **conn_handle**: Connection identifier
2. **server_session_keep_alive**: Optional[bool] - Controls logout behavior
   - `True`: Never logout (Fire & Forget)
   - `False`: Phase-dependent behavior
   - `None`: Delegate to auto-detection
3. **enable_auto_detection**: Optional[bool] - Check async query registry
   - Computed based on Phase 2 vs Phase 3 behavior
4. **error_strategy**: Always "BestEffort" for Python
5. **timeout_seconds**: Always 5 seconds for Python

## Test Strategy

### Approach: Mock at the DatabaseDriverClient Level
**Rationale**: Mocking at this level allows us to:
- Test the Python wrapper logic without invoking the actual core
- Verify the exact protobuf message structure and field values
- Avoid dealing with ctypes and C FFI complexity
- Test Phase 2 vs Phase 3 behavior differences

### Mock Setup
```python
# Patch: snowflake.connector.connection.database_driver_client
# Return: Mock DatabaseDriverClient with mocked methods:
#   - database_new()
#   - database_init()
#   - connection_new()
#   - connection_set_option_*()
#   - connection_init()
#   - connection_close()  # ← This is what we want to verify
```

## Test Cases

### 1. Default Parameters (Phase 2)
**Test**: `test_close_calls_connection_close_with_default_parameters`
- **Given**: Connection created with minimal params (account, user, password)
- **When**: `conn.close()` is called
- **Then**: Verify `connection_close` is called once with:
  - `conn_handle`: Set during initialization
  - `server_session_keep_alive`: None or not set
  - `enable_auto_detection`: True (Phase 2 default)
  - `error_strategy`: "BestEffort"
  - `timeout_seconds`: 5

### 2. server_session_keep_alive=True
**Test**: `test_close_calls_connection_close_with_keep_alive_true`
- **Given**: Connection with `server_session_keep_alive=True`
- **When**: `conn.close()` is called
- **Then**: Verify `server_session_keep_alive=True` in request

### 3. server_session_keep_alive=False (Phase 2)
**Test**: `test_close_calls_connection_close_with_keep_alive_false_phase2`
- **Given**: Connection with `server_session_keep_alive=False`
- **When**: `conn.close()` is called
- **Then**: 
  - Verify `server_session_keep_alive=False` in request
  - Verify `enable_auto_detection=True` (Phase 2 still respects auto-detection)
  - Verify FutureWarning is emitted

### 4. enable_server_session_keep_alive_auto_detection=False
**Test**: `test_close_calls_connection_close_with_auto_detection_disabled`
- **Given**: Connection with `enable_server_session_keep_alive_auto_detection=False`
- **When**: `conn.close()` is called
- **Then**: Verify `enable_auto_detection=False` in request

### 5. enable_server_session_keep_alive_auto_detection=True
**Test**: `test_close_calls_connection_close_with_auto_detection_enabled`
- **Given**: Connection with `enable_server_session_keep_alive_auto_detection=True`
- **When**: `conn.close()` is called
- **Then**: Verify `enable_auto_detection=True` in request

### 6. Phase 3 Defaults
**Test**: `test_close_calls_connection_close_with_phase3_defaults`
- **Given**: Connection with `ALLOW_BREAKING_CHANGE_SERVER_SESSION_KEEP_ALIVE_AUTO_DETECTION=True`
- **When**: `conn.close()` is called
- **Then**: 
  - Verify `enable_auto_detection=None` (Phase 3 disables by default)
  - Verify no FutureWarning

### 7. Combined Parameters
**Test**: `test_close_calls_connection_close_with_combined_parameters`
- **Given**: Connection with multiple logout parameters:
  - `server_session_keep_alive=None`
  - `enable_server_session_keep_alive_auto_detection=True`
- **When**: `conn.close()` is called
- **Then**: Verify all parameters are correctly passed

### 8. Idempotency
**Test**: `test_close_is_idempotent_and_only_calls_core_once`
- **Given**: A connection
- **When**: `conn.close()` is called 3 times
- **Then**: Verify `connection_close` is called only once

### 9. Error Handling (Optional)
**Test**: `test_close_handles_core_exceptions_gracefully`
- **Given**: Mock `connection_close` raises an exception
- **When**: `conn.close()` is called
- **Then**: Verify exception is handled or propagated appropriately

## Implementation Checklist

### Prerequisites
- [ ] Understand protobuf message structure (`ConnectionCloseRequest`)
- [ ] Understand mock.patch for the `database_driver_client` function
- [ ] Understand how to verify protobuf field values

### Test File Structure
- [ ] Create `python/tests/unit/test_connection_logout.py`
- [ ] Import required modules:
  - `unittest.mock` (Mock, patch)
  - `pytest`
  - `snowflake.connector.Connection`
  - `ConnectionCloseRequest`, `ConnectionCloseResponse`
- [ ] Create fixture for mocked database API client
- [ ] Create fixture for connection with mocked API

### Test Implementation
- [ ] Implement each test case from above
- [ ] Verify mock calls using `assert_called_once()` and `.call_args`
- [ ] Extract and verify `ConnectionCloseRequest` fields
- [ ] Handle FutureWarning assertions with `pytest.warns()`

### Validation
- [ ] Run tests: `uv run pytest tests/unit/test_connection_logout.py -v`
- [ ] Verify all tests pass
- [ ] Check test coverage for `Connection.close()` method
- [ ] Run linter: ensure no errors

## Notes

### Protobuf Field Checking
When verifying protobuf fields, handle optional fields carefully:
```python
# Check if field is set
if call_args.HasField("server_session_keep_alive"):
    assert call_args.server_session_keep_alive == expected_value
else:
    # Field not set (None)
    assert expected_value is None
```

### Mock Setup Complexity
The `Connection.__init__` makes several API calls. All must be mocked:
```python
mock_db_api.database_new = Mock(return_value=Mock(db_handle="test_db"))
mock_db_api.database_init = Mock()
mock_db_api.connection_new = Mock(return_value=Mock(conn_handle="test_conn"))
mock_db_api.connection_set_option_string = Mock()
mock_db_api.connection_set_option_int = Mock()
mock_db_api.connection_set_option_double = Mock()
mock_db_api.connection_init = Mock()
mock_db_api.connection_close = Mock(return_value=ConnectionCloseResponse())
```

### Phase 2 vs Phase 3 Logic
The effective auto-detection value is computed in `Connection.close()`:
- **Phase 2** (default): `effective_enable_auto = enable_auto_detection ?? True`
- **Phase 3** (opt-in): `effective_enable_auto = enable_auto_detection ?? None`

Tests must verify this logic works correctly.

## Success Criteria
- [ ] All 8+ test cases implemented and passing
- [ ] Tests verify exact protobuf field values
- [ ] Tests cover both Phase 2 and Phase 3 behavior
- [ ] Tests verify idempotency
- [ ] No linter errors
- [ ] Test execution time < 1 second (unit tests should be fast)

## Future Enhancements
1. Add tests for `_close_at_exit()` atexit handler
2. Add tests for error propagation from core
3. Add tests for connection state after close
4. Add integration tests that verify end-to-end behavior with real core
