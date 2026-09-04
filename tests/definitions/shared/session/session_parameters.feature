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

  @odbc_e2e
  Scenario Outline: should report canonical AUTOCOMMIT values through SQLGetConnectAttr
    Given Snowflake client is logged in
    When AUTOCOMMIT is set to <value> with ALTER SESSION
    Then SQLGetConnectAttr should report <expected>

    Examples:
      | value | expected           |
      | TRUE  | SQL_AUTOCOMMIT_ON  |
      | true  | SQL_AUTOCOMMIT_ON  |
      | FALSE | SQL_AUTOCOMMIT_OFF |
      | false | SQL_AUTOCOMMIT_OFF |

  @odbc_e2e
  Scenario Outline: should reject non-boolean session parameter values
    Given Snowflake client is logged in
    When ALTER SESSION sets <parameter> to <value>
    Then the statement should fail with SQL_ERROR

    Examples:
      | parameter                       | value |
      | AUTOCOMMIT                      | 1     |
      | AUTOCOMMIT                      | '1'   |
      | AUTOCOMMIT                      | 'on'  |
      | AUTOCOMMIT                      | 'yes' |
      | ODBC_TREAT_DECIMAL_AS_INT       | 1     |
      | ODBC_TREAT_DECIMAL_AS_INT       | '1'   |
      | ODBC_TREAT_DECIMAL_AS_INT       | 'on'  |
      | ODBC_TREAT_DECIMAL_AS_INT       | 'yes' |
      | ODBC_TREAT_BIG_NUMBER_AS_STRING | 1     |
      | ODBC_TREAT_BIG_NUMBER_AS_STRING | '1'   |
      | ODBC_TREAT_BIG_NUMBER_AS_STRING | 'on'  |
      | ODBC_TREAT_BIG_NUMBER_AS_STRING | 'yes' |
