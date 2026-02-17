# Logout Gherkins — Current State & Fix Plan

**Ticket:** SNOW-2872349  
**Related:** SNOW-2314152 (Phase migration), SNOW-2923705 (Fire&Forget), SNOW-2314136 (Log out)  
**Review standard:** `GHERKIN_BEST_PRACTICES.md`

---

## File Structure

```
tests/definitions/
├── core/session/logout.feature       # 26 scenarios (~50 test cases with Outlines)  ⚠️ ISSUE-7 (terminology)
├── shared/session/logout.feature     #  6 scenarios                                 ⚠️ ISSUE-1, ISSUE-2, ISSUE-7
├── python/session/logout.feature     # 18 scenarios                                 ⚠️ ISSUE-3, ISSUE-4, ISSUE-7
├── jdbc/session/logout.feature       #  9 scenarios                                 ⚠️ ISSUE-5, ISSUE-6, ISSUE-7
└── odbc/session/logout.feature       #  4 scenarios                                 ⚠️ ISSUE-7
```

---

## Current Limitations

Components that do NOT exist yet — do not write scenarios that depend on these:

- **AsyncQueryRegistry server-side check** — Only local HashSet exists. HTTP-based query status check is deferred to fire-and-forget ticket (SNOW-2923705)
- **Heartbeat stop on close** — Heartbeat component not yet implemented (SNOW-2881763)
- **Telemetry flush on close** — Telemetry component not yet implemented (SNOW-2912513)
- **QCC clear on close** — Query context cache cleanup not yet implemented

What IS available and tested:
- Parameter-based keep-alive (`server_session_keep_alive`: true/false/null)
- Error strategy injection (Strict / BestEffort)
- HTTP logout request construction and retry logic
- Token cleanup (session + master tokens cleared locally)
- Close vs active query / token refresh concurrency

---

## Issues to Fix

### ISSUE-1 · shared · Vague assertion: "No race conditions occur"

**File:** `shared/session/logout.feature` line 82  
**Scenario:** `should handle concurrent close calls safely`  
**Rule violated:** §6.1 No Vague Assertions

```gherkin
# CURRENT (bad):
  Then Only one logout request is sent
  And All close calls return successfully
  And No race conditions occur          # ← unverifiable, what does this mean?
```

**Fix:** Remove the vague line. The two preceding assertions already verify the
observable behavior (one logout sent, all calls succeed). If more precision is
needed, add a concrete assertion like `And No exceptions are thrown by any thread`.

---

### ISSUE-2 · shared · SESSION_GONE scenario belongs in core, not shared

**File:** `shared/session/logout.feature` lines 48–54  
**Scenario:** `should handle SESSION_GONE error when using invalidated session token`  
**Rule violated:** §1.3 Architecture — mock server / HTTP-level tests belong in `core/`

This scenario uses `Given Mock server is configured to return SESSION_GONE 390111`,
which is HTTP-level behavior. By architecture rules, tests that use mock servers
to verify protocol-level error handling belong in `core/session/logout.feature`.

Core already tests SESSION_GONE handling at line 174:
`should ignore SESSION_GONE 390111 for each <strategy_type>`.

**Fix options:**
1. **Move** to `core/session/logout.feature` if the scenario tests a distinct
   concern (token already invalidated externally vs. session gone during logout).
   Add a comment explaining how it differs from the existing Core SESSION_GONE test.
2. **Remove** if Core's existing test already covers this case sufficiently.

---

### ISSUE-3 · python · Vague Given: "set to any value"

**File:** `python/session/logout.feature` line 121  
**Scenario:** `should skip logout when server_session_keep_alive is true regardless of auto_detection`  
**Rule violated:** §6.3 Make Configuration Explicit

```gherkin
# CURRENT (bad):
  Given Snowflake Python client is created with server_session_keep_alive set to true
  And enable_server_session_keep_alive_auto_detection set to any value   # ← vague
```

**Fix:** Convert to Scenario Outline with explicit Examples:

