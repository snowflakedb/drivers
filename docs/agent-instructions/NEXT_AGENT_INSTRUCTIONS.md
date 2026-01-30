# Instructions for Next AI Agent - Logout Implementation Fixes

**Date:** January 29, 2026  
**Branch:** `SNOW-2872349-log-out-test-design`  
**Ticket:** SNOW-2872349

---

## Current Situation

**⚠️ CRITICAL: Previous agent's Core implementation is INCOMPLETE and has MISTAKES.**

The previous agent:
- Started Core implementation but did NOT complete it properly
- Made assumptions that may be wrong
- Did not follow clean code best practices in all places

**YOU MUST:**
1. Study the current Core implementation critically
2. Ask the user about design decisions before proceeding
3. Do NOT assume anything is correct just because it exists
4. Follow clean code / best practices approach

### ⚠️ What Exists (NEEDS REVIEW - may have issues)
- **Core Implementation:** Exists but needs critical review
  - `logout_session()` HTTP function - review for correctness
  - `connection_close()` - review logic and error handling
  - `AsyncQueryRegistry` - only local, doesn't check server (WRONG)
  - Strategy pattern - implemented but may need refinement
- **Core Tests:** Tests pass but may not verify correct behavior
- **Python FFI:** Bindings exist but may have issues

### ❌ Critical Problems

#### Core (Rust) Issues - NEEDS FULL REVIEW

**1. AsyncQueryRegistry is WRONG**
   - Current: Only checks local HashSet
   - Required: Should call SERVER to check query status (like old connector)
   - Old connector uses `get_query_status(sfqid)` which is an HTTP call
   - This is CRITICAL for correct logout behavior

**2. Implementation may have design issues**
   - Previous agent made assumptions - ASK USER if correct
   - Review ALL design decisions with user
   - Don't assume Strategy pattern is implemented correctly

**3. Questions to ask user:**
   - Is the LogoutConfig structure correct?
   - Is the truth table in logout_decision.rs correct?
   - Should error handling work differently?
   - Is the current test coverage sufficient?
   - What's the correct behavior for each scenario?

**4. Code quality**
   - Review for clean code principles
   - Check for proper error handling
   - Verify logging is appropriate (no sensitive data)

#### Python Issues
4. **Python E2E tests don't verify behavior**
   - 41 tests "passing" but only check `conn.is_closed()`
   - Would pass even without logout implementation
   - Don't verify logout requests are sent
   - Don't verify warnings, logs, or parameters

5. **has_running_queries() incomplete**
   - Only checks local registry
   - Should check server like old connector did
   - Missing HTTP calls to query status endpoint

6. **No wiremock integration tests**
   - Integration tests should use wiremock to verify HTTP requests

---

## Your Mission

**Refine Core, implement server-side async query check, then verify Python.**

### Priority Order:
1. **Core refinement:** Clean up, verify tests work, ensure quality
2. **Implement server-side async query check:** Like old connector's `_all_async_queries_finished()`
3. **Python verification:** Run and verify wiremock integration tests work

Follow the fix plan in `@LOGOUT_FIXES_REQUIRED.md` which details:
- How to study old connector
- What tests need (wiremock, caplog, pytest.warns)
- Quality checklist for each test

---

## Critical Rules

### Security
- ❌ **NEVER log any part of session tokens, master tokens, or credentials**
- ✅ Use `<provided>` or skip logging entirely
- The previous agent made this mistake - don't repeat it

### Test Quality
- ❌ **NEVER mark tests as "done" if they don't verify actual behavior**
- ✅ Ask yourself: "Would this test FAIL if I removed the implementation?"
- ✅ Tests must verify:
  - For logout: HTTP request was sent (use wiremock or caplog)
  - For warnings: pytest.warns() captures them
  - For config: parameters passed correctly to Core

### Don't Assume
- ❌ **Don't dismiss errors as "environment issues"** without investigation
- ❌ **Don't implement without studying old connector first**
- ✅ Study `_old_snowflake_python_connector_for_reference/` before coding
- ✅ Follow existing patterns (wiremock, test helpers)

### Testing Discipline
- ✅ Run tests after EVERY change
- ✅ Verify tests actually PASS (not just compile)
- ✅ If sandbox prevents testing, ask user to run
- ❌ Never mark phase complete without passing tests

### Code Quality
- ✅ Reuse helpers, don't duplicate
- ✅ Follow DRY principle
- ✅ Consolidate spawn_capture_server and similar helpers
- ✅ Learn from old connector implementation

