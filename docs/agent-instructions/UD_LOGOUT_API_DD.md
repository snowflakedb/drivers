# Scope doc: [Authentication] Log out / Connection Closing

**Author(s):** Filip Pawlowski  
**Date:** Jan 9, 2026  
**Last update:** Jan 20, 2026  
**Document status:** In Review

## Reviewers

| Reviewer | Status | Notes |
|----------|--------|-------|
| Jakub Szczerbinski | Approved | I think we should focus on key wrappers and finding a common retry strategy to implement in the core, that doesn't introduce breaking changes for key wrappers. If non-key wrappers require another retry strategy to be backward compatible, we will implement it later. |
| Michal Hofman | Not started | |
| Tomasz Urbaszek | Not started | |
| Piotr Fus | Approved | |
| Patryk Czajka | Not started | |
| Sean Noonan | Not started | |
| Bartosz Oler | Not started | |

## Support

All drivers

## Definitions

| Term | Definition |
|------|------------|
| Logging out a session | Closing the Snowflake server session – typically triggered when a driver (or other client) sends `POST /session?delete=true` |
| Closing a connection | Driver cleans up client-side resources; may or may not log out the GS session as well (depending on parameters or internal logic) |



## Server (GS) Behaviour

[More details in: Session renewal process in drivers and [Design Doc] [UD] Fire&forget - SF async API and client session Logout]

### Session Lifetime

Session is logged out when one of the following occurs:

1. GS receives an explicit logout for that session (`POST /session?delete=true`), or
2. GS closes it due to idle/session policies (idle session cleaner). An idle session (no HTTP activity) with no running jobs becomes eligible for closure (e.g., ~24h after last access by default)

### Session Logout Implications

When session is logged out (either explicitly or by GS idle/session cleanup):

- A background cleaner will abort all jobs (sync and async) in that session within "minutes", independent of the `ABORT_DETACHED_QUERY` setting
- The associated session and master tokens become unusable; the client must re-authenticate to obtain a new session

### Detached State

A detached query is a query whose original client connection has disappeared while the GS session remains open (no `POST /session?delete=true`). While the session is still open and at least one job is running (including detached jobs), GS does not treat the session as idle for closure.

For detached async queries:

| Setting | Behavior |
|---------|----------|
| `ABORT_DETACHED_QUERY = FALSE` (default) | Detached queries continue running to completion as long as the session is not explicitly closed and not subsequently idle-cleaned. Because jobs are still running, the session remains non-idle and is not closed by the idle cleaner. Limit: GS timeouts (e.g. `STATEMENT_TIMEOUT_IN_SECONDS`) |
| `ABORT_DETACHED_QUERY = TRUE` | Detached queries are cancelled roughly 5 minutes after detachment if the client does not reattach. After those jobs are cancelled and no other jobs remain, the session becomes idle and can later be closed by the idle/session cleaner according to policy (e.g., ~24h after last access) |



## Client (Driver) Behaviour

### Logout Aspects

#### Keep Server Session Alive Logic

Drivers may conditionally skip the logout request based on:

- **Parameter-Based**: User sets a configuration parameter to suppress logout
- **Registry-Based**: Driver automatically detects running async queries and skips logout

| Driver | Check Logic | Parameter-Based | Registry-Based (Auto-detect Async) |
|--------|-------------|-----------------|-----------------------------------|
| Python | `_all_async_queries_finished() and not server_session_keep_alive` | ✅ `server_session_keep_alive` | ✅ `_async_sfqids` dict |
| JDBC | `isSafeToClose()` | ❌ | ✅ `activeAsyncQueries` set |
| Go | `!cfg.KeepSessionAlive` | ✅ `KeepSessionAlive` | ❌ |
| .NET | None | ❌ | ❌ |
| NodeJS | None | ❌ | ❌ |
| ODBC | None | ❌ | ❌ |
| libsnowflakeclient | None | ❌ | ❌ |
| PHP | None | ❌ | ❌ |

More in: [Design Doc] [UD] Fire&forget - SF async API and client session Logout 

#### Logout Request Details

**Endpoint:** `POST /session?delete=true`

**Note:** All current drivers use HTTP POST method (not DELETE) for session logout. The `delete=true` query parameter indicates the action.

**Request details:**

| Driver | Query Parameters | Request Body | Timeout | Retry |
|--------|-----------------|--------------|---------|-------|
| Python | `delete=true` | `{}` (empty JSON) | 5s (hardcoded per-attempt), also influenced by `socket_timeout` (60s default) | Yes, up to 3 attempts if `retry=True` |
| JDBC | `delete=true, requestId={uuid}` | None | `loginTimeout` - timeout shared with login requests (300s default), also influenced by `socketTimeout` (300s default). | Yes (timeout-based, MIN_RETRY_COUNT=1) |
| Go | `delete=true, requestId={uuid1}, request_guid={uuid2}` | None | 5s (hardcoded) | Yes (MaxRetryCount) |
| .NET | `delete=true, requestId={uuid1}, request_guid={uuid2}` | None | 120s (hardcoded DEFAULT_REST_RETRY_SECONDS_TIMEOUT), also influenced by HttpTimeout (16s per-request) | Yes (MaxHttpRetries via RetryHandler) |
| NodeJS | `delete=true, requestId={uuid}` | None | `timeout` - per-request; configured for all requests in NodeJS driver (30s is default for logout - if not configured). | No |
| ODBC | `delete=true, requestId={uuid1}, request_guid={uuid2}` | `{}` (empty JSON) | 300s (hardcoded DEFAULT_RETRY_TIMEOUT), also influenced by DEFAULT_CURL_TIMEOUT (60s per-request) | Yes (maxHttpRetries) |
| libsnowflakeclient | `delete=true` | None | SF_CON_RETRY_TIMEOUT (300s default), also influenced by network_timeout (90s) | Yes (retry_count) |
| PHP/PDO | (via libsnowflakeclient) | (via libsnowflakeclient) | via libsnowflakeclient | (via libsnowflakeclient) |




