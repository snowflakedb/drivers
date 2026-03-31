@core
Feature: Session Logout - Core HTTP Layer Integration

  # Low-level HTTP protocol validation and core integration tests.
  # These tests cover UD Core implementation details not exposed to wrappers.

  # ===========================================================================
  #                      HTTP Request Construction
  # ===========================================================================

  Scenario: should construct logout request with correct HTTP method URL headers and body
    Given Mock HTTP server is configured to capture requests
    And UD Core connection is logged in
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

  Scenario: should timeout after 5 seconds by default when server does not respond
    # Tests that default timeout is applied when no override provided
    # Mock server holds connection open (10s) to verify timeout interrupts after 5s
    Given Mock HTTP server holds connection open for 10 seconds without responding
    And UD Core connection is logged in with no timeout override
    When Logout is initiated
    Then Close throws timeout error

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
    And The last attempt times out because remaining budget is less than server response time

  # ===========================================================================
  #                      Close vs Active Query Execution
  # ===========================================================================
  # Close does not cancel in-flight HTTP connections (e.g. by signaling
  # connected sockets to close). While technically possible, this would cross
  # abstraction boundaries and be hard to maintain. Instead, internal services
  # are invalidated during close, causing subsequent operations (e.g.
  # parse_response on SnowflakeRequestService) to fail naturally when the
  # server response eventually arrives.
  # From the moment close is entered, the connection is in closing state and
  # absolutely no new queries can be scheduled.

  Scenario: should reject new query with connection closed error when submitted after close started
    Given Mock HTTP server delays logout response by 5 seconds then returns 200
    And UD Core connection is logged in
    When Connection close is initiated on a separate thread
    And Query SELECT 1 is submitted while logout is still in-flight
    Then Query SELECT 1 fails with connection closed error
    And Mock HTTP server did not receive any query request
    And Close completes successfully after logout response arrives

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

  # ===========================================================================
  #                  Close vs Token Refresh
  # ===========================================================================
  # If token renewal is already in-flight when close is called, close waits
  # for it to complete before proceeding. This prevents a race condition where
  # close and renewal both try to modify tokens simultaneously.
  # This contention is expected to be rare in practice.

  Scenario: should wait for in-flight token renewal to complete then logout with refreshed token
    Given Mock HTTP server delays token refresh response by 3 seconds then returns new token
    And Mock HTTP server accepts logout requests with 200
    And UD Core connection is logged in
    And Token refresh is already in-flight
    When Connection close is requested while refresh is still in-flight
    Then Mock HTTP server received token refresh request before logout request
    And Logout request Authorization header contains the refreshed session token
    And Close completes successfully

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
    And Mock HTTP server returns SESSION_GONE 390111
    When Logout is executed
    Then Close succeeds
    And Error is ignored

    Examples:
      | strategy_type |
      | strict        |
      | best-effort   |

  Scenario Outline: should retry logout on retryable <error_type> for each <strategy_type>
    Given Core logout function called with <strategy_type> strategy
    And Mock HTTP server returns <error_type> on attempt 1
    And Mock HTTP server returns 200 on attempt 2
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
    And Mock HTTP server returns SESSION_TOKEN_EXPIRED 390112
    And Retry policy allows 0 retries
    When Logout is executed
    Then No token refresh request is sent to server
    And Close throws SESSION_TOKEN_EXPIRED error

  Scenario: should not attempt token refresh when retry count is 0 with best-effort strategy
    # Same logic: no retries → no point refreshing token
    Given Core logout function called with best-effort strategy
    And Mock HTTP server returns SESSION_TOKEN_EXPIRED 390112
    And Retry policy allows 0 retries
    When Logout is executed
    Then No token refresh request is sent to server
    And SESSION_TOKEN_EXPIRED is logged as WARN
    And Close succeeds

  Scenario Outline: should attempt token refresh on 390112 when retries allowed for each <strategy_type>
    # With 1 retry allowed, token refresh + retry logout is possible
    # Both strategies must attempt refresh - 390112 is NOT treated as a final error
    Given Core logout function called with <strategy_type> strategy
    And Mock HTTP server returns SESSION_TOKEN_EXPIRED 390112 on first attempt
    And Mock HTTP server returns 200 after token refresh
    And Retry policy allows 1 retry
    When Logout is executed
    Then Token refresh request is sent to server
    And Logout is retried with new session token
    And Close succeeds
    # Refresh is "free" - doesn't count in total retries limit of attempts

    Examples:
      | strategy_type |
      | strict        |
      | best-effort   |

  Scenario: should succeed when retried logout fits within remaining timeout budget after token refresh
    Given Core logout function called with strict strategy
    And Timeout configured to 5 seconds
    And Retry policy allows 5 attempts
    And Mock HTTP server returns SESSION_TOKEN_EXPIRED 390112 on first attempt immediately
    And Token refresh endpoint delays response by 2 seconds
    And Mock HTTP server returns 200 immediately on retry attempt after refresh
    When Logout is executed
    Then Token refresh is attempted
    And Logout is retried exactly once
    And Close succeeds

  Scenario: should fail when retried logout exceeds remaining timeout budget after token refresh
    Given Core logout function called with strict strategy
    And Timeout configured to 5 seconds
    And Retry policy allows 5 attempts
    And Mock HTTP server returns SESSION_TOKEN_EXPIRED 390112 on first attempt immediately
    And Token refresh endpoint delays response by 6 seconds
    And Mock HTTP server returns 200 immediately on retry attempt after refresh
    When Logout is executed
    Then Token refresh is attempted
    And Logout is retried exactly once
    And Close throws timeout error
  # ---------------------------------------------------------------------------
  #  Retry and Timeout Configuration (Honors Provided Values)
  # ---------------------------------------------------------------------------
  # Design doc: Approach 4 + Extension 1 - wrappers can override retry config
  # Default: 5s timeout, HTTP-wide retry count
  # Wrappers pass their historical defaults (Python: 15s, JDBC/ODBC: 300s)

  # -- Success path: retry then succeed (same outcome for both strategies) --

  Scenario Outline: should honor provided retry config and succeed for each <strategy_type>
    Given Core logout function called with <strategy_type> strategy
    And Retry policy configured with <max_attempts> max attempts
    And Mock HTTP server fails <failures> times then returns 200
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
    # Wrappers pass their defaults (Python: 15s, JDBC/ODBC: 300s)
    # Note: Failure path scenarios (timeout exceeded) are below, split by strategy

    Given Core logout function called with <strategy_type> strategy
    And Timeout configured to <timeout_seconds> seconds
    And Retry policy allows the default attempt number
    And Mock HTTP server delays response by <delay_seconds> seconds then returns 200
    When Logout is executed
    Then Close succeeds

    Examples:
      | strategy_type | timeout_seconds | delay_seconds |
      | strict        | 5               | 3             |
      | best-effort   | 5               | 3             |
      | strict        | 10              | 8             |
      | best-effort   | 10              | 8             |
      # Python default timeout — also tighter margin to catch hardcoded values < 13
      | strict        | 15              | 13            |
      | best-effort   | 15              | 13            |
      # Real-life JDBC/ODBC defaults — sanity check that large timeouts aren't ignored
      | strict        | 300             | 50            |
      | best-effort   | 300             | 50            |

  # -- Failure path: exhausted retries (outcome differs per strategy) --

  Scenario Outline: should throw after exhausted retries with strict strategy
    Given Core logout function called with strict strategy
    And Retry policy configured with <max_attempts> max attempts
    And Mock HTTP server returns 503 on all attempts
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
    And Mock HTTP server returns 503 on all attempts
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
    And Mock HTTP server delays response by <delay_seconds> seconds
    When Logout is executed
    Then Close throws timeout error

    Examples:
      | timeout_seconds | delay_seconds |
      | 3               | 5             |
      | 5               | 10            |

  Scenario Outline: should log WARN and succeed on timeout with best-effort strategy
    Given Core logout function called with best-effort strategy
    And Timeout configured to <timeout_seconds> seconds
    And Mock HTTP server delays response by <delay_seconds> seconds
    When Logout is executed
    And Timeout is logged as WARN
    Then Close succeeds

    Examples:
      | timeout_seconds | delay_seconds |
      | 3               | 5             |
      | 5               | 10            |

  # -- Non-retryable errors: outcome differs per strategy --

  Scenario Outline: should throw on non-retryable <error_code> in strict strategy
    Given Core logout function called with strict strategy
    And Mock HTTP server returns <error_code> error
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
    And Mock HTTP server returns <error_code> error
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
    And UD Core connection is logged in
    When Connection close is initiated
    Then Pre-logout metrics are recorded in telemetry batch
    And Metrics include whether auto-detection was performed
    And Metrics include whether async queries were detected
    And Metrics include whether logout will be sent or skipped
    And Metrics include skip reason if logout is skipped
    And Telemetry batch is flushed before logout is sent
    And Logout proceeds after telemetry flush completes
