@odbc
Feature: Session Logout - ODBC-specific behavior

  # ODBC implements Phase 3 (doc for: SNOW-2314152) unified behavior from the start.
  # Most Phase 3 behaviors are tested in shared scenarios with @python_not_needed @jdbc_not_needed tags.
  # This file contains only ODBC-specific implementation details.

  # ===========================================================================
  #                      ODBC-Specific Parameter Defaults
  # ===========================================================================

@odbc_e2e
  Scenario: should have server_session_keep_alive default to null
    Given Snowflake ODBC connection is created without SERVER_SESSION_KEEP_ALIVE attribute
    When Connection configuration is checked
    Then server_session_keep_alive defaults to null

  # ===========================================================================
  #                      ODBC-Specific Error Handling
  # ===========================================================================

@odbc_e2e
  Scenario: should use strict error handling strategy by default
    Given Snowflake ODBC connection is created with default parameters
    And Server will return 400 Bad Request error on logout
    When Connection is closed
    Then Error is propagated to caller
    And Error handling strategy is strict by default