**Headers for logout requests:**

| Driver | Authorization | Content-Type | Accept | User-Agent | X-Snowflake-Service | CLIENT_APP_ID / _VERSION |
|--------|--------------|--------------|--------|------------|---------------------|-------------------------|
| Python | `Snowflake Token="{session_token}"` | `application/json` | `application/json` | `PythonConnector/{ver} ({platform}) {impl}/{py_ver}` | ✅ (if set) | ❌ |
| JDBC | `Snowflake Token="{session_token}"` | ❌ | ❌ | `JDBC/{ver} ({os} {os_ver}) JAVA/{java_ver}` | ✅ (if set) | ❌ |
| Go | `Snowflake Token="{session_token}"` | `application/json` | `application/snowflake` | `Go/{driver_ver} ({os}-{arch}) {compiler}/{go_ver}` | ❌ | ✅ (Both Included) |
| .NET | `Snowflake Token="{session_token}"` | ❌ | `application/snowflake` | `.NET/{ver} ({osInfo}) {runtime}/{net_ver}` | ✅ (if set) | ❌ |
| NodeJS | `Snowflake Token="{session_token}"` | `application/json` | `application/json` | `JavaScript/{ver} ({platform}-{arch}) NodeJS/{node_ver}` | ✅ (if set) | ❌ |
| ODBC | `Snowflake Token="{session_token}"` | `application/json` | `application/json` | `ODBC/{ver} ({OS} {OS_ver}) CPP/{cpp_std}` | ❌ | ❌ |
| libsnowflakeclient | `Snowflake Token="{session_token}"` | `application/json` | `application/snowflake` | `C API/{ver} ({platform}_{os_ver}) STDC/{c_ver}` | ✅ (if set) | ❌ |
| PHP / PDO | `Snowflake Token="{session_token}"` | `application/json` | `application/snowflake` | `C API/{ver} ({platform}_{os_ver}) STDC/{c_ver}` | ✅ (if set) | ❌ |



#### Error Handling During Logout

| Driver | Ignored Error Codes | Behavior |
|--------|---------------------|----------|
| Python | All | Errors logged but ignored |
| JDBC | SESSION_EXPIRED_GS_CODE (390112), SESSION_GONE (390111) | Other errors thrown |
| Go | None | Errors returned to caller |
| .NET | All | Errors logged, not thrown |
| NodeJS | SESSION_TOKEN_EXPIRED (390112), GONE_SESSION (390111) | Other errors returned via callback |
| ODBC | SESSION_TOKEN_EXPIRED (390112), MASTER_TOKEN_EXPIRED (390114), SESSION_GONE (390111) | Other errors thrown |
| libsnowflakeclient | All | Comment: "will be cleaned after 7 days" |



#### Is connection closing triggered automatically

| Driver | Auto-cleanup (Session closing does NOT require explicit user code) | Mechanism of auto-cleanup | When Triggered | How to explicitly close | What Happens If never executed |
|--------|---------------------------------------------------------------------|---------------------------|----------------|------------------------|-------------------------------|
| Python | ✅ Yes | `atexit.register(self._close_at_exit)` in connection constructor | Process exit (normal interpreter termination) | `connection.close()` or `with snowflake.connector.connect() as conn:` | Session still closed via atexit handler |
| .NET | ✅ Yes | `~SnowflakeDbConnection()` destructor calls `Dispose(false)` → `CloseNonBlocking()` | GC finalizer runs (non-deterministic timing) | `Close()`, `CloseAsync()`, `Dispose()`, or `using` | Session still closed via GC finalizer |
| PHP/PDO | ✅ Yes | `pdo_dbh_methods.closer = snowflake_handle_closer` → `snowflake_term()` | PHP refcount drops to 0, or script end | `$pdo = null` or `unset($pdo)` (no explicit close()) | Session still closed at script end |
| JDBC | ❌ No | N/A | N/A | `connection.close()` or try-with-resources | Session leaks; GS cleans it up after ~24h idle |
| Go | ❌ No | N/A | N/A | `db.Close()` or `defer db.Close()` | Session leaks; GS cleans it up after ~24h idle |
| NodeJS | ❌ No | N/A | N/A | `connection.destroy(callback)` | Session leaks; heartbeat may block process exit |
| ODBC | ❌ No | N/A | N/A | `SQLDisconnect()` + `SQLFreeHandle()` | Handle + memory leak, session leaks; GS idle cleanup |
| libsnowflakeclient | ❌ No | N/A | N/A | `snowflake_term()` | Memory leak, session leaks; GS idle cleanup |



### Detailed Driver Implementations

#### Python Connector

**Close Entry Points:**

| Entry Point | Class.Method | Description |
|-------------|--------------|-------------|
| Direct call | `SnowflakeConnection.close()` | Explicit close with optional retry |
| Context manager | `SnowflakeConnection.__exit__()` | Commits/rollbacks then calls close() |
| Process exit | `SnowflakeConnection._close_at_exit()` | atexit handler, calls close(retry=False) |

**Note:** `_connections_registry` is a passive WeakSet for CRL lifecycle - does NOT trigger close.

**Parameters Related to Close:**

- `server_session_keep_alive` (bool, default False): When True, skips logout
- `client_session_keep_alive` (bool): Enables heartbeat while open; does NOT affect close() behavior
- `close(retry=True)`: Whether to retry failed logout (up to 3x)

**Token Cleanup:** Deref only (`self._rest = None`); no explicit memory clearing


#### JDBC Driver

**Close Entry Points:**

| Entry Point | Class.Method | Description |
|-------------|--------------|-------------|
| Direct call | `SnowflakeConnectionV1.close()` | Checks `isSafeToClose()` before logout |
| try-with-resources | `SnowflakeConnectionV1.close()` | AutoCloseable pattern |
| Pooled logical close | `LogicalConnection.close()` | Returns to pool, NO logout |
| Pool eviction | `SnowflakePooledConnection.close()` | Destroys physical connection, triggers logout |

