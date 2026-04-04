@odbc
Feature: ODBC TIMESTAMP round-trip bind and fetch

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ round-trip via SQL_C_TYPE_TIMESTAMP bind and fetch
    Given Snowflake client is logged in and a temporary table with a TIMESTAMP_NTZ column exists
    When A SQL_TIMESTAMP_STRUCT value is inserted via SQLBindParameter and then fetched back
    Then The fetched SQL_TIMESTAMP_STRUCT matches the inserted value

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ round-trip via SQL_C_CHAR string bind
    Given Snowflake client is logged in and a temporary table exists
    When A timestamp string is inserted via SQL_C_CHAR and then fetched back as SQL_C_TYPE_TIMESTAMP
    Then The fetched struct matches the inserted string value

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ round-trip NULL via bind parameter
    Given Snowflake client is logged in and a temporary table exists
    When A NULL timestamp is inserted via SQL_NULL_DATA indicator
    Then The fetched value should be NULL

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ round-trip multiple rows with re-execution
    Given Snowflake client is logged in and a temporary table exists
    When Multiple rows are inserted via repeated execution with changed bound values
    Then Both rows should be retrievable with correct values

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ round-trip via SQL_C_TYPE_TIMESTAMP bind and fetch
    Given Snowflake client is logged in and a temporary table with a TIMESTAMP_LTZ column exists
    When A SQL_TIMESTAMP_STRUCT value is inserted via SQLBindParameter and then fetched back
    Then The fetched SQL_TIMESTAMP_STRUCT matches the inserted value

  @odbc_e2e
  Scenario: TIMESTAMP_TZ round-trip via SQL_C_TYPE_TIMESTAMP bind and fetch
    Given Snowflake client is logged in and a temporary table with a TIMESTAMP_TZ column exists
    When A timestamp with an explicit timezone offset is inserted and then fetched back
    Then The fetched SQL_TIMESTAMP_STRUCT contains the UTC-converted value

