@jdbc
Feature: Session Logout - JDBC-specific behavior

  # ===========================================================================
  #                   Phase 2 Backward Compatibility Logic
  # ===========================================================================
  # Phase 2 (doc for: SNOW-2314152) behavior: JDBC defaults to auto-detection enabled
  # when server_session_keep_alive is null. This will migrate to Phase 3
  # behavior where null means "always logout". ODBC shows target behavior.

  Scenario: should have auto_detection enabled by default with JDBC default configuration
    # Phase 2 (doc for: SNOW-2314152) behavior. Will change in Phase 3 to always logout by default.
    # JDBC defaults: server_session_keep_alive=null, enable_server_session_keep_alive_auto_detection=true
    Given Snowflake JDBC connection is created with default parameters
    And server_session_keep_alive defaults to null
    When Connection connects
    Then enable_server_session_keep_alive_auto_detection defaults to true
    And Auto-detection will be performed on close

  Scenario: should skip logout with JDBC default configuration when async query detected
    # Phase 2 (doc for: SNOW-2314152) behavior. Will migrate to Phase 3 compliant behavior where null means "always logout".
    # JDBC defaults: server_session_keep_alive=null, enable_server_session_keep_alive_auto_detection=true
    Given Snowflake JDBC connection is created with default parameters
    And server_session_keep_alive defaults to null
    And enable_server_session_keep_alive_auto_detection defaults to true
    And Long-running async query is executed using SYSTEM$SLEEP(300)
    When Connection is closed
    Then Auto-detection is performed and finds running query
    And No logout request is sent
    And Session is kept alive
    And Test cleans up the running query after assertions complete

  Scenario: should emit deprecation warning when using auto_detection fallback with null or unset param
    # Phase 2 (doc for: SNOW-2314152) behavior. Warning users of upcoming breaking change to Phase 3 compliant behavior.
    Given Snowflake JDBC connection is created with server_session_keep_alive unset or explicitly null
    And enable_server_session_keep_alive_auto_detection defaults to true
    And Long-running async query is executed using SYSTEM$SLEEP(300)
    When Connection is closed and logout is skipped due to auto-detection
    Then Deprecation warning is logged
    And Warning mentions upcoming migration to Phase 3 compliant behavior
    And Warning states that explicit configuration will be required to keep current behavior
    And Test cleans up the running query after assertions complete

  # ===========================================================================
  #                     JDBC-Specific Defaults
  # ===========================================================================

  Scenario: should have enable_server_session_keep_alive_auto_detection default to true
    # Phase 2 (doc for: SNOW-2314152) default for backward compatibility. Phase 3 defaults to false.
    Given Snowflake JDBC connection is created without enable_server_session_keep_alive_auto_detection property
    When Connection is created
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