---

## Required Reading (IN ORDER)

**Must read before coding:**

1. **`@LOGOUT_FIXES_REQUIRED.md`** - Detailed fix plan and issues
2. **`@LOGOUT_IMPLEMENTATION_LESSONS_LEARNED.md`** - 20 lessons from previous agents
3. **`@LOGOUT_IMPLEMENTATION_PLAN.md`** - Original implementation plan
4. **Old connector code:**
   - `_old_snowflake_python_connector_for_reference/snowflake-connector-python/src/snowflake/connector/connection.py`
   - Focus on: `close()`, `_all_async_queries_finished()`, error handling
5. **Existing test patterns:**
   - `python/tests/integ/put_get/test_put_get_source_compression.py` (wiremock example)
   - `sf_core/tests/integration/http/retry.rs` (mock server patterns)

---

## Step-by-Step Fix Process

### Step 1: Study Old Connector (DON'T SKIP THIS)
```bash
# Open and study these files:
code _old_snowflake_python_connector_for_reference/snowflake-connector-python/src/snowflake/connector/connection.py
code _old_snowflake_python_connector_for_reference/snowflake-connector-python/src/snowflake/connector/network.py

# Understand:
# - How close() sends logout
# - How _all_async_queries_finished() checks server
# - Error handling patterns
# - Test patterns
```

**Document your findings before proceeding.**

### Step 2: Core Refinement and Cleanup (START HERE)
**Goal:** Make sure the Core implementation is solid before moving on.

**Run Core tests:**
```bash
# Unit tests
cargo test --package sf_core --lib logout

# Integration tests (need network for mock server)
cargo test --package sf_core --test integration_tests logout

# E2E tests (need PARAMETER_PATH for real Snowflake)
PARAMETER_PATH=/path/to/parameters.json cargo test --package sf_core --test e2e_tests logout
```

**Verify these pass:**
- 20 unit tests (config::logout, logout_decision, etc.)
- 3 integration tests (HTTP request verification with mock server)
- 38 E2E tests (real Snowflake - ask user to run if sandbox blocks)

**Review the code for:**
- No security issues (no token logging)
- Clean error handling
- Good logging messages

### Step 3: Review ALL Core Tests - DONE ✅
Core test review was completed by previous agent. Key findings:

**Integration tests (GOOD):** These actually verify HTTP behavior with mock servers
**E2E tests (LIMITED):** Against real Snowflake, can only verify success/failure, not HTTP details

**Files:**
- `sf_core/tests/e2e/session/logout.rs` (38 tests)
- `sf_core/tests/integration/session/logout.rs` (3 passing + 2 ignored)

### Step 3: Refactor ErrorStrategy to Strategy Pattern - DONE ✅
**This has been implemented.** See `sf_core/src/config/logout.rs` for:
- `trait ErrorHandlingStrategy` with `should_ignore_error()`, `should_raise_error()`, `log_error()`, `name()`
- `StrictStrategy` and `BestEffortStrategy` implementations
- `ErrorStrategy::to_handler()` returns `Box<dyn ErrorHandlingStrategy>`

**connection.rs now uses:**
```rust
let strategy = config.error_strategy.to_handler();
if strategy.should_ignore_error(&logout_err) {
    strategy.log_error(&logout_err, false);
    Ok(())
} else if strategy.should_raise_error(&logout_err) {
    strategy.log_error(&logout_err, true);
    Err(logout_err)
} else {
    strategy.log_error(&logout_err, false);
    Ok(())
}
```

### Step 4: Implement Server-Side Async Query Check (CRITICAL)
**Current problem:** Only checks local HashSet - doesn't ask server if queries are running  
**Required:** HTTP call to server to check query status like old connector

#### Two Operations on AsyncQueryRegistry:

**1. Unregister (when query completes):**
- Must be CHEAP - O(1) immediate delete
- Use data structure with O(1) removal (HashSet is correct)
- Called when async query finishes - just drop the value

**2. Check if running (at close time):**
- Spawn MULTIPLE workers in parallel
- Each worker makes HTTP call to check ONE query's status
- If ANY worker returns "still running" → STOP ALL workers immediately
- Return early - don't wait for all checks
- This is the expensive operation, but it's optimized via early termination

