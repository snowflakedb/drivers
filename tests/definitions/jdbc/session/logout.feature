Feature: Session Logout - JDBC-specific behavior

  # ===========================================================================
  #                   Phase 2 Backward Compatibility Logic
  # ===========================================================================
  # Phase 2 (doc for: SNOW-2314152) behavior: JDBC defaults to auto-detection enabled
  # when server_session_keep_alive is null. This will migrate to Phase 3
  # behavior where null means "always logout". ODBC shows target behavior.

  Scenario: should have Phase 2 defaults that enable auto_detection
    # Phase 2 (doc for: SNOW-2314152) defaults for backward compatibility. Will change in Phase 3.
    # JDBC Phase 2 defaults: server_session_keep_alive=null, enable_server_session_keep_alive_auto_detection=true
    Given Snowflake JDBC connection is created with default parameters
    And server_session_keep_alive defaults to null
    And enable_server_session_keep_alive_auto_detection defaults to true
    When Connection connects and then closes
    Then Auto-detection is performed

  # ===========================================================================
  #                   Phase 2 Truth Table - Explicit Tests
  # ===========================================================================

  Scenario: should skip logout when server_session_keep_alive is null and auto_detection true and async queries found
    # Phase 2 (doc for: SNOW-2314152) truth table: null + true + queries found → No logout + deprecation
    Given Snowflake JDBC connection is created with server_session_keep_alive set to null
    And enable_server_session_keep_alive_auto_detection is set to true
    And Long-running async query is executed using SYSTEM$SLEEP(300)
    When Connection is closed
    Then Auto-detection finds running query
    And No logout request is sent
    And Connection close metrics are recorded in telemetry
    And Deprecation warning is logged
    And Warning mentions migration to Phase 3 compliant behavior
    And Test cleans up the running query after assertions complete

  Scenario: should send logout when server_session_keep_alive is null and auto_detection true and no async queries found
    # Phase 2 (doc for: SNOW-2314152) truth table: null + true + no queries → Send logout + deprecation
    Given Snowflake JDBC connection is created with server_session_keep_alive set to null
    And enable_server_session_keep_alive_auto_detection is set to true
    And No async queries are running
    When Connection is closed
    Then Auto-detection finds no running queries
    And Logout request is sent
    And Connection close metrics are recorded in telemetry
    And Deprecation warning is logged
    And Warning mentions migration to Phase 3 compliant behavior

  Scenario: should send logout when server_session_keep_alive is null and auto_detection false
    # Phase 2 (doc for: SNOW-2314152) truth table: null + false → Send logout (no detection), No deprecation
    Given Snowflake JDBC connection is created with server_session_keep_alive set to null
    And enable_server_session_keep_alive_auto_detection is set to false
    When Connection is closed
    Then Auto-detection is not performed
    And Logout request is sent
    And Connection close metrics are recorded in telemetry
    And No deprecation warning is emitted

  # ===========================================================================
  #                     JDBC-Specific Defaults
  # ===========================================================================

  Scenario: should have enable_server_session_keep_alive_auto_detection default to true
    # Phase 2 (doc for: SNOW-2314152) default for backward compatibility. Phase 3 defaults to false.
    Given Snowflake JDBC connection is created without enable_server_session_keep_alive_auto_detection property
    When Connection configuration is checked
    Then enable_server_session_keep_alive_auto_detection defaults to true
    And Auto-detection is enabled by default

  Scenario: should use strict error handling strategy by default
    Given Snowflake JDBC connection is created with default parameters
    And Server will return 400 Bad Request error on logout
    When Connection is closed
    Then SQLException is thrown
    And Error is propagated to caller
    And close() method throws exception
    And Error handling strategy is strict by default

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
