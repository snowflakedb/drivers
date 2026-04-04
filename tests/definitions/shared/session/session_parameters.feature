@core @python @jdbc @odbc
Feature: Session parameters via connection options

  Unrecognized connection options should be forwarded as session
  parameters in the login request, so drivers can set arbitrary
  Snowflake session parameters without explicit support.

  @core_e2e @python_e2e @jdbc_e2e @odbc_e2e
  Scenario: should forward unrecognized connection option as session parameter
    Given Snowflake client is logged in with connection option QUERY_TAG set to "session_param_e2e_test"
    When Query "SELECT CURRENT_QUERY_TAG()" is executed
    Then the result should contain value "session_param_e2e_test"
