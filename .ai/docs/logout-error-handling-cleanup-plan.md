# Plan: Clean Up Logout Error Handling

Branch: `SNOW-2872349-logout-implementation`
Date: 2026-02-26

## Important Notes for Agents

- **Pre-commit hooks may fail** due to virtualenv version / hatch conflict. Use `--no-verify` if needed.
- **Commit after each change** compiles. Small incremental commits preferred.
- **Run `cargo check --lib -p sf_core`** after each change to verify compilation before committing.

## Context

The logout error handling on this branch works but has several issues identified during clean code review:

1. **Over-engineered Strategy pattern** - trait + 2 structs + Box<dyn> for 2 enum variants with trivial logic
2. **Non-JSON responses crash** - `response.json()` called blindly on non-2xx responses (proxy HTML causes confusing `ResponseParse` error)
3. **28 test compilation errors** - field rename `timeout` to `logout_total_timeout` not propagated to tests
4. **Redundant error branches** - 3-way if/else in `connection_close` where branch 1 and 3 are identical
5. **Debug format in user-facing error** - `format!("{:?}", err)` produces Rust struct dumps
6. **Magic timeout constants** - undocumented 5s/120s bounds
7. **No token refresh for 390112** - 390112 should be retried with token refresh (same as normal requests), not treated as immediate failure
8. **Custom LogoutError type** - logout is a Snowflake REST API call and should return `RestError` like login and query, enabling reuse of existing `RefreshContext` for token refresh

### Error handling algorithm (refined from old driver comparison)

Snowflake returns error codes as **HTTP 200 with `success:false` and `code` in the JSON body** (confirmed across Python, Go, JDBC, Node.js drivers). The errors are NOT returned as HTTP 4xx status codes.

The complete error flow through all tiers:

```
Tier 1: execute_with_retry (HTTP retry — sf_core/src/http/retry.rs)
│  HTTP-level retries only. Does NOT examine response bodies.
├── Retries on: 503, 429, 408, 5xx, transport errors
├── Passes through: 2xx, 400, 401, 403, 404
└── Returns: Ok(response) or Err(HttpError)

Tier 2: logout_session (Snowflake codes — sf_core/src/rest/snowflake/logout.rs)
│  Parses JSON body. Maps Snowflake error codes to RestError variants.
├── 200 + success:true           → Ok(())
├── 200 + success:false + 390111 → Ok(()) (session already gone — true success)
├── 200 + success:false + 390112 → Err(SessionExpired) ← signals "refresh token + retry"
├── 200 + success:false + other  → Err(RestError::LogoutFailed { code })
├── Non-2xx + JSON with 390111   → Ok(())
├── Non-2xx + JSON with 390112   → Err(SessionExpired)
├── Non-2xx + JSON with other    → Err(RestError::LogoutFailed { code })
├── Non-2xx + non-JSON body      → Err(RestError::InvalidSnowflakeResponse { ResponseStatus })
└── Err(HttpError)               → mapped to RestError (same pattern as async_exec.rs)

Tier 3: RefreshContext loop in connection_close (sf_core/src/apis/.../connection.rs)
│  Same pattern as statement.rs. Retries the entire logout on SessionExpired.
├── SessionExpired → refresh master token → retry logout_session → loop back to Tier 2
├── Ok(())         → success
├── Other RestError → propagates as ApiError → remapped to ApiError::LogoutFailed
└── Final ApiError → strategy.handle_failed_logout() → Strict raises, BestEffort suppresses
```

Only **390111 (SESSION_GONE)** is true success — session destroyed server-side.
**390112** is retried with token refresh (same pattern as normal Snowflake requests via RefreshContext). If refresh fails or retry fails, the error goes to strategy.

## Files to Modify

