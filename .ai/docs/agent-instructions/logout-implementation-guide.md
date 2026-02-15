# Logout Implementation Guide

**Ticket:** SNOW-2872349  
**Branch:** `SNOW-2872349-log-out-test-design`

---

## 📚 FIRST: Read These for Context

**Before reading this document, familiarize yourself with:**

1. **Design documents** in this directory:
   - `UD_LOGOUT_API_DD.md` — API design
   - `UD_Design_Doc_Fire_Forget.md` — Fire & Forget semantics
   - `UD_LOGOUT_TESTING_PLAN.md` — Testing plan

2. **Old Python connector** in `_old_snowflake_python_connector_for_reference/`:
   - `snowflake-connector-python/src/snowflake/connector/connection.py` — Study `close()`, `_all_async_queries_finished()`
   - `snowflake-connector-python/src/snowflake/connector/network.py` — Study `delete_session()`
   - This is the **reference implementation** — our behavior should mostly match it

3. **Cursor rules** in `.cursor/rules/`:
   - `test-generation-rules.mdc`, `test-rules.mdc`, `rust-error-handling-rules.mdc`, `rust-formatting-rules.mdc`
   - `datatype_gherkins.mdc`, `odbc-test-generation.mdc`

---

## 1. Main Goal

**Implement and test logout functionality across Rust (Core) and Python (Wrapper) components.**

### What This Feature Does

When a Snowflake connection is closed (`conn.close()`), we need to:
1. **Send logout HTTP request** to the server (`POST /session?delete=true`) to terminate the session
2. **Support "Fire & Forget"** — if async queries are running, skip logout to keep session alive
3. **Auto-detection** — optionally check (on the server) whether async queries are running before deciding to logout
4. **Error handling** — two strategies: Strict (propagate errors) or BestEffort (log and ignore)

### Key Behaviors (from old Python connector)

- `server_session_keep_alive=True` → Never send logout (keep session alive)
- `server_session_keep_alive=False` → Always send logout
- `server_session_keep_alive=None` + auto-detection → Check if async queries running, skip logout if yes
- `SESSION_GONE` error (390111) is treated as success (session already terminated)
- Logout should retry on transient errors (503, connection reset)
- `close()` should be idempotent (multiple calls = only one logout)

### Architecture

- **Rust Core (`sf_core`)**: implements HTTP logout, decision logic, async query registry
- **Python Wrapper**: FFI bindings to Core, exposes `Connection.close()` with config parameters
- **Tests**: integration tests with Wiremock (mock HTTP), E2E tests against real Snowflake

### Requirements

1. **Tests must pass** before moving to next phase
2. **Tests must verify actual behavior** (not just check `is_closed()`)
3. **Never change tests just to make them pass** — fix the underlying issue
4. **Study old connector** (`_old_snowflake_python_connector_for_reference/`) before coding
5. **Clean code** — follow best practices, avoid laziness, understand the codebase

---

## 2. Current State

### ⚠️ CRITICAL: Previous implementation is INCOMPLETE and has MISTAKES

The previous agent:
- Started Core implementation but did NOT complete it properly
- Made assumptions that turned out wrong
- Did not follow clean code best practices in all places

**You MUST:**
1. Study the current Core implementation critically
2. Ask the user about design decisions before proceeding
3. Do NOT assume anything is correct just because it exists
4. Follow clean code / best practices approach

### What Exists (NEEDS REVIEW — may have issues)

| Component | Status | Notes |
|-----------|--------|-------|
| `logout_session()` HTTP function | Exists | Review for correctness |
| `connection_close()` | Exists | Review logic and error handling |
| `AsyncQueryRegistry` | **WRONG** | Only local HashSet — doesn't check server |
| Strategy pattern (trait) | Exists | May need refinement |
| Python FFI bindings | Exists | May have issues |
| Core E2E tests (38) | Pass | But many only check `is_ok()` |
| Core integration tests (3+2 ignored) | Pass | Actually verify HTTP behavior |
| Python E2E tests (41) | "Pass" | **False positives** — only check `is_closed()` |
| Python integration tests (7) | Created | Not yet verified with `hatch` |

