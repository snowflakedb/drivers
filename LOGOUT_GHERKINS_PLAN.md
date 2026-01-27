# Logout Gherkins Plan - Phase 2

**Date:** January 27, 2026  
**Related Tickets:** SNOW-2314152 (Phase 3 Migration), SNOW-2923705 (Fire&Forget), SNOW-2314136 (Log out)  
**Design Docs:** UD_LOGOUT_API_DD.md, UD_Design_Doc_Fire_Forget.md

---

## Overview

This document outlines the planned Gherkin scenarios for Logout functionality across all Universal Driver implementations. The scenarios cover Phase 2 behavior with clear annotations for upcoming Phase 3 changes.

**Key Principles:**
- **Shared behavior** goes in `shared/session/logout.feature`
- **Wrapper-specific behavior** goes in each wrapper's directory
- **Phase 2 behaviors** marked with SNOW-2314152 comments explaining future changes
- **ODBC** implements Phase 3 from the start (reference implementation)
- **Fire-and-Forget** scenarios deferred to async API gherkins (out of scope)

---

## File Structure

```
tests/definitions/
├── shared/session/logout.feature          # 38 scenarios - Core mechanics
├── python/session/logout.feature          # 7 scenarios - Python Phase 2
├── jdbc/session/logout.feature            # 8 scenarios - JDBC Phase 2
├── go/session/logout.feature              # 5 scenarios - Go Phase 2
├── odbc/session/logout.feature            # 8 scenarios - ODBC Phase 3 (reference)
└── core/session/logout_internal.feature   # 4 scenarios - Core integration (optional)
```

---

## Shared Feature: `shared/session/logout.feature`

**Tag:** `@core @python @odbc @jdbc`

### Basic Logout Request (4 scenarios)

1. **should send logout request with correct endpoint method headers and payload**
   - Validates: POST /session?delete=true
   - Headers: Authorization, Content-Type, Accept, User-Agent
   - Payload: Empty JSON object `{}`

2. **should send logout request with default 5 second timeout**
   - Validates: Default timeout is 5s

3. **should send logout request with custom timeout when configured**
   - Validates: Custom timeout overrides default

4. **should not send logout when connection was never established**
   - Edge case: Close without successful connect

### Server Session Keep Alive - Explicit Control (3 scenarios)

5. **should not send logout when server_session_keep_alive is explicitly true**
   - Validates: Explicit true always skips logout

6. **should send logout when server_session_keep_alive is explicitly false**
   - Validates: Explicit false always sends logout

7. **should not start async queries detection when server_session_keep_alive is explicitly set**
   - Validates: Detection bypassed when true OR false (optimization)

### Auto-Detection Mechanics (3 scenarios)

8. **should skip logout when auto_detection enabled and running async query detected**
   - Validates: Detection finds queries and prevents logout

9. **should send logout when auto_detection enabled and no async queries detected**
   - Validates: Detection finds nothing and allows logout

10. **should return true when first running async query is detected without checking remaining queries**
    - Validates: Optimization - early return on first match

### Async Query Registry (2 scenarios)

11. **should register async query when asyncExec is true**
    - Validates: Registry populated correctly

12. **should unregister async query when query completes**
    - Validates: Registry cleanup on completion

### Resource Cleanup Contract (7 scenarios)

13. **should allow process to exit cleanly when connection closed regardless of parameters**
    - Validates: No hanging threads (heartbeat, telemetry, etc.)

14. **should stop heartbeat on close regardless of logout result**
    - Validates: Heartbeat stops even if logout fails

15. **should flush telemetry on close regardless of logout result**
    - Validates: Telemetry flushed even if logout fails

16. **should clear query result cache on close regardless of logout result**
    - Validates: QCC cleared even if logout fails

17. **should cleanup all tokens on close regardless of whether logout was sent**
    - Validates: Session and master tokens cleaned up

18. **should not allow token renewal after connection is closed**
    - Validates: No renewal even if query execution started

19. **should be idempotent when close called multiple times**
    - Validates: Safe to call close() repeatedly

### Error Handling - Strict Strategy (4 scenarios)

20. **should ignore SESSION_GONE error in strict strategy**
    - Validates: 390111 error code ignored

21. **should retry on transient error in strict strategy**
    - Validates: Retryable errors trigger retry logic

22. **should fail close on non-retryable error in strict strategy**
    - Validates: Non-retryable errors bubble up

23. **should handle session token expiration during logout in strict strategy**
    - Validates: Expired token handling

### Error Handling - Best-Effort Strategy (3 scenarios)

24. **should log all errors as WARN in best-effort strategy**
    - Validates: Errors logged, not thrown

