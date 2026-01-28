# [UD] [Design Doc] Query execution - Async API support Fire&Forget

**Author(s):** Filip Pawlowski  
**Date:** Jan 1, 2026  
**Last update:** Jan 12, 2026  
**Document status:** In Review

## Reviewers

| Reviewer | Status | Notes |
|----------|--------|-------|
| Jakub Szczerbinski | Not started | |
| Michal Hofman | Not started | |
| Tomasz Urbaszek | Not started | |
| Piotr Fus | Approved | |
| Patryk Czajka | In progress | |
| Sean Noonan | Not started | |
| Bartosz Oler | Not started | |
| Person | Not started | |

## Related Jira

**Main ticket:**
- SNOW-2923705: Fire&Forget - async queries & session lifecycle management

**Addressed UD Epics:**
- SNOW-2314152: [Query execution] Async API support

**Required UD Epics:**
- SNOW-2314152: [Query execution] Async API support
- SNOW-2314136: [Authentication] Log out
- Scope doc: [UD][Scope Doc] Authentication - Log out

## Goal

The goal is to plan an implementation of the "fire‑and‑forget" async queries flow in drivers (wrappers for Universal Driver). It allows users to submit a long‑running query asynchronously and close (or tear down) the client process, while keeping the server session alive (until all async queries issued within it are finished). Results should be available afterwards for extraction using the query ID - without the query being cancelled prematurely.

It requires independent components to be delivered on the Drivers side:

1. **Async execution API in drivers** - switching the Backend's query mode to async.
   - SNOW-2314152: [Query execution] Async API support
2. **Session lifecycle management in driver** (i.e. session logout).
   - SNOW-2314136: [Authentication] Log out
3. **Parameters configuration** - allowing client application to decide, whether session (and all associated queries) should be killed, or kept alive.

Unified implementation should decrease the frequency of bugs in this area and address strong customer demand.

## Table of Contents

