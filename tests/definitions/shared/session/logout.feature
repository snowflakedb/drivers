@core @python @odbc @jdbc
Feature: Session Logout

  # ===========================================================================
  #                           Basic Logout Request
  # ===========================================================================

  Scenario: should send logout with default settings
    Given Snowflake client is logged in with default parameters
    When Connection is closed
    Then Logout request is sent successfully
    And Connection is closed cleanly

  Scenario: should send logout request with correct endpoint method headers and payload
    Given Snowflake client is logged in
    When Connection is closed
    Then Logout request is sent to POST /session?delete=true endpoint
    And Authorization header contains Snowflake Token with session token
    And Content-Type header is application/json
    And Accept header is application/snowflake
    And User-Agent header contains wrapper and UD version hierarchy
    And Request body is empty JSON object

  Scenario: should send logout request with default 5 second timeout
    Given Snowflake client is logged in
    When Connection is closed
    Then Logout request completes within 5 seconds

  Scenario: should send logout request with custom timeout when configured
    Given Snowflake client is logged in with custom logout timeout of 10 seconds
    When Connection is closed
    Then Logout request completes within 10 seconds

  Scenario: should not send logout when connection was never established
    Given Connection attempt failed
    When Connection is closed
    Then No logout request is sent

  # ===========================================================================
  #                   Server Session Keep Alive - Explicit Control
  # ===========================================================================

  Scenario: should not send logout when server_session_keep_alive is explicitly true
    Given Snowflake client is logged in
    And server_session_keep_alive parameter is set to true
    When Connection is closed
    Then No logout request is sent
    And All client-side resources are cleaned up

  @python_not_needed
  Scenario: should send logout when server_session_keep_alive is explicitly false
    Given Snowflake client is logged in
    And server_session_keep_alive parameter is set to false
    When Connection is closed
    Then Logout request is sent
    And Auto-detection is not performed

  Scenario: should not start async queries detection when server_session_keep_alive is explicitly set
    Given Snowflake client is logged in
    And Async query is running
    And server_session_keep_alive parameter is set to true
    When Connection is closed
    Then Async query detection is not performed
    And No logout request is sent

  # ===========================================================================
  #                          Auto-Detection Mechanics
  # ===========================================================================

  Scenario: should skip logout when auto_detection enabled and running async query detected
    Given Snowflake client is logged in
    And enable_server_session_keep_alive_auto_detection is true
    And Async query is running
    When Connection is closed
    Then Async query detection finds running query
    And No logout request is sent

  Scenario: should send logout when auto_detection enabled and no async queries detected
    Given Snowflake client is logged in
    And enable_server_session_keep_alive_auto_detection is true
    And No async queries are running
    When Connection is closed
    Then Async query detection finds no running queries
    And Logout request is sent

  Scenario: should send logout when auto_detection explicitly disabled
    Given Snowflake client is logged in
    And server_session_keep_alive is null
    And enable_server_session_keep_alive_auto_detection is explicitly set to false
    When Connection is closed
    Then Auto-detection is not performed
    And Logout request is sent

  @python_not_needed @jdbc_not_needed
  Scenario: should have enable_server_session_keep_alive_auto_detection default to false
    # Phase 3 (doc for: SNOW-2314152) default. Phase 2 drivers (Python/JDBC) default this to true for backward compatibility.
    # Parameter names follow driver convention: enable_server_session_keep_alive_auto_detection (Python/Core) or ENABLE_SERVER_SESSION_KEEP_ALIVE_AUTO_DETECTION (ODBC)
    Given Snowflake client is created without enable_server_session_keep_alive_auto_detection parameter
    When Connection is created
    Then enable_server_session_keep_alive_auto_detection defaults to false
    And Auto-detection is disabled by default

  @python_not_needed @jdbc_not_needed
  Scenario: should always send logout with Phase 3 default configuration
    # Phase 3 (doc for: SNOW-2314152) unified behavior. Target model for Python and JDBC migration.
    # Phase 3 defaults: server_session_keep_alive=null, enable_server_session_keep_alive_auto_detection=false
    Given Snowflake client is logged in with default parameters
    And server_session_keep_alive defaults to null
    And enable_server_session_keep_alive_auto_detection defaults to false
    When Connection is closed
    Then Auto-detection is not performed
    And Logout request is sent
    And Behavior is predictable and explicit

  @python_not_needed @jdbc_not_needed
  Scenario: should skip logout when auto_detection explicitly enabled with running queries in Phase 3 model
    # Phase 3 (doc for: SNOW-2314152) safety-net behavior. Auto-detection requires explicit opt-in.
    Given Snowflake client is logged in
    And server_session_keep_alive is null
    And enable_server_session_keep_alive_auto_detection is explicitly set to true
    And Long-running async query is executed using SYSTEM$SLEEP(300)
    When Connection is closed
    Then Auto-detection is performed
    And Running query is detected
    And No logout request is sent
    And Test cleans up the running query after assertions complete

  Scenario: should return true when first running async query is detected without checking remaining queries
    Given Async query registry contains multiple queries
    And First query in registry is running
    When Auto-detection checks for running queries
    Then Detection returns true immediately
    And Remaining queries are not checked

  # ===========================================================================
  #                           Async Query Registry
  # ===========================================================================

  Scenario: should register async query when asyncExec is true
    Given Snowflake client is logged in
    When Query is executed with asyncExec set to true
    Then Query ID is added to async query registry

  Scenario: should unregister async query when query completes
    Given Snowflake client is logged in
    And Async query was executed and registered
    When Query completes successfully
    Then Query ID is removed from async query registry

  # ===========================================================================
  #                          Resource Cleanup Contract
  # ===========================================================================

  Scenario: should allow process to exit cleanly when connection closed regardless of parameters
    Given Snowflake client is logged in with heartbeat enabled
    And Telemetry is active
    When Connection is closed
    Then All background threads are stopped
    And Process can exit immediately

  Scenario: should stop heartbeat on close regardless of logout result
    Given Snowflake client is logged in with heartbeat enabled
    And Logout will fail due to network error
    When Connection is closed
    Then Heartbeat is stopped

  Scenario: should flush telemetry on close regardless of logout result
    Given Snowflake client is logged in
    And Telemetry has pending events
    And Logout will fail due to network error
    When Connection is closed
    Then Telemetry is flushed

  Scenario: should clear query result cache on close regardless of logout result
    Given Snowflake client is logged in
    And Query result cache has entries
    And Logout will fail due to network error
    When Connection is closed
    Then Query result cache is cleared

  Scenario: should cleanup all tokens on close regardless of whether logout was sent
    Given Snowflake client is logged in
    And server_session_keep_alive is set to true
    When Connection is closed
    Then Session token is cleared
    And Master token is cleared
    And No logout request is sent

  Scenario: should not allow token renewal after connection is closed
    Given Snowflake client is logged in
    And Query execution has started
    When Connection is closed
    Then Token renewal is blocked
    And Any token renewal attempts fail

  Scenario: should be idempotent when close called multiple times
    Given Snowflake client is logged in
    When Connection is closed
    And Connection is closed again
    And Connection is closed a third time
    Then Only one logout request is sent
    And No errors are thrown

  # ===========================================================================
  #                      Error Handling - Strategy Configuration
  # ===========================================================================

  Scenario: should support switching between error handling strategies
    Given Snowflake client is configured with strict error handling strategy
    When Connection is closed and logout fails with 400 error
    Then Error is propagated according to strict strategy
    When New connection is configured with best-effort error handling strategy
    And Connection is closed and logout fails with 400 error
    Then Error is logged but not thrown according to best-effort strategy

  # ===========================================================================
  #                      Error Handling - Strict Strategy
  # ===========================================================================

  @python_not_needed
  Scenario: should ignore SESSION_GONE error in strict strategy
    Given Snowflake client is logged in with strict error handling
    And Server will return SESSION_GONE error 390111
    When Connection is closed
    Then Close operation succeeds without error
    And Error 390111 is treated as success

  @python_not_needed
  Scenario: should retry on transient error in strict strategy
    Given Snowflake client is logged in with strict error handling
    And Server will return 503 error on first attempt
    And Server will succeed on second attempt
    When Connection is closed
    Then Logout is retried
    And Close operation succeeds

  @python_not_needed
  Scenario: should fail close on non-retryable error in strict strategy
    Given Snowflake client is logged in with strict error handling
    And Server will return 400 Bad Request error
    When Connection is closed
    Then Close operation throws error
    And Error is surfaced to caller

  @python_not_needed
  Scenario: should attempt token renewal and retry logout when session token expired in strict strategy
    Given Snowflake client is logged in with strict error handling
    And Session token will expire before logout
    When Connection is closed
    Then Session token renewal is attempted
    And Logout is retried with new token
    And Close operation succeeds

  @python_not_needed
  Scenario: should surface reauth error when master token expired in strict strategy
    Given Snowflake client is logged in with strict error handling
    And Master token has expired
    When Connection is closed
    Then Master token expiry error 390114 is surfaced
    And Close operation throws reauth error

  @python_not_needed
  Scenario: should log WARN on final logout failure after all retries exhausted in strict strategy
    Given Snowflake client is logged in with strict error handling
    And Server will return 503 error on all attempts
    When Connection is closed
    Then All retry attempts are exhausted
    And WARN log is emitted with failure details
    And Close operation throws error

  # ===========================================================================
  #                    Error Handling - Best-Effort Strategy
  # ===========================================================================

  @jdbc_not_needed
  Scenario: should log all errors as WARN in best-effort strategy
    Given Snowflake client is logged in with best-effort error handling
    And Server will return 500 Internal Server Error
    When Connection is closed
    Then Error is logged as WARN
    And Close operation succeeds

  @jdbc_not_needed
  Scenario: should never throw exception from close in best-effort strategy
    Given Snowflake client is logged in with best-effort error handling
    And Server will return 400 Bad Request error
    When Connection is closed
    Then No exception is thrown
    And Close operation succeeds

  @jdbc_not_needed
  Scenario: should succeed close even on logout timeout in best-effort strategy
    Given Snowflake client is logged in with best-effort error handling
    And Logout will timeout after 5 seconds
    When Connection is closed
    Then Timeout is logged as WARN
    And Close operation succeeds

  @jdbc_not_needed
  Scenario: should log WARN and suppress error when master token expired in best-effort strategy
    Given Snowflake client is logged in with best-effort error handling
    And Master token has expired
    When Connection is closed
    Then Master token expiry error 390114 is logged as WARN
    And Close operation succeeds

  @jdbc_not_needed
  Scenario: should log WARN on final logout failure after all retries exhausted in best-effort strategy
    Given Snowflake client is logged in with best-effort error handling
    And Server will return 503 error on all attempts
    When Connection is closed
    Then All retry attempts are exhausted
    And WARN log is emitted with failure details
    And Close operation succeeds

  # ===========================================================================
  #                        Timeout and Retry Behavior
  # ===========================================================================

  Scenario: should timeout logout request after configured timeout
    Given Snowflake client is logged in with logout timeout of 3 seconds
    And Server will not respond to logout request
    When Connection is closed
    Then Logout request times out after 3 seconds
    And Timeout is handled according to error strategy

  Scenario: should retry logout on retryable HTTP errors
    Given Snowflake client is logged in
    And Server will return 503 Service Unavailable
    When Connection is closed
    Then Logout is retried according to retry policy
    And Exponential backoff is applied

  Scenario: should not retry logout on non-retryable errors
    Given Snowflake client is logged in
    And Server will return 400 Bad Request
    When Connection is closed
    Then No retry is attempted
    And Error is handled according to error strategy

  Scenario: should respect max retry attempts from HTTP policy
    Given Snowflake client is logged in with max 2 retry attempts
    And Server will always return 503 error
    When Connection is closed
    Then Logout is attempted at most 3 times
    And Final error is handled according to error strategy

  Scenario: should use exponential backoff for logout retries
    Given Snowflake client is logged in
    And Server will return 503 error twice then succeed
    When Connection is closed
    Then First retry waits exponential backoff duration
    And Second retry waits longer exponential backoff duration
    And Third attempt succeeds

  Scenario: should not block process exit when timeout expires
    Given Snowflake client is logged in
    And Logout will timeout
    When Connection is closed
    Then Process can exit immediately after timeout
    And No background threads remain

  # ===========================================================================
  #                        Edge Cases and Concurrency
  # ===========================================================================

  Scenario: should handle concurrent close calls safely
    Given Snowflake client is logged in
    When Connection is closed from multiple threads concurrently
    Then Only one logout request is sent
    And All close calls return successfully
    And No race conditions occur

  Scenario: should handle close during active query execution
    Given Snowflake client is logged in
    And Query is executing
    When Connection is closed
    Then Resources are cleaned up safely
    And Query execution is interrupted

  Scenario: should handle close during session token refresh
    Given Snowflake client is logged in
    And Session token refresh is in progress
    When Connection is closed
    Then Refresh operation is cancelled
    And Logout proceeds with available token

  Scenario: should handle network failure during logout
    Given Snowflake client is logged in
    And Network will fail during logout
    When Connection is closed
    Then Network error is handled according to error strategy
    And Client-side resources are cleaned up

  Scenario: should handle close with expired session token
    Given Snowflake client is logged in
    And Session token has already expired
    When Connection is closed
    Then Token renewal is attempted
    And Logout proceeds with renewed token or fails gracefully

  Scenario: should handle close when server is unreachable
    Given Snowflake client is logged in
    And Server is unreachable
    When Connection is closed
    Then Connection error is handled according to error strategy
    And Client-side resources are cleaned up
