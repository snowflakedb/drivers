@jdbc
Feature: Session Logout - JDBC-specific behavior

  # ===========================================================================
  #                   JDBC Default Configuration
  # ===========================================================================

  Scenario: should use JDBC default 300 second timeout
    # JDBC historically uses 300s (loginTimeout) for logout
    Given Snowflake JDBC connection is created with default timeout configuration
    When Connection is closed
    Then Logout timeout of 300 seconds is passed to Core
    And Logout request uses 300 second timeout

  Scenario: should use strict error handling strategy by default
    Given Snowflake JDBC connection is created with default parameters
    And Server will return 400 Bad Request error on logout
    When Connection is closed
    Then close() throws SQLException
    And Error handling strategy is strict by default

  # ===========================================================================
  #                   Session Lifecycle Parameters
  # ===========================================================================
  # Phase 2 (doc for: SNOW-2314152): UD mirrors old JDBC behavior (auto-detection enabled by default).
  # Phase 3: All drivers converge on unified model (auto-detection disabled by default).

  Scenario: should have auto_detection enabled and server_session_keep_alive null by default
    # Phase 2 (doc for: SNOW-2314152): JDBC defaults mirror old driver behavior
    # server_session_keep_alive=null, enable_server_session_keep_alive_auto_detection=true
    # Using these defaults emits deprecation warning because auto-detection
    # will be switched off by default in the future (Phase 3)
    Given Snowflake JDBC connection is created with default parameters
    When Connection configuration is checked
    Then server_session_keep_alive defaults to null
    And enable_server_session_keep_alive_auto_detection defaults to true
    When Connection is closed
    Then Deprecation warning is logged
    And Warning states that auto_detection will be disabled by default in the future

  Scenario: should forward server_session_keep_alive true to Core when explicitly set
    # Phase 2 (doc for: SNOW-2314152) truth table: true + any → No logout, No deprecation
    # Full logout-skip behavior tested in Core; this freezes the JDBC default to prevent BCRs
    Given Snowflake JDBC connection is created with server_session_keep_alive set to true
    When Connection is closed
    Then server_session_keep_alive true is passed to Core

  Scenario: should always send logout when server_session_keep_alive is false
    # E2E sanity check: Verifies JDBC wrapper + Core integration works end-to-end
    # Phase 2 (doc for: SNOW-2314152) truth table: false + any → Always logout, ignore auto-detect
    Given Snowflake JDBC connection is created with server_session_keep_alive set to false
    And Long-running async query is executed using SYSTEM$SLEEP(10)
    When Connection is closed
    Then Auto-detection is not performed
    And Logout request is sent
    And No deprecation warning is emitted
    And Test cleans up the running query after assertions complete

  Scenario: should pass correct parameters when server_session_keep_alive is null and auto_detection true
    # Tests wrapper parameter passing (not E2E HTTP behavior - covered by Core tests)
    # Phase 2 (doc for: SNOW-2314152) truth table: null + true → parameters passed to Core
    Given Snowflake JDBC connection is created with server_session_keep_alive set to null
    And enable_server_session_keep_alive_auto_detection is set to true
    When Connection is closed
    Then server_session_keep_alive null is passed to Core
    And enable_server_session_keep_alive_auto_detection true is passed to Core
    And Deprecation warning is logged
    And Warning mentions migration to Phase 3 compliant behavior

  Scenario: should pass correct parameters when server_session_keep_alive is null and auto_detection false
    # Tests wrapper parameter passing (not E2E HTTP behavior - covered by Core tests)
    # Phase 2 (doc for: SNOW-2314152) truth table: null + false → parameters passed to Core
    Given Snowflake JDBC connection is created with server_session_keep_alive set to null
    And enable_server_session_keep_alive_auto_detection is set to false
    When Connection is closed
    Then server_session_keep_alive null is passed to Core
    And enable_server_session_keep_alive_auto_detection false is passed to Core
    And No deprecation warning is emitted

  # ===========================================================================
  #                         Resource Management
  # ===========================================================================

  Scenario: should invalidate all active statements on close regardless of logout result
    Given Snowflake JDBC connection is logged in
    And Multiple prepared statements are created
    And Statement is executing
    And Logout will fail due to network error
    When Connection is closed
    Then All statements are invalidated
    And Statements cannot be reused
    And Statement.isClosed() returns true
    And Subsequent statement operations throw SQLException
