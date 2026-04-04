@odbc
Feature: ODBC SQLDescribeCol for TIMESTAMP types
  # Validates that SQLDescribeCol correctly describes TIMESTAMP_NTZ,
  # TIMESTAMP_LTZ, and TIMESTAMP_TZ columns without crashing.
  # This is the regression test for the EXIT_DESC bug where
  # TIMESTAMP_LTZ and TIMESTAMP_TZ were unmapped, causing odbcserver
  # to exit with code 21 (EXIT_DESC).

  @odbc_e2e
  Scenario: SQLDescribeCol for TIMESTAMP_NTZ
    Given Snowflake client is logged in
    When A TIMESTAMP_NTZ column is described via SQLDescribeCol
    Then Data type is SQL_TYPE_TIMESTAMP with expected precision and column size

  @odbc_e2e
  Scenario: SQLDescribeCol for TIMESTAMP_LTZ
    Given Snowflake client is logged in
    When A TIMESTAMP_LTZ column is described via SQLDescribeCol
    Then Data type is SQL_TYPE_TIMESTAMP with expected precision and column size

  @odbc_e2e
  Scenario: SQLDescribeCol for TIMESTAMP_TZ
    Given Snowflake client is logged in
    When A TIMESTAMP_TZ column is described via SQLDescribeCol
    Then Data type is SQL_TYPE_TIMESTAMP with expected precision and column size
