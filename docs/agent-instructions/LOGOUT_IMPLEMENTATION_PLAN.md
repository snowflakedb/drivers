# Logout Implementation Plan

**Date:** January 27, 2026  
**Ticket:** SNOW-2872349  
**Purpose:** Step-by-step implementation guide for logout functionality tests  
**Target:** Core (Rust) and Python drivers

---

## Overview

This plan outlines the **exact order** for implementing logout functionality to satisfy all Gherkin tests. Implementation follows a **bottom-up, dependency-first** approach:

1. **Core HTTP Layer** → Foundation for all logout behavior
2. **Core E2E Basic Logout** → Simple end-to-end flow
3. **Core E2E Advanced Features** → Error handling, keep-alive, auto-detection
4. **Python E2E** → Wrapper integration and Phase 2 behavior

---

## Phase 1: Core HTTP Layer (Integration Tests)

**File:** `sf_core/tests/integration/session/logout.rs`  
**Feature:** `tests/definitions/core/session/logout.feature`  
**Priority:** CRITICAL - Foundation for everything  
**Estimated Scenarios:** 4

### Implementation Order:

#### 1.1 HTTP Request Construction
**Test:** `should_construct_logout_request_with_correct_http_method_url_headers_and_body`  
**Dependencies:** None  
**Implementation needs:**
- Mock HTTP server to capture requests
- Logout function that constructs HTTP request
- Validate:
  - Method: POST
  - Path: /session
  - Query params: `delete=true`, `requestId`, `request_guid`
  - Headers: Authorization, Content-Type, Accept, User-Agent
  - Body: `{}`

**Code to implement:**
```rust
// In sf_core/src/rest/snowflake/logout.rs (new file)
pub async fn logout_session(
    client: &reqwest::Client,
    session_token: String,
    // ... other params
) -> Result<(), LogoutError>
```

---

#### 1.2 Retry Policy Integration
**Test:** `should_apply_retry_policy_to_logout_http_request`  
**Dependencies:** 1.1 (HTTP request construction)  
**Implementation needs:**
- Integrate with existing `sf_core::http::retry` module
- Mock server returns 503 then 200
- Validate retry logic triggers
- Validate backoff delay occurs

**Code to implement:**
```rust
// Use existing execute_bytes_with_retry from sf_core::http::retry
```

---

#### 1.3 Connection Reset Handling
**Test:** `should_handle_http_connection_reset_during_logout`  
**Dependencies:** 1.1, 1.2  
**Implementation needs:**
- Mock server resets connection
- Validate retry on connection reset
- Should use existing retry infrastructure

---

#### 1.4 Telemetry Metrics
**Test:** `should_record_connection_close_decision_metrics_before_logout`  
**Dependencies:** 1.1  
**Implementation needs:**
- Telemetry recording before logout
- Metrics include: auto-detection performed, queries found, logout sent/skipped
- Must flush before logout executes

**Code to implement:**
```rust
// In sf_core/src/telemetry (if exists) or new module
pub struct ConnectionCloseMetrics {
    auto_detection_performed: bool,
    async_queries_found: Option<bool>,
    logout_sent: bool,
    skip_reason: Option<String>,
}
```

---

## Phase 2: Core Connection Close Logic

**File:** `sf_core/src/apis/database_driver_v1/connection.rs` (or new module)  
**Priority:** HIGH - Core business logic  

### Implementation Order:

#### 2.1 Basic Connection Close
**Implementation needs:**
- `connection_close()` function
- Calls logout HTTP when appropriate
- Basic resource cleanup

**Code structure:**
```rust
pub fn connection_close(
    connection: &Connection,
    config: &LogoutConfig,
) -> Result<(), ConnectionCloseError>
```

---

#### 2.2 Server Session Keep Alive Logic
**Implementation needs:**
- `LogoutConfig` struct with fields:
  - `server_session_keep_alive: Option<bool>`
  - `enable_server_session_keep_alive_auto_detection: Option<bool>`
- Decision logic:
  - `true` → skip logout
  - `false` → send logout
  - `null` → check auto-detection config

**Code structure:**
```rust
pub struct LogoutConfig {
    pub server_session_keep_alive: Option<bool>,
    pub enable_auto_detection: Option<bool>,
    pub error_strategy: ErrorStrategy,
    pub timeout: Duration,
}

pub enum ErrorStrategy {
    Strict,
    BestEffort,
}
```

---

#### 2.3 Async Query Registry
**Implementation needs:**
- Registry to track async query IDs
- `register_async_query(query_id)` function
- `unregister_async_query(query_id)` function
- `has_running_async_queries()` function (early return on first found)