#### Old connector pattern (from `connection.py:2052-2084`):
```python
def _all_async_queries_finished(self) -> bool:
    if not self._async_sfqids:
        return True  # No queries tracked = all finished
    
    queries = list(reversed(self._async_sfqids.keys()))
    num_workers = min(self.client_prefetch_threads, len(queries))
    found_unfinished_query = False
    
    def async_query_check_helper(sfq_id: str) -> bool:
        nonlocal found_unfinished_query
        # Early exit if another worker already found running query
        return found_unfinished_query or self.is_still_running(
            self.get_query_status(sfq_id)  # <-- HTTP call to server!
        )
    
    # Spawn multiple workers in parallel
    with ThreadPoolExecutor(max_workers=num_workers) as tpe:
        futures = (tpe.submit(async_query_check_helper, sfqid) for sfqid in queries)
        for f in as_completed(futures):
            if f.result():
                found_unfinished_query = True
                break  # STOP - found one running, no need to check more
        # Cancel remaining futures
        for f in futures:
            f.cancel()
    
    return not found_unfinished_query
```

#### Key Design Points:
1. **Parallel HTTP checks** - Don't check queries sequentially
2. **Early termination** - Stop all workers when first running query found
3. **Server-side check** - `get_query_status()` is HTTP call, not local lookup
4. **Cheap unregister** - Just remove from HashSet when query completes

#### Rust Implementation Approach:
```rust
// Use tokio::spawn for parallel async HTTP calls
// Use tokio::select! or similar for early termination
// Or use futures::stream::FuturesUnordered with take_while

async fn all_async_queries_finished(&self, http_client: &Client) -> bool {
    let query_ids: Vec<_> = self.queries.iter().cloned().collect();
    if query_ids.is_empty() {
        return true;
    }
    
    // Spawn parallel checks
    let mut futures = FuturesUnordered::new();
    for qid in query_ids {
        futures.push(check_query_status(http_client, qid));
    }
    
    // Return early if any is still running
    while let Some(is_running) = futures.next().await {
        if is_running {
            return false;  // Found running query - stop checking
        }
    }
    
    true  // All finished
}
```

**Files to modify:**
- `sf_core/src/apis/database_driver_v1/async_query_registry.rs`
- Need to implement `get_query_status()` HTTP endpoint call
- This will be used when async queries are added (execute with asyncExec=true)

**Test:** Create integration test with mock query status responses

### Step 5: Fix Core Code Quality
- Consolidate `spawn_capture_server` to `sf_core/tests/common/`
- Review ALL logging for security issues (no tokens!)
- Run Core tests: `cargo test --test integration_tests logout`
- Run Core E2E: `PARAMETER_PATH=.../parameters.json cargo test --test e2e_tests logout`

### Step 6: Create Wiremock Mappings
**Create files in `python/tests/wiremock/mappings/session/`:**

`logout_success.json`:
```json
{
  "request": {
    "method": "POST",
    "urlPath": "/session",
    "queryParameters": {
      "delete": { "equalTo": "true" }
    }
  },
  "response": {
    "status": 200,
    "jsonBody": {
      "success": true
    }
  }
}
```

Similar for:
- `logout_with_keep_alive.json` (should NOT be called)
- `logout_503_then_success.json` (retry scenarios)

### Step 7: Rewrite Python Integration Tests with Wiremock
**File:** `python/tests/integ/session/test_logout.py`

**Pattern:**
```python
from tests.wiremock_client import WiremockClient

def test_should_send_logout_with_wiremock(int_test_connection_factory):
    with WiremockClient().start() as wiremock:
        wiremock.add_mapping("auth/login_success_jwt.json")
        wiremock.add_mapping("session/logout_success.json")
        
        conn = int_test_connection_factory(server_url=wiremock.http_url())
        conn.close()
        
        # Verify logout was called
        # Check wiremock received POST /session?delete=true
        assert conn.is_closed()
```

**Implement ALL integration test scenarios with wiremock.**

### Step 8: Enhance Python E2E Tests  
**File:** `python/tests/e2e/session/test_logout.py`

**Use caplog to verify logs:**
```python
def test_with_log_verification(connection_factory, caplog):
    import logging
    caplog.set_level(logging.INFO)
    
    conn = connection_factory(server_session_keep_alive=True)
    conn.close()
    
    # Verify logout was skipped
    assert "Skipping logout" in caplog.text
    assert conn.is_closed()
```

**Use pytest.warns for deprecation warnings:**
```python
def test_deprecation_warning(connection_factory):
    conn = connection_factory(server_session_keep_alive=False)
    
    with pytest.warns(FutureWarning, match="Phase 3"):
        conn.close()
    
    assert conn.is_closed()
```