```gherkin
  Scenario Outline: should skip logout when server_session_keep_alive is true regardless of auto_detection
    Given Snowflake Python client is created with server_session_keep_alive set to true
    And enable_server_session_keep_alive_auto_detection is set to <auto_detection>
    When Connection is closed
    Then No logout request is sent
    And server_session_keep_alive true is passed to Core
    And No deprecation warning is emitted

    Examples:
      | auto_detection |
      | true           |
      | false          |
```

---

### ISSUE-4 · python · Retry param scenarios test implementation internals

**File:** `python/session/logout.feature` lines 149–165  
**Scenarios:**
- `should pass retry true to telemetry and logout by default`
- `should pass retry false when explicitly specified`  
**Rule violated:** §6.1 — assertions should verify observable behavior, not method calls

```gherkin
# CURRENT (bad):
  Then Telemetry close is called with retry=True         # ← tests method call
  And Logout delete_session is called with retry=True     # ← tests method call
```

**Fix:** Rewrite to test observable outcome of the `retry` parameter.
Introduce a transient failure to observe whether retry happens or not:

```gherkin
Scenario: should retry logout on transient failure when close called with default retry
  Given Snowflake Python client is logged in
  And Server will return 503 on first logout attempt then succeed
  When close() is called with default parameters
  Then Logout succeeds after retry
  And Two logout requests were sent to server

Scenario: should not retry logout on transient failure when close called with retry false
  Given Snowflake Python client is logged in
  And Server will return 503 on first logout attempt then succeed
  When close(retry=False) is called
  Then Logout is not retried
  And Only one logout request was sent to server
  And Error is handled according to best-effort strategy
```

**Why this is better:** Tests the observable difference between `retry=True` and
`retry=False` (number of HTTP requests sent) rather than intercepting internal
method calls. Uses mock server request counting as the assertion mechanism.

---

### ISSUE-5 · jdbc · Redundant assertions in error strategy default

**File:** `jdbc/session/logout.feature` lines 19–22  
**Scenario:** `should use strict error handling strategy by default`

```gherkin
# CURRENT (redundant):
  Then SQLException is thrown                          # ← says "exception thrown"
  And Error is propagated to caller                    # ← restates the same thing
  And close() method throws exception                  # ← restates again
  And Error handling strategy is strict by default
```

**Fix:** Keep one concrete assertion plus the strategy declaration:

```gherkin
  Then close() throws SQLException
  And Error handling strategy is strict by default
```

---

### ISSUE-6 · jdbc · Missing deprecation assertion for `true`

**File:** `jdbc/session/logout.feature` line 44–50  
**Scenario:** `should skip logout when server_session_keep_alive is true`  
**Rule violated:** §7.4 — Every truth table row must assert deprecation or no-deprecation

```gherkin
# CURRENT (incomplete):
  Given Snowflake JDBC connection is created with server_session_keep_alive set to true
  When Connection is closed
  Then No logout request is sent
  And server_session_keep_alive true is passed to Core
  # ← Missing: And No deprecation warning is emitted
```

Per the JDBC Phase 2 truth table (design doc line 399): `true + any → No deprecation`.
All other JDBC truth table rows explicitly assert deprecation/no-deprecation. This one doesn't.

**Fix:** Add `And No deprecation warning is emitted` to the Then block.

---

### ISSUE-7 · all files · Inconsistent terminology across scenarios

**Files:** All `.feature` files  
**Rule violated:** Consistency — same concept should use same words everywhere

Inconsistencies found:

| Inconsistency | Files | Fix |
|---|---|---|
| "UD Core client is logged in" vs "UD Core connection is logged in" | core/ | Pick one: `UD Core connection is logged in` |
| "Mock server" vs "Mock HTTP server" | core/, shared/ | Standardize: `Mock HTTP server` |
| "Given Python connection is established" vs "Given Snowflake Python client is logged in" | python/ | Standardize: `Given Snowflake Python client is logged in` |
| "When Connection is closed" vs "When Connection close is initiated" | core/, shared/ | Use `is closed` for simple close, `close is initiated` for concurrent/async scenarios |

**Fix:** Do a pass through all files and standardize. Not critical for correctness
but improves readability and makes step definitions reusable across scenarios.

---

## Core: `core/session/logout.feature`

Core owns HTTP protocol, retry/timeout mechanics, error strategy injection, and concurrency.
**Status: ⚠️ ISSUE-7** (terminology only) — all structural issues have been fixed.
The file contains inline TODO comments for implementation guidance; these are not gherkin
structural issues and will be resolved during test implementation.

