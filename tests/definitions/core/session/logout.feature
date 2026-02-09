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

  Scenario: should ignore SESSION_GONE 390111 with strict strategy injected
    Given Core logout function called with strict strategy
    And Mock server returns SESSION_GONE 390111
    When Logout is executed
    Then Close succeeds
    And Error is ignored

  Scenario: should ignore SESSION_GONE 390111 with best-effort strategy injected
    Given Core logout function called with best-effort strategy
    And Mock server returns SESSION_GONE 390111
    When Logout is executed
    Then Close succeeds
    And Error is ignored

  Scenario: should retry transient errors with strict strategy injected
    Given Core logout function called with strict strategy
    And Mock server returns 503 on attempt 1
    And Mock server returns 200 on attempt 2
    When Logout is executed
    Then Logout is retried
    And Close succeeds

  Scenario: should retry transient errors with best-effort strategy injected
    Given Core logout function called with best-effort strategy
    And Mock server returns 503 on attempt 1
    And Mock server returns 200 on attempt 2
    When Logout is executed
    Then Logout is retried
    And Close succeeds

  Scenario: should throw on non-retryable error with strict strategy injected
    Given Core logout function called with strict strategy
    And Mock server returns 400 Bad Request
    When Logout is executed
    Then Close throws error
    And Error is surfaced to caller

  Scenario: should log and succeed on non-retryable error with best-effort strategy injected
    Given Core logout function called with best-effort strategy
    And Mock server returns 400 Bad Request
    When Logout is executed
    Then Error is logged as WARN
    And Close succeeds without throwing

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

  Scenario: should handle final retry failure with strict strategy injected
    Given Core logout function called with strict strategy
    And Mock server returns 503 on all attempts
    When Logout is executed with max 3 attempts
    Then All retries exhausted
    And WARN log emitted
    And Close throws error

  Scenario: should handle final retry failure with best-effort strategy injected
    Given Core logout function called with best-effort strategy
    And Mock server returns 503 on all attempts
    When Logout is executed with max 3 attempts
    Then All retries exhausted
    And WARN log emitted
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