### What Works

- HTTP logout request structure
- Basic connection close flow
- Protobuf bindings
- SESSION_GONE (390111) treated as success
- Idempotency (checks if already closed)
- Retry policy applied to logout request
- ErrorStrategy refactored to proper Strategy pattern (trait + impls)
- `spawn_capture_server` consolidated to common test module

### What's Broken

| Issue | Severity | Details |
|-------|----------|---------|
| AsyncQueryRegistry doesn't check server | 🔴 CRITICAL | Only checks local HashSet — must do HTTP calls |
| Query registry lifecycle | 🔴 CRITICAL | `unregister()` never called — registry grows forever |
| Python E2E tests don't verify behavior | 🟡 MEDIUM | Would pass even if logout was broken |
| Decision logic may be incorrect | 🟡 MEDIUM | Truth table not verified against old connector |
| Missing Python integration test verification | 🟡 MEDIUM | Created but not confirmed working |

---

## 3. Critical Rules

### Security
- ❌ **NEVER log any part of session tokens, master tokens, or credentials**
- ✅ Use `<provided>` or skip logging entirely
- Previous agent made this mistake — don't repeat it

### Test Quality
- ❌ **NEVER mark tests as "done" if they don't verify actual behavior**
- ✅ Ask yourself: "Would this test FAIL if I removed the implementation?"
- ✅ Tests must verify:
  - For logout: HTTP request was sent (use Wiremock or caplog)
  - For warnings: `pytest.warns()` captures them
  - For config: parameters passed correctly to Core

### Don't Assume
- ❌ **Don't dismiss errors as "environment issues"** without investigation
- ❌ **Don't implement without studying old connector first**
- ✅ Study `_old_snowflake_python_connector_for_reference/` before coding
- ✅ Follow existing patterns (Wiremock, test helpers)

### Testing Discipline
- ✅ Run tests after EVERY change
- ✅ Verify tests actually PASS (not just compile)
- ✅ If sandbox prevents testing, ask user to run
- ❌ Never mark phase complete without passing tests

### Code Quality
- ✅ Reuse helpers, don't duplicate
- ✅ Follow DRY principle
- ✅ Learn from old connector implementation

---

## 4. Critical Issues in Detail

### 🔴 ISSUE 1: AsyncQueryRegistry Doesn't Check Server

**Current (WRONG):**
```rust
pub async fn has_running_queries(&self, ...) -> Result<bool> {
    let query_ids = self.queries.lock().unwrap().clone();
    Ok(!query_ids.is_empty())  // ❌ Only checks local HashSet!
}
```

**Old connector (CORRECT) — `connection.py:2052-2084`:**
```python
def _all_async_queries_finished(self) -> bool:
    if not self._async_sfqids:
        return True

    queries = list(reversed(self._async_sfqids.keys()))
    num_workers = min(self.client_prefetch_threads, len(queries))
    found_unfinished_query = False

    def async_query_check_helper(sfq_id: str) -> bool:
        nonlocal found_unfinished_query
        return found_unfinished_query or self.is_still_running(
            self.get_query_status(sfq_id)  # <-- HTTP call to server!
        )

    with ThreadPoolExecutor(max_workers=num_workers) as tpe:
        futures = (tpe.submit(async_query_check_helper, sfqid) for sfqid in queries)
        for f in as_completed(futures):
            if f.result():
                found_unfinished_query = True
                break  # STOP — found one running, no need to check more
        for f in futures:
            f.cancel()

    return not found_unfinished_query
```

**Two operations required:**

1. **Unregister (when query completes):** Must be O(1) immediate delete (HashSet is correct)
2. **Check if running (at close time):**
   - Spawn MULTIPLE workers in parallel
   - Each worker makes HTTP call to check ONE query's status
   - If ANY returns "still running" → STOP ALL immediately, return early
   - Only if ALL finish → return false

**Rust implementation approach:**
```rust
async fn all_async_queries_finished(&self, http_client: &Client) -> bool {
    let query_ids: Vec<_> = self.queries.iter().cloned().collect();
    if query_ids.is_empty() {
        return true;
    }

    let mut futures = FuturesUnordered::new();
    for qid in query_ids {
        futures.push(check_query_status(http_client, qid));
    }

    while let Some(is_running) = futures.next().await {
        if is_running {
            return false;  // Found running query — stop checking
        }
    }

    true
}
```