### HTTP Request Construction (2)
- `should construct logout request with correct HTTP method URL headers and body`
- `should not send logout when connection was never established`

### Parameter-Based Logout Control (2)
- `should not send logout when server_session_keep_alive is explicitly true`
- `should send logout when server_session_keep_alive is explicitly false`

### Default Configuration and Timeout Concepts (3)
- `should use default 5 second timeout for logout requests`
- `should cancel individual request when per-request socket timeout exceeded`
- `should respect total retry budget timeout across all attempts`

### Close vs Active Query Execution (2)
- `should reject new query with connection closed error when submitted after close started`
- `should fail in-flight query when server response arrives after closing process started`

### Close vs Token Refresh (2)
- `should wait for in-flight token renewal to complete then logout with refreshed token`
- `should not start token renewal when query receives 390112 after closing process started`

### Error Strategy — Backend Behaviors (7)
Same outcome for both strategies:
- `should ignore SESSION_GONE 390111 for each <strategy_type>` (2 examples)
- `should retry logout on retryable <error_type> for each <strategy_type>` (6 examples: 503, 429, reset × 2 strategies)
- `should not attempt token refresh when retry count is 0 with strict strategy`
- `should not attempt token refresh when retry count is 0 with best-effort strategy`
- `should attempt token refresh on 390112 when retries allowed for each <strategy_type>` (2 examples)
- `should include token refresh time in total logout timeout budget`

### Retry/Timeout Configuration — Success Path (2)
- `should honor provided retry config and succeed for each <strategy_type>` (6 examples)
- `should honor provided timeout config and succeed for each <strategy_type>` (6 examples: 5s, 10s, 300s × 2)

### Retry/Timeout Configuration — Failure Path (4)
- `should throw after exhausted retries with strict strategy` (2 examples)
- `should log WARN and succeed after exhausted retries with best-effort strategy` (2 examples)
- `should throw on timeout with strict strategy` (2 examples)
- `should log WARN and succeed on timeout with best-effort strategy` (2 examples)

### Non-Retryable Errors (2)
- `should throw on non-retryable <error_code> in strict strategy` (4 examples: 400, 403, 404, 390114)
- `should log and suppress non-retryable <error_code> in best-effort strategy` (4 examples)

### Telemetry (1)
- `should record connection close decision metrics before logout` (Requires: SNOW-2912513)

---

## Shared: `shared/session/logout.feature`

Minimal shared E2E scenarios tagged `@core @python`.
**Status: ⚠️ 3 issues** (ISSUE-1, ISSUE-2, ISSUE-7)

- `should cleanup all tokens on close regardless of whether logout was sent` (Scenario Outline, 3 examples: True/False/None)
- `should be idempotent when close called multiple times`
- `should reject queries client-side after connection is closed`
- `should handle SESSION_GONE error when using invalidated session token` ← **ISSUE-2**
- `should allow process to exit cleanly when session kept alive` (Requires: heartbeat + telemetry)
- `should handle concurrent close calls safely` ← **ISSUE-1**

---

## Python: `python/session/logout.feature`

Phase 2 backward compatibility, truth table, atexit, retry param.
**Status: ⚠️ 3 issues** (ISSUE-3, ISSUE-4, ISSUE-7)

### Defaults (3)
- `should use Python default 5 second timeout`
- `should have auto_detection enabled and server_session_keep_alive null by default` (no deprecation)
- `should have enable_server_session_keep_alive_auto_detection default to true`

### Phase 2 Truth Table (7)
All per design doc SNOW-2314152 Phase 2 Python table:

| server_session_keep_alive | auto_detect | queries | Logout? | Deprecation? |
|---------------------------|-------------|---------|---------|-------------|
| None | True | found | No | No |
| None | True | none | Yes | No |
| None | False | — | Yes | No |
| False | True | found | No | **Yes** |
| False | True | none | Yes | **Yes** |
| False | False | — | Yes | **Yes** |
| True | any | — | No | No |

