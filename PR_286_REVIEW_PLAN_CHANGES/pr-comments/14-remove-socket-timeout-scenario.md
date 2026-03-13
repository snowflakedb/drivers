# #14 -- Remove socket timeout scenario

**File**: `tests/definitions/core/session/logout.feature` lines 62-73
**Reviewer**: boler (explicitly requested removal)

## Remove entirely

```gherkin
  Scenario: should cancel individual request when per-request socket timeout exceeded
    # Tests that per-request timeout is passed to socket and interrupts slow responses
    Given Mock HTTP server holds connection open for 8 seconds on first attempt then succeeds immediately
    And UD Core connection is logged in
    And Per-request socket timeout is set to 2 seconds
    And Total retry budget timeout is set to 10 seconds
    When Logout is initiated
    Then First request is cancelled after 2 seconds due to socket timeout
    # Implementation: Use mock TCP server that holds connection, verify timing
    And Retry proceeds because total budget still has time remaining
    And Second request succeeds immediately
    And Close succeeds
```

## Rationale

fpawlowski and boler agreed that per-request socket timeout is out of scope for the logout feature and belongs in SNOW-2314153. boler's final comment: "Can you update the PR and remove the gherkin?"

## Related gherkin cleanup list

This removal should be treated as part of a slightly broader cleanup pass over logout gherkins that still mention or rely on socket-timeout concepts.

1. **Remove this scenario entirely**
   - `tests/definitions/core/session/logout.feature`
   - `Scenario: should cancel individual request when per-request socket timeout exceeded`

2. **Keep the implementation-level timeout comment in the total-budget scenario**
   - `tests/definitions/core/session/logout.feature`
   - `Scenario: should respect total retry budget timeout across all attempts`
   - The explanatory comment:
     - `Each request's effective socket timeout = min(remaining_budget, configured_socket_timeout)`
   - is acceptable because it documents an **underlying implementation detail**
   - No gherkin change is needed here as long as we do **not** expose socket-timeout configuration as something specified at the test level

3. **Remove nearby scenario setup that existed only to carry socket-timeout wording**
   - `tests/definitions/core/session/logout.feature`
   - `Scenario: should fail in-flight query when server response arrives after closing process started`
   - Remove: `And Socket timeout is set to 10 seconds`
   - The explicit timeout precondition should stay removed rather than renamed, because that scenario is about close-vs-query behavior, not timeout configuration.

4. **Remove the same socket-timeout precondition from the close-vs-token-refresh scenario**
   - `tests/definitions/core/session/logout.feature`
   - `Scenario: should not start token renewal when query receives 390112 after closing process started`
   - Remove: `And Socket timeout is set to 10 seconds`
   - This scenario is about close-vs-refresh behavior after closing starts, not about socket-timeout configuration.

5. **Do not reintroduce test-level socket-timeout configuration indirectly through timeout rewrites**
   - When updating the timeout matrix / budget scenarios, keep them framed around:
     - configured logout timeout
     - total retry budget
     - remaining budget after retries or refresh
   - It is okay for comments to mention implementation details like an effective underlying socket timeout
   - Do **not** bring back gherkin steps that imply the test can configure per-request socket timeout unless SNOW-2314153 is explicitly in scope.

6. **Current repo scan result**
   - A fresh search across `tests/definitions/` currently finds four remaining socket-timeout references in `tests/definitions/core/session/logout.feature`:
     - the scenario being removed entirely
     - the explanatory comment in `should respect total retry budget timeout across all attempts`
     - `And Socket timeout is set to 10 seconds` in `should fail in-flight query when server response arrives after closing process started`
     - `And Socket timeout is set to 10 seconds` in `should not start token renewal when query receives 390112 after closing process started`
   - So the practical cleanup scope is:
     - this scenario removal
     - removal of the two restored `Socket timeout is set to 10 seconds` steps
     - keeping the implementation-level comment in the total-budget scenario as-is
     - avoiding future reintroduction during timeout refactors

## Full scenarios before and after

### 1. Remove the dedicated socket-timeout scenario

**Before**

```gherkin
Scenario: should cancel individual request when per-request socket timeout exceeded
  # Tests that per-request timeout is passed to socket and interrupts slow responses
  Given Mock HTTP server holds connection open for 8 seconds on first attempt then succeeds immediately
  And UD Core connection is logged in
  And Per-request socket timeout is set to 2 seconds
  And Total retry budget timeout is set to 10 seconds
  When Logout is initiated
  Then First request is cancelled after 2 seconds due to socket timeout
  # Implementation: Use mock TCP server that holds connection, verify timing
  And Retry proceeds because total budget still has time remaining
  And Second request succeeds immediately
  And Close succeeds
```

**After**

```gherkin
# Scenario removed entirely.
```

### 2. Keep the implementation-level socket-timeout comment in the total-budget scenario

**Before**

