Feature: Session Logout

  # Core-level HTTP protocol details are in core/session/logout.feature
  # Auto-detection scenarios moved to fire-and-forget ticket (SNOW-2923705)
  # Resource cleanup (heartbeat/telemetry/QCC) scenarios delegated to respective tickets

  # ===========================================================================
  #                          Token Cleanup
  # ===========================================================================

  Scenario Outline: should cleanup all tokens on close regardless of whether logout was sent
    # Tests that tokens are cleared regardless of logout decision
    Given Snowflake client is logged in
    And server_session_keep_alive is set to <server_session_keep_alive>
    When Connection is closed
    Then Session token in Connection.tokens is null
    And Master token in Connection.tokens is null

    Examples:
      | server_session_keep_alive |
      | False                     |
      | True                      |
      | None                      |


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

  Scenario: should reject queries client-side after connection is closed
    Given Snowflake client is logged in
    And Simple query SELECT 1 executes successfully
    When Connection is closed
    And Query is attempted on closed connection
    Then the query fails with a connection-closed error

  # ===========================================================================
  #                        Process Exit and Thread Management
  # ===========================================================================

  Scenario: should allow process to exit cleanly when session kept alive
    # Requires: SNOW-2881763 (Heartbeat), SNOW-2912513 (Telemetry)
    Given Connection with heartbeat enabled
    And Telemetry cache is active
    And server_session_keep_alive is set to true
    When Connection is closed
    Then Heartbeat is stopped
    And Telemetry cache is flushed
    And Process can exit immediately without hanging


  # ===========================================================================
  #                        Concurrency
  # ===========================================================================

  Scenario: should handle concurrent close calls safely
    Given Snowflake client is logged in
    When Connection is closed from multiple threads concurrently
    Then Only one logout request is sent
    And All close calls return successfully