> **Python vs JDBC difference:** Python `None` + auto_detect → no deprecation.
> JDBC `null` + auto_detect → deprecation. This is intentional: Python changes
> the default from old driver's `False` to `None`, so users who never set the
> param see no deprecation. JDBC had no prior param, so any auto-detection usage
> gets deprecation. See scenario `should skip logout when server_session_keep_alive
> is true regardless of auto_detection` — **ISSUE-3** (vague "any value").

### Error Strategy Default (1)
- `should use best-effort error handling strategy by default`

### retry Parameter (2) ← **ISSUE-4**
- `should pass retry true to telemetry and logout by default`
- `should pass retry false when explicitly specified`

### Auto-cleanup (5)
- `should unregister atexit handler when close called explicitly`
- `should call close with retry false from atexit handler`
- `should emit deprecation warning on first auto-cleanup run per process`
- `should not register atexit handler when auto-cleanup explicitly disabled`
- `should emit telemetry and WARN when connection leaked at process exit`

---

## JDBC: `jdbc/session/logout.feature`

Phase 2 backward compatibility, truth table, strict strategy default.
**Status: ⚠️ 3 issues** (ISSUE-5, ISSUE-6, ISSUE-7)

### Defaults (2) ← ISSUE-5 is here
- `should use JDBC default 300 second timeout`
- `should use strict error handling strategy by default`

### Phase 2 Defaults + Truth Table (6)

| server_session_keep_alive | auto_detect | queries | Logout? | Deprecation? |
|---------------------------|-------------|---------|---------|-------------|
| null (default) | true (default) | — | depends | **Yes** |
| true | any | — | No | No |
| false | any | — | **Yes (forced)** | No |
| null | true | found | No | **Yes** |
| null | true | none | Yes | **Yes** |
| null | false | — | Yes | No |

> **JDBC vs Python difference for `false`:** JDBC `false` means "force logout,
> skip auto-detection" (already Phase 3 behavior). Python `false` still runs
> auto-detection (legacy behavior, emits deprecation). This is intentional.

Scenarios:
- `should have auto_detection enabled and server_session_keep_alive null by default` (**with deprecation**)
- `should skip logout when server_session_keep_alive is true` (no deprecation) ← **ISSUE-6** missing assertion
- `should always send logout when server_session_keep_alive is false` (no deprecation)
- `should skip logout when ... null and auto_detection true and async queries found` (deprecation)
- `should send logout when ... null and auto_detection true and no async queries found` (deprecation)
- `should send logout when ... null and auto_detection false` (no deprecation)

### Resource Management (1)
- `should invalidate all active statements on close regardless of logout result`

---

## ODBC: `odbc/session/logout.feature`

Phase 3 reference implementation (simplest).
**Status: ⚠️ ISSUE-7** (terminology only)

- `should use ODBC default 300 second timeout`
- `should have enable_server_session_keep_alive_auto_detection default to false` (Phase 3 key default)
- `should have server_session_keep_alive default to null`
- `should use strict error handling strategy by default`

---

## Out of Scope

- **Auto-detection logic** — Moved to fire-and-forget ticket (SNOW-2923705)
- **Resource cleanup** (heartbeat/telemetry/QCC) — Delegated to respective tickets
- **Go driver** — Deferred
- **Async query registry** — Part of fire-and-forget
- **Server-side behavior** (ABORT_DETACHED_QUERY) — Not driver concern

---

## Key Design Decisions Reflected in Gherkins

1. **UD Python default is `None`** (not old driver's `False`) — avoids noisy deprecation for users who never set the param
2. **`False` + auto_detect=True → deprecation** in Python (Phase 2) — warns that False will mean "force logout" in Phase 3
3. **JDBC `false` already forces logout** (Phase 3 behavior for `false`) — no auto-detection, no deprecation
4. **JDBC defaults emit deprecation** (null + true) — because old JDBC never had this param
5. **ODBC implements Phase 3 from day one** — auto-detection disabled by default
6. **Error strategy tested via Core injection** — wrappers only test they pass correct default strategy
7. **Socket timeout vs retry budget** are independent concepts, tested separately
8. **Token refresh depends on retry count** — 0 retries = no refresh, 1+ retries = refresh + retry
9. **Close doesn't cancel in-flight HTTP** — services invalidated, processing fails naturally