- [Related Jira](#related-jira)
- [Goal](#goal)
- [Table of Contents](#table-of-contents)
- [Definitions](#definitions)
- [Context](#context)
  - [User perspective](#user-perspective)
  - [Server (GS) Behaviour](#server-gs-behaviour)
  - [Current Drivers (declared API and behaviour)](#current-drivers-declared-api-and-behaviour)
    - [Python Connector](#python-connector)
    - [JDBC Driver](#jdbc-driver)
    - [Go Driver (gosnowflake)](#go-driver-gosnowflake)
    - [Node.js Driver](#nodejs-driver)
    - [.NET Driver](#net-driver)
    - [ODBC, PHP Driver](#odbc-php-driver)
    - [libsnowflakeclient (C Library)](#libsnowflakeclient-c-library)
  - [Universal Driver - current implementation & gaps](#universal-driver---current-implementation--gaps)
    - [Existing Async Infrastructure](#existing-async-infrastructure)
    - [Connection & Session Management](#connection--session-management)
  - [Context Summary](#context-summary)
- [Universal Driver Fire‑and‑Forget Design](#universal-driver-fireandforget-design)
  - [Design Decisions](#design-decisions)
    - [Decision 1: Fire & Forget Trigger (Activation Strategy)](#decision-1-fire--forget-trigger-activation-strategy)
    - [Recommendation: Option C (Hybrid) + Extension 1 + Extension 3](#recommendation-option-c-hybrid--extension-1--extension-3)
    - [Suggested timeline](#suggested-timeline)
    - [Decision 2: Parameter Naming](#decision-2-parameter-naming)
    - [Recommendation: Option A (Strict unification)](#recommendation-option-a-strict-unification)
  - [Requirements and plan](#requirements-and-plan)
    - [Requirements Across Drivers](#requirements-across-drivers)
    - [Possible improvements (deprioritized for now)](#possible-improvements-deprioritized-for-now)

## Definitions

- **Async query (driver‑side)**: public API that submits a statement and returns control to the caller immediately (regardless of the query's result).
  - Sends param to the SF Backend: `asyncExec=true`

- **Fire‑and‑forget (F&F) semantics (in drivers)**: The ability to:
  1. Submit an async query.
  2. Allow the connection to be closed on the client side / the client process to exit.
  3. Have the query continue running to completion on the server (subject to timeouts).
  4. After the query ends with success, be able to fetch results by the query ID from the same or a new connection.

- **Logging out a session**: killing both client and server session - driver (or other client) sends `/session?delete=true` request. After logout, queries in that session are cancelled within a few minutes regardless of `ABORT_DETACHED_QUERY` (see more: Session renewal process in drivers).

- **Closing a connection**: driver cleans up client‑side resources; may or may not Log out the GS session as well (depending on parameters or internal logic).

- **Detached query**: a query whose original client connection is gone (e.g. closed), but the Snowflake server-session remains (no `/session?delete=true`), so the query may continue depending on `ABORT_DETACHED_QUERY` and session policies.

## Context

The current state of F&F in Drivers is inconsistent and misleading. Universal Driver wrappers do not yet have this functionality implemented, while core contains parts of the required logic.

### User perspective
"Fire-and-Forget" (F&F) queries are a valuable feature in analytical data processing, offering significant advantages. They allow for the immediate release of client-side resources, reducing CPU usage and the need for active polling. Instead, only server-side resources remain occupied until the query result is available.

Furthermore, even for queries expected to complete within the client session's duration, the F&F pattern is beneficial. A client can initiate a long-running operation at the beginning of a session, then "forget" about it while performing other local or lighter tasks, and only later retrieve the result for subsequent processing steps.

Customers frequently attempt "Fire-and-Forget" workflows, but the current experience is fragmented. Users often encounter SQL execution canceled errors because they rely on `ABORT_DETACHED_QUERY = FALSE` to protect their queries, unaware that explicit session logout overrides this parameter. Confusion persists because different drivers handle "closing a connection" differently: some log out the session immediately (killing queries), while others preserve it if async queries are running or if appropriate local parameter is set.

### Server (GS) Behaviour

To implement Fire-and-Forget correctly, we must align with GS behavior hierarchy:

1. **asyncExec=true**: If request `POST queries/v1/query-request` includes `asyncExec=true` url parameter, the Backend must immediately return the control to the caller (regardless of the query results).
2. **Explicit Logout**: If a driver sends `POST /session?delete=true`, GS terminates the session and cancels all running jobs, ignoring `ABORT_DETACHED_QUERY`.
   - More details in: [Scope doc] [Authentication] Log out 
3. **The Detached State**: If there is no activity from a session within 5 minutes, the client is considered "detached". All related queries will be killed, unless `ABORT_DETACHED_QUERY` is set to False.
4. **Timeouts**: If the `STATEMENT_TIMEOUT_IN_SECONDS` passes, queries will be cancelled regardless of the `ABORT_DETACHED_QUERY` parameter value.

Described dependencies are represented as well in the diagram below:

**Pic. 3.2.1** - Lifecycle of any query (sync and async) from the Server's perspective

### Current Drivers (declared API and behaviour)

The existing drivers established the current behavioral baseline. Their handling of async termination varies significantly. 

F&F is achieved by skipping the transmission of the `POST /session?delete=true` request when the connection closes. There are two approaches to achieve that:

1. **Manual Configuration (Parameter-Based)** The user manually switches a parameter (variable names differ across drivers) to suppress the logout signal.
2. **Automatic Detection (Registry-Based)** The driver automatically detects running queries to determine if logout should be skipped. This is generally CPU-heavier as it requires checking the status of all async queries issued within that session.

Currently F&F is supported in Python, JDBC and GO drivers (and .NodeJS if connection destruction is skipped intentionally, but it may have other side effects). 

**Detailed comparison of features required for the Fire&Forget:**

#### Python Connector

- **F&F**: supported
- **Async Model**: `cursor.execute_async(sql)`.
- **Closure Behavior**: 
  - Registry-Based (skips logout if RUNNING async queries detected - method `_all_async_queries_finished()`) 
  - Parameter-Based (`server_session_keep_alive=True` forces session to stay alive).
- **Reattach**: `get_results_from_sfqid(query_id)`.

In practice Python has two behaviours: explicit keep‑alive (True) and auto‑detect‑driven close (False / None) - described in the table below:

| User param value (server_session_keep_alive) | Auto-detect returns | Logout? | Behaviour on close |
|---------------------------------------------|---------------------|---------|-------------------|
| True | any | No | Always keep server session alive; F&F regardless of async state. |
| False (default) / None | False | No | Running async queries detected → skip logout → F&F (current auto‑detect path). |
| False (default) / None | True | Yes | No running async queries → send logout → cancel remaining jobs. |



#### JDBC Driver

- **F&F**: supported
- **Async Model**: `SnowflakeStatement.executeAsyncQuery(sql)`.
- **Closure Behavior**: Purely Registry-Based (skips logout if running async queries detected).
- **Reattach**: `SnowflakeConnection.createResultSet(queryId)`.

There is no User parameter to keep the server session alive - only the async registry is consulted - described in the table below:

| Auto-detect returns | Logout? | Behaviour on close |
|---------------------|---------|-------------------|
| False | No | Auto‑detect → skip logout when async queries still running. |
| True | Yes | Auto‑detect → logout when all async queries finished. |

#### Go Driver (gosnowflake)

- **F&F**: supported
- **Async Model**: `WithAsyncMode(ctx)` context modifiers.
- **Closure Behavior**: Purely Parameter-Based. Default is logout on `Close()`. Requires `Config.KeepSessionAlive = true` to suppress.
- **Reattach**: `WithQueryIDChan` to capture IDs and `WithFetchResultByID` to fetch results later.

No registry‑based auto‑detect; purely parameter‑based - described in the table:

| User param value (KeepSessionAlive) | Logout? | Behaviour on close |
|-------------------------------------|---------|-------------------|
| True | No | KeepSessionAlive=true → skip logout (F&F semantics). |
| False (default) | Yes | KeepSessionAlive=false → always logout on close (no auto‑detect). |


#### Node.js Driver

- **F&F**: possible (via workaround)
- **Async Model**: `execute({ asyncExec: true })`.
- **Closure Behavior**: Sends logout (`/session?delete=true`) on `connection.destroy()`, effectively cancelling running queries. There is no API parameter to suppress this.
- **Workaround**: Fire & Forget can only be achieved by intentionally not calling `destroy()`. However, this may cause local resources not being cleaned up correctly - e.g. the Node.js process may not exit automatically because the heartbeat keeps the event loop alive.
- **Reattach**: `connection.getResultsFromQueryId(queryId)`.

#### .NET Driver

- **F&F**: not supported
- **Async Model**: `cmd.ExecuteInAsyncMode()`.
- **Closure Behavior**: Always sends logout (`POST /session?delete=true`). No mechanism to skip.
- **Reattach**: `cmd.GetResultsFromQueryId(queryId)` and `GetResultsFromQueryIdAsync`.

#### ODBC, PHP Driver

- **F&F**: not supported
- **Async Model**: Synchronous execution only.
- **Closure Behavior**: Always sends logout (`POST /session?delete=true`) via `snowflake_term()`. No mechanism to skip.
- **Reattach**: No public API contract for reattaching by ID.

#### libsnowflakeclient (C Library)

- **F&F**: not supported
- **Async Model**: `snowflake_async_execute()`.
- **Closure Behavior**: `snowflake_term()` always sends logout (`POST /session?delete=true`). No mechanism to skip.
- **Reattach**: `snowflake_init_async_query_result(sf, query_id)`.




| Feature | Python | JDBC | GO | NodeJS | .NET | ODBC | libsfclient | PHP |
|---------|--------|------|-----|--------|------|------|-------------|-----|
| Session logout (sends `/session?delete=true` when connection is closed on the client side) | Yes | Yes* | Yes | Yes* | Yes | Yes* | Yes | Yes |
| Async execution (with active client session) | Yes | Yes | Yes | Yes | Yes | No | Yes | No |
| Parameter to omit session logout when closing client-session (keep server session alive) | Yes | No | Yes | No | No | No | No | No |
| Automatically omitting session logout if there are async queries running (that were issued using the current session) | Yes | Yes | No | No | No | No | No | No |
| Fire&Forget (async execution with closed client session) | Yes | Yes | Yes | No** | No | No | No | No |

\* Only if the client application calls the closing function explicitly - no automatic exit when exiting the context.  
\*\* Could be achieved by skipping explicit closing of the connection (which is normally recommended); can cause lack of proper resources release after connection is closed.

### Universal Driver - current implementation & gaps

#### Existing Async Infrastructure

The file `sf_core/src/rest/snowflake/async_exec.rs` contains async primitives (`submit_statement_async`), but they are currently used solely for poll-to-completion workflows:

- **Current Flow**: `execute_blocking_with_async()` submits the query but immediately enters `wait_for_completion()` with exponential backoff.
- **Missing**: There is no "submit and return" path that returns a durable handle to the caller without blocking.

#### Connection & Session Management

The Connection struct in `sf_core` is minimal and lacks the state management found in mature drivers:

- **No Async Registry**: The struct has no field to track active query IDs (e.g., `async_queries` map).
- **No Logic-Aware Close**: UD currently exposes `connection_release()` to free handles, but there is no logic-aware `connection_close()` equivalent that inspects async state and decides whether to log out (send `/session?delete=true`).
- **No Token Renewal**: Session token renewal for long-running queries is not implemented (SNOW-2371565), meaning queries running longer than the token lifespan (~1h), may fail to be checked for results within the same session. Using another session should be sufficient for F&F though, so it is not blocking (will be delivered in SNOW-2314154: [Authentication] Renew session).

### Context Summary

F&F feature requires:

1. User API to issue queries in Snowflake Async mode.
2. Logic preventing the `/session?delete=true` request from being sent on local connection closing.

There are 2 possible approaches to preventing logout:

- **Manually**: By exposing the appropriate parameter (different names across drivers).
- **Automatically**: By checking statuses of all async queries issued within that session by driver - i.e. if any of them is still running (CPU-heavier as it requires HTTP call for each one of them).

Universal driver has fragments of code needed for this feature, but:

- Session lifecycle management needs to be implemented.
- Async execution path needs to be extracted from active polling and exposed in protobuf as API.



## Universal Driver Fire‑and‑Forget Design

This section outlines the specific architectural decisions for the Universal Driver.

### Design Decisions

#### Decision 1: Fire & Forget Trigger (Activation Strategy)

How does the driver decide whether to send the `DELETE /session` request or skip it?

**Option A: Manual (Parameter-Based)**

Pros:
- Easy to understand mechanics (If flag = true, skip logout).
- Gives customers explicit control over costs (opt-in).

Cons:
- **Parameter Bloat**: Drivers already suffer from an overwhelming number of configuration flags. Adding another parameter increases the cognitive load for users.
- **Naming Confusion**: Risk of confusing heartbeat settings (`client_session_keep_alive`) with persistence settings (local session vs server session).
- **Breaking behaviour**: Drivers that previously had the auto-detection logic implemented (Python, JDBC), will now require customers to set some parameter to achieve the same result.

**Option B: Automatic (Registry-Based)**

Pros:
- **Zero-config**: New users are automatically protected from accidental costs or query kills.

Cons:
- **Performance Overhead**: CPU-heavy checks using HTTP calls (costly on local resources - painful on FaaS e.g. AWS Lambda).
- **Latency**: Significantly prolongs the closing operation (~ 50% for async heavy sessions).
- **Unpredictability**: Logic happens "under the hood." If detection hits an edge case (e.g., network error), the driver might behave inconsistently (killing a query the user expected to survive), making debugging difficult compared to explicit configuration.

**Option C: Hybrid (A + B)**

Logic: First check flag, then only check all Async query IDs (if flag was not set / kept the default False).

Pros:
- "Best of both worlds" safety net.

Cons:
- **Parameter Bloat**: Drivers already suffer from an overwhelming number of configuration flags. Adding another parameter increases the cognitive load for users.
- **Naming Confusion**: Risk of confusing heartbeat settings (`client_session_keep_alive`) with persistence settings (local session vs server session).
- **Higher development effort**. Could be added later without breaking changes.


**Possible Extensions:**

**Extension 1: Enable Auto-Detection Parameter**

Applicable if we go with Option B (Automatic) or Option C (Hybrid). For some workloads (e.g. high‑throughput, short‑lived jobs on AWS Lambda / K8s) auto-detection is undesirable: users may prefer an explicit control and extra checks only add CPU and latency. An explicit "enable auto‑detection" parameter (e.g. `enable_server_session_keep_alive_auto_detection`) lets users control it:

- `True` → run registry check and; if async queries are still running, skip logout (safety‑net).
- `False` → never run registry check;

Downsides:
- This described opt-in/opt-out from auto-detection is already naturally achieved by setting the `server_session_keep_alive` param (Option A). Both True and False values could mean switching to manual control, while unset / null would be interpreted as auto-detection enabled. In such a case, additional param can be perceived as redundancy.
- May be confusing, on what is the difference between disabled auto-detection and `server_session_keep_alive` set to False - both result in "forced Logout" mode.

**Extension 2: Treat unset / null value of server_session_keep_alive as "enable auto‑detection"**

Alternative to the Extension 1

Define the safety‑net behaviour as:
- `server_session_keep_alive = null / None` (is unset) → auto‑detection allowed (registry may skip or issue the logout).
- `server_session_keep_alive = true / false` → auto‑detection disabled (no registry checks; explicit behaviour only).

Downsides:
- Behaviour becomes implicit: relying on the safety‑net requires leaving `server_session_keep_alive` unset rather than flipping a clear "enable auto‑detection" flag.
- This may be non‑intuitive for users, especially after switching to opt-in ("I have to set the flag to null to turn on the safety‑net") and harder to document.
- I.e. it would make deprecation of auto-detection less customer-friendly during the opt-in stage - turning on the old behaviour back (auto-detection), would require 'unsetting' the param / passing null-like value.

**Extension 3: Log deprecation warnings + send telemetry** - when the user is using the default value, that will change its behaviour in the future.

#### Recommendation: Option C (Hybrid) + Extension 1 + Extension 3

We will deliver both ways of controlling the connection closing, allowing explicit control and keeping the drivers backward-compatible right after the GA - to increase the adoption.

#### Suggested timeline

Migration of this feature should happen in 3 phases: 1. current state (old drivers), 2. UD-mirror (first UD release, fully backward-compatible), 3. Desired behaviour. Those steps are described below - for drivers that have already supported Fire&Forget before - to plan their migration appropriately.

**Phase 1 – Current behaviour (pre‑UD / old drivers)**

Current state, already described in the [section] [current drivers behaviour].

**Notation (shortcuts)**

**User param value**  
The setting controlling whether the server session is kept alive on close:
- Python: `server_session_keep_alive`
- JDBC: `serverSessionKeepAlive`
- Go: `ServerSessionKeepAlive` (renamed in the coming BCR in old driver)
- Values: `True`, `False`, or `null` (`None` / `<unset>` etc.)

**Enable auto-detect**  
Whether registry‑based auto‑detection of async queries is enabled:
- Python: `enable_server_session_keep_alive_auto_detection`
- JDBC: `enableServerSessionKeepAliveAutoDetection`
- Go: `EnableServerSessionKeepAliveAutoDetection`
- Values: `True` or `False`, or `null` (`None` / `<unset>` etc.)
- Rule: if this flag is omitted / null in config, it is treated as `False` (auto‑detect disabled).

**Auto-detect returns**  
Result of the internal async registry check (when auto‑detect is enabled):
- `False` → at least one async query still running.
- `True` → all async queries finished.
- `any` → registry result is not consulted for this combination (either auto‑detect is disabled or overridden by the user param).

**Phase 2 – After UD release (Hybrid in core; per‑driver defaults keep old behaviour)**

Hybrid in core; per‑driver defaults keep old behaviour, but all interfaces are exposed

**Python – Phase 2**

Defaults: `User param value = None`, `Enable auto-detect = True`.

| User param value | Enable auto-detect | Auto-detect returns | Logout? | Deprecation (WARN)? | Behaviour on close |
|------------------|-------------------|---------------------|---------|---------------------|-------------------|
| True | any | any | No | No | Explicit keep-alive: never send `/session?delete=true` (F&F regardless of async). |
| False | True (default) | False | No | Yes (User param set to False will ignore auto-detection in future) | Legacy Python: async running → skip logout. |
| False | True (default) | True | Yes | Yes (User param set to False will ignore auto-detection in future) | Legacy Python: no async → send logout. |
| False | False | n/a | Yes | Yes (User param set to False will ignore auto-detection in future) | Force logout: auto-detect disabled, always send `/session?delete=true`. |
| None (default) | True (default) | False | No | No | Legacy Python: async running → skip logout. |
| None (default) | True (default) | True | Yes | No | Legacy Python: no async → send logout. |
| None (default) | False | n/a | Yes | No | Force logout: auto-detect disabled, always send `/session?delete=true`. |

All old settings:
- `server_session_keep_alive=True` → still "never logout".
- `server_session_keep_alive=False` or omitted (`None`) with auto‑detect True → same legacy registry behaviour (default).
- Explicitly setting `Enable auto-detect = False` is new behaviour.


**JDBC – Phase 2**

Defaults: `User param value = null`, `Enable auto-detect = True`.

| User param value | Enable auto-detect | Auto-detect returns | Logout? | Deprecation (WARN)? | Behaviour on close |
|------------------|-------------------|---------------------|---------|---------------------|-------------------|
| True | any | any | No | No | Explicit keep-alive: never send `/session?delete=true` (F&F regardless of async). |
| False | any | any | Yes | No | Explicit kill: always send `/session?delete=true`; ignore registry (strong "close & cancel"). |
| null (default) | True (default) | False | No | Yes (Auto-detect will not be enabled by default in future) | Legacy JDBC: async running → skip logout (matches old behaviour). |
| null (default) | True (default) | True | Yes | Yes (Auto-detect will not be enabled by default in future) | Legacy JDBC: no async → send logout (matches old behaviour). |
| null (default) | False / null | n/a | Yes | No | Force logout: auto-detect disabled, always send `/session?delete=true`. |

All old settings:
- "No keep‑alive param, default behaviour" → `User param value = null`, `Enable auto-detect = True` → same registry behaviour.
- Setting `User param value` to True or False, or changing `Enable auto-detect`, is new and intentional.


**Go – Phase 2**

Defaults: `User param value = <unset>`, `Enable auto-detect = <unset>`.

| User param value | Enable auto-detect | Auto-detect returns | Logout? | Deprecation (WARN)? | Behaviour on close |
|------------------|-------------------|---------------------|---------|---------------------|-------------------|
| True | any | any | No | No | Explicit keep-alive: KeepSessionAlive=true → skip logout (F&F), same as today. |
| False | any | any | Yes | No | Legacy Go default: always logout on close, no registry checks. |
| <unset> (default) | True | False | No | No | New safety-net mode: async running → skip logout (registry consulted). |
| <unset> (default) | True | True | Yes | No | New safety-net mode: no async → send logout. |
| <unset> (default) | False / <unset> (default) | n/a | Yes | No | Default Phase‑3 behaviour: no auto-detect; always logout on close. |

All old settings:
- `ServerSessionKeepAlive=true` → still "never logout".
- `ServerSessionKeepAlive=false` → still "always logout".
- `Enable auto-detect` only changes behaviour when explicitly set and `ServerSessionKeepAlive` is not set (default).
- For Go UD wrapper, the legacy paths (e.g. DSN with no explicit keep-alive flag) is mapped to the unset `ServerSessionKeepAlive` (by default), so they continue to use the legacy behaviour. In that case, with `EnableServerSessionKeepAliveAutoDetection=true`, the safety-net rows above apply.


**Phase 3 – Ultimate unified model (all drivers)**

In Phase 3, all UD wrappers share the same conceptual semantics:

**User param value (keep‑alive flag):**

Values:
- `True` → explicit keep server session alive on close (F&F).
- `False` → explicit force logout (always send `/session?delete=true`).
- `null` → delegate to auto‑detect (if enabled) or default behaviour.

**Enable auto-detect (auto‑detection flag):**

Values:
- `False` → no registry‑based auto‑detect; behaviour controlled solely by keep‑alive flag.
- `True` → safety net: consult async registry when keep‑alive is None.
- `unset / null` (default) → treated as False.

**Auto-detect returns** = result of function like `_all_async_queries_finished()`:
- `False` → at least one async query still running.
- `True` → no async queries running.

**Phase 3 behaviour (conceptual)**

Defaults: `User param value = null*`, `Enable auto-detect = null*`.

| User param value (server_session_keep_alive) | Enable auto-detect (enable_server_session_keep_alive_auto_detection) | Auto-detect returns | Logout? | Behaviour on close |
|----------------------------------------------|-------------------------------------------------------------------|---------------------|---------|-------------------|
| True | any | any | No | Always keep server session alive; explicit Fire‑and‑Forget. |
| False | any | any | Yes | Always send `/session?delete=true`; explicit "kill session & all jobs". |
| null* (default) | False / null* (default) | n/a | Yes | Default Phase‑3 behaviour: no auto-detect; always logout on close. |
| null* (default) | True | True | Yes | Safety-net: no async queries running → send logout. |
| null* (default) | True | False | No | Safety-net: async running → skip logout; keep session (and async queries) alive. |

\* null means the appropriate value in each language that represents an unset / empty data

#### Decision 2: Parameter Naming

How should we expose the "Keep Alive" configuration in the wrappers?

**Option A: Strict Unification**

Concept: Force all drivers to use a new, single standard name (e.g. `server_session_keep_alive`).

Pros:
- Consistent behaviour; easier code transfer between drivers; clean API.

Cons:
- Major breaking change for existing users of Python and Go drivers.

Note: after consulting with GO driver owner (Piotr Fus) we determined that the param name change there can be done before introducing UD, so downsides will be minimised.

**Option B: Keep legacy names (Hybrid)**

Concept: Retain legacy names for backward compatibility, but enforce a standard for the core and future wrappers.

- **Legacy Wrappers**: For drivers already supporting F&F, retain the existing parameter names (Go: `KeepSessionAlive`, Python: `server_session_keep_alive`).
- **New/Standardized**: For `sf_core` internal config and any new wrappers (or wrappers adding this feature for the first time), adopt the name: `server_session_keep_alive` (following the standard language-specific naming convention).

Pros:
- Balances backward compatibility (no breaking changes) with future standardization.

Cons:
- Perpetuates inconsistency; confusing for users working with multiple languages; complicates documentation.

#### Recommendation: Option A (Strict unification)

Reason: after consulting with GO driver owner (Piotr Fus) we determined that the param name change there can be done before introducing UD, so downsides will be minimised.

### Requirements and plan

#### Requirements Across Drivers

To ensure a consistent F&F experience, all UD wrappers must adhere to the following contracts:

1. **Parameter Mapping:**
   - Wrappers must map their language-specific "keep alive" parameter (e.g., Go's `KeepSessionAlive`) to the `sf_core`'s `server_session_keep_alive` configuration field.

2. **Graceful Exit:**
   - Wrappers must guarantee that calling `Close()`-like function (ending the local session) with `server_session_keep_alive=true` fully releases the main thread. No background threads (heartbeat/telemetry) may remain active to block the process termination.
   - Requires delivery of: [UD][Scope Doc] Authentication - Log out

3. **Async API Exposure:**
   - Wrappers must expose the new `submit_async_query` functionality from `sf_core` as a public method that returns a Query ID (or a handle containing it) immediately.
   - If a driver-specific standard suggests naming for such a method, it should be used.
   - Otherwise the suggested name is `execute_async`.

#### Possible improvements (deprioritized for now)

- **Token Renewal**: Implementation of session renewal to support extremely long-running monitoring. SNOW-2314154: [Authentication] Renew session
- **Heartbeat Management**: Robust handling of heartbeat during detached states. SNOW-2881763: [Authentication] Heartbeat (keep session alive)
- **Query Registry (Optional)**: Investigation into a "Safety Mode" registry that warns users if they are closing a session with active queries.
  - Possibly with a flag to switch it off for compute-sensitive pipelines;
  - In the hierarchy below the actual `server_session_keep_alive` check - to avoid unnecessary operations.
- **Re-attach Capability:**
  - Implement the ability to create a Result Set / Reader object purely from a Query ID string, without requiring the original connection object.
  - E.g. current `get_results_from_sfqid(query_id)` in Python Driver.

## References (optional)

Docs regarding: 
- Session renewal process in drivers
- Python, JDBC, Go doc change proposal to provide more details to customers on 'keep server session' behaviour for async queries