### 🔴 ISSUE 2: Query Registry Lifecycle

- No code calls `registry.unregister(query_id)` when queries finish
- Registry grows indefinitely

**Must hook unregister into:**
- After `poll_query_status()` returns SUCCESS
- After `fetch_query_results()` completes
- On query cancellation

### 🟡 ISSUE 3: Python E2E Tests Don't Verify Behavior

```python
# Current — would pass even if logout never sent:
def test_should_send_logout(self, connection_factory):
    conn = connection_factory()
    conn.close()
    assert conn.is_closed()  # ❌ Meaningless

# Required — verify actual behavior:
def test_should_send_logout(self, connection_factory, caplog):
    import logging
    caplog.set_level(logging.INFO)
    conn = connection_factory()
    conn.close()
    assert "Session logout completed successfully" in caplog.text  # ✅
    assert conn.is_closed()
```

### 🟡 ISSUE 4: Decision Logic Not Verified

Current `should_send_logout()` logic may not match old connector. Phase 2 vs Phase 3 behavior differences may be incorrect. Must be verified against actual old connector behavior, not just documentation.

---

## 5. Step-by-Step Fix Process

### Step 0: CRITICAL — Review and Question Everything

**Before writing ANY code, you MUST:**

1. **Read current Core implementation files:**
   - `sf_core/src/config/logout.rs` — LogoutConfig, ErrorStrategy, Strategy pattern
   - `sf_core/src/apis/database_driver_v1/connection.rs` — connection_close()
   - `sf_core/src/apis/database_driver_v1/logout_decision.rs` — should_send_logout()
   - `sf_core/src/apis/database_driver_v1/async_query_registry.rs` — AsyncQueryRegistry
   - `sf_core/src/rest/snowflake/logout.rs` — logout_session() HTTP function

2. **ASK the user about each design decision:**
   - "Is this the right approach for X?"
   - "Should we keep Y or redesign it?"
   - "The current implementation does Z — is this what you wanted?"

3. **Document your concerns** before implementing fixes

### Step 1: Study Old Connector (DON'T SKIP THIS)

Study these files and understand how they work before writing code:
- `_old_snowflake_python_connector_for_reference/snowflake-connector-python/src/snowflake/connector/connection.py`
- `_old_snowflake_python_connector_for_reference/snowflake-connector-python/src/snowflake/connector/network.py`

Focus on: `close()`, `_all_async_queries_finished()`, `delete_session()`, error handling patterns.

### Step 2: Core Refinement and Cleanup

**Run Core tests:**
```bash
# Unit tests
cargo test --package sf_core --lib logout

# Integration tests (need network for mock server)
cargo test --package sf_core --test integration_tests logout

# E2E tests (need PARAMETER_PATH for real Snowflake)
PARAMETER_PATH=/Users/fpawlowski/PycharmProjects/universal-driver/parameters.json cargo test --package sf_core --test e2e_tests logout
```

**Review the code for:**
- No security issues (no token logging)
- Clean error handling
- Good logging messages

### Step 3: Implement Server-Side Async Query Check

See Issue 1 and 2 above. This is the most critical piece.

**Files to modify:**
- `sf_core/src/apis/database_driver_v1/async_query_registry.rs`
- Need to implement `get_query_status()` HTTP endpoint call

### Step 4: Fix Python Tests

**Integration tests** (`python/tests/integ/session/test_logout.py`):
- Use Wiremock to verify HTTP requests
- Pattern from: `python/tests/integ/put_get/test_put_get_source_compression.py`

**E2E tests** (`python/tests/e2e/session/test_logout.py`):
- Use `caplog` to verify log messages
- Use `pytest.warns()` for deprecation warnings
- Each test must answer: "Would this FAIL if logout was broken?"

### Step 5: Run and Verify

