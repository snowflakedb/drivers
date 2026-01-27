@python
Feature: Session Logout - Python-specific behavior

  # ===========================================================================
  #                   Phase 2 Backward Compatibility Logic
  # ===========================================================================
  # Phase 2 (doc for: SNOW-2314152) behavior: Python defaults to auto-detection enabled
  # when server_session_keep_alive is null. This will change in Phase 3 to
  # always logout by default. ODBC already implements Phase 3 behavior.

  Scenario: should have Phase 2 defaults that enable auto_detection
    # Phase 2 (doc for: SNOW-2314152) defaults for backward compatibility. Will change in Phase 3.
    # Python Phase 2 defaults: server_session_keep_alive=null, enable_server_session_keep_alive_auto_detection=true
    Given Snowflake Python client is created with default parameters
    And server_session_keep_alive defaults to null
    And enable_server_session_keep_alive_auto_detection defaults to true
    When Client connects and then closes
    Then Auto-detection is performed

  # ===========================================================================
  #                   Phase 2 Truth Table - Explicit Tests
  # ===========================================================================

  Scenario: should skip logout when server_session_keep_alive is null and auto_detection true and async queries found
    # Phase 2 (doc for: SNOW-2314152) truth table: null + true + queries found → No logout, No deprecation
    Given Snowflake Python client is created with server_session_keep_alive set to null
    And enable_server_session_keep_alive_auto_detection is set to true
    And Long-running async query is executed using SYSTEM$SLEEP(300)
    When Client closes connection
    Then Auto-detection finds running query
    And No logout request is sent
    And Connection close metrics are recorded in telemetry
    And No deprecation warning is emitted
    And Test cleans up the running query after assertions complete

  Scenario: should send logout when server_session_keep_alive is null and auto_detection true and no async queries found
    # Phase 2 (doc for: SNOW-2314152) truth table: null + true + no queries → Send logout, No deprecation
    Given Snowflake Python client is created with server_session_keep_alive set to null
    And enable_server_session_keep_alive_auto_detection is set to true
    And No async queries are running
    When Client closes connection
    Then Auto-detection finds no running queries
    And Logout request is sent
    And Connection close metrics are recorded in telemetry
    And No deprecation warning is emitted

  Scenario: should send logout when server_session_keep_alive is null and auto_detection false
    # Phase 2 (doc for: SNOW-2314152) truth table: null + false → Send logout (no detection)
    Given Snowflake Python client is created with server_session_keep_alive set to null
    And enable_server_session_keep_alive_auto_detection is set to false
    When Client closes connection
    Then Auto-detection is not performed
    And Logout request is sent
    And Connection close metrics are recorded in telemetry
    And No deprecation warning is emitted

  Scenario: should skip logout when server_session_keep_alive is false and auto_detection true and async queries found
    # Phase 2 (doc for: SNOW-2314152) truth table: false + true + queries found → No logout + deprecation
    # Legacy Python behavior: false still allows auto-detection to run
    Given Snowflake Python client is created with server_session_keep_alive set to false
    And enable_server_session_keep_alive_auto_detection is set to true
    And Long-running async query is executed using SYSTEM$SLEEP(300)
    When Client closes connection
    Then Auto-detection finds running query
    And No logout request is sent
    And Connection close metrics are recorded in telemetry
    And Deprecation warning is emitted
    And Warning mentions that false will force logout in Phase 3
    And Test cleans up the running query after assertions complete

  Scenario: should skip logout when server_session_keep_alive is false and auto_detection true and no async queries found
    # Phase 2 (doc for: SNOW-2314152) truth table: false + true + no queries → No logout + deprecation
    # Legacy Python behavior: false means "don't logout" rather than "force logout"
    Given Snowflake Python client is created with server_session_keep_alive set to false
    And enable_server_session_keep_alive_auto_detection is set to true
    And No async queries are running
    When Client closes connection
    Then Auto-detection finds no running queries
    And No logout request is sent
    And Connection close metrics are recorded in telemetry
    And Deprecation warning is emitted
    And Warning mentions that false will force logout in Phase 3

  Scenario: should skip logout when server_session_keep_alive is false and auto_detection false
    # Phase 2 (doc for: SNOW-2314152) truth table: false + false → No logout + deprecation
    # Legacy Python behavior: false means "keep session alive"
    Given Snowflake Python client is created with server_session_keep_alive set to false
    And enable_server_session_keep_alive_auto_detection is set to false
    When Client closes connection
    Then Auto-detection is not performed
    And No logout request is sent
    And Connection close metrics are recorded in telemetry
    And Deprecation warning is emitted
    And Warning mentions that false will force logout in Phase 3

  # ===========================================================================
  #                     Python-Specific Defaults
  # ===========================================================================

  Scenario: should have enable_server_session_keep_alive_auto_detection default to true
    # Phase 2 (doc for: SNOW-2314152) default for backward compatibility. Phase 3 defaults to false.
    Given Snowflake Python client is created without enable_server_session_keep_alive_auto_detection parameter
    When Connection configuration is checked
    Then enable_server_session_keep_alive_auto_detection defaults to true
    And Auto-detection is enabled by default

  Scenario: should perform auto_detection when server_session_keep_alive is explicitly false
    # Phase 2 (doc for: SNOW-2314152) behavior. In Phase 3, false will mean "force logout" without auto-detection.
    # Python Phase 2: server_session_keep_alive=false still runs auto-detection (legacy behavior)
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