25. **should never throw exception from close in best-effort strategy**
    - Validates: close() is infallible

26. **should succeed close even on logout timeout in best-effort strategy**
    - Validates: Timeout doesn't fail close()

### Timeout and Retry Behavior (6 scenarios)

27. **should timeout logout request after configured timeout**
    - Validates: Timeout mechanism works

28. **should retry logout on retryable HTTP errors**
    - Validates: HTTP retry policy applied

29. **should not retry logout on non-retryable errors**
    - Validates: No retry on 4xx errors

30. **should respect max retry attempts from HTTP policy**
    - Validates: Max attempts honored

31. **should use exponential backoff for logout retries**
    - Validates: Backoff strategy applied

32. **should not block process exit when timeout expires**
    - Validates: Process can exit after timeout

### Edge Cases and Concurrency (6 scenarios)

33. **should handle concurrent close calls safely**
    - Validates: Thread-safe close()

34. **should handle close during active query execution**
    - Validates: Cleanup during query

35. **should handle close during session token refresh**
    - Validates: Cleanup during refresh

36. **should handle network failure during logout**
    - Validates: Network errors handled

37. **should handle close with expired session token**
    - Validates: Expired token edge case

38. **should handle close when server is unreachable**
    - Validates: Server unavailable edge case

---

## Python-Specific: `python/session/logout.feature`

**Tag:** `@python`

1. **should send logout with default settings**
   - Tag: `@python_e2e`
   - Validates: Basic Python wrapper integration

2. **should fallback to auto_detection when server_session_keep_alive is null by default**
   - Tag: `@python_e2e`
   - **Comment:** Phase 2 behavior (SNOW-2314152). Will change in Phase 3 to always logout by default unless explicitly configured.

3. **should skip logout when server_session_keep_alive is null and async query detected**
   - Tag: `@python_e2e`
   - **Comment:** Phase 2 behavior (SNOW-2314152). In Phase 3, null will mean "always logout" and auto-detection will require explicit enable.

4. **should emit deprecation warning when using auto_detection fallback with null param**
   - Tag: `@python_e2e`
   - **Comment:** Phase 2 behavior (SNOW-2314152). This warning prepares users for Phase 3 breaking change.

5. **should detect async queries using _async_sfqids registry**
   - Tag: `@python_e2e`
   - Validates: Python-specific registry implementation

6. **should support server_session_keep_alive parameter**
   - Tag: `@python_e2e`
   - Validates: Parameter recognized and applied

7. **should use best-effort error handling strategy**
   - Tag: `@python_e2e`
   - Validates: Python uses best-effort (never throws from close)

---

## JDBC-Specific: `jdbc/session/logout.feature`

**Tag:** `@jdbc`

1. **should send logout with default settings**
   - Tag: `@jdbc_e2e`
   - Validates: Basic JDBC wrapper integration

2. **should depend on auto_detection by default when server_session_keep_alive is null**
   - Tag: `@jdbc_e2e`
   - **Comment:** Phase 2 behavior (SNOW-2314152). Will change in Phase 3 to always logout by default. ODBC implementation shows target behavior.

3. **should skip logout when server_session_keep_alive is null and async query detected**
   - Tag: `@jdbc_e2e`
   - **Comment:** Phase 2 behavior (SNOW-2314152). Will migrate to ODBC-style Phase 3 behavior where null means "always logout".

4. **should emit deprecation warning when using auto_detection fallback with null param**
   - Tag: `@jdbc_e2e`
   - **Comment:** Phase 2 behavior (SNOW-2314152). Warning users of upcoming breaking change to align with ODBC.

5. **should detect async queries using activeAsyncQueries registry**
   - Tag: `@jdbc_e2e`
   - Validates: JDBC-specific registry implementation

6. **should expose server_session_keep_alive parameter**
   - Tag: `@jdbc_e2e`
   - Validates: Parameter exposed in JDBC API

7. **should expose enable_server_session_keep_alive_auto_detection parameter**
   - Tag: `@jdbc_e2e`
   - Validates: Auto-detection control parameter exposed

8. **should use strict error handling strategy**
   - Tag: `@jdbc_e2e`
   - Validates: JDBC uses strict (can throw from close)

---

## Go-Specific: `go/session/logout.feature`

**Tag:** `@go`

1. **should send logout with default settings**
   - Tag: `@go_e2e`
   - Validates: Basic Go wrapper integration

2. **should depend on auto_detection by default when KeepSessionAlive is null**
   - Tag: `@go_e2e`
   - **Comment:** Phase 2 behavior (SNOW-2314152). Will migrate to ODBC Phase 3 behavior in future release.

