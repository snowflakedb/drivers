@core @python @jdbc @odbc
Feature: Session parameters via connection options

  Unrecognized connection options should be forwarded as session
  parameters in the login request, so drivers can set arbitrary
  Snowflake session parameters without explicit support.

  @core_e2e @python_e2e @jdbc_e2e @odbc_e2e
  Scenario: should forward unrecognized connection option as session parameter
    Given Snowflake client is logged in with connection option TIMEZONE set to "Europe/Warsaw"
    When Query "SHOW PARAMETERS LIKE 'TIMEZONE'" is executed
    Then the session parameter value should be "Europe/Warsaw"

  @jdbc_e2e @odbc_e2e
  Scenario: should enable session keep-alive via connection string
    Given Snowflake client is logged in with connection option CLIENT_SESSION_KEEP_ALIVE set to "true"
    When Query "SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE'" is executed
    Then the session parameter value should be "true"

  @jdbc_e2e @odbc_e2e
  Scenario: should set heartbeat frequency via connection string
    Given Snowflake client is logged in with CLIENT_SESSION_KEEP_ALIVE=true and CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY=1800
    When Query "SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY'" is executed
    Then the session parameter value reflects the configured frequency
