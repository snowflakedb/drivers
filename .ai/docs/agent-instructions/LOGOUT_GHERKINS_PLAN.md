# Logout Gherkins — Current State

**Ticket:** SNOW-2872349  
**Related:** SNOW-2314152 (Phase migration), SNOW-2923705 (Fire&Forget), SNOW-2314136 (Log out)

---

## File Structure

```
tests/definitions/
├── core/session/logout.feature       # 26 scenarios (~50 test cases with Outlines)
├── shared/session/logout.feature     # 6 scenarios
├── python/session/logout.feature     # 18 scenarios
├── jdbc/session/logout.feature       # 9 scenarios
└── odbc/session/logout.feature       # 4 scenarios
```

---

## Core: `core/session/logout.feature`

Core owns HTTP protocol, retry/timeout mechanics, error strategy injection, and concurrency.

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

- `should cleanup all tokens on close regardless of whether logout was sent` (3 examples: True/False/None)
- `should be idempotent when close called multiple times`
- `should reject queries client-side after connection is closed`
- `should handle SESSION_GONE error when using invalidated session token`
- `should allow process to exit cleanly when session kept alive` (Requires: heartbeat + telemetry)
- `should handle concurrent close calls safely`

---

## Python: `python/session/logout.feature`

Phase 2 backward compatibility, truth table, atexit, retry param.

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

### Error Strategy Default (1)
- `should use best-effort error handling strategy by default`

### retry Parameter (2)
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

### Defaults (2)
- `should use JDBC default 300 second timeout`
- `should use strict error handling strategy by default`

### Phase 2 Defaults + Truth Table (7)
- `should have auto_detection enabled and server_session_keep_alive null by default` (**with deprecation**)
- `should skip logout when server_session_keep_alive is true` (no deprecation)
- `should always send logout when server_session_keep_alive is false` (no deprecation)
- `should skip logout when ... null and auto_detection true and async queries found` (deprecation)
- `should send logout when ... null and auto_detection true and no async queries found` (deprecation)
- `should send logout when ... null and auto_detection false` (no deprecation)

### Resource Management (1)
- `should invalidate all active statements on close regardless of logout result`

---

## ODBC: `odbc/session/logout.feature`

Phase 3 reference implementation (simplest).

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
3. **JDBC defaults emit deprecation** (null + true) — because old JDBC never had this param
4. **ODBC implements Phase 3 from day one** — auto-detection disabled by default
5. **Error strategy tested via Core injection** — wrappers only test they pass correct default strategy
6. **Socket timeout vs retry budget** are independent concepts, tested separately
7. **Token refresh depends on retry count** — 0 retries = no refresh, 1+ retries = refresh + retry
8. **Close doesn't cancel in-flight HTTP** — services invalidated, processing fails naturally
