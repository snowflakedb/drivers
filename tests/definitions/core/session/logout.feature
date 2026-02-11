@core
Feature: Session Logout - Core HTTP Layer Integration

  # Low-level HTTP protocol validation and core integration tests.
  # These test UD Core implementation details not exposed to wrappers.

  # ===========================================================================
  #                      HTTP Request Construction
  # ===========================================================================

  Scenario: should construct logout request with correct HTTP method URL headers and body
    Given Mock HTTP server is configured to capture requests
    And UD Core client is logged in
    When Logout is initiated
    Then HTTP method is POST
    And Request URL path is /session
    And Query parameter delete is set to true
    And Query parameter requestId is present and static across attempts
    And Query parameter request_guid is present and unique per attempt
    And Authorization header is present with format "Snowflake Token={session_token}"
    And Content-Type header is application/json
    And Accept header is application/snowflake
    And User-Agent header contains UD version and Rust version
    And Request body is exactly empty JSON object {}

  Scenario: should not send logout when connection was never established
    Given Mock HTTP server is configured
    And Connection attempt failed before authentication
    When Connection close is attempted
    Then No HTTP request is sent to server
    And Local resources are cleaned up

  # ===========================================================================
  #                      Parameter-Based Logout Control
  # ===========================================================================

  Scenario: should not send logout when server_session_keep_alive is explicitly true
    Given Mock HTTP server is configured
    And UD Core connection is logged in with server_session_keep_alive set to true
    When Connection is closed
    Then No logout HTTP request is sent to server

  Scenario: should send logout when server_session_keep_alive is explicitly false
    Given Mock HTTP server is configured
    And UD Core connection is logged in with server_session_keep_alive set to false
    When Connection is closed
    Then Logout HTTP request is sent to server

  # ===========================================================================
  #                      Default Configuration
  # ===========================================================================

  Scenario: should use default 5 second timeout for logout requests
    Given Mock HTTP server is configured
    And UD Core connection with default timeout
    When Logout is initiated
    Then Request timeout is 5 seconds

  Scenario: should cancel individual request when per-request socket timeout exceeded
    # Tests that per-request timeout is passed to socket and interrupts slow responses
    Given Mock HTTP server holds connection open for 8 seconds on first attempt then succeeds immediately
    And UD Core connection is logged in
    And Per-request socket timeout is set to 2 seconds
    And Total retry budget timeout is set to 10 seconds
    When Logout is initiated
    Then First request is cancelled after 2 seconds due to socket timeout
    And Retry proceeds because total budget still has time remaining
    And Second request succeeds immediately
    And Close succeeds

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

  # ===========================================================================
  #                      Close vs Active Query Execution
  # ===========================================================================

  Scenario: should reject new query after close is initiated
    Given Mock HTTP server is configured
    And UD Core connection is logged in
    When Connection close is initiated
    And A new query SELECT 1 is submitted after close started
    Then Query is rejected immediately with connection closed error
    And No query HTTP request is sent to server

  Scenario: should cancel running query when close is initiated
    Given Mock HTTP server delays query response by 10 seconds
    And UD Core connection is logged in
    And Query is submitted and server has not responded yet
    When Connection close is initiated while query response is pending
    Then In-flight query request is cancelled
    And Query caller receives cancellation or connection closed error
    And Logout proceeds without waiting for query to complete

  # ===========================================================================
  #                  Close vs Token Refresh Race Conditions
  # ===========================================================================

  Scenario: should cancel in-progress token refresh when close is initiated
    Given Mock HTTP server delays token refresh response by 5 seconds
    And UD Core connection is logged in
    And Session token refresh request is in-flight
    When Connection close is initiated before refresh response arrives
    Then Token refresh request is cancelled
    And Session and master tokens are cleared despite incomplete refresh
    And Logout proceeds with the token that was available at close initiation time

  Scenario: should not renew token when 390112 arrives for a query after close already cleared tokens
    # Timeline: query executing → close() called → tokens cleared →
    # server responds 390112 to the in-flight query → query handler must NOT renew
    # because renewed tokens would overwrite the already-cleared token state
    Given Mock HTTP server returns 390112 SESSION_TOKEN_EXPIRED to query after 3 second delay
    And UD Core connection is logged in
    And Query is submitted and waiting for server response
    When Connection close is initiated while query is in-flight
    And Server responds 390112 SESSION_TOKEN_EXPIRED to the in-flight query after close started
    Then No token refresh request is sent to server
    And Previously cleared tokens are not overwritten by a renewal
    And Query fails with connection closed error

  # ===========================================================================
  #                  Error Strategy Behavior (Injected Strategy Testing)
  # ===========================================================================
  # Tests Core logout behavior with different error strategies injected
  # Both strategies are tested to ensure Core implements strategy pattern correctly

  # ---------------------------------------------------------------------------
  #  Backend Behaviors (Same for Both Strategies)
  # ---------------------------------------------------------------------------

  Scenario Outline: should ignore SESSION_GONE 390111 for each <strategy_type>
    Given Core logout function called with <strategy_type> strategy
    And Mock server returns SESSION_GONE 390111
    When Logout is executed
    Then Close succeeds
    And Error is ignored

    Examples:
      | strategy_type |
      | strict        |
      | best-effort   |

  Scenario Outline: should retry logout on retryable <error_type> for each <strategy_type>
    Given Core logout function called with <strategy_type> strategy
    And Mock server returns <error_type> on attempt 1
    And Mock server returns 200 on attempt 2
    When Logout is executed
    Then Logout is retried
    And Close succeeds

    Examples:
      | strategy_type | error_type              |
      | strict        | 503 Service Unavailable |
      | best-effort   | 503 Service Unavailable |
      | strict        | 429 Too Many Requests   |
      | best-effort   | 429 Too Many Requests   |
      | strict        | connection reset        |
      | best-effort   | connection reset        |

  Scenario: should not attempt token refresh when retry count is 0 with strict strategy
    # Token refresh implies a subsequent retry of logout with new token.
    # If no retries are allowed, refreshing the token would be pointless.
    Given Core logout function called with strict strategy
    And Mock server returns SESSION_TOKEN_EXPIRED 390112
    And Retry policy allows 0 retries
    When Logout is executed
    Then No token refresh request is sent to server
    And Close throws SESSION_TOKEN_EXPIRED error

  Scenario: should not attempt token refresh when retry count is 0 with best-effort strategy
    # Same logic: no retries → no point refreshing token
    Given Core logout function called with best-effort strategy
    And Mock server returns SESSION_TOKEN_EXPIRED 390112
    And Retry policy allows 0 retries
    When Logout is executed
    Then No token refresh request is sent to server
    And SESSION_TOKEN_EXPIRED is logged as WARN
    And Close succeeds

  Scenario Outline: should attempt token refresh on 390112 when retries allowed for each <strategy_type>
    # With 1 retry allowed, token refresh + retry logout is possible
    # Both strategies must attempt refresh - 390112 is NOT treated as a final error
    Given Core logout function called with <strategy_type> strategy
    And Mock server returns SESSION_TOKEN_EXPIRED 390112 on first attempt
    And Mock server returns 200 after token refresh
    And Retry policy allows 1 retry
    When Logout is executed
    Then Token refresh request is sent to server
    And Logout is retried with new session token
    And Close succeeds
    # TODO: Decide whether the token refresh request itself counts as a retry attempt

    Examples:
      | strategy_type |
      | strict        |
      | best-effort   |

  Scenario: should include token refresh time in total logout timeout budget
    # Token refresh is a network call that must be accounted for in total timeout
    Given Core logout function called
    And Mock server returns SESSION_TOKEN_EXPIRED 390112 on first attempt
    And Token refresh endpoint delays response by 3 seconds
    And Mock server returns 200 after token refresh
    And Total retry budget timeout is set to 5 seconds
    When Logout is executed
    Then Token refresh is attempted
    And Token refresh time is counted against total timeout budget
    And Remaining budget for retry logout is reduced by token refresh duration
    And Total wall-clock time does not exceed 7 seconds for closing the connection

  # ---------------------------------------------------------------------------
  #  Retry and Timeout Configuration (Honors Provided Values)
  # ---------------------------------------------------------------------------
  # Design doc: Approach 4 + Extension 1 - wrappers can override retry config
  # Default: 5s timeout, HTTP-wide retry count
  # Wrappers pass their historical defaults (Python: 5s, JDBC/ODBC: 300s)

  # -- Success path: retry then succeed (same outcome for both strategies) --

  Scenario Outline: should honor provided retry config and succeed for each <strategy_type>
    Given Core logout function called with <strategy_type> strategy
    And Retry policy configured with <max_attempts> max attempts
    And Mock server fails <failures> times then returns 200
    When Logout is executed
    Then Exactly <expected_attempts> attempts are made
    And Close succeeds

    Examples:
      | strategy_type | max_attempts | failures | expected_attempts |
      | strict        | 1            | 0        | 1                 |
      | best-effort   | 1            | 0        | 1                 |
      | strict        | 3            | 1        | 2                 |
      | best-effort   | 3            | 1        | 2                 |
      | strict        | 5            | 4        | 5                 |
      | best-effort   | 5            | 4        | 5                 |

  Scenario Outline: should honor provided timeout config and succeed for each <strategy_type>
    # Wrappers pass their historical defaults (Python: 5s, JDBC/ODBC: 300s)
    Given Core logout function called with <strategy_type> strategy
    And Timeout configured to <timeout_seconds> seconds
    And Mock server delays response by <delay_seconds> seconds then returns 200
    When Logout is executed
    Then Request completes within <timeout_seconds> seconds
    And Close succeeds

    Examples:
      | strategy_type | timeout_seconds | delay_seconds |
      | strict        | 5               | 3             |
      | best-effort   | 5               | 3             |
      | strict        | 10              | 5             |
      | best-effort   | 10              | 5             |
      | strict        | 300             | 10            |
      | best-effort   | 300             | 10            |

  # -- Failure path: exhausted retries (outcome differs per strategy) --

  Scenario Outline: should throw after exhausted retries with strict strategy
    Given Core logout function called with strict strategy
    And Retry policy configured with <max_attempts> max attempts
    And Mock server returns 503 on all attempts
    When Logout is executed
    Then Exactly <max_attempts> attempts are made
    And No further retries after max reached
    And WARN log is emitted
    And Close throws error

    Examples:
      | max_attempts |
      | 2            |
      | 3            |

  Scenario Outline: should log WARN and succeed after exhausted retries with best-effort strategy
    Given Core logout function called with best-effort strategy
    And Retry policy configured with <max_attempts> max attempts
    And Mock server returns 503 on all attempts
    When Logout is executed
    Then Exactly <max_attempts> attempts are made
    And No further retries after max reached
    And WARN log is emitted
    And Close succeeds

    Examples:
      | max_attempts |
      | 2            |
      | 3            |

  # -- Failure path: timeout (outcome differs per strategy) --

  Scenario Outline: should throw on timeout with strict strategy
    Given Core logout function called with strict strategy
    And Timeout configured to <timeout_seconds> seconds
    And Mock server delays response by <delay_seconds> seconds
    When Logout is executed
    Then Request times out after <timeout_seconds> seconds
    And Close throws timeout error

    Examples:
      | timeout_seconds | delay_seconds |
      | 3               | 5             |
      | 5               | 10            |

  Scenario Outline: should log WARN and succeed on timeout with best-effort strategy
    Given Core logout function called with best-effort strategy
    And Timeout configured to <timeout_seconds> seconds
    And Mock server delays response by <delay_seconds> seconds
    When Logout is executed
    Then Request times out after <timeout_seconds> seconds
    And Timeout is logged as WARN
    And Close succeeds

    Examples:
      | timeout_seconds | delay_seconds |
      | 3               | 5             |
      | 5               | 10            |

  # -- Non-retryable errors: outcome differs per strategy --

  Scenario Outline: should throw on non-retryable <error_code> in strict strategy
    Given Core logout function called with strict strategy
    And Mock server returns <error_code> error
    When Logout is executed
    Then Close throws error immediately
    And Error is surfaced to caller
    And No retries are attempted

    Examples:
      | error_code                  |
      | 400 Bad Request             |
      | 403 Forbidden               |
      | 404 Not Found               |
      | MASTER_TOKEN_EXPIRED 390114 |

  Scenario Outline: should log and suppress non-retryable <error_code> in best-effort strategy
    Given Core logout function called with best-effort strategy
    And Mock server returns <error_code> error
    When Logout is executed
    Then Error is logged as WARN
    And Close succeeds without throwing
    And No retries are attempted

    Examples:
      | error_code                  |
      | 400 Bad Request             |
      | 403 Forbidden               |
      | 404 Not Found               |
      | MASTER_TOKEN_EXPIRED 390114 |

  # ===========================================================================
  #                      Telemetry Integration
  # ===========================================================================

  Scenario: should record connection close decision metrics before logout
    # Requires: SNOW-2912513 (Telemetry)
    Given Telemetry client is configured
    And UD Core client is logged in
    When Connection close is initiated
    Then Pre-logout metrics are recorded in telemetry batch
    And Metrics include whether auto-detection was performed
    And Metrics include whether async queries were detected
    And Metrics include whether logout will be sent or skipped
    And Metrics include skip reason if logout is skipped
    And Telemetry batch is flushed before logout is sent
    And Logout proceeds after telemetry flush completes