**Code structure:**
```rust
// In Connection struct
pub struct Connection {
    // ... existing fields
    async_query_registry: Arc<Mutex<HashSet<String>>>,
}
```

---

#### 2.4 Auto-Detection Logic
**Implementation needs:**
- Function to check if async queries are running
- Integration with keep-alive decision
- Should short-circuit on first running query found

**Code structure:**
```rust
fn should_skip_logout(
    config: &LogoutConfig,
    async_registry: &AsyncQueryRegistry,
) -> (bool, String) // (skip, reason)
```

---

## Phase 3: Core E2E Tests (Shared Scenarios)

**File:** `sf_core/tests/e2e/session/logout.rs`  
**Feature:** `tests/definitions/shared/session/logout.feature`  
**Estimated Scenarios:** 38 (Core), 32 (Python)  

### Implementation Order:

#### 3.1 Basic Logout Request (5 scenarios)
**Priority:** CRITICAL  
**Tests:**
1. `should_send_logout_with_default_settings`
2. `should_send_logout_request_with_correct_endpoint_method_headers_and_payload`
3. `should_send_logout_request_with_default_5_second_timeout`
4. `should_send_logout_request_with_custom_timeout_when_configured`
5. `should_not_send_logout_when_connection_was_never_established`

**Implementation order:** Sequential (1→2→3→4→5)  
**Dependencies:** Phase 1 (HTTP layer), Phase 2.1 (basic close)

---

#### 3.2 Server Session Keep Alive (2 scenarios for Core)
**Priority:** HIGH  
**Tests:**
1. `should_not_send_logout_when_server_session_keep_alive_is_explicitly_true`
2. `should_send_logout_when_server_session_keep_alive_is_explicitly_false` (Core only)
3. `should_not_start_async_queries_detection_when_server_session_keep_alive_is_explicitly_set`

**Implementation order:** Sequential  
**Dependencies:** Phase 2.2 (keep-alive logic)

---

#### 3.3 Auto-Detection Mechanics (3 scenarios)
**Priority:** HIGH  
**Tests:**
1. `should_skip_logout_when_auto_detection_enabled_and_running_async_query_detected`
2. `should_send_logout_when_auto_detection_enabled_and_no_async_queries_detected`
3. `should_send_logout_when_auto_detection_explicitly_disabled`

**Implementation order:** Sequential  
**Dependencies:** Phase 2.3 (registry), Phase 2.4 (auto-detection)

---

#### 3.4 Phase 3 Default Configuration (3 scenarios for Core)
**Priority:** MEDIUM  
**Tests:**
1. `should_have_enable_server_session_keep_alive_auto_detection_default_to_false` (Core only)
2. `should_always_send_logout_with_phase_3_default_configuration` (Core only)
3. `should_skip_logout_when_auto_detection_explicitly_enabled_with_running_queries_in_phase_3_model` (Core only)

**Implementation order:** Sequential  
**Dependencies:** Phase 3.3

---

#### 3.5 Async Query Registry (2 scenarios)
**Priority:** HIGH  
**Tests:**
1. `should_register_async_query_when_async_exec_is_true`
2. `should_unregister_async_query_when_query_completes`

**Implementation order:** Sequential  
**Dependencies:** Phase 2.3 (registry infrastructure)

---

#### 3.6 Resource Cleanup Contract (7 scenarios)
**Priority:** HIGH  
**Tests:**
1. `should_allow_process_to_exit_cleanly_when_connection_closed_regardless_of_parameters`
2. `should_stop_heartbeat_on_close_regardless_of_logout_result`
3. `should_flush_telemetry_on_close_regardless_of_logout_result`
4. `should_clear_query_result_cache_on_close_regardless_of_logout_result`
5. `should_cleanup_all_tokens_on_close_regardless_of_whether_logout_was_sent`
6. `should_not_allow_token_renewal_after_connection_is_closed`
7. `should_be_idempotent_when_close_called_multiple_times`

**Implementation order:** Can be parallel after basic close works  
**Dependencies:** Phase 2.1 (basic close), existing heartbeat/telemetry/QCC modules

**Code to implement:**
```rust
// In connection_close():
// 1. Set closed flag
// 2. Stop heartbeat
// 3. Flush telemetry
// 4. Clear QCC
// 5. Clear tokens
// 6. Block token renewal
// 7. Ensure idempotent (check closed flag)
```

---