| File | Change |
|------|--------|
| `sf_core/src/config/logout.rs` | Remove trait/structs/deprecated, rename enum to `ErrorHandlingStrategy`, add `handle_failed_logout` |
| `sf_core/src/rest/snowflake/mod.rs` | Add `RestError::LogoutFailed` variant |
| `sf_core/src/rest/snowflake/logout.rs` | Delete `LogoutError`, return `RestError`, map 390112→SessionExpired |
| `sf_core/src/apis/database_driver_v1/connection.rs` | Use RefreshContext for logout (same as statement.rs), simplify error flow |
| `sf_core/src/apis/database_driver_v1/error.rs` | Update `ApiError::LogoutFailed` if needed (check source field) |
| `sf_core/src/protobuf_apis/database_driver_v1.rs` | Update import from `ErrorStrategy` to `ErrorHandlingStrategy` |
| `sf_core/tests/integration/session/logout.rs` | Fix compilation errors + update strategy + implement 390112 tests |
| `sf_core/tests/e2e/session/logout.rs` | Fix 6 compilation errors |

## Changes (in dependency order)

### 1. Simplify ErrorStrategy — `sf_core/src/config/logout.rs`

**Remove** `LogoutError` dependency — strategy will work on `ApiError` instead.

**Remove** (lines 55-174):
- `ErrorHandlingStrategy` trait (the old one, not the new enum name)
- `StrictStrategy` struct + impl
- `BestEffortStrategy` struct + impl

**Remove** from `ErrorStrategy` impl:
- `to_handler()` method (lines 198-203)
- Deprecated `should_ignore_error` method (lines 209-222)

**Remove** `SESSION_GONE_ERROR_CODE` — no longer needed here (390111 is handled inside `logout_session`, not in the strategy).

**Rename enum** from `ErrorStrategy` to `ErrorHandlingStrategy`.

**Add** single method on `ErrorHandlingStrategy` enum that works on `ApiError`:

```rust
impl ErrorHandlingStrategy {
    /// Handle a failed logout after all retry mechanisms have been exhausted.
    ///
    /// Called after both retry layers have given up:
    /// - HTTP retries (execute_with_retry) for 503, 429, transport errors
    /// - Token refresh (RefreshContext) for 390112 session token expired
    ///
    /// By this point, recoverable errors (390111 session gone, 390112 token expired)
    /// have already been resolved. What remains are unrecoverable failures
    /// (network unreachable, timeout exceeded, unknown server errors).
    ///
    /// Strict: surface the error to the caller (close() may fail)
    /// BestEffort: suppress the error, log WARN (close() always succeeds)
    pub fn handle_failed_logout(self, result: Result<(), ApiError>) -> Result<(), ApiError> {
        match result {
            Ok(()) => Ok(()),
            Err(e) => match self {
                ErrorHandlingStrategy::Strict => {
                    tracing::error!(error = %e, "Logout failed after retries exhausted");
                    Err(e)
                }
                ErrorHandlingStrategy::BestEffort => {
                    tracing::warn!(error = %e, "Logout failed after retries exhausted, suppressed");
                    Ok(())
                }
            }
        }
    }
}
```

**Update tests** at bottom of file: test `handle_failed_logout` with mock `ApiError::LogoutFailed` for both Strict (raises) and BestEffort (suppresses).

### 2. Change `logout_session` to return `RestError` — `sf_core/src/rest/snowflake/logout.rs`

**Core change:** `logout_session` is a Snowflake REST API call — it should return `RestError` like login and query, not a custom `LogoutError` type. This enables `RefreshContext` to handle 390112 token refresh automatically.

**Delete** the entire `LogoutError` enum. Replace with existing error types:

| Old `LogoutError` variant | New error (from existing types) |
|---|---|
| `UrlConstruction` | `RestError::UrlJoin { path: "/session" }` (already exists) |
| `Http` | `SfError` variants via `map_http_error()` → `RestError::AsyncQuery` |
| `ResponseParse` | `SnowflakeResponseError::ResponseFormat` → `RestError::InvalidSnowflakeResponse` |
| `LogoutFailed` | New: `RestError::LogoutFailed { message, code }` (add variant) |

**Add** `RestError::LogoutFailed` variant to `sf_core/src/rest/snowflake/mod.rs:888`:
```rust
#[snafu(display("Logout failed: {message} (code: {code})"))]
LogoutFailed {
    message: String,
    code: i32,
    #[snafu(implicit)]
    location: Location,
},
```

**Add** `map_http_error` function in `logout.rs` (reuse pattern from `async_exec.rs:342-374`):
```rust
fn map_http_error(err: HttpError) -> RestError {
    // Same HttpError → SfError mapping as async_exec.rs, then wrap in RestError::AsyncQuery
    // Or add a new RestError variant that wraps SfError/HttpError for logout
}
```

