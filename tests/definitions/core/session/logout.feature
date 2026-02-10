@core
Feature: Session Logout - Core HTTP Layer Integration

  # Low-level HTTP protocol validation and core integration tests.
  # These test UD Core implementation details not exposed to wrappers.

  # ===========================================================================
  #                      HTTP Request Construction
  # ===========================================================================

  Scenario: should construct logout request with correct HTTP method URL headers and body
    Given Mock HTTP server is configured to capture requests
    And UD Core client is logged in with session token
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
    And UD Core connection with server_session_keep_alive set to true
    When Connection is closed
    Then No logout HTTP request is sent to server

  Scenario: should send logout when server_session_keep_alive is explicitly false
    Given Mock HTTP server is configured
    And UD Core connection with server_session_keep_alive set to false
    When Connection is closed
    Then Logout HTTP request is sent to server

  # ===========================================================================
  #                      Timeout and Retry Behavior
  # ===========================================================================

  Scenario: should apply retry policy to logout HTTP request
    Given Mock HTTP server returns 503 error on first attempt
    And Mock HTTP server returns 200 on second attempt
    And Retry policy is configured to allow 2 attempts
    When Logout is initiated
    Then First request receives 503 response
    And Retry policy is consulted
    And Second request is made after backoff delay
    And Logout succeeds

  Scenario: should apply retry policy to unsuccessful logout HTTP request
    Given Mock HTTP server returns 503 error on first attempt
    And Mock HTTP server returns 503 on second attempt
    And Retry policy allows 2 attempts
    When Logout is not initiated
    Then First request receives 503 response
    And Retry policy is consulted
    And Second request is made after backoff delay
    And Logout fails
