# Logout Implementation - Fixes Required

**Date:** January 29, 2026  
**Purpose:** Document quality issues found in initial implementation and fixes needed

---

## Critical Issues Found

### 1. Python E2E Tests Don't Verify Behavior ⚠️ CRITICAL

**Problem:** All 40 "passing" Python E2E tests would also pass with old code that doesn't call logout at all. They only check `conn.is_closed()` which was always true.

**Impact:** Tests provide false confidence - they don't actually verify logout functionality works.

**Root Cause:** Tests don't verify:
- Logout HTTP request was actually sent
- Correct parameters passed to Core
- Server session actually terminated
- Deprecation warnings emitted

**Fix Required:**
1. Study old Python connector: `_old_snowflake_python_connector_for_reference/snowflake-connector-python/`
2. Use `caplog` fixture to verify log messages about logout decisions
3. Use `pytest.warns()` to verify deprecation warnings
4. For integration tests: use wiremock to verify HTTP requests (like `test_put_get_source_compression.py`)
5. For auto-detection: verify `has_running_queries()` checks server like old connector did

**Example of proper test:**
```python
def test_should_send_logout_with_verification(connection_factory, caplog):
    conn = connection_factory()
    conn.close()
    
    # Verify logout was actually sent
    assert "Session logout completed successfully" in caplog.text
    assert conn.is_closed()
```

### 2. Code Duplication - spawn_capture_server

**Problem:** `spawn_capture_server` defined in multiple test files instead of reused.

**Files:**
- `sf_core/tests/integration/session/logout.rs`
- Possibly others

**Fix Required:**
- Move to common test helpers
- Reuse across all integration tests
- Follow DRY principle

### 3. has_running_queries() Implementation Incomplete

**Problem:** Current implementation just checks local registry. Old Python connector checked actual server status.

**Fix Required:**
- Study old connector's `_all_async_queries_finished()` method
- Implement HTTP calls to check query status on server
- Don't rely only on local registry

### 4. Python Tests Structure

**Current Issues:**
- E2E tests (tests/e2e/) run against real Snowflake but don't verify behavior
- Integration tests (tests/integ/) exist but not implemented with wiremock
- No clear separation of what each level tests

**Fix Required:**
- **Integration tests:** Use wiremock to verify HTTP requests (logout sent, correct headers, retry behavior)
- **E2E tests:** Use caplog/pytest.warns to verify observable behavior (warnings, logs, timing)
- **Unit tests:** Test pure Python logic (Phase 2 defaults calculation, deprecation logic)

---

## Quality Standards Violated

### Tests Must Actually Test Something
❌ **Current:** Tests pass regardless of implementation  
✅ **Required:** Tests must fail if implementation is removed

### No Security Leaks
❌ **Fixed:** Session token prefix was being logged  
✅ **Required:** NO part of ANY token ever logged

### Code Reuse
❌ **Current:** Helper functions duplicated across files  
✅ **Required:** Common helpers in shared location

### Learn from Existing Code
❌ **Current:** Implemented without studying old connector  
✅ **Required:** Study and follow patterns from old connector

---

## Fix Plan

### Phase 1: Study Old Connector (2-3 hours)
**Files to study:**
- `_old_snowflake_python_connector_for_reference/snowflake-connector-python/src/snowflake/connector/network.py`
  - How logout request is made
  - Error handling patterns
- `_old_snowflake_python_connector_for_reference/snowflake-connector-python/src/snowflake/connector/connection.py`
  - `close()` method implementation
  - `_all_async_queries_finished()` logic
  - How it checks server for running queries
  
**Deliverable:** Document how old connector:
- Sends logout
- Checks async queries on server
- Handles errors
- Tests logout functionality

### Phase 2: Fix Core Issues (1-2 hours)
**Tasks:**
1. ✅ Remove session token logging (DONE)
2. Consolidate `spawn_capture_server` to common helpers
3. Review all logging for security issues
4. Run Core tests to ensure still passing

### Phase 3: Implement has_running_queries() Properly (2-3 hours)
**Tasks:**
1. Study old connector's query status checking
2. Implement HTTP calls to check query status on server
3. Update `AsyncQueryRegistry.has_running_queries()` to call server
4. Add integration tests with mock query status responses

### Phase 4: Rewrite Python Integration Tests with Wiremock (4-5 hours)
**Tasks:**
1. Create wiremock mapping files for:
   - `auth/login_success_jwt.json` (reuse existing)
   - `session/logout_success.json` (new)
   - `session/logout_with_keep_alive.json` (new)
   - `session/logout_503_then_success.json` (new)
2. Rewrite Python integration tests using wiremock pattern
3. Verify logout HTTP requests are actually sent
4. Verify request headers, body, query params

### Phase 5: Enhance Python E2E Tests (2-3 hours)
**Tasks:**
1. Use `caplog` to verify log messages
2. Use `pytest.warns()` to verify deprecation warnings
3. Keep E2E tests for real Snowflake but add actual verification
4. Test Phase 2 truth table properly

### Phase 6: Implement Auto-Cleanup Tests (1-2 hours)
**Tasks:**
1. Test atexit handler registration
2. Test deprecation warnings on auto-cleanup
3. Test auto_cleanup=False disables handler

---

## Estimated Total: 12-18 hours of fixes

---

## Test Quality Checklist

Before marking any test as "done":
- [ ] Would this test FAIL if I removed the implementation?
- [ ] Does it verify actual behavior, not just that methods can be called?
- [ ] For Python tests: Does it verify Core was called with correct parameters?
- [ ] For logout tests: Does it verify logout request was sent?
- [ ] For warning tests: Does it verify warnings were emitted?
- [ ] No security information in logs?
- [ ] Following patterns from old connector?

---

## Next Agent Instructions

1. **Start by studying old connector** - don't implement anything until you understand how it was done before
2. **Fix Core issues first** - consolidate helpers, security review
3. **Then work up the stack** - Core → Python integration → Python E2E
4. **Each test must verify actual behavior** - use wiremock, caplog, pytest.warns
5. **Run tests after every change** - don't mark anything done until tests pass AND verify behavior
6. **Ask for help** if sandbox prevents testing - don't assume tests work

---

## Current Implementation Status

### ✅ Working
- Core HTTP logout_session() function (security issue fixed)
- Core connection_close() logic
- Phase 3 truth table implementation
- Python FFI bindings exist
- Integration tests compile

### ⚠️ Needs Fixes
- Python E2E tests don't verify behavior
- has_running_queries() doesn't check server
- Code duplication (spawn_capture_server)
- No wiremock integration tests

### ❌ Not Started
- Proper Python integration tests with wiremock
- Auto-cleanup tests
- Deprecation warning verification
- Phase 2 truth table verification

---

## References

- Old connector: `_old_snowflake_python_connector_for_reference/`
- Wiremock example: `python/tests/integ/put_get/test_put_get_source_compression.py`
- Integration test pattern: `sf_core/tests/integration/http/retry.rs`