**Change return type** of `logout_session` from `Result<(), LogoutError>` to `Result<(), RestError>`.

**Map 390112 → `SessionExpired`** inside `logout_session`. When the JSON response has `success:false` and `code:"390112"`, return:
```rust
Err(RestError::InvalidSnowflakeResponse {
    source: SnowflakeResponseError::SessionExpired { location: Location::default() }
})
```
This makes `RefreshContext` detect it and trigger token refresh automatically — same as when queries get HTTP 401.

**Handle non-2xx responses**: try JSON parse, fall back to `RestError::InvalidSnowflakeResponse { source: ResponseStatus { status, body } }`.

**Inline** `generate_request_id()` and `generate_request_guid()` — replace with `uuid::Uuid::new_v4()`.

### 3. Use RefreshContext in `connection_close` — `sf_core/src/apis/database_driver_v1/connection.rs`

**Extract** magic timeout bounds to named constants:
```rust
const MIN_PER_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_PER_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
```

**Replace** the manual logout data extraction + HTTP call (Phase 1+2, lines 598-710) with the `RefreshContext` pattern from `statement.rs:336-356`:

```rust
// Extract connection data under lock (same as current code does)
let conn = conn_ptr.lock().map_err(|_| ConnectionLockingSnafu {}.build())?;
let http_client = conn.http_client.clone().context(ConnectionNotInitializedSnafu)?;
let server_url = conn.server_url.clone().context(ConnectionNotInitializedSnafu)?;
let client_info = conn.client_info.clone().context(ConnectionNotInitializedSnafu)?;
// Build RefreshContext from same connection (clones tokens_lock internally)
let refresh_ctx_result = RefreshContext::new(&conn);
drop(conn); // Release lock before HTTP calls

// Phase 2: HTTP logout with token refresh (same pattern as statement.rs:336-356)
let logout_result = if send_logout {
    let rt = crate::async_bridge::runtime().context(RuntimeCreationSnafu)?;
    let mut ctx = refresh_ctx_result?;
    rt.block_on(async {
        let mut last_error: Option<RestError> = None;
        loop {
            let session_token = ctx.refresh_token(last_error).await?;
            match logout_session(
                &http_client, &server_url, &session_token,
                &client_info, per_request_timeout, &logout_retry_policy,
            ).await {
                Ok(()) => return Ok(()),
                Err(e) => last_error = Some(e),
            }
        }
    })
} else { Ok(()) };
```

Note: `RefreshContext` wraps non-SessionExpired errors as `ApiError::Query`. Remap to `ApiError::LogoutFailed`:
```rust
let logout_result = logout_result.map_err(|e| match e {
    ApiError::Query { source, .. } => LogoutFailedSnafu { message: format!("{source}") }.build(),
    other => other,
});
```

**Apply strategy** on `ApiError`:
```rust
let logout_result = config.error_strategy.handle_failed_logout(logout_result);
```

**RefreshContext field visibility:** `RefreshContext` fields are private, but this is fine. Statement.rs uses the same pattern — extract `http_client` etc. from `Connection` (pub fields) separately, and only use `RefreshContext` for token management. The `Connection` fields `http_client`, `server_url`, `client_info` are all `pub`.

### 4. Fix test compilation — `sf_core/tests/integration/session/logout.rs`

**Mechanical renames** (22 errors):
- `config.timeout` → `config.logout_total_timeout` (14 occurrences)
- struct field `timeout:` → `logout_total_timeout:` (6 occurrences)
- `timeout_seconds:` → `logout_total_timeout_seconds:` (2 occurrences: lines 193, 1393)

**Update strategy usage** (3 occurrences at lines 1030, 1175, 1350). Since `logout_session` now returns `RestError` (not `LogoutError`), and the strategy works on `ApiError`:
```rust
// Before:
let strategy = config.error_strategy.to_handler();
let handled_ok = match &result {
    Ok(()) => true,
    Err(e) => strategy.should_ignore_error(e) || !strategy.should_raise_error(e),
};

// After: wrap RestError as ApiError::LogoutFailed, then apply strategy
let api_result = result.map_err(|e| LogoutFailedSnafu { message: format!("{e}") }.build());
let handled_result = config.error_strategy.handle_failed_logout(api_result);
```