#### 3.7 Error Handling - Strict Strategy (6 scenarios for Core)
**Priority:** HIGH  
**Tests:**
1. `should_ignore_session_gone_error_in_strict_strategy`
2. `should_retry_on_transient_error_in_strict_strategy`
3. `should_fail_close_on_non_retryable_error_in_strict_strategy`
4. `should_attempt_token_renewal_and_retry_logout_when_session_token_expired_in_strict_strategy`
5. `should_surface_reauth_error_when_master_token_expired_in_strict_strategy`
6. `should_log_warn_on_final_logout_failure_after_all_retries_exhausted_in_strict_strategy`

**Implementation order:** Sequential (1→2→3→4→5→6)  
**Dependencies:** Phase 2.2 (error strategy enum)

**Code to implement:**
```rust
impl ErrorStrategy {
    pub fn handle_logout_error(&self, error: LogoutError) -> Result<(), Error> {
        match self {
            ErrorStrategy::Strict => {
                if error.code == 390111 { // SESSION_GONE
                    Ok(()) // Ignore
                } else {
                    Err(error) // Propagate
                }
            }
            ErrorStrategy::BestEffort => {
                log::warn!("Logout failed: {:?}", error);
                Ok(()) // Always succeed
            }
        }
    }
}
```

---

#### 3.8 Error Handling - Best-Effort Strategy (5 scenarios)
**Priority:** HIGH  
**Tests:**
1. `should_log_all_errors_as_warn_in_best_effort_strategy`
2. `should_never_throw_exception_from_close_in_best_effort_strategy`
3. `should_succeed_close_even_on_logout_timeout_in_best_effort_strategy`
4. `should_log_warn_and_suppress_error_when_master_token_expired_in_best_effort_strategy`
5. `should_log_warn_on_final_logout_failure_after_all_retries_exhausted_in_best_effort_strategy`

**Implementation order:** Sequential  
**Dependencies:** Phase 3.7 (strategy implementation)

---

#### 3.9 Strategy Configuration (1 scenario)
**Priority:** MEDIUM  
**Test:** `should_support_switching_between_error_handling_strategies`  
**Dependencies:** Phase 3.7, 3.8

---

#### 3.10 Timeout and Retry Behavior (6 scenarios)
**Priority:** MEDIUM  
**Tests:**
1. `should_timeout_logout_request_after_configured_timeout`
2. `should_retry_logout_on_retryable_http_errors`
3. `should_not_retry_logout_on_non_retryable_errors`
4. `should_respect_max_retry_attempts_from_http_policy`
5. `should_use_exponential_backoff_for_logout_retries`
6. `should_not_block_process_exit_when_timeout_expires`

**Implementation order:** Can be parallel  
**Dependencies:** Phase 1.2 (retry policy)

---

#### 3.11 Edge Cases and Concurrency (6 scenarios)
**Priority:** LOW (implement last)  
**Tests:**
1. `should_handle_concurrent_close_calls_safely`
2. `should_handle_close_during_active_query_execution`
3. `should_handle_close_during_session_token_refresh`
4. `should_handle_network_failure_during_logout`
5. `should_handle_close_with_expired_session_token`
6. `should_handle_close_when_server_is_unreachable`

**Implementation order:** Can be parallel  
**Dependencies:** All above phases working

---

## Phase 4: Python Wrapper Integration

**Files:** `python/tests/e2e/session/test_logout.py`, `python/tests/integ/session/test_logout.py`  
**Features:** `tests/definitions/shared/session/logout.feature`, `tests/definitions/python/session/logout.feature`  
**Estimated Scenarios:** 32 shared + 14 Python-specific = 46

### Implementation Order:

#### 4.1 Python FFI Bindings
**Priority:** CRITICAL  
**Implementation needs:**
- Expose Core `connection_close()` to Python
- Map Python parameters to Core `LogoutConfig`
- Handle Result types from Core

**Code to implement:**
```python
# In python/src/snowflake/connector/connection.py (or similar)
def close(self, retry=True):
    config = LogoutConfig(
        server_session_keep_alive=self._server_session_keep_alive,
        enable_auto_detection=self._enable_auto_detection,
        error_strategy=ErrorStrategy.BestEffort,  # Python default
        timeout=5.0,
    )
    core_connection_close(self._handle, config)
```

---

#### 4.2 Python Basic Integration (Shared Scenarios)
**Priority:** HIGH  
**Tests:** All 32 shared scenarios that apply to Python (excludes `@python_not_needed`)  
**Implementation order:** Same as Phase 3 (follow Core order)  
**Dependencies:** Phase 4.1 (FFI bindings), corresponding Core implementation

**Note:** These tests verify Python wrapper correctly calls Core functions

---

#### 4.3 Python Phase 2 Defaults Configuration
**Priority:** HIGH  
**Test:** `test_should_have_phase_2_defaults_that_enable_auto_detection`  
**Dependencies:** Phase 4.1  
**Implementation needs:**
- Python connection defaults:
  - `server_session_keep_alive = None`
  - `enable_server_session_keep_alive_auto_detection = True`

