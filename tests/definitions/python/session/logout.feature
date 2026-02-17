@python
Feature: Session Logout - Python-specific behavior

  # ===========================================================================
  #                   Python Default Configuration
  # ===========================================================================

  Scenario: should use Python default 5 second timeout
    # Python historically uses 5s timeout for logout
    Given Snowflake Python client is created with default timeout configuration
    When Connection is closed
    Then Logout timeout of 5 seconds is passed to Core
    And Logout request completes within 5 seconds

  # ===========================================================================
  #                   Session Lifecycle Parameters
  # ===========================================================================
  # Phase 2 (doc for: SNOW-2314152) behavior: Python defaults to auto-detection enabled
  # when server_session_keep_alive is null. This will change in Phase 3 to
  # always logout by default. ODBC already implements Phase 3 behavior.
  # Auto-detection logic scenarios moved to fire-and-forget ticket (SNOW-2923705)

  Scenario: should have auto_detection enabled and server_session_keep_alive null by default
    # Phase 2 (doc for: SNOW-2314152) defaults for backward compatibility. Will change in Phase 3.
    # Python Phase 2 defaults: server_session_keep_alive=null (actually False in old driver),
    # enable_server_session_keep_alive_auto_detection=true
    # NOTE: Per design doc, Python defaults do NOT emit deprecation warning.
    # Deprecation only happens when server_session_keep_alive is explicitly set to False.
    Given Snowflake Python client is created with default parameters
    When Connection configuration is checked
    Then server_session_keep_alive defaults to null
    And enable_server_session_keep_alive_auto_detection defaults to true
    And No deprecation warning is emitted for default configuration

  # ===========================================================================
  #                   Parameter Passing Verification
  # ===========================================================================

  Scenario: should pass correct parameters when server_session_keep_alive is none and auto_detection true
    # Tests wrapper parameter passing (not E2E HTTP behavior - covered by Core tests)
    # Phase 2 (doc for: SNOW-2314152) truth table: None + True → parameters passed to Core
    Given Snowflake Python client is created with server_session_keep_alive set to none
    And enable_server_session_keep_alive_auto_detection is set to true
    When Client closes connection
    Then server_session_keep_alive none is passed to Core
    And enable_server_session_keep_alive_auto_detection true is passed to Core
    And No deprecation warning is emitted

  Scenario: should send logout when server_session_keep_alive is none and auto_detection false
    # E2E sanity check: Verifies Python wrapper + Core integration works end-to-end
    # Phase 2 (doc for: SNOW-2314152) truth table: None + False → Send logout (no detection), No deprecation
    Given Snowflake Python client is created with server_session_keep_alive set to none
    And enable_server_session_keep_alive_auto_detection is set to false
    When Client closes connection
    Then Auto-detection is not performed
    And Logout request is sent
    And Connection close metrics are recorded in telemetry
    And No deprecation warning is emitted

  Scenario: should pass correct parameters when server_session_keep_alive is false
    # Tests wrapper parameter passing (not E2E HTTP behavior - covered by Core tests)
    # Phase 2: False (explicit) always emits deprecation warning
    Given Snowflake Python client is created with server_session_keep_alive set to false
    When Client closes connection
    Then server_session_keep_alive false is passed to Core
    And Deprecation warning is emitted
    And Warning mentions that false will force logout in Phase 3

  # ===========================================================================
  #                     Wrapper Defaults
  # ===========================================================================

  Scenario Outline: should skip logout when server_session_keep_alive is true regardless of auto_detection
    # Phase 2 truth table: True + any + any → No logout, No deprecation
    # Verifies Python correctly passes true to Core
    Given Snowflake Python client is created with server_session_keep_alive set to true
    And enable_server_session_keep_alive_auto_detection is set to <auto_detection>
    When Connection is closed
    Then No logout request is sent
    And server_session_keep_alive true is passed to Core
    And No deprecation warning is emitted

    Examples:
      | auto_detection |
      | true           |
      | false          |

  Scenario: should have enable_server_session_keep_alive_auto_detection default to true
    # Phase 2 (doc for: SNOW-2314152) default for backward compatibility. Phase 3 defaults to false.
    Given Snowflake Python client is created without enable_server_session_keep_alive_auto_detection parameter
    When Connection configuration is checked
    Then enable_server_session_keep_alive_auto_detection defaults to true
    And Auto-detection is enabled by default

  Scenario: should use best-effort error handling strategy by default
    Given Snowflake Python client is created with default parameters
    And Server will return 500 Internal Server Error on logout
    When Connection is closed
    Then Error is logged as WARN
    And close() method does not raise exception
    And Connection cleanup succeeds
    And Error handling strategy is best-effort by default

  # ===========================================================================
  #                       retry Parameter Support
  # ===========================================================================
  # Old Python driver: close(retry: bool = True) parameter (line 1182)
  # Tests observable retry behavior by introducing transient failures

  Scenario: should retry logout on transient failure when close called with default retry
    # Old Python driver: close(retry=True) is default (line 1182)
    Given Snowflake Python client is logged in
    And Server will return 503 on first logout attempt then succeed
    When close() is called with default parameters
    Then Logout succeeds after retry
    And Two logout requests were sent to server

  Scenario: should not retry logout on transient failure when close called with retry false
    # Old Python driver: close(retry=False) disables retries (line 1182)
    # Used by atexit handler (line 2390)
    Given Snowflake Python client is logged in
    And Server will return 503 on first logout attempt then succeed
    When close(retry=False) is called
    Then Logout is not retried
    And Only one logout request was sent to server
    And Error is handled according to best-effort strategy

  # ===========================================================================
  #                       Auto-cleanup Deprecation
  # ===========================================================================
  # Phase 1 (doc for: SNOW-2314152) deprecation: Python still registers atexit handlers.
  # Will be disabled by default in Phase 2, removed in Phase 3.

  Scenario: should unregister atexit handler when close called explicitly
    # Old Python driver: line 1185 - prevents double-close
    Given Snowflake Python client is created with auto_cleanup enabled
    And atexit handler is registered at connection init
    When close() is called explicitly
    Then atexit handler is unregistered
    And Subsequent process exit will not trigger second close

  Scenario: should call close with retry false from atexit handler
    # Old Python driver: atexit calls close(retry=False) (line 2390)
    # Phase 1 (doc for: SNOW-2314152) deprecation. Will be disabled by default in Phase 2.
    Given Snowflake Python client is created with auto_cleanup enabled
    And Connection was not closed explicitly
    When Process exits
    Then atexit handler calls close(retry=False)
    And No retries are attempted during atexit close
    And All exceptions during atexit close are suppressed
    And Session is logged out if conditions allow

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