**Implement and un-ignore 390112 token refresh tests:**

- Line 766: `should_attempt_token_refresh_on_390112_when_retries_allowed_for_each_strategy_type` — **Un-ignore and implement.** Mock server returns 200+`success:false`+390112 on first attempt, mock refresh endpoint, mock server returns 200+`success:true` on second attempt. Verify: refresh was called, logout retried with new token, close succeeds.

- Line 788: `should_not_attempt_token_refresh_when_retry_count_is_0_for_each_strategy_type` — **Un-ignore and implement.** Same 390112 mock but `max_retry_attempts=1` (no retries). Verify: no refresh attempted. Strict raises, BestEffort suppresses.

**Keep ignored** (different scope):
- Line 525: `should_wait_for_in_flight_token_renewal_to_complete_then_logout_with_refreshed_token` — concurrent refresh race
- Line 809: `should_include_token_refresh_time_in_total_logout_timeout_budget` — timing budget enforcement

### 5. Fix test compilation — `sf_core/tests/e2e/session/logout.rs`

Mechanical renames (6 errors):
- `timeout_seconds: None` → `logout_total_timeout_seconds: None` (lines 29, 55, 66, 77, 111, 149)

## Commit Strategy

Commit after each change compiles successfully. Pre-commit hooks may fail due to virtualenv/hatch conflict — use `--no-verify` if needed.

1. After Change 1 (simplify ErrorStrategy) — `cargo check --lib -p sf_core`
2. After Change 2 (logout_session → RestError) — `cargo check --lib -p sf_core`
3. After Change 3 (RefreshContext in connection_close) — `cargo check --lib -p sf_core`
4. After Change 4+5 (test fixes) — `cargo check --tests -p sf_core`
5. After implementing 390112 refresh tests — `cargo test --test integration_tests -p sf_core -- session::logout`

## Verification

```bash
# Library compiles
cargo check --lib -p sf_core

# All tests compile (0 errors expected)
cargo check --tests -p sf_core

# Unit tests pass (logout config + logout HTTP)
cargo test -p sf_core -- config::logout::tests
cargo test -p sf_core -- rest::snowflake::logout::tests

# Decision logic + async registry tests pass
cargo test -p sf_core -- logout_decision::tests
cargo test -p sf_core -- async_query_registry::tests

# Integration tests pass (mock-server-based, run locally)
cargo test --test integration_tests -p sf_core -- session::logout

# E2E tests compile (need real Snowflake, verify compilation only)
cargo test --no-run --test e2e_tests -p sf_core

# Verify no remaining #[ignore] tests that should have been un-ignored
# Expected: only 6 ignored tests remain (2 close-vs-query, 1 close-vs-refresh-race,
#           1 refresh-in-budget, 1 telemetry, 1 heartbeat-e2e)
```

## Key Existing Code to Reuse

| Function/Type | Location | Purpose |
|---|---|---|
| `execute_with_retry()` | `sf_core/src/http/retry.rs:73` | HTTP-level retry (already used by logout) |
| `RefreshContext` | `sf_core/src/apis/.../connection.rs:346` | Token refresh loop (used by statement.rs) |
| `refresh_session()` | `sf_core/src/rest/snowflake/mod.rs:371` | Token refresh HTTP call |
| `SessionTokens::is_master_expired()` | `sf_core/src/rest/snowflake/mod.rs:61` | Master token expiry check |
| `map_http_error()` | `sf_core/src/rest/snowflake/async_exec.rs:342` | HttpError → SfError mapping pattern |
| `read_response_json()` | `sf_core/src/rest/snowflake/mod.rs:832` | Shared response parser (NOT used by logout — see note below) |

**Note on `read_response_json`:** Logout cannot reuse this function because it discards JSON bodies for non-2xx responses. Snowflake returns 390111 (SESSION_GONE) with HTTP 410 + JSON body. `read_response_json` would lose the error code. Logout keeps its own body parsing that always attempts JSON parse regardless of HTTP status.