```bash
# Python integration tests (with Wiremock)
cd python && hatch run test:all tests/integ/session/test_logout.py -v

# Python E2E tests (with real Snowflake)
cd python && PARAMETER_PATH=/Users/fpawlowski/PycharmProjects/universal-driver/parameters.json hatch run test:all tests/e2e/session/test_logout.py -v
```

---

## 6. Implementation Plan (Phases)

### Phase 1: Core HTTP Layer (Integration Tests) ✅ DONE

**File:** `sf_core/tests/integration/session/logout.rs`  
HTTP request construction, retry policy integration, connection reset handling.

### Phase 2: Core Connection Close Logic ✅ DONE (needs review)

- `connection_close()` function
- `LogoutConfig` with `server_session_keep_alive`, `enable_auto_detection`, `error_strategy`, `timeout`
- `AsyncQueryRegistry` (exists but wrong — see Issue 1)
- Auto-detection logic

### Phase 3: Core E2E Tests ✅ DONE (38 tests, need quality review)

**File:** `sf_core/tests/e2e/session/logout.rs`

Covers: basic logout, keep-alive, auto-detection, Phase 3 defaults, registry, resource cleanup, error strategies (strict + best-effort), timeout/retry, edge cases/concurrency.

### Phase 4: Python Wrapper

**4.1 Python FFI Bindings** ✅ DONE  
**4.2 Python Shared Scenarios (32 tests)** ⚠️ Tests exist but don't verify behavior  
**4.3 Python Phase 2 Config & Truth Table (14 tests)** ⬜ PENDING  
**4.4 Python Auto-cleanup with Deprecation (4 tests)** ⬜ PENDING

### Phase 5: Integration Optimization ⬜ PENDING

Test: `should_return_true_when_first_running_async_query_is_detected_without_checking_remaining_queries`

### Critical Path

```
Core Refinement → Server-Side Async Query Check → Python Integration Tests → Python E2E Tests
```

---

## 7. Test Commands

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

# Integration tests (with Wiremock)
hatch run test:all tests/integ/session/test_logout.py -v

# E2E tests (with real Snowflake)
hatch run test:all tests/e2e/session/test_logout.py -v