**Code to implement:**
```python
class SnowflakeConnection:
    def __init__(self, **kwargs):
        self._server_session_keep_alive = kwargs.get('server_session_keep_alive', None)
        self._enable_auto_detection = kwargs.get(
            'enable_server_session_keep_alive_auto_detection', 
            True  # Phase 2 default
        )
```

---

#### 4.4 Python Phase 2 Truth Table (6 scenarios)
**Priority:** HIGH  
**Tests:**
1. `test_should_skip_logout_when_server_session_keep_alive_is_none_and_auto_detection_true_and_async_queries_found`
2. `test_should_send_logout_when_server_session_keep_alive_is_none_and_auto_detection_true_and_no_async_queries_found`
3. `test_should_send_logout_when_server_session_keep_alive_is_none_and_auto_detection_false`
4. `test_should_skip_logout_when_server_session_keep_alive_is_false_and_auto_detection_true_and_async_queries_found`
5. `test_should_send_logout_when_server_session_keep_alive_is_false_and_auto_detection_true_and_no_async_queries_found`
6. `test_should_send_logout_when_server_session_keep_alive_is_false_and_auto_detection_false`

**Implementation order:** Sequential (test each truth table row)  
**Dependencies:** Phase 4.3 (defaults), Core auto-detection

**Implementation needs:**
- All parameter combinations work correctly
- Deprecation warnings emitted when `server_session_keep_alive = false`
- Telemetry records decision
- Test cleanup of async queries (`SYSTEM$SLEEP(300)`)

---

#### 4.5 Python Default Configurations (2 scenarios)
**Priority:** MEDIUM  
**Tests:**
1. `test_should_have_enable_server_session_keep_alive_auto_detection_default_to_true`
2. `test_should_perform_auto_detection_when_server_session_keep_alive_is_explicitly_false`

**Implementation order:** Sequential  
**Dependencies:** Phase 4.3

---

#### 4.6 Python Error Handling Strategy (1 scenario)
**Priority:** MEDIUM  
**Test:** `test_should_use_best_effort_error_handling_strategy_by_default`  
**Dependencies:** Phase 3.8 (Core best-effort), Phase 4.1

**Implementation needs:**
- Python wrapper configures Core with `ErrorStrategy::BestEffort`
- Validate errors logged but not thrown

---

#### 4.7 Python Auto-cleanup Deprecation (4 scenarios)
**Priority:** LOW (can be last)  
**Tests:**
1. `test_should_register_atexit_handler_that_calls_close_in_legacy_mode`
2. `test_should_emit_deprecation_warning_on_first_auto_cleanup_run_per_process`
3. `test_should_not_register_atexit_handler_when_auto_cleanup_explicitly_disabled`
4. `test_should_emit_telemetry_and_warn_when_connection_leaked_at_process_exit`

**Implementation order:** Sequential  
**Dependencies:** Phase 4.2 (basic Python close works)

**Implementation needs:**
```python
# In __init__:
if kwargs.get('auto_cleanup', True):  # Phase 1: default on
    atexit.register(self._close_at_exit)

def _close_at_exit(self):
    if self._first_auto_cleanup_in_process:
        warnings.warn(DeprecationWarning("Auto-cleanup will be disabled..."))
        self._first_auto_cleanup_in_process = False
    self.close(retry=False)
```

---

## Phase 5: Core Integration Test (Shared Scenario)

**File:** `sf_core/tests/integration/session/logout.rs` (1 additional test)  
**Test:** `should_return_true_when_first_running_async_query_is_detected_without_checking_remaining_queries`  
**Priority:** MEDIUM  
**Dependencies:** Phase 2.3 (registry), Phase 2.4 (auto-detection)