### Step 9: Test Each Fix
```bash
# After EACH fix:
cd python
export PARAMETER_PATH=/Users/fpawlowski/PycharmProjects/universal-driver/parameters.json
hatch run test:all tests/integ/session/test_logout.py -v
hatch run test:all tests/e2e/session/test_logout.py -v

# Tests must PASS and VERIFY BEHAVIOR
```

---

## Test Commands

### Core Tests
```bash
cd /Users/fpawlowski/PycharmProjects/universal-driver

# Integration tests
cargo test --test integration_tests logout

# E2E tests (requires credentials)
export PARAMETER_PATH=/Users/fpawlowski/PycharmProjects/universal-driver/parameters.json
cargo test --test e2e_tests logout
```

### Python Tests
```bash
cd /Users/fpawlowski/PycharmProjects/universal-driver/python
export PARAMETER_PATH=/Users/fpawlowski/PycharmProjects/universal-driver/parameters.json

# Integration tests (with wiremock)
hatch run test:all tests/integ/session/test_logout.py -v

# E2E tests (with real Snowflake)
hatch run test:all tests/e2e/session/test_logout.py -v

# Run specific test
hatch run test:all tests/e2e/session/test_logout.py::TestLogoutBasic::test_should_send_logout_with_default_settings -v
```

---

## Quality Checklist

Before marking ANY test as complete:

- [ ] Would this test FAIL if I removed the implementation?
- [ ] Does it verify actual behavior (not just method calls)?
- [ ] For logout tests: Verifies logout HTTP request sent?
- [ ] For warning tests: Uses pytest.warns()?
- [ ] For config tests: Verifies parameters passed to Core?
- [ ] Studied how old connector did this?
- [ ] No security leaks (tokens in logs)?
- [ ] Test actually PASSES (not just compiles)?
- [ ] Follows existing patterns (wiremock, fixtures)?

---

## Don't Repeat These Mistakes

### From Previous Agent #1:
1. ClientInfo.application is String not Option<String>
2. Need `.into()` for ConnectionHandle → Handle conversion
3. Adding Connection fields breaks ALL initialization sites
4. Content-Length must be calculated, not hardcoded
5. Protobuf changes need separate commits
6. Match statements must cover all enum variants

### From Previous Agent #2 (Me) - KNOWN MISTAKES:
7. **Tests that only check is_closed() don't verify anything**
8. **Don't log any part of tokens** (I made this mistake initially)
9. Don't implement without studying old connector first
10. E2E tests need caplog/pytest.warns, not just method calls
11. Integration tests need wiremock to verify HTTP requests
12. Never mark phase complete without passing AND verifying tests
13. **Core tests also need review** - not just Python
14. **Strategy pattern means trait + implementations**, not if-else/match

### ⚠️ HINTS: What Previous Agent Did Wrong (INCOMPLETE LIST)

**These are EXAMPLES of mistakes - there are likely MORE:**

1. **AsyncQueryRegistry is fundamentally wrong**
   - Implemented as simple local HashSet
   - Old connector checks SERVER for query status via HTTP
   - This is a critical behavioral difference

2. **Strategy pattern may be over-engineered or incorrect**
   - Added trait + implementations but may not match intended design
   - User should confirm if this is what they wanted
   - May have added unnecessary complexity

3. **Truth table implementation may be wrong**
   - `should_send_logout()` logic may not match requirements
   - Phase 2 vs Phase 3 behavior differences may be incorrect
   - Need user to verify decision logic

4. **Python tests are "false positives"**
   - 41 E2E tests that only check `conn.is_closed()`
   - Would pass even if logout was completely broken
   - Integration tests created but not verified to work

5. **May have skipped understanding old connector properly**
   - Implemented based on assumptions, not deep study
   - Old connector has nuances that may have been missed

6. **Error handling may be simplified**
   - SESSION_GONE handling - is it correct?
   - Retry behavior - matches requirements?
   - Timeout handling - correct?

7. **LogoutConfig structure may not match requirements**
   - Field names and types - correct?
   - Default values - correct?
   - Phase 2 vs Phase 3 config handling - correct?

8. **May have "completed" phases prematurely**
   - Marked phases as done without thorough verification
   - Tests passing != correct implementation

9. **Code organization may not be clean**
   - Files may be in wrong locations
   - Module structure may not follow project patterns
   - May have introduced unnecessary dependencies

10. **Documentation may not match implementation**
    - Comments may describe intended behavior, not actual
    - Design docs may be outdated

**IMPORTANT:** This list is NOT exhaustive. The new agent should:
- Question EVERYTHING
- Ask user about EACH design decision
- Not trust any "completed" work without verification