```gherkin
Scenario: should respect total retry budget timeout across all attempts
  # Tests that total timeout caps wall-clock time across ALL retries
  # Each request's effective socket timeout = min(remaining_budget, configured_socket_timeout)
  # 2s server delay, 5s total budget:
  #   Attempt 1: effective timeout = min(5s, 10s) = 5s → waits 2s → 503 (remaining ~3s)
  #   Attempt 2: effective timeout = min(3s, 10s) = 3s → waits 2s → 503 or timeout (remaining ~1s)
  #   Attempt 3: effective timeout = min(1s, 10s) = 1s → timeout before 2s response arrives
  #   Attempt 4: should never start (budget exhausted)
  Given Mock HTTP server responds with 503 after 2 second delay on each attempt
  And UD Core connection is logged in
  And Total retry budget timeout is set to 5 seconds
  # Any number above 3 should be sufficient for max retries
  And Retry policy allows 10 attempts
  When Logout is initiated
  Then Fewer than 4 attempts are made
  And The last attempt timeouts because remaining budget is less than server response time
  And Total wall-clock time does not exceed 7 seconds for closing the connection
```

**After**

```gherkin
Scenario: should respect total retry budget timeout across all attempts
  # Tests that total timeout caps wall-clock time across ALL retries
  # Each request's effective socket timeout = min(remaining_budget, configured_socket_timeout)
  # 2s server delay, 5s total budget:
  #   Attempt 1: effective timeout = min(5s, 10s) = 5s → waits 2s → 503 (remaining ~3s)
  #   Attempt 2: effective timeout = min(3s, 10s) = 3s → waits 2s → 503 or timeout (remaining ~1s)
  #   Attempt 3: effective timeout = min(1s, 10s) = 1s → timeout before 2s response arrives
  #   Attempt 4: should never start (budget exhausted)
  Given Mock HTTP server responds with 503 after 2 second delay on each attempt
  And UD Core connection is logged in
  And Total retry budget timeout is set to 5 seconds
  # Any number above 3 should be sufficient for max retries
  And Retry policy allows 10 attempts
  When Logout is initiated
  Then Fewer than 4 attempts are made
  And The last attempt timeouts because remaining budget is less than server response time
  And Total wall-clock time does not exceed 7 seconds for closing the connection
```

No change is proposed here. The socket-timeout wording is acceptable in this comment because it documents the underlying implementation, not a test-level configurable setting.

### 3. Remove socket-timeout precondition from the close-vs-query scenario

**Before**

```gherkin
Scenario: should fail in-flight query when server response arrives after closing process started
  # The server completes the query — the HTTP connection is not cancelled.
  # The query fails because post-response processing cannot operate on
  # invalidated services after close.
  Given Mock HTTP server delays query response by 3 seconds then returns query result
  And Mock HTTP server accepts logout requests with 200
  And UD Core connection is logged in
  And Socket timeout is set to 10 seconds
  And Query is submitted and server has not responded yet
  When Connection close is initiated
  And Server returns query response after closing process started
  Then Mock HTTP server successfully completed query response delivery
  And Query caller receives connection closed error
  And Mock HTTP server received POST /session?delete=true logout request
  And Close completes successfully
```

**After**

```gherkin
Scenario: should fail in-flight query when server response arrives after closing process started
  # The server completes the query — the HTTP connection is not cancelled.
  # The query fails because post-response processing cannot operate on
  # invalidated services after close.
  Given Mock HTTP server delays query response by 3 seconds then returns query result
  And Mock HTTP server accepts logout requests with 200
  And UD Core connection is logged in
  And Query is submitted and server has not responded yet
  When Connection close is initiated
  And Server returns query response after closing process started
  Then Mock HTTP server successfully completed query response delivery
  And Query caller receives connection closed error
  And Mock HTTP server received POST /session?delete=true logout request
  And Close completes successfully
```

### 4. Remove socket-timeout precondition from the close-vs-token-refresh scenario

**Before**

```gherkin
Scenario: should not start token renewal when query receives 390112 after closing process started
  # After closing process starts, a query receiving 390112 cannot initiate
  # renewal — the internal services required for renewal are no longer available.
  Given Mock HTTP server returns 390112 SESSION_TOKEN_EXPIRED to query after 3 second delay
  And Mock HTTP server accepts logout requests with 200
  And UD Core connection is logged in
  And Socket timeout is set to 10 seconds
  And Query is submitted and waiting for server response
  When Connection close is initiated
  And Server responds 390112 SESSION_TOKEN_EXPIRED to the in-flight query
  Then Mock HTTP server did not receive any token refresh request
  And Query caller receives connection closed error
  And Close completes successfully
```

**After**

```gherkin
Scenario: should not start token renewal when query receives 390112 after closing process started
  # After closing process starts, a query receiving 390112 cannot initiate
  # renewal — the internal services required for renewal are no longer available.
  Given Mock HTTP server returns 390112 SESSION_TOKEN_EXPIRED to query after 3 second delay
  And Mock HTTP server accepts logout requests with 200
  And UD Core connection is logged in
  And Query is submitted and waiting for server response
  When Connection close is initiated
  And Server responds 390112 SESSION_TOKEN_EXPIRED to the in-flight query
  Then Mock HTTP server did not receive any token refresh request
  And Query caller receives connection closed error
  And Close completes successfully
```

## Human Comment

_Add human comment here._

## Comment Answer Proposition

I thought I had already applied this socket-timeout cleanup locally, but it looks like the AI stashed those edits before pushing. My bad. I re-applied the changes now, including the broader socket-timeout cleanup around the related logout gherkins.