3. **should emit deprecation warning when using auto_detection fallback**
   - Tag: `@go_e2e`
   - **Comment:** Phase 2 behavior (SNOW-2314152). Preparing for migration to unified Phase 3 model.

4. **should support KeepSessionAlive parameter**
   - Tag: `@go_e2e`
   - Validates: Go-specific parameter name

5. **should use strict error handling strategy**
   - Tag: `@go_e2e`
   - Validates: Go uses strict strategy

---

## ODBC-Specific: `odbc/session/logout.feature`

**Tag:** `@odbc`

**Note:** ODBC implements Phase 3 behavior from the start (reference implementation)

1. **should send logout with default settings**
   - Tag: `@odbc_e2e`
   - Validates: Basic ODBC wrapper integration

2. **should always send logout when server_session_keep_alive is null and auto_detection disabled by default**
   - Tag: `@odbc_e2e`
   - **Comment:** Phase 3 unified behavior (SNOW-2314152). This is the target model that Python, JDBC, and Go will migrate to.

3. **should not send logout when server_session_keep_alive is explicitly true**
   - Tag: `@odbc_e2e`
   - Validates: Explicit true respected

4. **should send logout when server_session_keep_alive is explicitly false**
   - Tag: `@odbc_e2e`
   - Validates: Explicit false respected

5. **should skip logout when server_session_keep_alive is null and auto_detection explicitly enabled with running queries**
   - Tag: `@odbc_e2e`
   - **Comment:** Phase 3 safety-net behavior (SNOW-2314152). Auto-detection requires explicit opt-in unlike Phase 2 drivers.

6. **should expose server_session_keep_alive parameter**
   - Tag: `@odbc_e2e`
   - Validates: Parameter exposed in ODBC API

7. **should expose enable_server_session_keep_alive_auto_detection parameter with default false**
   - Tag: `@odbc_e2e`
   - **Comment:** Phase 3 default (SNOW-2314152). Phase 2 drivers (Python/JDBC/Go) default this to true for backward compatibility.

8. **should support both strict and best-effort error handling strategies**
   - Tag: `@odbc_e2e`
   - Validates: ODBC can use either strategy

---

## Core Integration (Optional): `core/session/logout_internal.feature`

**Tag:** `@core`

1. **should construct logout request with correct URL parameters**
   - Tag: `@core_int`
   - Validates: HTTP request construction

2. **should apply retry policy to logout HTTP request**
   - Tag: `@core_int`
   - Validates: Retry policy integration

3. **should handle HTTP connection reset during logout**
   - Tag: `@core_int`
   - Validates: Connection reset handling

4. **should track logout metrics in telemetry**
   - Tag: `@core_int`
   - Validates: Telemetry integration

---

## Summary Statistics

| Feature File | Scenarios | Test Type | Phase |
|--------------|-----------|-----------|-------|
| shared/session/logout.feature | 38 | E2E | Both |
| python/session/logout.feature | 7 | E2E | 2 |
| jdbc/session/logout.feature | 8 | E2E | 2 |
| go/session/logout.feature | 5 | E2E | 2 |
| odbc/session/logout.feature | 8 | E2E | 3 |
| core/session/logout_internal.feature | 4 | Integration | Both |
| **Total** | **70** | | |

---

## Phase Transition Strategy

### Phase 2 (Current - Python, JDBC, Go)
- **Default behavior:** `server_session_keep_alive = null` → Auto-detection enabled
- **Fallback:** Check async registry, skip logout if queries found
- **Warning:** Emit deprecation warnings about future change
- **BCR:** Users relying on auto-detection must explicitly enable it

### Phase 3 (Target - ODBC reference)
- **Default behavior:** `server_session_keep_alive = null` → Always logout
- **Safety-net:** `enable_auto_detection = true` → Check registry (explicit opt-in)
- **No warnings:** Clean, predictable behavior
- **Migration path:** Phase 2 drivers will adopt this over time

---

## Out of Scope

The following are intentionally excluded from this plan:

1. **Fire-and-Forget scenarios** - Deferred to async API gherkins
2. **Async execution API** - Separate feature set
3. **Query result retrieval by ID** - Part of async API
4. **Heartbeat implementation details** - Covered by SNOW-2881763
5. **Telemetry batch implementation** - Covered by SNOW-2912513

---

## Next Steps

1. Review and approve this plan
2. Create feature files with full Gherkin syntax (Given/When/Then)
3. Validate scenarios match design doc requirements
4. Begin implementation in sf_core and wrappers
5. Update plan based on implementation findings
