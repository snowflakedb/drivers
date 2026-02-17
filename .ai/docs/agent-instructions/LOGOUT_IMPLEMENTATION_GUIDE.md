# Logout Implementation Guide

**Purpose:** Implement logout feature in UD Core (Rust) and wrappers

## Architecture Reality

### FFI = Protobuf Messages
NOT direct function calls. Single C API entry point:
```rust
sf_core_api_call_proto(
    api: "DatabaseDriver",
    method: "connection_close",
    request: protobuf_bytes,
    ...
)
```

Add to `protobuf/database_driver_v1.proto`:
```protobuf
message ConnectionCloseRequest {
  ConnectionHandle conn_handle = 1;
  optional bool server_session_keep_alive = 2;
  LogoutErrorStrategy error_strategy = 3;
}

enum LogoutErrorStrategy {
  LOGOUT_ERROR_STRATEGY_STRICT = 1;
  LOGOUT_ERROR_STRATEGY_BEST_EFFORT = 2;
}
```

### Config = HashMap (Not Struct)
```rust
pub struct Connection {
    pub settings: HashMap<String, Setting>,
    // NOT: pub logout_config: LogoutConfig
}

// Read like this:
let keep_alive = conn.settings.get("server_session_keep_alive");
```

### Async Entry = block_on()
```rust
pub fn connection_close(conn_handle: Handle) -> Result<bool, ApiError> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        close_session(&client, &url, &token).await
    })
}
```

### Error Strategy = Enum (Not Trait)
```rust
enum LogoutErrorStrategy {
    Strict,
    BestEffort,
}

impl LogoutErrorStrategy {
    fn handle_error(&self, error: RestError) -> Result<(), ApiError> {
        match self {
            Strict => Err(error),
            BestEffort => { warn!("{}",error); Ok(()) }
        }
    }
}
```

## Key Files

### Core (Rust)
```
sf_core/src/rest/snowflake/mod.rs                    # Add close_session()
sf_core/src/apis/database_driver_v1/connection.rs    # Add connection_close()
sf_core/src/config/logout.rs                         # LogoutConfig, ErrorStrategy
sf_core/tests/integration/session/logout.rs          # Mock TCP server tests
sf_core/tests/e2e/session/logout.rs                  # Real Snowflake tests
protobuf/database_driver_v1.proto                    # Add ConnectionCloseRequest
```

### Python
```
python/src/snowflake/connector/connection.py         # Update close()
python/tests/e2e/session/test_logout.py              # E2E tests
python/tests/integ/session/test_logout.py            # Wiremock tests
```

## Implementation Sequence

### Week 1: Protobuf + HTTP Function
1. Update `database_driver_v1.proto`
2. Add `close_session()` to `sf_core/src/rest/snowflake/mod.rs`
3. Add `LogoutFailed` error variant

### Week 2: Connection API
4. Add `connection_close()` to `connection.rs`
5. Read settings, decision logic, cleanup
6. Add protobuf handler glue

### Week 3: Integration Tests
7. Mock TCP server tests (not Wiremock)
8. Test retry, strategies, keep-alive

### Week 4: E2E Tests
9. Real Snowflake tests
10. Verify session closes
11. Bug fixes

### Week 5: Python Wrapper
12. Regenerate protobuf
13. Update `Connection.close()`
14. Python tests

## HTTP Logout Function

```rust
// sf_core/src/rest/snowflake/mod.rs

pub async fn close_session(
    client: &reqwest::Client,
    server_url: &str,
    session_token: &str,
    client_info: &ClientInfo,
) -> Result<(), RestError> {
    let request_id = uuid::Uuid::new_v4();
    let logout_url = format!("{}/session", server_url);

    let request = client
        .post(&logout_url)
        .query(&[
            ("delete", "true"),
            ("requestId", &request_id.to_string()),
            ("request_guid", &uuid::Uuid::new_v4().to_string()),
        ])
        .header(header::AUTHORIZATION, authorization_header(session_token))
        .header(header::ACCEPT, "application/snowflake")
        .header("User-Agent", user_agent(client_info))
        .json(&serde_json::json!({}))
        .build()?;

    let response = client.execute(request).await?;

    if response.status().is_success() {
        return Ok(());
    }

    // Handle SESSION_GONE (390111)
    let error_text = response.text().await?;
    if error_text.contains("390111") {
        return Ok(());
    }

    Err(RestError::LogoutFailed { message: error_text })
}
```

## Connection Close API