**Parameters Influencing Close:** None; logout decision is registry-based (async queries check)

**Token Cleanup:** Deref only (`sfSession = null`); no explicit memory clearing

**Pool Behavior:** `LogicalConnection.close()` fires event to pool manager; physical session stays alive until `SnowflakePooledConnection.close()` is called during eviction.


#### Go Driver (gosnowflake)

**Close Entry Points:**

| Entry Point | Class.Method | Description |
|-------------|--------------|-------------|
| Direct call | `snowflakeConn.Close()` | Via sql.DB or direct connection |
| Defer pattern | `snowflakeConn.Close()` | Idiomatic `defer db.Close()` |
| sql.DB pool | `snowflakeConn.Close()` | Pool manages via `SetMaxIdleConns()`, `SetConnMaxLifetime()` |

**Parameters Influencing Close:**

- `Config.KeepSessionAlive` (bool, default false): When True, skips logout

**Token Cleanup:** GC only; no explicit nullification in `Close()`


#### .NET Driver

**Close Entry Points:**

| Entry Point | Class.Method | Description |
|-------------|--------------|-------------|
| Direct call | `SnowflakeDbConnection.Close()` | Returns to pool or closes session |
| using statement | `SnowflakeDbConnection.Dispose()` | Calls `Close()` |
| Async | `SnowflakeDbConnection.CloseAsync()` | Async variant |
| Finalizer | `SnowflakeDbConnection.~SnowflakeDbConnection()` | GC calls `Dispose(false)` → non-blocking close |
| Pool expiration | `SessionPool.CleanExpiredSessions()` | Closes expired sessions |

**Parameters Influencing Close:**

- `poolingEnabled`: Controls pooling behavior
- `ExpirationTimeout`: Max session age before closure
- `MinPoolSize/MaxPoolSize`: Pool sizing

**Token Cleanup:** Explicit null (`sessionToken = null` in `SFSession.close()`)


#### Node.js Driver

**Close Entry Points:**

| Entry Point | Class.Method | Description |
|-------------|--------------|-------------|
| Direct call | `Connection.destroy(callback)` | Explicit termination |
| State handler | `StateConnected.destroy()` | Internal state machine handler |

**Parameters Influencing Close:** None; always sends logout when `destroy()` called

**Token Cleanup:** No explicit clearing; tokens in tokenInfo object

**Note:** If `clientSessionKeepAlive=true`, not calling `destroy()` leaves setInterval heartbeat running (no `.unref()`), possibly preventing graceful process exit.


#### ODBC Driver

**Close Entry Points:**

| Entry Point | Class.Method | Description |
|-------------|--------------|-------------|
| ODBC API | `Connection::disconnect()` | Called via `SQLDisconnect()` |
| Handle free | `Connection::~Connection()` | Via `SQLFreeHandle()`, C++ destructor |

**Parameters Influencing Close:** None; always sends logout

**Token Cleanup:** Destructor only; `disconnect()` does NOT clear tokens

#### libsnowflakeclient (C Library)

**Close Entry Points:**

| Entry Point | Function | Description |
|-------------|----------|-------------|
| Direct call | `snowflake_term()` | Explicit termination, must be called |

**Parameters Influencing Close:** None; always sends logout if tokens exist

**Token Cleanup:** Explicit free (`SF_FREE(sf->token)`, `SF_FREE(sf->master_token)`, etc.)

#### PHP/PDO Driver

**Close Entry Points:**

| Entry Point | Function | Description |
|-------------|----------|-------------|
| PDO destructor | `snowflake_handle_closer()` | When PDO object is garbage collected |
| Explicit null | `snowflake_handle_closer()` | `$pdo = null` triggers destructor |
| Script end | `snowflake_handle_closer()` | PHP cleanup at termination |

**Parameters Influencing Close:** None (delegates to libsnowflakeclient)

**Token Cleanup:** Via `snowflake_term()` (explicit free)


### Resource Cleanup by Driver

All cleanup occurs during close, regardless of logout decision:

| Driver | Heartbeat | Telemetry | QCC | Token Cleanup | HTTP Pool | Mutexes/Locks |
|--------|-----------|-----------|-----|---------------|-----------|---------------|
| Python | `_cancel_heartbeat()` | `_telemetry.close()` | `clear_cache()` | Deref only | Per-conn close | N/A |
| JDBC | `stopHeartbeatForThisSession()` | `closeTelemetryClient()` | `qcc.clearCache()` | Deref only | Shared | N/A |
| Go | `stopHeartBeat()` | `telemetry.sendBatch()` | N/A | GC only | `CloseIdleConnections()` | N/A |
| .NET | `stopHeartBeatForThisSession()` | N/A | N/A | Explicit null | Shared | N/A |
| Node.js | `clearInterval()` | N/A | `clearCache()` | No explicit | Shared agent | N/A |
| ODBC | `stopHeartBeatForThisSessionSync()` | via TelemetryHandler | N/A | Destructor | libcurl shared | N/A |
| libsnowflakeclient | N/A | N/A | `qcc_terminate()` | `SF_FREE()` | libcurl shared | `_mutex_term()` |
| PHP/PDO | via libsf | via libsf | via libsf | via libsf | via libsf | via libsf |

### Notes

During logout, drivers interact only with Snowflake GS. Logout does not include: OAuth token revocation, SAML/IdP logout, external browser cleanup, CSP token revocation (AWS STS, Azure AD, GCP IAM), or Federated Logout (SLO).

## Recommendations

Design & implementation recommendations for Authentication - Log out.

### Out of scope

The following improvements are intentionally excluded from the current scope and will be addressed in separate efforts. Existing behavior in drivers and GS remains unchanged for now:

- Telemetry SNOW-2912513: Universal Driver - Telemetry
- QCC cleanup
- Heartbeat cancel SNOW-2881763: [Authentication] Heartbeat (keep session alive)

These items do not block the unified logout semantics described in this document.

### Design decisions

#### 1. Errors During Logout

**Problem**

