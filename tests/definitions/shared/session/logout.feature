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

  @core_int
  # TODO: Add @python_int when Python test is implemented
  Scenario: should reject queries client-side after connection is closed
    Given Snowflake client is logged in
    And Simple query SELECT 1 executes successfully
    When Connection is closed
    And Query is attempted on closed connection
    Then Query fails with connection closed error

  # TODO: Add @core_int when test is implemented
  @python_not_needed
  Scenario: should handle SESSION_GONE error when using invalidated session token
    # Tests Core handling when server returns SESSION_GONE (token already invalidated)
    Given Mock server is configured to return SESSION_GONE 390111
    And Session token is invalidated on server
    When Logout is attempted with invalidated token
    Then Client treats SESSION_GONE as successful logout
    And Close operation succeeds

  # ===========================================================================
  #                        Process Exit and Thread Management
  # ===========================================================================

  # TODO: Add @core_e2e @python_e2e when test is implemented
  Scenario: should allow process to exit cleanly when session kept alive
    # Requires: SNOW-2881763 (Heartbeat), SNOW-2912513 (Telemetry)
    Given Connection with heartbeat enabled
    And Telemetry is active
    And server_session_keep_alive is set to true
    When Connection is closed
    Then All background threads are stopped
    And Heartbeat thread is terminated
    And Telemetry thread is terminated
    And Process can exit immediately without hanging


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