```rust
// sf_core/src/apis/database_driver_v1/connection.rs

pub fn connection_close(conn_handle: Handle) -> Result<bool, ApiError> {
    let conn_ptr = CONN_HANDLE_MANAGER.get_obj(conn_handle)?;

    // Read settings
    let keep_alive = {
        let guard = conn_ptr.lock()?;
        guard.settings.get("server_session_keep_alive")
            .and_then(|s| match s {
                Setting::String(v) => Some(v == "true"),
                _ => None,
            })
    };

    // Decision
    if keep_alive == Some(true) {
        cleanup_connection(&conn_ptr)?;
        return Ok(false);  // Didn't logout
    }

    // Logout
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        close_session(&client, &url, &token, &client_info).await
    })?;

    cleanup_connection(&conn_ptr)?;
    Ok(true)  // Did logout
}

fn cleanup_connection(conn: &Arc<Mutex<Connection>>) -> Result<()> {
    let mut guard = conn.lock()?;
    *guard.tokens.blocking_write() = None;  // Clear tokens
    guard.http_client = None;               // Drop client
    guard.settings.clear();                 // Clear settings
    Ok(())
}
```

## Python Wrapper

```python
# python/src/snowflake/connector/connection.py

def close(
    self,
    server_session_keep_alive: Optional[bool] = None,
    error_strategy: str = "best_effort",
) -> None:
    if self._closed:
        return

    try:
        self.db_api.connection_close(
            ConnectionCloseRequest(
                conn_handle=self.conn_handle,
                server_session_keep_alive=server_session_keep_alive,
                error_strategy=LogoutErrorStrategy.LOGOUT_ERROR_STRATEGY_BEST_EFFORT,
            )
        )
    finally:
        self._closed = True
        self.db_api.connection_release(
            ConnectionReleaseRequest(conn_handle=self.conn_handle)
        )
```

## Testing

### Integration (Mock TCP)
```rust
#[tokio::test]
async fn test_logout_success() {
    let (addr, _, server) = spawn_test_server(1, |_| async {
        b"HTTP/1.1 200 OK\r\n\r\n{\"success\":true}".to_vec()
    }).await;

    let result = close_session(&client, &format!("http://{}", addr), "token", &info).await;
    assert!(result.is_ok());
}
```

### E2E (Real Snowflake)
```rust
#[tokio::test]
async fn test_real_logout() {
    let settings = load_test_settings();
    let tokens = login(&settings).await.unwrap();
    let result = close_session(&client, &settings.url, &tokens.session_token, &info).await;
    assert!(result.is_ok());
}
```

## Phase 1 Scope

### ✅ Included
- Parameter-based keep-alive (true/false/null)
- HTTP logout with retry
- Error strategies (Strict/BestEffort)
- Token cleanup
- HTTP client cleanup
- Settings cleanup
- Idempotent close
- SESSION_GONE (390111) handling
- Socket timeout + retry budget
- request_guid rotation

### ❌ Excluded (Phase 2+)
- AsyncQueryRegistry (auto-detection)
- Heartbeat stop
- Telemetry flush
- Query context cache clear (doesn't exist)
- Fire-and-forget scenarios

## Common Pitfalls

1. ❌ Don't make `connection_close()` async (FFI requires sync)
2. ❌ Don't pass trait objects (use enums)
3. ❌ Don't assume heartbeat exists (doesn't)
4. ❌ Don't use Wiremock (use mock TCP servers)
5. ❌ Don't implement AsyncQueryRegistry in Phase 1 (too complex)
6. ❌ Don't forget `blocking_write()` for tokens (async RwLock in sync)

## Success Criteria

- [ ] Can call `connection_close()` from Python
- [ ] Session closes on Snowflake server (E2E verified)
- [ ] Retry logic works (integration tests pass)
- [ ] Error strategies work (strict raises, best_effort logs)
- [ ] Keep-alive parameter works (skips logout when true)
- [ ] Idempotent (multiple close() safe)
- [ ] Tokens nullified (security verified)
- [ ] All tests pass (integration + E2E)

## Test Commands

```bash
# Core integration
cargo test --test integration_tests logout

# Core E2E (needs credentials)
PARAMETER_PATH=parameters.json cargo test --test e2e_tests logout

# Python integration (Wiremock)
cd python && hatch run test:all tests/integ/session/test_logout.py

# Python E2E (real Snowflake)
cd python && PARAMETER_PATH=parameters.json hatch run test:all tests/e2e/session/test_logout.py
```

## Timeline

- Week 1-2: Core foundation (protobuf + HTTP + API)
- Week 3: Core integration tests
- Week 4: Core E2E tests
- Week 5: Python wrapper
- Week 6: Polish + docs

**Total:** 5-6 weeks for Phase 1 (Core + Python)