Drivers today handle errors from `POST /session?delete=true` inconsistently. Some ignore them, some bubble them (all or some of them) up to the caller. This leads to fragmented behavior and complicates a unified UD implementation.

**Approach 1 – Keep existing per‑driver behavior (status quo)**

Pros:
- No behavioral change for existing drivers.

Cons:
- UD core must forward low‑level logout errors to wrappers, and each wrapper handles them differently.
- Behavior remains deeply inconsistent across languages.
- Increases maintenance and support complexity.

**Approach 2 – Standardize on "logout is best‑effort and non‑fatal"**

Definition:

All drivers treat failures from `POST /session?delete=true` as non‑fatal:
- The application‑level `close()` operation always succeeds (unless something unrelated fails, e.g. local resource cleanup).
- If the logout HTTP call fails (network issue, token expiry, etc.), the driver does not throw to the user solely because the logout failed.

Pros:
- Consistent semantics across all drivers.
- Breaking change compared to old drivers.
- Simplifies UD and wrappers: error handling is unified and localized.
- Avoids surprising application failures on `close()`, which should be expected to be non-critical and non-blocking.

Cons:
- A failed logout may leave a server session alive way longer (until GS idle/session cleaner closes it). If there are any unfinished queries running, costs may increase for such customers.
  - Note: raising error won't mitigate this, as the session would not be deleted anyway.
  - It will however be more successful in highlighting the issue than a WARN log, due to raising a process failure / exception.
  - Note: this won't happen if the `ABORT_DETACHED_QUERY` param is set to TRUE.
- Real connectivity/auth problems during logout are not surfaced directly to applications (only via logs/telemetry).

**Approach 3 – Always raise logout errors to the customer**

Definition:

All drivers treat any failure from `POST /session?delete=true` as fatal:
- If the logout HTTP call does not succeed (non‑2xx status, network error, etc.), the driver raises an error to the caller of `close()`.

Pros:
- Surfaces connectivity/auth issues during logout immediately and explicitly to applications.
- Makes it harder to "silently" ignore systemic issues (e.g. persistent firewall errors, misconfigured proxies).
- Potentially easier to debug: failures are visible in application logs and control flow, not only in driver‑internal logs.

Cons:
- Breaks the expectation that `close()` is best‑effort and rarely fails.
- Breaking change compared to old drivers.
- May be costful especially on long‑running production services, if the rest of the process succeeded but was rolled back due to logout temporary issues.
- Risk of destabilizing existing applications and frameworks that assume `close()` cannot throw, or do not handle errors on cleanup paths.
- Still does not prevent sessions from remaining alive if the failure happened after work was already done on GS; it only forces the app to see the error.
  - Note: this won't happen if the `ABORT_DETACHED_QUERY` param is set to TRUE.

**Approach 4 – Ignore "expected" session‑state errors, raise the rest**

Definition:

Drivers selectively ignore only well‑known "session already gone/expired" errors from `POST /session?delete=true`, and raise everything else:
- If logout fails with one of the following GS codes, the driver does not raise an error:
  - SESSION_TOKEN_EXPIRED (390112)
  - MASTER_TOKEN_EXPIRED (390114)
  - SESSION_GONE (390111)
- For any other error (network error, 5xx, unexpected 4xx, etc.), the driver raises an error to the caller of `close()`.

Pros:
- Avoids noisy errors in the common "session already dead" scenarios (expired token, session already cleaned up by GS), which are usually benign at logout time.
- Still surfaces real connectivity or server problems (e.g. repeated 5xx, TLS issues) directly to applications, instead of hiding them.
- Benefits of both pure best‑effort (Approach 2) and "raise everything" (Approach 3) - we won't fail often and with no real reason.

Cons:
- `close()` can still fail and bubble errors to applications, which may be surprising and requires careful handling in higher‑level frameworks.
- Breaking change compared to old drivers.
- More complex logic in drivers: they must map GS error codes and maintain the "ignore list".
- Semantics are a bit more subtle to explain:
  - "Some logout errors are ignored, others are fatal," which may still confuse users.
- Drawbacks of both pure best‑effort (Approach 2) and "raise everything" (Approach 3):
  - we may still incur additional costs unknowingly (expired tokens do not mean no queries running in session)
    - Note: this won't happen if the `ABORT_DETACHED_QUERY` param is set to TRUE.
  - errors may still cause unexpected process failures for customers.

**Approach 5 – Strict: only SESSION_GONE ignored, all other errors treated as normal requests**

Definition:

Drivers handle `POST /session?delete=true` using the same semantics as any other Snowflake request, with one narrow exception:
- If logout fails with SESSION_GONE (390111), the driver does not raise an error (session is already gone; nothing more to do).
- For all other outcomes:
  - Errors like SESSION_TOKEN_EXPIRED (390112) and MASTER_TOKEN_EXPIRED (390114) are treated as normal auth failures:
    - The driver may renew the session token and retry the logout.
  - Network errors, 5xx, unexpected 4xx, etc. are handled via the standard driver retry policy (if any) for Snowflake requests.
  - If, after renewals/retries, the logout still fails, the driver raises an error to the caller of `close()`.

In short:
> Only SESSION_GONE is silently ignored; every other failure is handled like a normal request and may ultimately cause `close()` to fail.

Pros:
- Consistent with general request semantics: logout behaves like any other Snowflake call (renew on token expiry, retry on transient errors, then fail).
- Never silently ignores real auth/network problems: connectivity issues, persistent 5xx, or repeated token problems will be visible to the application.
- Stricter handling of expired tokens:
  - If SESSION_TOKEN_EXPIRED / MASTER_TOKEN_EXPIRED are returned but the session still exists and has running jobs, the driver will attempt to renew and perform the logout for real, rather than silently giving up.
- Clear, predictable rule:
  - SESSION_GONE → ignore (already dead).
  - Everything else → treat as a real error path.