#TODO:    And Further decisions are passed to the Strategy - 2 tests, showing different behaviour based on the fact which strategy was chosen


  Scenario: should handle HTTP connection reset during logout
    Given Mock HTTP server resets connection on first attempt
    And Mock HTTP server succeeds on second attempt
    When Logout is initiated
    Then Connection reset is detected
    And Request is retried according to retry policy
    And Logout succeeds on retry

  Scenario: should use default 5 second timeout for logout requests
    Given Mock HTTP server is configured
    And UD Core connection with default timeout
    When Logout is initiated
    Then Request timeout is 5 seconds

  Scenario: should honor custom timeout configuration
    Given Mock HTTP server is configured
    And UD Core connection with custom timeout of 10 seconds
    When Logout is initiated
    Then Request timeout is 10 seconds

  Scenario: should honor total timeout across multiple retries
    Given Mock HTTP server delays each response by 2 seconds
    And Total timeout is configured to 3 seconds
    And Retry policy allows 3 attempts
    When Logout is initiated
    Then Only 1 attempt is made before timeout
    And Total timeout is respected across retries

  Scenario: should cancel request when timeout exceeded during execution
    Given Mock HTTP server delays response beyond timeout
    And Timeout is set to 2 seconds
    When Logout is initiated
    Then Request is cancelled after 2 seconds
    And Timeout error is returned

  # ===========================================================================
  #                      Concurrency and State Management
  # ===========================================================================

  Scenario: should handle close during session token refresh
    Given Mock HTTP server simulates concurrent logout and refresh
    And Session token refresh is in progress
    When Connection close is initiated
    Then Refresh operation is cancelled or completed
    And Logout proceeds with available token
    And No race conditions occur

  Scenario: should handle close during active query execution
    Given Connection has active query in progress
    When Connection close is initiated
    Then New operations are rejected with connection closed error
    And Active query execution is interrupted
    And Resources are cleaned up

  # ===========================================================================
  #                  Error Strategy Behavior (Injected Strategy Testing)
  # ===========================================================================
  # Tests Core logout behavior with different error strategies injected
  # Both strategies are tested to ensure Core implements strategy pattern correctly

  # ---------------------------------------------------------------------------
  #  Backend Behaviors (Same for Both Strategies)
  # ---------------------------------------------------------------------------

  Scenario Outline: should ignore SESSION_GONE 390111 with <strategy> strategy
    Given Core logout function called with <strategy> strategy
    And Mock server returns SESSION_GONE 390111
    When Logout is executed
    Then Close succeeds
    And Error is ignored

    Examples:
      | strategy    |
      | strict      |
      | best-effort |

  Scenario Outline: should retry on <error_type> with <strategy> strategy
    Given Core logout function called with <strategy> strategy
    And Mock server returns <error_type> on attempt 1
    And Mock server returns 200 on attempt 2
    When Logout is executed
    Then Logout is retried
    And Close succeeds

    Examples:
      | strategy    | error_type              |
      | strict      | 503 Service Unavailable |
      | best-effort | 503 Service Unavailable |
      | strict      | 429 Too Many Requests   |
      | best-effort | 429 Too Many Requests   |
      | strict      | connection reset        |
      | best-effort | connection reset        |

  Scenario Outline: should attempt session token renewal on 390112 with <strategy> strategy
    # SESSION_TOKEN_EXPIRED: recoverable via master token refresh
    Given Core logout function called with <strategy> strategy
    And Mock server returns SESSION_TOKEN_EXPIRED 390112
    And Master token is valid for renewal
    When Logout is executed
    Then Session token renewal is attempted using master token
    And Logout is retried with new session token
    And Close succeeds

    Examples:
      | strategy    |
      | strict      |
      | best-effort |

  # ---------------------------------------------------------------------------
  #  Retry and Timeout Configuration (Honors Provided Values)
  # ---------------------------------------------------------------------------
  # Design doc: Approach 4 + Extension 1 - wrappers can override retry config
  # Default: 5s timeout, HTTP-wide retry count
  # Wrappers pass their historical defaults (Python: 5s, JDBC/ODBC: 300s)

  # -- Success path: retry then succeed (same outcome for both strategies) --

  Scenario Outline: should honor provided retry config and succeed with <strategy> strategy
    Given Core logout function called with <strategy> strategy
    And Retry policy configured with <max_attempts> max attempts
    And Mock server fails <failures> times then returns 200
    When Logout is executed
    Then Exactly <expected_attempts> attempts are made
    And Close succeeds

    Examples:
      | strategy    | max_attempts | failures | expected_attempts |
      | strict      | 1            | 0        | 1                 |
      | best-effort | 1            | 0        | 1                 |
      | strict      | 3            | 1        | 2                 |
      | best-effort | 3            | 1        | 2                 |
      | strict      | 5            | 4        | 5                 |
      | best-effort | 5            | 4        | 5                 |

  Scenario Outline: should honor provided timeout config and succeed with <strategy> strategy
    # Wrappers pass their historical defaults (Python: 5s, JDBC/ODBC: 300s)
    Given Core logout function called with <strategy> strategy
    And Timeout configured to <timeout_seconds> seconds
    And Mock server delays response by <delay_seconds> seconds then returns 200
    When Logout is executed
    Then Request completes within <timeout_seconds> seconds
    And Close succeeds

    Examples:
      | strategy    | timeout_seconds | delay_seconds |
      | strict      | 5               | 3             |
      | best-effort | 5               | 3             |
      | strict      | 300             | 10            |
      | best-effort | 300             | 10            |

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

  # ---------------------------------------------------------------------------
  #  Strategy-Specific Behaviors
  # ---------------------------------------------------------------------------

  Scenario: should throw on non-retryable errors in strict strategy
    # Parametrized test implementation should cover: 400, 401, 403, 404, 405, 409
    Given Core logout function called with strict strategy
    And Mock server returns non-retryable HTTP error
    When Logout is executed
    Then Close throws error immediately
    And Error is surfaced to caller
    And No retries are attempted

  Scenario: should log and suppress non-retryable errors in best-effort strategy
    # Parametrized test implementation should cover: 400, 401, 403, 404, 405, 409
    Given Core logout function called with best-effort strategy
    And Mock server returns non-retryable HTTP error
    When Logout is executed
    Then Error is logged as WARN
    And Close succeeds without throwing
    And No retries are attempted

  Scenario: should handle master token expired 390114 with strict strategy injected
    Given Core logout function called with strict strategy
    And Mock server returns MASTER_TOKEN_EXPIRED 390114
    When Logout is executed
    Then Close throws reauth error

  Scenario: should handle master token expired 390114 with best-effort strategy injected
    Given Core logout function called with best-effort strategy
    And Mock server returns MASTER_TOKEN_EXPIRED 390114
    When Logout is executed
    Then Error is logged as WARN
    And Close succeeds

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
