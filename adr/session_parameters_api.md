# ADR: Session Parameters API

## Status

Accepted

## Context

The universal driver needs to manage Snowflake session parameters internally, which control various aspects of session behavior (e.g., `QUERY_TAG`, `TIMEZONE`, `TIMESTAMP_OUTPUT_FORMAT`). These parameters can be:

1. Set at connection initialization time
2. Modified during the session via `ALTER SESSION SET` statements
3. Cached for potential internal driver use

The old `snowflake-connector-python` does not expose a session parameters API, and the universal driver follows the same approach. Users interact with session parameters exclusively through SQL:
- Set at connection time via `session_parameters` dict
- Modified via `ALTER SESSION SET` statements
- Queried via `SHOW PARAMETERS` SQL statements

The implementation provides internal infrastructure for parameter caching while keeping implementation details private.

## Decisions

### 1. Private Implementation Helper

The universal driver implements a **private** helper method for internal driver use only:

```python
connection._get_session_parameter(name: str) -> str | None
```

**Rationale:**
- This is an implementation detail for internal driver logic, not an API
- The underscore prefix marks this as private per Python naming conventions
- Provides a mechanism for driver internals to access cached session parameter values
- No public API for session parameters is exposed to users
- Users should interact with session parameters via standard SQL (`ALTER SESSION SET`)
- Keeps implementation simple and avoids API surface expansion

### 2. Connection-Time Parameter Initialization

Session parameters can be set at connection creation via a `session_parameters` dictionary:

```python
conn = snowflake.connector.connect(
    account='myaccount',
    user='myuser',
    password='mypassword',
    session_parameters={
        'QUERY_TAG': 'my_app',
        'TIMEZONE': 'America/Los_Angeles'
    }
)
```

**Implementation flow:**
1. Python extracts `session_parameters` dict from kwargs
2. Calls `ConnectionSetSessionParameters` RPC with the dict
3. Rust stores parameters in `Connection.init_session_parameters`
4. During `connection_init`, parameters are added to `LoginParameters`
5. Sent to Snowflake in auth request's `SESSION_PARAMETERS` field
6. Server applies parameters during session initialization

**Rationale:**
- Provides declarative, connection-time parameter setting
- Avoids race conditions from setting parameters after connection
- Dedicated RPC method is cleaner than special-key workarounds
- Clean separation from regular connection options (account, user, etc.)

### 3. Session Parameters Cache with SQL Fallback

The Rust `Connection` struct maintains a session parameters cache:

```rust
pub struct Connection {
    // ... other fields
    pub session_parameters: Arc<RwLock<HashMap<String, String>>>,
}
```

**Cache population:**
- Initialized from auth response `_parameters` field after login
- Parameter names normalized to uppercase for case-insensitive access
- Updated when parameters are retrieved via SQL fallback

**SQL Fallback:**
When a parameter is not found in cache, `connection_get_parameter`:
1. Checks cache first (fast path)
2. If not found, executes `SHOW PARAMETERS LIKE 'param_name' IN SESSION`
3. Parses the result to extract the value
4. Updates cache with retrieved value
5. Returns the value or `None` if parameter doesn't exist

**Rationale:**
- Fast path avoids server roundtrips for frequently accessed parameters
- SQL fallback ensures accuracy for parameters not in initial auth response
- Automatic cache update prevents repeated queries for same parameter
- `Arc<RwLock<>>` allows concurrent reads with exclusive writes
- Uppercase normalization provides case-insensitive behavior matching Snowflake semantics

### 4. Case-Insensitive Parameter Names

Parameter names are case-insensitive:

```python
connection._get_session_parameter('QUERY_TAG')  # Works
connection._get_session_parameter('query_tag')  # Also works
connection._get_session_parameter('Query_Tag')  # Also works
```

**Implementation:** Parameter names are converted to uppercase before cache storage and lookup.

**Rationale:**
- Matches Snowflake's case-insensitive identifier behavior
- Improves usability and reduces user errors
- Standard practice in SQL systems

### 5. No `ConnectionGetAllParameters` API

The implementation does **not** include a "get all parameters" operation.

**Rationale:**
- No current use case for bulk parameter retrieval
- Can be added later if needed without breaking changes
- Reduces initial implementation complexity
- Users can query individual parameters as needed

### 6. No Direct Cache Write API

The implementation does **not** expose `connection_set_parameter()` for direct cache updates.

**Rationale:**
- Session parameters should be modified via SQL (`ALTER SESSION SET`)
- Direct cache writes would create inconsistency between cache and server state
- Simplifies implementation and reduces error-prone API surface
- Cache updates happen automatically through query execution