Cons:
- `close()` can still fail and bubble errors to applications, which may be surprising and requires careful handling in frameworks and user code.
- Breaking change compared to old drivers.
- Unnecessarily (kind of) prolonged time of logout - most customers do not have long, async queries running in their sessions, that will incur humongous costs if session is not killed. For such a majority it is an inconvenient extension of the clean-up process with no real benefit. The unwanted costs could be simply mitigated by setting the `ABORT_DETACHED_QUERY` param to TRUE (supposing no conflicting config requirements).
- More complex logic around logout:
  - Drivers must map error codes (to catch the SESSION_GONE one) and wire logout into the normal retry/reauth flow.
- Shares some drawbacks of both "best‑effort" and "raise everything" approaches:
  - We may still incur additional costs unknowingly.
    - Note: this won't happen if the `ABORT_DETACHED_QUERY` param is set to TRUE.
  - Errors at logout time can still cause unexpected process failures for customers if their code assumes `close()` never throws.

**Approach 6 – Adoption‑focused (strategy-based per driver)**

Definition:

We can group existing drivers' logout behaviour into two categories: best‑effort / non‑fatal (e.g. Python, .NET, libsnowflakeclient), where logout failures are silent, and strict / may throw (e.g. JDBC, ODBC, Go, Node.js), where callers already expect that `close()` might raise and handle that accordingly. Approach 6 leverages this observation by implementing both Approach 5 and Approach 2 modes in UD Core and letting wrappers select which behaviour to use, following a Strategy‑style pattern.

In other words, this is an adoption‑focused, flexible approach: each driver keeps its current "behaviour category" for logout errors (either best‑effort / non‑fatal or strict / may throw) and chooses between them via a strategy.

UD Core exposes a LogoutErrorStrategy-like abstraction with at least two strategies:

1. **Strict strategy** (≈ Approach 5): SESSION_GONE (390111) is ignored; all other failures are handled like normal requests (renew + retry within bounded timeouts) and may cause `close()` to fail.

2. **Best‑effort strategy** (≈ Approach 2): logout is non‑fatal; SESSION_GONE (390111) is ignored; all other failures are handled like normal requests (renew + retry within bounded timeouts); after retries, any remaining error is logged at WARN and not bubbled to the caller.

Each wrapper:
- Picks a default strategy that matches its legacy behaviour to avoid breaking changes.
- May expose a configuration switch (for example `logout_error_mode = "strict" | "best_effort"`) to let users opt into the other behaviour.

UD Core:
- Handles retries/timeouts uniformly for both strategies.
- Delegates only the final error surfacing decision (log vs throw) to the selected strategy.

**Extensions:**

**Extension 1 – Retry logout issues:**
- Drivers may retry the logout request a small, bounded number of times (e.g. up to 3 attempts with short backoff, as Python already does).

**Extension 2 – Log failures at WARN level:**
- All non‑successful logout attempts (after retries) must be logged at WARN:
  - Include HTTP status / error code, requestId, and a high‑level reason.

**Recommended:**

**Adoption-focused:** Approach 6 (Strategy-based choosing between Approach 2 and Approach 5)
- Extension 1 – bounded retries
- Extension 2 – WARN‑level logging

**Alternatives:**

**Strict approach:** Approach 5 (SESSION_GONE ignored, rest raised)
- Extension 1 – bounded retries
- Extension 2 – WARN‑level logging

**Focused on Customers Adoption:** Approach 1 (rewritten the same way as old drivers)
- Extension 1 – bounded retries
- Extension 2 – WARN‑level logging

**Flexible approach:** Approach 2 (standardize logout as best‑effort and non‑fatal)
- Extension 1 – bounded retries
- Extension 2 – WARN‑level logging

#### 2. Server Session Keep Alive (Fire‑and‑Forget Semantics)

Decision discussed and made in: [Design Doc] [UD] Fire&forget - SF async API and client session Logout (below is a shortened, duplicated rationale and decision).

**TL;DR** – Parameter-based keep‑alive + optional auto‑detection are rolled out across UD wrappers in 3 phases.

**Context:**

To support Fire‑and‑Forget (F&F), drivers must be able to:
1. Submit async queries.
2. Close local connections / processes.
3. Keep the GS session (and async queries) alive until completion, when desired.

Existing drivers do this via:
- Parameter‑based flags (e.g. `server_session_keep_alive`, `KeepSessionAlive`), or
- Registry‑based auto‑detection of active async queries.

**Decision:**

For Universal Driver (UD) and its wrappers, we adopt the hybrid, phased model from the Fire‑and‑Forget design:

UD Core exposes two logical config fields; wrappers map their language‑specific names onto them:
- `server_session_keep_alive` (keep‑alive flag)
- `enable_server_session_keep_alive_auto_detection` (auto‑detect flag)

Final semantics (Phase 3 – unified model):

- **`server_session_keep_alive = true`**
  - Explicit F&F: never send `POST /session?delete=true` on close; keep GS session and jobs alive.
  
- **`server_session_keep_alive = false`**
  - Explicit kill: always send `POST /session?delete=true` on close; terminate session and all jobs.
  
- **`server_session_keep_alive = null`**
  - If `enable_server_session_keep_alive_auto_detection = false/null` → always logout on close (no registry check).
  - If `enable_server_session_keep_alive_auto_detection = true`:
    - async registry reports running async queries → skip logout (safety‑net F&F).
    - async registry reports no async queries → send logout.

Phased delivery:
1. **Phase 1 – Old drivers:** keep existing per‑driver behaviour (Python/JDBC registry, Go param‑only, etc.).
2. **Phase 2 – UD‑mirror:** UD Core exposes both fields; each wrapper uses defaults that mirror its legacy behaviour, while allowing explicit configuration.
3. **Phase 3 – Unified model:** all UD wrappers converge on the final semantics above.

**Rationale:**
- Provides a single, consistent semantic model for "keep server session" vs "logout and kill jobs" across all drivers, while preserving existing behaviour during migration.
- Separates explicit intent (keep‑alive flag) from safety‑net behaviour (optional async registry), making configuration and documentation clearer.
- The three‑phase rollout allows wrappers to stay backward compatible initially, then gradually move customers to the unified model without abrupt breaking changes.

