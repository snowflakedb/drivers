@python
Feature: Session Logout - Python-specific behavior

  # ===========================================================================
  #                   Phase 2 Backward Compatibility Logic
  # ===========================================================================
  # Phase 2 (doc for: SNOW-2314152) behavior: Python defaults to auto-detection enabled
  # when server_session_keep_alive is null. This will change in Phase 3 to
  # always logout by default. ODBC already implements Phase 3 behavior.

  Scenario: should have auto_detection enabled by default with Python default configuration
    # Phase 2 (doc for: SNOW-2314152) behavior. Will change in Phase 3 to always logout by default.
    # Python defaults: server_session_keep_alive=null, enable_server_session_keep_alive_auto_detection=true
    Given Snowflake Python client is created with default parameters
    And server_session_keep_alive defaults to null
    When Client connects
    Then enable_server_session_keep_alive_auto_detection defaults to true
    And Auto-detection will be performed on close

  Scenario: should skip logout with Python default configuration when async query detected
    # Phase 2 (doc for: SNOW-2314152) behavior. Will migrate to Phase 3 compliant behavior where null means "always logout".
    # Python defaults: server_session_keep_alive=null, enable_server_session_keep_alive_auto_detection=true
    Given Snowflake Python client is created with default parameters
    And server_session_keep_alive defaults to null
    And enable_server_session_keep_alive_auto_detection defaults to true
    And Long-running async query is executed using SYSTEM$SLEEP(300)
    When Client closes connection
    Then Auto-detection is performed and finds running query
    And No logout request is sent
    And Session is kept alive
    And Test cleans up the running query after assertions complete

  Scenario: should emit deprecation warning when using auto_detection fallback with null or unset param
    # Phase 2 (doc for: SNOW-2314152) behavior. Warning users of upcoming breaking change to Phase 3 compliant behavior.
    Given Snowflake Python client is created with server_session_keep_alive unset or explicitly null
    And enable_server_session_keep_alive_auto_detection defaults to true
    And Long-running async query is executed using SYSTEM$SLEEP(300)
    When Client closes connection and logout is skipped due to auto-detection
    Then Deprecation warning is emitted
    And Warning mentions upcoming migration to Phase 3 compliant behavior
    And Warning states that explicit configuration will be required to keep current behavior
    And Test cleans up the running query after assertions complete

  # ===========================================================================
  #                     Python-Specific Defaults
  # ===========================================================================

  Scenario: should have enable_server_session_keep_alive_auto_detection default to true
    # Phase 2 (doc for: SNOW-2314152) default for backward compatibility. Phase 3 defaults to false.
    Given Snowflake Python client is created without enable_server_session_keep_alive_auto_detection parameter
    When Connection is created
    Then enable_server_session_keep_alive_auto_detection defaults to true
    And Auto-detection is enabled by default

  Scenario: should perform auto_detection when server_session_keep_alive is explicitly false
    # Phase 2 (doc for: SNOW-2314152) behavior. In Phase 3, false will mean "force logout" without auto-detection.
    # Python Phase 2: sska=false still runs auto-detection (legacy behavior)
    Given Snowflake Python client is created with server_session_keep_alive explicitly set to false
    And enable_server_session_keep_alive_auto_detection defaults to true
    And Long-running async query is executed using SYSTEM$SLEEP(300)
    When Client closes connection
    Then Auto-detection is performed and finds running query
    And No logout request is sent
    And Deprecation warning is emitted
    And Warning mentions that false value behavior will change to force logout in Phase 3
    And Test cleans up the running query after assertions complete

  Scenario: should use best-effort error handling strategy by default
    Given Snowflake Python client is created with default parameters
    And Server will return 500 Internal Server Error on logout
    When Connection is closed
    Then Error is logged as WARN
    And close() method does not raise exception
    And Connection cleanup succeeds
    And Error handling strategy is best-effort by default

  # ===========================================================================
  #                       Auto-cleanup Deprecation
  # ===========================================================================
  # Phase 1 (doc for: SNOW-2314152) deprecation: Python still registers atexit handlers.
  # Will be disabled by default in Phase 2, removed in Phase 3.

  Scenario: should register atexit handler that calls close in legacy mode
    # Phase 1 (doc for: SNOW-2314152) deprecation. Will be disabled by default in Phase 2.
    Given Snowflake Python client is created with auto_cleanup enabled
    When Client connects
    Then atexit handler is registered
    When Process exits without explicit close
    Then atexit handler invokes close()
    And Session is logged out

  Scenario: should emit deprecation warning on first auto-cleanup run per process
    # Phase 1 (doc for: SNOW-2314152) deprecation. Prepares users for explicit close() requirement.
    Given Snowflake Python client is created with auto_cleanup enabled
    And No auto-cleanup has run yet in this process
    When Process exits without explicit close
    Then atexit handler runs
    And Deprecation warning is emitted once
    When Another connection is created and process exits
    Then No additional deprecation warning is emitted

  Scenario: should not register atexit handler when auto-cleanup explicitly disabled
    Given Snowflake Python client is created with auto_cleanup disabled
    When Client connects
    Then No atexit handler is registered
    When Process exits without explicit close
    Then No automatic close is performed

  Scenario: should emit telemetry and WARN when connection leaked at process exit
    Given Snowflake Python client is logged in
    And Connection is not explicitly closed
    When Process exit is detected
    Then Leak detection emits WARN log
    And Telemetry event is sent with leak information
    And Connection details are included for debugging
