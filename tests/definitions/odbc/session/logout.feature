@odbc
Feature: Session Logout - ODBC-specific behavior

  # ODBC implements Phase 3 (doc for: SNOW-2314152) unified behavior from the start.
  # This file contains only ODBC-specific defaults and configuration.

  # ===========================================================================
  #                      ODBC Default Configuration
  # ===========================================================================

  Scenario: should use ODBC default 300 second timeout
    # ODBC historically uses 300s (DEFAULT_RETRY_TIMEOUT) for logout
    Given Snowflake ODBC connection is created with default timeout configuration
    When Connection is closed
    Then Logout timeout of 300 seconds is passed to Core
    And Logout request uses 300 second timeout

  # ===========================================================================
  #                      ODBC-Specific Parameter Defaults
  # ===========================================================================

  Scenario: should have enable_server_session_keep_alive_auto_detection default to false
    # Phase 3 (doc for: SNOW-2314152) key default.
    # ODBC implements Phase 3 from day one: auto-detection is opt-in, not default.
    Given Snowflake ODBC connection is created without ENABLE_SERVER_SESSION_KEEP_ALIVE_AUTO_DETECTION attribute
    When Connection configuration is checked
    Then enable_server_session_keep_alive_auto_detection defaults to false
    And Auto-detection is disabled by default

  Scenario: should have server_session_keep_alive default to null
    Given Snowflake ODBC connection is created without SERVER_SESSION_KEEP_ALIVE attribute
    When Connection configuration is checked
    Then server_session_keep_alive defaults to null

  # ===========================================================================
  #                      ODBC-Specific Error Handling
  # ===========================================================================

  Scenario: should use strict error handling strategy by default
    Given Snowflake ODBC connection is created with default parameters
    And Server will return 400 Bad Request error on logout
    When Connection is closed
    Then close() returns error to caller
    And Error handling strategy is strict by default