#### 3. Logout Request Specification

What would the Logout request and its data look like.

**3.1 Endpoint & Method**

- **Method:** `POST`
- **URL:** `/session?delete=true`

**3.2 Request Parameters**

| Parameter | Value | Definition & Behavior |
|-----------|-------|----------------------|
| `delete` | `true` | Required. Indicates the session should be terminated. |
| `requestId` | `{uuid1}` | Static. The logical request identifier. This value remains constant across all retry attempts for the same logout operation to allow server-side tracing. |
| `request_guid` | `{uuid2}` | Rotated. The transmission identifier. This value must be regenerated for every retry attempt. This allows GS to distinguish specific HTTP attempts within the same logical requestId flow. |



**3.3 Headers**

| Header | Value | Notes |
|--------|-------|-------|
| `Authorization` | `Snowflake Token="{session_token}"` | Required for authentication. |
| `Content-Type` | `application/json` | Required because the request body `{}` is JSON. |
| `Accept` | `application/snowflake` | Unified. Replaces legacy `application/json`. Aligns with internal Snowflake API standards. |
| `User-Agent` | `{WrapperUA} UD/{core_ver} Rust/{rust_ver}` | See 3.4 User-Agent Composition below. |
| `X-Snowflake-Service` | `service_name` | Optional: added if passed by the user - when the driver is acting on behalf of a specific Snowflake service (e.g. Snowpipe). |
| `client_app_id` | `<ignored>` | Only present in Go driver, not needed for logout. |
| `client_app_version` | `<ignored>` | Only present in Go driver, not needed for logout. |



**3.4 User-Agent Composition**

The User-Agent string is constructed hierarchically to preserve legacy wrapper identification while adding UD:

**Format:**

```
{WrapperUA} UD/{core_ver} Rust/{rust_ver}
```

- `{WrapperUA}`: The legacy User-Agent string passed down from the language binding.
- `UD/{core_ver}`: The version of the Universal Driver Core shared library.
- `Rust/{rust_ver}`: The version of the Rust compiler/runtime used to build the Core.

**Original {WrapperUA} formats:**

| Driver | Legacy Wrapper UA Format |
|--------|--------------------------|
| Python | `PythonConnector/{ver} ({platform}) {impl}/{py_ver}` |
| JDBC | `JDBC/{ver} ({os} {os_ver}) JAVA/{java_ver}` |
| Go | `Go/{driver_ver} ({os}-{arch}) {compiler}/{go_ver}` |
| .NET | `.NET/{ver} ({osInfo}) {runtime}/{net_ver}` |
| Node.js | `JavaScript/{ver} ({platform}-{arch}) NodeJS/{node_ver}` |
| ODBC | `ODBC/{ver} ({OS} {OS_ver}) CPP/{cpp_std}` |
| PHP | `PHP/{ver} ({platform}_{os_ver}) STDC/{c_ver}` |



**3.5 Request Body**

- **Payload:** `{}` (Empty JSON Object)
- **Rationale:** We standardize on an empty JSON object (instead of null or empty string) to ensure strict compatibility with JSON parsers.

**3.6 Timeout Strategy Decision**

Logout may occur during application shutdown or GC. A long or indefinite hang here is undesired as it blocks process termination. Skipping the logout risks burning credits on the customer's side for detached queries. Existing drivers vary wildly (Python/Go: ~5s, Node/ODBC/JDBC: ~300s). Only JDBC (through `loginTimeout`) and NodeJS (through driver-wide timeout) exposed some way to (implicitly) impact logout timeouts. Possible approaches are:


**Approach 1: New user-facing configuration parameter**

Description: Introduce a specific config parameter (e.g., `logout_timeout`) for logout duration.

Pros:
- Maximum flexibility for users.

Cons:
- Increases configuration bloat.
- Most users do not need granular control here; they simply expect shutdown to not hang.

**Approach 2: Use shared "Snowflake HTTP Requests Retries Timeout" param for retries**

Description: Use a parameter like `SF_CON_RETRY_TIMEOUT` (from libsnowflakeclient) - scoped for all requests to Snowflake from the driver. Name and scope of it will be fully determined in: SNOW-2314153: [Networking] Retries + timeouts.

