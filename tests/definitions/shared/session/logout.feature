@core @python
Feature: Session Logout

  # Core-level HTTP protocol details are in core/session/logout.feature
  # Auto-detection scenarios moved to fire-and-forget ticket (SNOW-2923705)
  # Resource cleanup (heartbeat/telemetry/QCC) scenarios delegated to respective tickets

  # ===========================================================================
  #                          Token Cleanup
  # ===========================================================================

  @core_e2e @python_e2e
  Scenario: should cleanup all tokens on close regardless of whether logout was sent
    Given Snowflake client is logged in
    And server_session_keep_alive is set to any of (true, false, None)
    When Connection is closed
    Then Session token is cleared
    And Master token is cleared

  @core_e2e @python_e2e
  Scenario: should be idempotent when close called multiple times
    Given Snowflake client is logged in
    When Connection is closed
    And Connection is closed again
    And Connection is closed a third time
    Then Only one logout request is sent
    And No errors are thrown

  # ===========================================================================
  #                    Post-Logout Session Invalidation
  # ===========================================================================

  @core_int @python_int
  Scenario: should reject queries client-side after connection is closed
    Given Snowflake client is logged in
    And Simple query SELECT 1 executes successfully
    When Connection is closed
    And Query is attempted on closed connection
    Then Query fails with connection closed error

  @core_int @python_not_needed
  Scenario: should handle SESSION_GONE error when using invalidated session token
    # Tests Core handling when server returns SESSION_GONE (token already invalidated)
    Given Mock server is configured to return SESSION_GONE 390111
    And Session token is invalidated on server
    When Logout is attempted with invalidated token
    Then Client treats SESSION_GONE as successful logout
    And Close operation succeeds

  # ===========================================================================
  #                      Error Handling - Backend Behaviors
  # ===========================================================================
  # These behaviors apply regardless of error strategy (strict or best-effort)

  @core_e2e @python_e2e
  Scenario: should ignore SESSION_GONE error 390111 regardless of strategy
    Given Snowflake client is logged in
    And Server will return SESSION_GONE error 390111
    When Connection is closed
    Then Close operation succeeds without error
    And Error 390111 is treated as success
    And Behavior is same for both strict and best-effort strategies

  @core_e2e @python_e2e
  Scenario: should retry on transient errors regardless of strategy
    Given Snowflake client is logged in
    And Server will return 503 error on first attempt
    And Server will succeed on second attempt
    When Connection is closed
    Then Logout is retried
    And Close operation succeeds
    And Behavior is same for both strict and best-effort strategies

  @core_e2e @python_not_needed
  Scenario: should attempt token renewal on 390112 regardless of strategy
    Given Snowflake client is logged in
    And Server will return SESSION_TOKEN_EXPIRED 390112
    And Token renewal is available
    When Connection is closed
    Then Session token renewal is attempted
    And Logout is retried with new token
    And Close operation succeeds
    And Behavior is same for both strict and best-effort strategies

  # ===========================================================================
  #                      Error Handling - Strict Strategy
  # ===========================================================================

  @core_e2e @python_not_needed
  Scenario: should fail close on non-retryable error in strict strategy
    Given Snowflake client is logged in with strict error handling
    And Server will return 400 Bad Request error
    When Connection is closed
    Then Close operation throws error
    And Error is surfaced to caller

  @core_e2e @python_not_needed
  Scenario: should surface reauth error when master token expired in strict strategy
    Given Snowflake client is logged in with strict error handling
    And Master token has expired error 390114
    When Connection is closed
    Then Master token expiry error 390114 is surfaced
    And Close operation throws reauth error

  @core_e2e @python_not_needed
  Scenario: should log WARN and throw error on final failure in strict strategy
    Given Snowflake client is logged in with strict error handling
    And Server will return 503 error on all attempts
    When Connection is closed
    Then All retry attempts are exhausted
    And WARN log is emitted with failure details
    And Close operation throws error

  # ===========================================================================
  #                    Error Handling - Best-Effort Strategy
  # ===========================================================================

  @core_e2e @python_e2e @jdbc_not_needed
  Scenario: should log WARN and suppress non-retryable error in best-effort strategy
    Given Snowflake client is logged in with best-effort error handling
    And Server will return 400 Bad Request error
    When Connection is closed
    Then Error is logged as WARN
    And Close operation succeeds
    And No exception is thrown

  @core_e2e @python_e2e @jdbc_not_needed
  Scenario: should log WARN and suppress master token error in best-effort strategy
    Given Snowflake client is logged in with best-effort error handling
    And Master token has expired error 390114
    When Connection is closed
    Then Master token expiry error 390114 is logged as WARN
    And Close operation succeeds

  @core_e2e @python_e2e @jdbc_not_needed
  Scenario: should log WARN and succeed on final failure in best-effort strategy
    Given Snowflake client is logged in with best-effort error handling
    And Server will return 503 error on all attempts
    When Connection is closed
    Then All retry attempts are exhausted
    And WARN log is emitted with failure details
    And Close operation succeeds

  @core_e2e @python_e2e @jdbc_not_needed
  Scenario: should succeed close even on logout timeout in best-effort strategy
    Given Snowflake client is logged in with best-effort error handling
    And Logout will timeout after 5 seconds
    When Connection is closed
    Then Timeout is logged as WARN
    And Close operation succeeds

  # ===========================================================================
  #                        Timeout and Retry Behavior
  # ===========================================================================

  @core_e2e @python_e2e
  Scenario: should respect max retry attempts from HTTP policy
    Given Snowflake client is logged in with max 2 retry attempts
    And Server will always return 503 error
    When Connection is closed
    Then Logout is attempted at most 3 times
    And Final error is handled according to error strategy

  @core_e2e @python_e2e
  Scenario: should use exponential backoff for logout retries
    Given Snowflake client is logged in
    And Server will return 503 error twice then succeed
    When Connection is closed
    Then First retry waits exponential backoff duration
    And Second retry waits longer exponential backoff duration
    And Third attempt succeeds

  # ===========================================================================
  #                        Concurrency
  # ===========================================================================

  @core_e2e @python_e2e
  Scenario: should handle concurrent close calls safely
    Given Snowflake client is logged in
    When Connection is closed from multiple threads concurrently
    Then Only one logout request is sent
    And All close calls return successfully
    And No race conditions occur
