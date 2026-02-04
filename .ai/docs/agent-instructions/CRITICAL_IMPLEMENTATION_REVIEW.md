# Critical Review: Logout Implementation Issues

**Reviewer:** Independent analysis  
**Date:** January 30, 2026

---

## Executive Summary

**Status:** Implementation is structurally complete but has **critical correctness issues** that will cause incorrect behavior in production.

**Severity:**
- 🔴 **CRITICAL:** 2 issues (will break Fire & Forget)
- 🟡 **MEDIUM:** 3 issues (tests don't verify behavior)
- 🟢 **LOW:** 2 issues (code quality)

---

## 🔴 CRITICAL ISSUE #1: AsyncQueryRegistry Doesn't Check Server

### What's Wrong
**Current implementation** (`async_registry.rs` - file appears deleted/missing):
```rust
pub async fn has_running_queries(&self, ...) -> Result<bool> {
    let query_ids = self.queries.lock().unwrap().clone();
    Ok(!query_ids.is_empty())  // ❌ Only checks local HashSet!
}
```

**Old connector** (connection.py:2053-2083):
```python
def _all_async_queries_finished(self) -> bool:
    for sfqid in queries:
        status = self.get_query_status(sfqid)  # ✅ HTTP call to server!
        if self.is_still_running(status):
            return False
    return True
```

### Why This Breaks Everything
- Registry never removes queries → always non-empty → logout always skipped
- Queries that finished on server appear "running" locally forever
- Fire & Forget won't work: sessions kept alive forever
- **This defeats the entire purpose of auto-detection**

### What Must Be Done
**Parallel worker pattern** (like old connector's ThreadPoolExecutor):
```rust
pub async fn has_running_queries(
    &self,
    client: &reqwest::Client,
    server_url: &str,
    session_token: &str,
) -> Result<bool> {
    let query_ids = /* get from registry */;
    
    if query_ids.is_empty() {
        return Ok(false);
    }
    
    // Spawn parallel workers to check each query status
    // CRITICAL: As soon as ANY worker finds running query:
    //   1. Stop/cancel all other workers
    //   2. Return true immediately
    // Only if ALL finish without finding running → return false
    
    use tokio::select;
    use tokio::sync::oneshot;
    
    let (cancel_tx, mut cancel_rx) = oneshot::channel();
    
    let handles = query_ids.iter().enumerate().map(|(i, query_id)| {
        let client = client.clone();
        let server_url = server_url.to_string();
        let session_token = session_token.to_string();
        let query_id = query_id.clone();
        let mut cancel_rx = cancel_rx.resubscribe();
        
        tokio::spawn(async move {
            select! {
                _ = cancel_rx.recv() => Ok(false),  // Cancelled
                result = async {
                    // ✅ HTTP call to server
                    let status = get_query_status(&client, &server_url, &session_token, &query_id).await?;
                    Ok::<bool, Error>(is_still_running(status))
                } => result
            }
        })
    }).collect::<Vec<_>>();
    
    // Check as completed - first running query cancels rest
    for handle in handles {
        if handle.await?? {  // Found running query
            drop(cancel_tx);  // Cancel all remaining workers
            return Ok(true);
        }
    }
    Ok(false)  // All finished
}
```

**Architecture requirements:**
1. **Parallel execution:** Multiple HTTP calls simultaneously (not sequential)
2. **Early cancellation:** Stop checking once first running query found
3. **Cheap unregister:** O(1) deletion (HashSet is correct)

**Must implement:**
1. `get_query_status()` HTTP endpoint call
2. Worker spawning with cancellation channel
3. Test with 3+ queries, verify only first is checked

---

## 🔴 CRITICAL ISSUE #2: Query Registry Lifecycle & Data Structure

### What's Missing

**Part A: Unregister never called**
- No code that calls `registry.unregister(query_id)` when queries finish
- Registry grows indefinitely (even 1ms queries stay forever)

**Part B: Data structure requirements**
- `unregister()` must be **O(1) deletion** (immediate, cheap)
- **Use HashSet** (current) or similar O(1) lookup/delete structure
- NOT Vec/List (O(n) deletion)

### Old Connector Pattern
```python
# Dict with O(1) delete
self._async_sfqids = {}  # dict[query_id, metadata]

# When query finishes:
del self._async_sfqids[query_id]  # O(1) removal
```

### What Must Be Done
1. **Hook unregister into query completion:**
   - After `poll_query_status()` returns SUCCESS
   - After `fetch_query_results()` completes
   - On query cancellation

2. **Data structure:** Current HashSet is correct (O(1) operations)

3. **Verify:** Queries actually get removed during testing

---

## 🟡 MEDIUM ISSUE #3: Python E2E Tests Don't Verify Behavior

### Example of Weak Test
```python
def test_should_send_logout_with_default_settings(self, connection_factory):
    conn = connection_factory()
    conn.close()
    assert conn.is_closed()  # ❌ Would pass even if logout was never sent!
```

### What Old Connector Tests Do
They use integration tests with **wiremock** to verify actual HTTP requests:
- Mock the `/session?delete=true` endpoint
- Verify it was called (or NOT called for keep-alive)
- Check request headers, query params, body
- Verify retry behavior

### How to Fix
**Option A:** Add wiremock integration tests (like other features)  
**Option B:** Use `caplog` to verify Core logged "sending logout" vs "skipping logout"  
**Option C:** Both (integration with wiremock + E2E with caplog)

**Recommendation:** Option C - follows existing project patterns

---

## 🟡 MEDIUM ISSUE #4: Integration Tests Don't Exist for Python

### What's Missing
File `python/tests/integ/session/test_logout.py` has only 1 test, marked skip

### What Should Exist
Like `python/tests/integ/put_get/test_put_get_source_compression.py`:
- Wiremock mappings in `python/tests/wiremock/mappings/session/`
- Tests verify HTTP behavior: logout sent/skipped, retry, headers
- 10-15 integration tests minimum

### Why This Matters
- E2E tests against real Snowflake can't verify HTTP details (headers, query params, etc.)
- Integration tests with wiremock are the **only** way to verify protocol compliance
- Without these, we don't know if logout requests are formatted correctly

---

## 🟡 MEDIUM ISSUE #5: Decision Logic May Be Incorrect

### Current Implementation
Located in `connection.rs` function `should_send_logout()`:
```rust
match config.server_session_keep_alive {
    Some(true) => false,  // Skip
    Some(false) => true,  // Send
    None => {
        if enable_auto_detection == Some(true) {
            !has_queries  // Check registry
        } else {
            true  // Send by default
        }
    }
}
```

### Questions to Verify
1. **Phase 2 Python behavior:** Old connector checks queries even when `server_session_keep_alive=False`
   - Current UD: `Some(false)` → force logout, skip auto-detection
   - Old connector: May still check queries?
   - **Need to verify:** Does truth table match actual old connector behavior?

2. **Enable auto-detection default:**
   - Python Phase 2: Should default to `true` (backward compat)
   - Core Phase 3: Should default to `false` (explicit)
   - **Is this correctly propagated from Python → Core?**

### How to Verify
Compare with old connector's actual behavior, not just documentation

---

## 🟢 LOW ISSUE #6: Missing Test Infrastructure

### Files That Don't Exist
- `python/tests/wiremock/mappings/session/*.json` - Wiremock mappings
- Helper functions for logout testing
- Fixtures for connection with specific logout config

### What's Needed
Study existing test infrastructure:
- `python/tests/wiremock_client.py` - Already exists
- `python/tests/integ/put_get/` - Pattern to follow
- Create similar structure for session/logout

---

## 🟢 LOW ISSUE #7: Code Organization

### Questionable Decisions
1. **File deleted:** `async_registry.rs` appears to be missing (file not found error)
2. **Multiple helpers:** `spawn_capture_server` duplicated in test files
3. **Config structure:** Multiple related structs in different files

### Suggestions
- Consolidate test helpers to `sf_core/tests/common/`
- Review if async_registry should be separate module or part of connection
- Consider if LogoutConfig could be simpler

---

## What Must Be Redone (Priority Order)

### 🔴 Priority 1: Fix Core Async Query Detection (BLOCKING)
**Status:** Fundamentally broken, must be completely rewritten

**Actions:**
1. Implement `get_query_status()` HTTP endpoint call
2. Modify `has_running_queries()` to check server, not local registry
3. Add integration test with mock query status responses
4. Verify early-return optimization works

**Estimated effort:** 4-6 hours

---

### 🔴 Priority 2: Implement Query Registry Lifecycle
**Status:** Missing critical piece

**Actions:**
1. Hook `register()` into async query execution
2. Hook `unregister()` into query completion/fetching
3. Test registry actually removes completed queries

**Estimated effort:** 2-3 hours  
**Dependency:** Requires async query execution API

---

### 🟡 Priority 3: Add Python Integration Tests with Wiremock
**Status:** Completely missing

**Actions:**
1. Create wiremock mappings (5-10 json files)
2. Implement 10-15 integration tests
3. Verify logout HTTP requests match spec
4. Test all scenarios: success, retry, keep-alive, etc.

**Estimated effort:** 4-5 hours

---

### 🟡 Priority 4: Fix Python E2E Tests
**Status:** Tests pass but don't verify behavior

**Actions:**
1. Add `caplog` fixture to verify logs
2. Use `pytest.warns()` for deprecation warnings
3. Verify parameters passed correctly to Core
4. Each test must answer: "Would this FAIL if logout was broken?"

**Estimated effort:** 3-4 hours

---

### 🟡 Priority 5: Verify Decision Logic Correctness
**Status:** Implemented but not verified against old connector

**Actions:**
1. Study old connector's close() method in detail
2. Create truth table from actual old behavior (not docs)
3. Compare with current implementation
4. Fix any discrepancies

**Estimated effort:** 2-3 hours

---

## What Should Be Revisited

### 1. Error Strategy Pattern
**Current:** Implemented as trait with Strict/BestEffort implementations  
**Question:** Is this the right abstraction level?  
**Action:** Show user, ask if design matches vision

### 2. LogoutConfig Structure
**Current:** 4 fields (keep_alive, enable_detection, strategy, timeout)  
**Question:** Are these the right parameters? Correct defaults?  
**Action:** User should review and confirm

### 3. Test Coverage
**Current:** Many tests marked `#[ignore]` awaiting subsystems  
**Question:** Is this acceptable or should we mock those subsystems?  
**Action:** Discuss testing strategy with user

---

## Comparison with Old Connector

### What Old Connector Does RIGHT (that we missed)

1. **HTTP Status Check**
   ```python
   status = self.get_query_status(sfq_id)  # HTTP call!
   ```
   - Current UD: No HTTP call, just checks HashSet
   
2. **Parallel Query Status Checking**
   ```python
   with ThreadPoolExecutor(max_workers=num_workers) as tpe:
       futures = (tpe.submit(check, qid) for qid in queries)
   ```
   - Current UD: Sequential (but has early-return comment)

3. **Integration Tests with Mock Server**
   - Old connector extensively tests HTTP protocol
   - Current UD: Only 4 Core integration tests, 0 Python integration tests

4. **Cancels Heartbeat Before Logout**
   ```python
   self._cancel_heartbeat()
   self._telemetry.close()
   # Then logout...
   ```
   - Current UD: Has stubs but not implemented

### What Old Connector Does (that we copied correctly)

1. ✅ SESSION_GONE (390111) treated as success
2. ✅ Idempotency (checks if already closed)
3. ✅ Best-effort error handling (logs, doesn't throw)
4. ✅ Retry policy applied to logout request

---

## Recommendations for Next Agent

### Must Do (Before Claiming Complete)
1. **Implement server-side query status checking** - This is non-negotiable
2. **Add Python integration tests with wiremock** - Following existing patterns
3. **Fix E2E tests to verify behavior** - Use caplog/pytest.warns
4. **Run all tests with `hatch run test:all`** - Not direct pytest

### Should Review (Ask User First)
1. Strategy pattern design - is this what you wanted?
2. Truth table logic - does it match old connector behavior exactly?
3. Test coverage strategy - mock subsystems or mark ignore?
4. File organization - is async_registry in right place?

### Can Keep (Looks Good)
1. HTTP logout function structure
2. Retry integration
3. Protobuf API design
4. Phase 2/3 configuration approach (concept is right, execution needs verification)

---

## Test Quality Rubric

**For each test, ask:**
1. ❌ **Would it FAIL if implementation was removed?**
2. ❌ **Does it verify HTTP requests sent (wiremock/caplog)?**
3. ❌ **Does it check parameters passed correctly?**
4. ❌ **Does it verify warnings/logs emitted?**

**Current Python tests:** Mostly ❌ on all criteria

---

## Bottom Line

**What works:**
- HTTP logout request structure
- Basic connection close flow
- Protobuf bindings
- 15 Core E2E tests passing (basic scenarios)

**What's broken:**
- ❌ Async query detection (doesn't check server)
- ❌ Query registry lifecycle (never unregisters)
- ❌ Python tests (don't verify behavior)
- ❌ Integration tests (don't exist for Python)

**Effort to fix:** ~15-20 hours of focused work

**Can it ship?** NO - Priority 1 & 2 issues will cause production bugs