# Run specific test
hatch run test:all tests/e2e/session/test_logout.py::TestLogoutBasic::test_should_send_logout_with_default_settings -v
```

---

## 8. Test Quality Checklist

Before marking ANY test as complete:

- [ ] Would this test FAIL if I removed the implementation?
- [ ] Does it verify actual behavior (not just method calls)?
- [ ] For logout tests: verifies logout HTTP request sent?
- [ ] For warning tests: uses `pytest.warns()`?
- [ ] For config tests: verifies parameters passed to Core?
- [ ] Studied how old connector did this?
- [ ] No security leaks (tokens in logs)?
- [ ] Test actually PASSES (not just compiles)?
- [ ] Follows existing patterns (Wiremock, fixtures)?

---

## 9. Lessons Learned (Don't Repeat These Mistakes)

### Technical Facts

1. **Python tests run with `hatch run test:all`**, not direct `pytest`
2. **Homebrew Rust in PATH** can shadow rustup — check `rustc --version`
3. **`ClientInfo.application`** is `String`, not `Option<String>`
4. **`ConnectionHandle`** needs `.into()` to convert to `Handle`
5. **`parameters.json`** needs both `SNOWFLAKE_TEST_*` prefixed and unprefixed fields; private key must be array of strings; no trailing commas (Python 3.13+)
6. **`@pytest.mark.skip_reference`** for tests checking new params not in old driver — use sparingly, backward compatibility is a key goal
7. **`Content-Length`** must be calculated dynamically, never hardcoded
8. **Adding enum variants** breaks ALL match statements — search and update them all
9. **Adding fields to `Connection` struct** breaks ALL initialization sites
10. **Protobuf changes** need separate commits; run `./scripts/generate_proto.sh`
11. **Module registration**: new Rust test modules need `mod.rs`, Python needs `__init__.py`
12. **`SnowflakeTestClient`** calls `connection_release()` in its `Drop` impl

### Process Mistakes

13. **Tests that only check `is_closed()` don't verify anything** — false positives
14. **Don't log any part of tokens** — previous agent logged session token prefix
15. **Don't implement without studying old connector first**
16. **Marking phases "complete" when tests only compile but don't pass** — a phase is only done when tests PASS
17. **Removing assertions to make tests pass** — never weaken a test; fix the underlying issue
18. **Don't dismiss errors as "environment issues"** without investigation
19. **Strategy pattern means trait + implementations**, not if-else/match
20. **Don't blindly `git add -A`** — it may add unwanted test infrastructure files or markdowns
21. **Commit separation**: implementation first, test fixes separately; proto changes in own commit

### Architecture Knowledge

22. **Core `logout_session()` is a pure HTTP function** — takes individual params, not Connection object. HTTP layer and business logic are properly separated.
23. **Integration tests (with mock servers)** test error scenarios (503, timeouts). **E2E tests (real Snowflake)** verify end-to-end flow. Don't force specific errors in E2E.
24. **Sandbox restrictions** can cause test failures (MacOS proxy). Agent should attempt tests; if blocked, ask user.

---

## 10. Key Files

### Core (Rust)
```
sf_core/src/rest/snowflake/logout.rs               — HTTP logout function
sf_core/src/apis/database_driver_v1/connection.rs   — connection_close()
sf_core/src/apis/database_driver_v1/logout_decision.rs — should_send_logout()
sf_core/src/apis/database_driver_v1/async_query_registry.rs — AsyncQueryRegistry
sf_core/src/config/logout.rs                        — LogoutConfig, ErrorStrategy, Strategy trait
sf_core/tests/integration/session/logout.rs         — Integration tests (mock server)
sf_core/tests/e2e/session/logout.rs                 — E2E tests (real Snowflake)
sf_core/tests/common/test_server.rs                 — Shared test server helpers
```

### Python
```
python/src/snowflake/connector/connection.py        — Connection.close() FFI binding
python/tests/e2e/session/test_logout.py             — E2E tests
python/tests/integ/session/test_logout.py           — Integration tests (Wiremock)
python/tests/wiremock/mappings/session/             — Wiremock mapping files
```

### Test Definitions (Gherkin)
```
tests/definitions/shared/session/logout.feature     — Shared scenarios
tests/definitions/python/session/logout.feature     — Python-specific
tests/definitions/core/session/logout.feature       — Core integration
tests/definitions/jdbc/session/logout.feature       — JDBC (future)
tests/definitions/odbc/session/logout.feature       — ODBC (future)
```

### Reference
```
_old_snowflake_python_connector_for_reference/snowflake-connector-python/src/snowflake/connector/connection.py
_old_snowflake_python_connector_for_reference/snowflake-connector-python/src/snowflake/connector/network.py
```

---

## 11. Success Criteria

### Core (Rust)
- [ ] ALL Core tests pass and verify actual behavior
- [ ] `AsyncQueryRegistry` checks server via HTTP (parallel, early termination)
- [ ] `unregister()` called on query completion
- [ ] Strategy pattern is clean (trait + impls)
- [ ] No security issues (no token logging)

### Python
- [ ] ALL Python tests pass with `hatch run test:all`
- [ ] Integration tests verify HTTP requests via Wiremock
- [ ] E2E tests verify behavior with caplog / pytest.warns
- [ ] `has_running_queries()` checks server status
- [ ] Following old connector patterns

### Final Validation
```bash
# All Core tests
cargo test --test integration_tests logout && cargo test --test e2e_tests logout

# All Python tests
cd python && hatch run test:all tests/integ/session/test_logout.py tests/e2e/session/test_logout.py -v

# Test format validator
./tests/tests_format_validator/target/release/tests_format_validator
```

---

## 12. If You Get Stuck

**Sandbox issues:** Tests fail in sandbox but might work locally. Ask user to run:
```bash
PARAMETER_PATH=/Users/fpawlowski/PycharmProjects/universal-driver/parameters.json cargo test --test e2e_tests logout
cd python && hatch run test:all tests/e2e/session/test_logout.py -v
```

**Need help:** Don't guess. Ask user about:
- Design decisions
- How to verify specific behavior
- Test results if sandbox blocks

Take your time. Do it right. Quality over speed.