## Architecture

### Protobuf Layer

**Messages:**
```protobuf
message ConnectionSetSessionParametersRequest {
  ConnectionHandle conn_handle = 1;
  map<string, string> parameters = 2;
}

message ConnectionSetSessionParametersResponse {
}

message ConnectionGetParameterRequest {
  ConnectionHandle conn_handle = 1;
  string key = 2;
}

message ConnectionGetParameterResponse {
  optional string value = 1;
}
```

**RPC Methods:**
```protobuf
rpc ConnectionSetSessionParameters(ConnectionSetSessionParametersRequest)
    returns (ConnectionSetSessionParametersResponse);

rpc ConnectionGetParameter(ConnectionGetParameterRequest)
    returns (ConnectionGetParameterResponse);
```

### Rust Core Layer

**Connection struct** (`sf_core/src/apis/database_driver_v1/connection.rs`):
```rust
pub struct Connection {
    // ... other fields
    /// Session parameters cache (populated after login)
    pub session_parameters: Arc<RwLock<HashMap<String, String>>>,
    /// Session parameters to send during initialization
    pub init_session_parameters: Option<HashMap<String, String>>,
}
```

**Functions:**
```rust
// Set parameters before connection_init
pub fn connection_set_session_parameters(
    conn_handle: Handle,
    parameters: HashMap<String, String>
) -> Result<(), ApiError>

// Get parameter from cache
pub fn connection_get_parameter(
    conn_handle: Handle,
    key: String
) -> Result<Option<String>, ApiError>
```

**Module:** `sf_core/src/apis/database_driver_v1/session_parameters.rs`
- Implements `connection_get_parameter` with two-tier lookup:
  1. Cache lookup (fast path)
  2. SQL fallback via `SHOW PARAMETERS LIKE 'param' IN SESSION`
- Executes query using `with_valid_session` for automatic token refresh
- Parses result and updates cache

**Handlers:** `sf_core/src/protobuf_apis/database_driver_v1.rs`
- `connection_set_session_parameters`: Stores parameters for init
- `connection_get_parameter`: Retrieves from cache

### Python Wrapper Layer

**File:** `python/src/snowflake/connector/connection.py`

**Method:**
```python
def _get_session_parameter(self, name: str) -> str | None:
    """Get a session parameter value (internal method)."""
    request = ConnectionGetParameterRequest(
        conn_handle=self.conn_handle,
        key=name
    )
    response = self.db_api.connection_get_parameter(request)
    return response.value if response.value else None
```

**Init-time parameters:**
- Extracted from `kwargs` via `kwargs.pop('session_parameters', None)`
- Sent via `ConnectionSetSessionParametersRequest` before `connection_init`
- Prevents accidental iteration in regular connection option processing

## Consequences

### Positive

- **Simple API:** Single read method, no complex state management
- **SQL-based modification:** Leverages existing, well-understood `ALTER SESSION` statements
- **Automatic cache updates:** Server-side changes reflected in cache (when implemented)
- **Case-insensitive:** Matches Snowflake behavior, reduces errors
- **Efficient initialization:** Single JSON payload for multiple parameters
- **Clean separation:** Session parameters distinct from connection credentials
- **Thread-safe:** `Arc<RwLock<>>` enables concurrent access

### Negative

- **No public API:** Users cannot programmatically inspect session parameters
- **SQL fallback overhead:** First access to uncached parameters requires query execution
- **Internal implementation only:** `_get_session_parameter` is private, subject to change

### Future Enhancements

1. **Automatic cache sync:** Update cache from query response metadata (currently only updated on explicit get or during login)
2. **Parameter validation:** Validate parameter names/values at connection time
3. **Cache preloading:** Option to preload commonly used parameters during connection initialization

Note: No public API for reading session parameters is planned. Users should manage session parameters via SQL statements.

## Testing

Integration tests verify:
- Roundtrip: `ALTER SESSION SET` + `_get_session_parameter()`
- Init-time parameters set correctly
- Case-insensitive parameter name handling
- Nonexistent parameters return `None`
- Multiple parameters work correctly
- Init parameters can be overridden at runtime
- Parameters persist across query executions

**Test file:** `python/tests/integ/test_session_parameters.py`

## Migration Notes

This is a **new feature** with no migration impact. The old `snowflake-connector-python` does not have an equivalent API.

Users currently setting session parameters via SQL will continue to work without changes. The new API provides additional programmatic access without breaking existing workflows.