**Implementation needs:**
- Mock registry with multiple queries
- Validate early return (doesn't check all)
- Performance optimization test

---

## Implementation Timeline

### Sprint 1: Foundation (Core)
**Duration:** ~1 week  
**Deliverables:**
- Phase 1: Core HTTP Layer (4 integration tests) ✅
- Phase 2.1: Basic connection close ✅
- Phase 3.1: Basic logout E2E tests (5 scenarios) ✅

**Exit criteria:** Can successfully logout a Snowflake session from Core

---

### Sprint 2: Core Advanced Features
**Duration:** ~1-2 weeks  
**Deliverables:**
- Phase 2.2: Keep-alive logic ✅
- Phase 2.3: Async query registry ✅
- Phase 2.4: Auto-detection ✅
- Phase 3.2: Keep-alive E2E tests (2-3 scenarios) ✅
- Phase 3.3: Auto-detection E2E tests (3 scenarios) ✅
- Phase 3.5: Registry E2E tests (2 scenarios) ✅

**Exit criteria:** Keep-alive and auto-detection working in Core

---

### Sprint 3: Core Error Handling & Resources
**Duration:** ~1-2 weeks  
**Deliverables:**
- Phase 3.6: Resource cleanup (7 scenarios) ✅
- Phase 3.7: Strict strategy (6 scenarios) ✅
- Phase 3.8: Best-effort strategy (5 scenarios) ✅
- Phase 3.9: Strategy switching (1 scenario) ✅
- Phase 3.10: Timeout/retry (6 scenarios) ✅

**Exit criteria:** Complete error handling and cleanup

---

### Sprint 4: Python Integration
**Duration:** ~1 week  
**Deliverables:**
- Phase 4.1: Python FFI bindings ✅
- Phase 4.2: Python shared scenarios (32 tests) ✅
- Phase 4.3: Python Phase 2 defaults ✅
- Phase 4.4: Python truth table (6 tests) ✅
- Phase 4.5: Python default configs (2 tests) ✅
- Phase 4.6: Python error strategy (1 test) ✅

**Exit criteria:** Python wrapper works with all shared scenarios

---

### Sprint 5: Python Phase 2 Features & Polish
**Duration:** ~3-5 days  
**Deliverables:**
- Phase 4.7: Auto-cleanup deprecation (4 tests) ✅
- Phase 3.11: Edge cases (6 scenarios) ✅
- Phase 3.4: Phase 3 defaults (3 scenarios) ✅
- Phase 5: Integration test (1 test) ✅

**Exit criteria:** All Core and Python tests passing

---

## Critical Path

```
Phase 1 (HTTP Layer)
  ↓
Phase 2.1 (Basic Close)
  ↓
Phase 3.1 (Basic E2E) → Can test end-to-end!
  ↓
Phase 2.2 (Keep-Alive) + Phase 2.3 (Registry)
  ↓
Phase 2.4 (Auto-Detection)
  ↓
Phase 3.2 + 3.3 + 3.5 (Keep-Alive & Auto-Detection E2E)
  ↓
Phase 3.6 + 3.7 + 3.8 (Resources & Error Handling)
  ↓
Phase 4.1 (Python FFI)
  ↓
Phase 4.2 + 4.3 + 4.4 (Python Integration)
  ↓
Phase 3.10 + 3.11 + 4.7 (Polish & Edge Cases)
```

---

## Key Implementation Files

### Core (Rust):
```
sf_core/src/rest/snowflake/logout.rs          # New: HTTP logout function
sf_core/src/apis/database_driver_v1/
  connection.rs                                 # Modify: Add close logic
  async_query_registry.rs                       # New: Registry module
sf_core/src/config/
  logout_config.rs                              # New: Configuration structs
```

### Python:
```
python/src/snowflake/connector/
  connection.py                                 # Modify: Add close(), atexit
  logout_config.py                              # New: Config mapping
```

---

## Test Execution Strategy

### During Development:
```bash
# Run specific test
cargo test --test integration should_construct_logout_request

# Run all logout integration tests
cargo test --test integration logout

# Run all logout E2E tests
cargo test --test e2e logout

# Python tests
pytest python/tests/e2e/session/test_logout.py::TestLogoutBasic::test_should_send_logout_with_default_settings -v
```

### TDD Cycle:
1. Pick next scenario from plan
2. Read Gherkin scenario
3. Run test (should fail)
4. Implement minimal code to pass
5. Refactor
6. Move to next scenario

---

## Validation Checkpoints

### After Phase 1:
```bash
cargo test --test integration logout
# Expected: 4/4 passing
```

### After Phase 3:
```bash
cargo test --test e2e logout
# Expected: 38/38 passing (Core scenarios)
```

### After Phase 4:
```bash
pytest python/tests/e2e/session/test_logout.py -v
# Expected: 46/46 passing (32 shared + 14 Python-specific)
```

### Final:
```bash
# Run validator
./tests/tests_format_validator/target/release/tests_format_validator
# Expected: All logout features ✅
```

---

## Dependencies Summary

| Phase | Depends On | Blocks |
|-------|-----------|--------|
| 1.1 HTTP Construction | None | Everything |
| 1.2 Retry Policy | 1.1 | 1.3, 3.7, 3.10 |
| 1.3 Connection Reset | 1.1, 1.2 | - |
| 1.4 Telemetry | 1.1 | - |
| 2.1 Basic Close | 1.1 | 3.1, 3.6 |
| 2.2 Keep-Alive Logic | 2.1 | 3.2 |
| 2.3 Registry | 2.1 | 2.4, 3.5 |
| 2.4 Auto-Detection | 2.3 | 3.3 |
| 3.1 Basic E2E | 2.1, 1.* | 4.1 |
| 3.2 Keep-Alive E2E | 2.2 | - |
| 3.3 Auto-Detection E2E | 2.4 | 3.4 |
| 3.4 Phase 3 Defaults | 3.3 | - |
| 3.5 Registry E2E | 2.3 | - |
| 3.6 Resource Cleanup | 2.1 | - |
| 3.7 Strict Strategy | 2.2 | 3.9 |
| 3.8 Best-Effort Strategy | 2.2 | 3.9 |
| 3.9 Strategy Config | 3.7, 3.8 | 4.6 |
| 3.10 Timeout/Retry | 1.2 | - |
| 3.11 Edge Cases | All above | - |
| 4.1 Python FFI | 3.1 | 4.2-4.7 |
| 4.2 Python Shared | 4.1 | 4.7 |
| 4.3 Python Defaults | 4.1 | 4.4 |
| 4.4 Python Truth Table | 4.3 | - |
| 4.5 Python Configs | 4.3 | - |
| 4.6 Python Strategy | 3.8, 4.1 | - |
| 4.7 Python Auto-cleanup | 4.2 | - |
| 5 Integration Opt | 2.4 | - |

---

## Success Criteria

### Per Phase:
- ✅ All tests in phase passing
- ✅ No regressions in existing tests
- ✅ Code reviewed
- ✅ Documentation updated (inline comments)

### Final:
- ✅ All 73 test scenarios passing (38 Core E2E + 5 Core integration + 46 Python)
- ✅ Validator reports all scenarios implemented
- ✅ No TODO/skip markers in test code
- ✅ Code coverage >80% for new logout module
- ✅ Design docs validated against implementation

---

## Current State of Test Files

### Core Tests (Rust)
**Status:** ✅ Basic skeleton complete, needs enhancement

**Files:**
- `sf_core/tests/integration/session/logout.rs` - 5 test stubs with TODO markers
- `sf_core/tests/e2e/session/logout.rs` - 38 test stubs with TODO markers

**What needs improvement:**
1. **Add detailed test setup code:**
   - Mock HTTP server setup (pattern exists in `integration/http/retry.rs`)
   - Request capture and validation helpers
   - Helper functions at bottom of file (follow DRY)

2. **Add assertion patterns:**
   - Comment out assertions with TODO markers
   - Show expected assertion structure
   - Reference existing patterns from retry tests

3. **Follow existing patterns:**
   - Study `sf_core/tests/integration/http/retry.rs` for mock server patterns
   - Study `sf_core/tests/integration/session/session_refresh.rs` for session test patterns
   - Reuse `spawn_test_server`, `spawn_capture_server` helper patterns

### Python Tests  
**Status:** ⚠️ Basic skeleton created, needs significant enhancement

**Files:**
- `python/tests/e2e/session/test_logout.py` - 46 test methods, very basic stubs
- `python/tests/integ/session/test_logout.py` - 1 test method, basic stub

**What needs improvement:**
1. **Add proper test structure:**
   - Import necessary fixtures (`connection_factory`, `reference_connector`)
   - Add helper functions at bottom following DRY
   - Study existing auth tests for connection patterns

2. **Add detailed test implementations:**
   - Use `connection_factory` fixture to create connections with specific config
   - Add wiremock client for intercepting/validating HTTP requests (pattern exists)
   - Add `compatibility.py` helpers for OLD/NEW driver checks if needed

3. **Follow existing patterns:**
   - Study `python/tests/e2e/authentication/test_pat.py` for connection patterns
   - Study `python/tests/e2e/authentication/test_private_key_auth.py` for error handling patterns  
   - Check `python/tests/wiremock_client.py` for HTTP interception patterns
   - Reuse helper functions from `python/tests/e2e/authentication/auth_helpers.py`

4. **Add TODO with actual assertions:**
   - Comment out code with `# TODO: SNOW-2872349 - Uncomment when implemented`
   - Show what assertions should look like
   - Include fixture usage examples

### ODBC Tests
**Status:** ❌ Not created (ODBC not in current scope)

**Gherkins exist but no tests:**
- `tests/definitions/odbc/session/logout.feature` - 3 scenarios defined
- No test files created
- ODBC will be implemented in future sprint

**For future implementation:**
- Follow pattern from `odbc_tests/tests/e2e/authentication/private_key_auth.cpp`
- Use Catch2 framework with same structure
- Will need ~3 test functions in `odbc_tests/tests/e2e/session/logout.cpp`

### JDBC Tests
**Status:** ❌ Not created (JDBC not in current scope)

**Gherkins exist but no tests:**
- `tests/definitions/jdbc/session/logout.feature` - 7 scenarios defined
- No test files created
- JDBC will be implemented in future sprint

---

## Suggestions for Implementation Agent

### Before Starting Implementation

1. **Study existing test patterns** in the same test category:
   - Core integration: `sf_core/tests/integration/http/retry.rs`
   - Core E2E: `sf_core/tests/e2e/authentication/pat.rs`
   - Python E2E: `python/tests/e2e/authentication/test_pat.py`

2. **Identify reusable helpers:**
   - Check for existing mock server helpers
   - Check for existing connection helpers
   - Check for existing assertion helpers

3. **Set up test environment:**
   ```bash
   export PARAMETER_PATH=/path/to/parameters.json
   # Ensure parameters.json has required test credentials
   ```

### During Implementation

1. **Enhance test file structure** before implementing logic:
   - Add imports for all needed modules
   - Add helper functions at bottom of file
   - Extract repeated patterns into helpers (DRY principle)

2. **Example enhancement for Core integration test:**
   ```rust
   // ADD AT TOP:
   use sf_core::rest::snowflake::logout_session; // Once implemented
   use sf_core::config::logout::LogoutConfig;     // Once implemented
   
   // IN TEST:
   let config = LogoutConfig {
       server_session_keep_alive: None,
       enable_auto_detection: Some(false),
       error_strategy: ErrorStrategy::Strict,
       timeout: Duration::from_secs(5),
   };
   let result = logout_session(&client, session_token, config).await;
   assert!(result.is_ok());
   ```

3. **Example enhancement for Python E2E test:**
   ```python
   # ADD AT TOP:
   from ...wiremock_client import WiremockClient
   from ...config import get_test_parameters
   
   # IN TEST:
   def test_should_send_logout_with_default_settings(self, connection_factory):
       # Given Snowflake client is logged in with default parameters
       connection = connection_factory()
       
       # When Connection is closed  
       connection.close()
       
       # Then Logout request is sent successfully
       # TODO: SNOW-2872349 - Add wiremock verification
       # wiremock.verify_request("POST", "/session", params={"delete": "true"})
       
       # And Connection is closed cleanly
       # TODO: SNOW-2872349 - Verify resources cleaned
       assert connection._closed  # Or similar state check
   ```

### Test Implementation Order

**Start with simplest tests first:**

1. **Core Phase 1** (integration/session/logout.rs):
   - Implement mock server helpers first (reuse from retry.rs)
   - Start with `should_construct_logout_request` - pure HTTP validation
   - Then retry tests - reuse retry infrastructure

2. **Core Phase 3.1** (e2e/session/logout.rs):
   - Start with `should_send_logout_with_default_settings`
   - This requires implementing actual `logout_session()` function
   - Use real Snowflake connection from test parameters

3. **Python Phase 4.1-4.2** (e2e/session/test_logout.py):
   - Start with shared scenarios after Core works
   - Python just wraps Core, so should be straightforward
   - Focus on parameter passing and fixture usage

4. **Python Phase 4.3-4.7** (Python-specific):
   - Implement truth table tests
   - These test Python wrapper logic, not Core
   - May require mocking Core responses

### Common Pitfalls to Avoid

1. **Don't implement tests out of order** - follow the dependency graph
2. **Don't skip helper function extraction** - keep files DRY
3. **Don't forget resource cleanup** - use `try/finally` or `defer`
4. **Don't hardcode values** - use test parameters and config
5. **Don't forget to test OLD driver behavior** for Python (compatibility.py)

### Test Infrastructure Needed

**Core:**
- Logout HTTP function: `sf_core/src/rest/snowflake/logout.rs`
- Connection close function with logic
- Async query registry
- Error strategy enum

**Python:**
- FFI bindings to Core logout functions
- Parameter mapping (Python names → Core config)
- Deprecation warning infrastructure
- Auto-cleanup (atexit) integration

---

## Notes for Implementation Agent

1. **Function naming:** Convert Gherkin "should X" → `should_x` (snake_case for Rust, `test_should_x` for Python)

2. **Gherkin steps as comments:** Every test must have all Given/When/Then steps as comments (validator checks this)

3. **Test isolation:** Each test should:
   - Create its own connection
   - Clean up resources
   - Not depend on test execution order

4. **Long-running queries:** Tests using `SYSTEM$SLEEP(300)` must clean up:
   ```python
   try:
       # Test logic
   finally:
       # Cancel the sleep query
       cursor.execute(f"SELECT SYSTEM$CANCEL_QUERY('{query_id}')")
   ```

5. **Mock servers:** Core integration tests need mock HTTP servers:
   - Use `tokio::net::TcpListener` for mocking
   - Capture and validate requests
   - Return controlled responses

6. **Telemetry validation:** Tests checking telemetry should:
   - Enable telemetry collection
   - Verify metrics exist
   - Not prescribe specific field names

7. **Deprecation warnings:** Python Phase 2 tests must validate warnings:
   - Check warning message content
   - Verify mentions of Phase 3 migration
   - One warning per process (not per connection)

8. **Error strategies:** Core must support both:
   - Strict: Can throw from `close()`
   - Best-effort: Never throws from `close()`

9. **Thread safety:** Concurrent close tests must verify:
   - Only one logout sent
   - No panics
   - No data races

10. **Phase 3 behaviors:** Core Phase 3 scenarios are future-proofing:
    - Verify defaults can be set to Phase 3 values
    - Python excluded (uses Phase 2)
    - Will be default for ODBC

---

## Quick Start for Implementation Agent

```bash
# 1. Start with Core HTTP integration test
cd sf_core
cargo test --test integration should_construct_logout_request
# This will fail - implement sf_core/src/rest/snowflake/logout.rs

# 2. Then basic E2E
cargo test --test e2e should_send_logout_with_default_settings
# Implement connection_close() calling logout

# 3. Follow the phase order above
# Each test guides what to implement next
```

---

## JDBC and ODBC (Future Phases)

**Not in scope for current implementation:**
- JDBC gherkins defined but no tests created
- ODBC gherkins defined but no tests created
- Will be implemented in future sprints
- Follow same pattern as Python (FFI → Shared → Specific)

---

## Test File Status Summary

| File | Status | Tests | Notes |
|------|--------|-------|-------|
| `sf_core/tests/integration/session/logout.rs` | ⚠️ Needs enhancement | 5 | Has structure, needs detailed assertions |
| `sf_core/tests/e2e/session/logout.rs` | ⚠️ Needs enhancement | 38 | Has structure, needs real implementation calls |
| `python/tests/e2e/session/test_logout.py` | ⚠️ Needs major work | 46 | Very basic stubs, needs fixtures and helpers |
| `python/tests/integ/session/test_logout.py` | ⚠️ Needs work | 1 | Basic stub, needs registry mocking |

**All tests currently have:**
- ✅ Correct function names matching Gherkin
- ✅ All Gherkin steps as comments  
- ✅ Proper `#[ignore]` / `@pytest.mark.skip` markers
- ✅ TODO markers (SNOW-2872349)

**All tests need:**
- ❌ Detailed setup code (fixtures, mocks, connections)
- ❌ Assertion logic (currently commented or missing)
- ❌ Helper function extraction (DRY principles)
- ❌ Integration with actual logout implementation

---

## Related Documents

- `tests/definitions/shared/session/logout.feature` - Shared gherkin scenarios
- `tests/definitions/python/session/logout.feature` - Python-specific gherkins
- `tests/definitions/core/session/logout.feature` - Core integration gherkins  
- `tests/definitions/jdbc/session/logout.feature` - JDBC gherkins (future)
- `tests/definitions/odbc/session/logout.feature` - ODBC gherkins (future)
- `UD_LOGOUT_API_DD.md` - API design document
- `UD_Design_Doc_Fire_Forget.md` - Fire-and-forget design (Phase 2 truth tables)
- `UD_LOGOUT_TESTING_PLAN.md` - Testing requirements
- `.cursor/rules/test-generation-rules.mdc` - Test generation conventions

---

## Implementation Checklist

- [ ] Phase 1: Core HTTP Layer (4 tests)
- [ ] Phase 2.1-2.4: Core business logic
- [ ] Phase 3.1: Basic E2E (5 tests)
- [ ] Phase 3.2-3.3: Keep-alive & Auto-detection (5 tests)
- [ ] Phase 3.4-3.5: Registry & Phase 3 defaults (5 tests)
- [ ] Phase 3.6: Resource cleanup (7 tests)
- [ ] Phase 3.7-3.9: Error handling (12 tests)
- [ ] Phase 3.10: Timeout/retry (6 tests)
- [ ] Phase 3.11: Edge cases (6 tests)
- [ ] Phase 4.1: Python FFI bindings
- [ ] Phase 4.2: Python shared scenarios (32 tests)
- [ ] Phase 4.3-4.6: Python Phase 2 behavior (9 tests)
- [ ] Phase 4.7: Python auto-cleanup (4 tests)
- [ ] Phase 5: Integration optimization (1 test)

**Total:** 73 tests across 5 files
