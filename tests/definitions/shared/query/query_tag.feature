@core @python @jdbc @odbc
Feature: Query tag

  QUERY_TAG labels queries in Snowflake QUERY_HISTORY. It can be set at the
  connection level (forwarded as a session parameter at login, tagging every
  query in the session) or per-statement (tagging only that query without
  mutating session state).

  @core_e2e @python_e2e @jdbc_e2e @odbc_e2e
  Scenario: should tag queries when QUERY_TAG is set at connection level
    Given Snowflake client is logged in with connection option QUERY_TAG set to "conn_tag_e2e"
    When Query "SELECT CURRENT_QUERY_TAG()" is executed
    Then the result should contain value "conn_tag_e2e"

  @core_e2e @python_e2e @jdbc_e2e
  Scenario: should tag a single query via statement-level query tag
    Given Snowflake client is logged in
    When Query "SELECT CURRENT_QUERY_TAG()" is executed with statement-level QUERY_TAG "stmt_tag_e2e"
    Then the result should contain value "stmt_tag_e2e"

  @core_e2e @python_e2e @jdbc_e2e
  Scenario: should not leak statement-level query tag into session state
    Given Snowflake client is logged in
    When Query "SELECT CURRENT_QUERY_TAG()" is executed with statement-level QUERY_TAG "stmt_tag_e2e"
    And Query "SELECT CURRENT_QUERY_TAG()" is executed without a statement-level tag
    Then the last result should contain empty value