Pros:
- No new parameters (less config bloat).
- Consistent with other HTTP operations (debatable if it's predictable or confusing - see notes below).
- Can be changed in the future if needed (by introducing a new param and using the old one only if the new one is not set).

Cons:
- Less flexible - no way to control logout timeout separately.
- Can be perceived as Behaviour Change (see notes below).
- Potential confusion - one parameter affecting more than one area / type of requests (debatable if it's predictable or confusing - see notes below).

**Approach 3: Reuse login parameter**

Description: Reuse the `login_timeout` parameter.

Pros:
- No new parameters (less config bloat).
- Logically groups "session lifecycle" timeouts (if someone can accept long waiting for login, such user may be assumed to value reliability over performance in logout as well).
- Can be changed in the future if needed (by introducing a new param and using the old one only if the new one is not set).

Cons:
- Less flexible - no way to control logout timeout separately.
- Requires login timeout param to exist - which wasn't a case for all drivers.
- Can be perceived as Behaviour Change (see notes below).

**Approach 4: No user param. One, chosen timeout value for logout in core and wrappers (unified).**

Description: Pick a specific constant based on the existing implementations (e.g., Python/Go use ~5s, JDBC/ODBC/libsfclient: ~300s).

Pros:
- No new parameters (less config bloat).
- Can be changed in the future if needed (by introducing a dedicated param and using the default only if the param is not set).

Cons:
- Less flexible - no way to control logout timeout separately.
- Can be perceived as Behaviour Change (see notes below).

**Approach 5: No user param. Keep old drivers behavior (Core Input)**

Description: Expose a timeout argument in the Core connection-closing function; Allow wrappers to pass their own defaults to override this behaviour - but do not use any user-facing param. Effectively Approach 4 + Extension 1 (utilized by passing the exact old values of timeout for each driver).

Pros:
- 100% backward compatibility.

Cons:
- Perpetuates fragmentation (Node waiting 360s vs Python 5s) - in aspects of Network communication, that should be driver-agnostic.
- Complicates Core API.

**Extensions:**
- **Extension 1:** Expose a timeout argument in the Core connection-closing function. Allow wrappers to pass their own timeout values to override this behaviour.
- **Extension 2:** Log deprecation warnings (if old values are preserved), that user is using the default value, that may change in the future.

**Notes:**

Most Pros/Cons sections contain arguments regarding users confusion with timeout parameters. There are 2 important aspects to that:
1. Firstly, it's more of a scope for the SNOW-2314153: [Networking] Retries + timeouts.
2. Secondly, it is hard to judge, what is more confusing - having general HTTP requests timeout parameter (set by some user with SQLqueries in mind) affecting the logout process as well, or having it not affecting all HTTP requests (if logout required another param being set). It could be slightly limited by defaulting to the driver general HTTP requests timeout parameter only if no other was provided.

Behaviour Change argument is deliberately missing the "Breaking" prefix. The aspects that could be influenced are either reliability of the logout process (shorter timeout means we may more easily abandon otherwise successful logout or do less retries) or performance (time that logout consumes). Reliability is hard to measure here (as long as we stay consistent on 'unsuccessful logout may raise an exception' - see more). Therefore, users would be able to notice the change only for the drivers that had their timeout increased - by the fact that sometimes the process takes more time on the logout process.

Reliability aspects could be mitigated by simply setting the `ABORT_DETACHED_QUERY = TRUE` at the session level.

**Recommended:**

**Selected base approach:** Approach 4 (choose one default) + Extension 1 (expose param for wrappers to override) + make the decision which out of timeout params should be passed to logout, when all those are designed in SNOW-2314153: [Networking] Retries + timeouts.

A well reasoned logout timeouts decision requires full retries and timeouts policy to be designed. Therefore, for now the least invasive (and easy to extend) approach was chosen.

- **Timeout Value default:** 5 seconds.
  - **Rationale:** Aligns with Python and Go. 5s is sufficient for a healthy HTTP call but short enough not to hang a script exiting.

- **Retry Count:** Reuse HTTP-wide default.
  - **Rationale:** This simplifies maintenance while ensuring transient network issues are handled - every driver that retried logout requests took this approach.


#### 4. Auto-cleanup

**Problem**

Auto-cleanup defines what happens to connections and sessions when the application does not explicitly call a `close()`/`destroy()`-style API (e.g., process exit, GC finalizers, script end). If cleanup does not happen on the client side, GS eventually closes idle sessions via the idle/session cleaner, but until then the session (and any detached queries) may continue to consume resources unless mitigated by `ABORT_DETACHED_QUERY = TRUE`.

Today, old drivers are inconsistent: some (Python, .NET, PHP/PDO) register language‑level hooks and attempt to close sessions automatically on exit, while others (JDBC, Go, Node.js, ODBC, libsnowflakeclient) rely entirely on explicit `close()` and GS idle cleanup.

For UD we therefore need to standardize:
- **Responsibility boundary:** whether auto‑cleanup lives in UD Core, in each UD wrapper, or we remain explicit‑only and rely on GS idle cleanup.
- **Customer guarantees:** do we want to declare that connections will be autocleaned or we want to do our best, but recommend explicit closing anyway.
- **Shutdown constraints:** how to avoid hidden or long‑running network calls during teardown (process exit, GC) that could make applications hang or fail in hard‑to‑debug ways, while still preserving or intentionally improving the behaviour customers see when migrating from legacy drivers to UD wrappers.


**Approach 1 – No auto-cleanup**

UD Core and UD wrappers do nothing automatically on process exit / GC beyond what is explicitly called by the application:
- No atexit/finalizer hooks are introduced by UD Core or UD wrappers, beyond preserving existing behaviour where absolutely required.
- UD Core only reacts to explicit `close()`/`destroy()` calls from wrappers.
- If a UD wrapper does not call `close()`, the corresponding GS session is cleaned up only by GS idle/session cleanup and session policies.

Pros:
- **Simple and explicit:** All cleanup is driven by explicit `close()`/`destroy()` in user or framework code.
- **No hidden network calls at shutdown:** Process exit cannot be unexpectedly delayed by logout attempts triggered from low‑level hooks.
- **Core stays platform‑agnostic:** UD Core does not depend on runtime‑specific lifecycle mechanisms.
- **User keeps control:** Closing session is done only when user decides to do it. Therefore it can be deliberately skipped as well - which nowadays was useful e.g. as a workaround for enforcing fire&forget mode in some drivers.

Cons:
- **Easy to leak sessions:** Any caller that forgets to close connections relies on GS idle/session cleanup (~24h idle); detached queries may continue (unless mitigated by `ABORT_DETACHED_QUERY = TRUE`).
- **Regression risk during migration:** For languages where legacy drivers already perform auto‑cleanup (Python, .NET, PHP/PDO), moving to UD wrappers with "no auto‑cleanup" would be a behavioural step back.
- **Customers can face unexpected costs:** new users of Snowflake may not remember to close sessions when connecting to Snowflake. If they have `ABORT_DETACHED_QUERY` param set to FALSE, their credits may be consumed unexpectedly by detached queries. 


**Approach 2 – Auto-cleanup in UD Core (process-wide)**

UD Core becomes responsible for auto‑cleanup for all UD wrappers:
- UD Core registers a process‑level shutdown hook or equivalent entry point.
- On shutdown, UD Core inspects still‑open connections and performs a best‑effort close, including logout (`POST /session?delete=true`) according to the standardized logout semantics and timeouts.
- UD wrappers do not need their own auto‑cleanup; they delegate entirely to UD Core.

Pros:
- **Uniform semantics across all UD wrappers:** Any language built on UD Core gets the same auto‑cleanup behaviour.
- **Reduces accidental leaks** when user code fails to call `close()`.
- **Centralized logic:** Auto‑cleanup policy is implemented once (timeouts, retries, logging).

Cons:
- **Difficult to implement correctly across runtimes:** UD Core is a shared library and cannot reliably own "process exit" semantics in Python, Java, .NET, Node.js, PHP, C, etc.
- **Risk of fragile shutdowns:** Invoking network operations from shutdown hooks:
  - May run after parts of the runtime (threads, TLS, event loop, logging) are already torn down.
  - May hang or delay termination, especially with retries.
- **Hidden behaviour:** Users may see non‑deterministic latency and WARN logs at exit, without any explicit `close()` in their code.
- **Tight coupling to host lifecycle:** Forces UD Core to model process/context lifetime, complicating its API and its embedding in different hosts.

**Approach 3 – Iteratively introduce auto-cleanup in UD wrappers (language-specific)**

UD Core remains explicit only. Auto‑cleanup is implemented optionally and idiomatically by each UD wrapper that replaces a legacy driver (when it makes sense in regards to the appropriate standard and convention):
- UD Core exposes a clear, explicit close API and does not register connection-closing shutdown hooks.
- Each UD wrapper decides whether to attach this close API to runtime hooks:
  - Where legacy drivers already provide auto‑cleanup, the UD wrapper should preserve this behaviour by mapping existing hooks.
  - Where legacy drivers are explicit‑only, UD wrappers can keep that model if there is no reliable way to achieve it.

Pros:
- **Keeps behaviour unchanged** (or improved without users disruptions).
- **Keeps UD Core simple and portable:** No process‑lifecycle hooks or runtime‑specific logic in the core.
- **Controlled rollout:** Auto‑cleanup behaviour is defined and documented per wrapper, allowing independent iteration and A/B changes per language.
- **No hidden cross‑language surprises:** Changing auto‑cleanup for one language does not implicitly affect others.

Cons:
- **Not perfectly uniform:** Behaviour still differs by language:
  - Some UD wrappers auto‑close sessions on exit.
  - Others require explicit `close()`.
- **Wrapper work required:** Each UD wrapper must consciously decide which hooks to implement when migrating from the legacy driver.
- **Leaked sessions remain possible:** In drivers/wrappers without auto‑cleanup, failure to call `close()` still leaves cleanup to GS and session policies.

**Ideas for particular drivers:**

| Driver | Current Level | Target | Implementation Idea |
|--------|---------------|--------|---------------------|
| Python | ✅ Full | Maintain | Already uses `atexit.register()` |
| .NET | ✅ Full | Maintain | Already uses finalizer pattern |
| PHP/PDO | ✅ Full | Maintain | Already uses PDO handle closer |
| ODBC | ⚠️ Partial | Upgrade to Full | Add `atexit()` or `DllMain` |
| libsnowflakeclient | ⚠️ Partial | Upgrade to Full | Add `atexit()` with connection tracking |
| JDBC | ❌ None | Add | Add cleaner API or shutdown hook |
| Go | ❌ None | Add | Add context-based cleanup or `SetFinalizer` |
| NodeJS | ❌ None | Add | Add `process.on('beforeExit')` handler |


**Approach 4 – Iteratively deprecate auto-cleanup in UD wrappers (language-specific)**

Move all UD wrappers toward an explicit-only cleanup model over time. UD Core stays explicit-only (no process/GC hooks). Wrappers that currently auto-clean (Python, .NET, PHP/PDO) start by preserving behaviour for compatibility, then gradually deprecate and remove it.

**Behaviour:**

**UD Core:** expose only explicit close / destroy APIs; no atexit/finalizers/shutdown hooks.

**Wrappers with legacy auto-cleanup (Python, .NET, PHP/PDO):**
- **Phase 1** – keep existing hooks but gate them behind a config flag (default on), log/metric whenever auto-cleanup runs;
- **Phase 2** – flip default so auto-cleanup is off unless explicitly enabled;
- **Phase 3** – remove auto-cleanup and its config once usage is low, rely on explicit close + GS idle/session cleanup + policies.

**Wrappers without legacy auto-cleanup (JDBC, Go, Node.js, ODBC, libsnowflakeclient):**
- Remain explicit-only from day one; no new auto-cleanup added.
- WARN logs on deprecation: If relying on the old auto-clean-up is detected, deprecation warning logs are emitted.

Pros:
- Relies on the standard-based approaches - best practices already recommended to drivers users.
- Clear long-term model and responsibility.
- No hidden network calls in the final state (shutdown is simpler, less fragile).
- Backwards-compatible path for current users: keep existing behaviour first, then deprecate behind flags with warnings and docs.
- Telemetry from auto-cleanup events can highlight missing `close()` calls before we remove the safety net.

Cons:
- Extra wrapper complexity during transition (flags, deprecation, telemetry, removal).
- Temporary cross-language differences while some wrappers auto-clean and others do not.
- Potential regressions for apps that implicitly relied on auto-cleanup once defaults flip; requires clear docs and examples on explicit cleanup and interaction with `ABORT_DETACHED_QUERY` / session policies.

**Extensions:**

**Extension 1 – Telemetry&log-only leak detection in UD Core**
- UD Core does not auto‑close connections, but it can try detecting potentially leaked connections at context shutdown and emit telemetry and logs.
- UD Core tracks connection lifecycle (created / explicitly closed) - partially already implemented (HandleManager).

**Recommended:**

**Explicit and flexible approach:** Approach 4 (iterative auto-cleanup deprecation)
- Extension 1 – telemetry & logs on core-detected leaks

**Alternatives:**

**Implicit and flexible approach:** Approach 3 (auto-cleanup in wrappers)
- Extension 1 – telemetry & logs on core-detected leaks

## References

- Session renewal process in drivers
- [Design Doc] [UD] Fire&forget - SF async API and client session Logout
- [Scope doc] [Query execution] Async API 

