# Testing Plan: Authentication – Log out

This plan covers the treatment of old driver tests during migration to the **Universal Driver (UD)** and the implementation of shared **UD Core** Gherkin scenarios.

---

## 1. Action Definitions

* **Keep** – Test intent and level stay the same (only wiring changes to UD).
* **Modify** – Same test intent, but expectations change (e.g., strategy-based behavior, UD ownership).
* **Split** – A single test’s concerns are separated.
* **Move to core tests** – Behavior should no longer be validated in wrapper tests; it is covered in UD Core tests.
* **Modify & Move to core tests** – Wrapper test is narrowed to wrapper-specific aspects; protocol / semantics move to UD Core tests.

---

## 2. Old Drivers – Test Summary and Actions

### 2.0 Cross-driver Categories
All tests are tagged using a shared set of categories for cross-driver comparison:
* **Server session keep-alive** (`server_session_keep_alive`)
* **Client auto-cleanup** (atexit / finalizer / destructor)
* **Basic & idempotent close**
* **Resource cleanup** (statements / tokens / pools)
* **Logout error handling** (SESSION_GONE / expiry / HTTP errors)
* **Connection lifecycle state machine** (Node.js specific)
* **Pooling, timeout & non-blocking close**
* **Telemetry & side effects**
* **Logout HTTP protocol** (request shape / headers / IDs)

### 2.1 Driver Mapping (Summary)

| Driver | Category | Action | Key Change / Reason |
| :--- | :--- | :--- | :--- |
| **Python** | Keep-alive | **Modify** | Protocol & GS semantics owned by UD Core. |
| **Python** | Auto-cleanup | **Keep** | Phase 1 deprecation: Python still registers `atexit`. |
| **JDBC** | Resource Cleanup | **Keep** | JDBC must invalidate statements regardless of UD internals. |
| **Go** | HTTP Protocol | **Move to Core** | Close-session HTTP details are now UD Core responsibility. |
| **.NET** | Timeouts | **Modify** | Assert logout is bounded by UD’s default ~5s policy. |
| **Node.js** | State Machine | **Keep/Modify** | Manage transitions between "connecting" and UD `close()`. |
| **libsf** | Error Handling | **Modify** | Align with UD rule: `390111` is safe to ignore. |

---

## 3. Core Tests to Implement

### 3.1 Basic Logout & Resource Cleanup
* **Basic Connection Close**: Verify UD sends `POST /session?delete=true` and the session is closed.
* **Local Resource Cleanup**: 
    * Session and master tokens must be cleared/dereferenced.
    * *Future*: Extend to heartbeat, telemetry, and HTTP pools.

### 3.2 Server Session Keep-Alive
* **Flag = True**: Closing connection does **not** send `/session?delete=true`; only client-side resources are cleaned.
* **Flag = False/Default**: Closing connection always attempts logout regardless of async jobs.

### 3.3 Logout HTTP Protocol (UD Core)
Verify the logout request contains:
* **Method**: `POST`
* **URL**: `/session?delete=true`
* **Query Params**: `requestId` (static) and `request_guid` (unique per attempt).
* **Headers**: 
    * `Authorization: Snowflake Token="{session_token}"`
    * `Accept: application/snowflake`
    * `User-Agent: {WrapperUA} UD/{core_ver} Rust/{rust_ver}`
* **Body**: Exactly `{}`.

### 3.4 Error Handling & Strategies
* **SESSION_GONE (390111)**: Treated as success; no error; no retries.
* **Token Expiry (390112)**: UD attempts renewal and retries logout.
* **Master Token Expiry (390114)**:
    * **Strict Strategy**: Surfaces reauth error to client.
    * **BestEffort Strategy**: Logs `WARN`, suppresses error, close succeeds.
* **Retries**: 
    * Honor core-default timeouts (~5s) or wrapper overrides.
    * Log `WARN` on final failure after all retries.

### 3.5 Detached Queries & ABORT interaction
* **Keep-Alive True + ABORT=FALSE**: Async query continues on GS after client close.
* **Explicit Logout**: GS cancels jobs after cleanup delay.
* **ABORT_DETACHED_QUERY=TRUE**: GS cancels jobs even if logout fails (e.g., network failure).

---

## 4. Wrapper Auto-cleanup Deprecation

* **Legacy Mode**: The first auto-cleanup run logs a per-process **deprecation warning**.
* **Disabled Mode**: No auto-cleanup hooks call UD `close()`; no warnings emitted.
* **Leak Detection**: If an un-disconnected connection is detected at process exit, UD Core sends telemetry + `WARN` log.