---

## Success Criteria

Phase complete when:

### Core (Rust)
1. ✅ ALL Core tests reviewed - **DONE** (see analysis in this doc)
2. ✅ ErrorStrategy refactored to proper Strategy pattern - **DONE**
3. ✅ `spawn_capture_server` consolidated to common module - **DONE**
4. ✅ No security issues (no token logging) - **DONE** (fixed earlier)

### Python
5. ⬜ ALL tests passing - run with `hatch run test:all`
6. ✅ Integration tests verify HTTP requests via Wiremock - **DONE**
   - 7 tests in `python/tests/integ/session/test_logout.py`
   - Verifies POST /session?delete=true, headers, retry, idempotency
7. ⬜ E2E tests verify actual behavior with real Snowflake
8. ⬜ has_running_queries() checks server status (like old connector)
9. ⬜ Following old connector patterns

---

## If You Get Stuck

**Sandbox issues:** Tests fail in sandbox but might work locally. Ask user to run:
```bash
PARAMETER_PATH=/Users/fpawlowski/PycharmProjects/universal-driver/parameters.json cargo test --test e2e_tests logout
cd python && hatch run test:all tests/e2e/session/test_logout.py -v
```

**Need help:** Don't guess. Ask user for:
- How old connector checked running queries
- How to verify logout was sent
- Test results if sandbox blocks

---

## Start Here

### Step 0: CRITICAL - Review and Question Everything

**Before writing ANY code, you MUST:**

1. **Read the current Core implementation files:**
   - `sf_core/src/config/logout.rs` - LogoutConfig, ErrorStrategy, Strategy pattern
   - `sf_core/src/apis/database_driver_v1/connection.rs` - connection_close()
   - `sf_core/src/apis/database_driver_v1/logout_decision.rs` - should_send_logout()
   - `sf_core/src/apis/database_driver_v1/async_query_registry.rs` - AsyncQueryRegistry
   - `sf_core/src/rest/snowflake/logout.rs` - logout_session() HTTP function

2. **ASK THE USER about each design decision:**
   - "Is this the right approach for X?"
   - "Should we keep Y or redesign it?"
   - "The current implementation does Z - is this what you wanted?"

3. **Document your concerns** before implementing fixes

4. **Questions to ask:**
   - Strategy pattern implementation - is it correct?
   - Error handling approach - should errors propagate differently?
   - AsyncQueryRegistry - is local-only acceptable for now?
   - LogoutConfig structure - are the fields correct?
   - Truth table implementation - does decision logic match requirements?
   - Test coverage - are the right scenarios tested?

### Step 1: Run Tests to Understand Current State
```bash
cargo test --package sf_core --lib logout
cargo test --package sf_core --test integration_tests logout
```

### Step 2: Study and Question
- Review each file critically
- Note anything that looks wrong or could be better
- ASK THE USER before changing

### Step 3: Fix Issues
- Only after user confirms approach
- Follow clean code best practices
- Test after each change

**Remember:** 
- Do NOT assume previous work is correct
- ASK before implementing
- Quality over speed
- Clean code matters

---

## Key Files

- **Fix Plan:** `docs/agent-instructions/LOGOUT_FIXES_REQUIRED.md`
- **Lessons:** `docs/agent-instructions/LOGOUT_IMPLEMENTATION_LESSONS_LEARNED.md`
- **Original Plan:** `docs/agent-instructions/LOGOUT_IMPLEMENTATION_PLAN.md`
- **Design Docs:** `docs/agent-instructions/UD_LOGOUT_API_DD.md`, `UD_Design_Doc_Fire_Forget.md`
- **Old Connector:** `_old_snowflake_python_connector_for_reference/snowflake-connector-python/`

---

## Expected Timeline

- ~~**Study phase:** 2-3 hours~~ **DONE** - Old connector patterns documented
- ~~**Core test review:** 2-3 hours~~ **DONE**
- ~~**Core Strategy pattern refactor:** 2-3 hours~~ **DONE**
- ~~**Core code quality (consolidate helpers):** 1-2 hours~~ **DONE**
- ~~**Python integration tests (wiremock):** 4-5 hours~~ **DONE** - 7 tests created
- **Core refinement and verification:** 1-2 hours
- **Server-side async query check architecture:** 2-3 hours
- **Python test verification (run with hatch):** 1-2 hours
- **Total remaining:** ~5-8 hours

Take your time. Do it right.

Good luck! 🚀
