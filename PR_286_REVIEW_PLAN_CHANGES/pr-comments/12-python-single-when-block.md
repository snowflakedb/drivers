# #12 -- Rewrite deprecation scenario to single When block

**File**: `tests/definitions/python/session/logout.feature` lines 153-161
**Reviewer**: boler
**Status**: chosen direction is one subprocess-based scenario

## Before

```gherkin
Scenario: should emit deprecation warning on first auto-cleanup run per process
  # Phase 1 (doc for: SNOW-2314152) deprecation. Prepares users for explicit close() requirement.
  Given Snowflake Python client is created with auto_cleanup enabled
  And No auto-cleanup has run yet in this process
  When Process exits without explicit close
  Then atexit handler runs
  And Deprecation warning is emitted once
  When Another connection is created and process exits
  Then No additional deprecation warning is emitted
```

## After

Replace with one subprocess-based scenario:

```gherkin
Scenario: should emit deprecation warning only once when multiple auto-cleanup handlers run during process exit
  # Run this scenario in a dedicated Python subprocess to isolate process-global atexit state.
  Given A separate Python subprocess creates 10 Snowflake clients with auto_cleanup enabled
  And None of the connections are explicitly closed
  When The subprocess exits
  Then Auto-cleanup is triggered for all 10 leaked connections
  And Each auto-cleanup close is invoked with retry false
  And Deprecation warning is emitted once per process
```

## Rationale

The original two-`When` scenario was ambiguous, but splitting it into two independent scenarios still leaves awkward process-lifecycle semantics: once auto-cleanup has truly run, the process is already exiting.

Using one dedicated subprocess is a cleaner model because:

1. `atexit` registration and any "warn once per process" state are process-global.
2. The subprocess gives strong isolation from the surrounding test runner.
3. Creating 10 leaked connections in that Python subprocess lets the test prove both things we actually care about:
   - multiple auto-cleanup handlers were invoked
   - the deprecation warning is still emitted only once for the process

## Human Comment

Accepted approach: merge this into one subprocess-driven test.

Implementation guidance:

1. Spawn a dedicated Python subprocess.
2. In that Python subprocess, create 10 connections with `auto_cleanup` enabled and intentionally leak them.
3. Let the subprocess exit naturally so `atexit` runs.
4. Assert externally observable evidence that cleanup ran for all 10 connections.
5. Assert that the deprecation warning appears exactly once in subprocess output/logs.
6. Be careful to pass login/configuration data to the Python subprocess safely and avoid logging any secrets in command arguments, subprocess output, or failure messages.

Prefer proving "multiple atexit handlers ran" through per-connection cleanup side effects rather than by introspecting Python's internal `atexit` state.

## Comment Answer Proposition

_Add proposed reviewer reply here._
