@odbc
Feature: ODBC DATE to SQL_C_CHAR and SQL_C_WCHAR conversions

  @odbc_e2e
  Scenario: DATE to SQL_C_CHAR
    Given Snowflake client is logged in
    When basic DATE is fetched as SQL_C_CHAR
    Then String representation matches "yyyy-mm-dd" format
    When pre-epoch DATE is fetched as SQL_C_CHAR
    Then String representation matches "yyyy-mm-dd" format
    When leap day DATE is fetched as SQL_C_CHAR
    Then String representation matches "yyyy-mm-dd" format
    When epoch DATE is fetched as SQL_C_CHAR
    Then String representation matches "yyyy-mm-dd" format
    When end of year DATE is fetched as SQL_C_CHAR
    Then String representation matches "yyyy-mm-dd" format
    When first day of year DATE is fetched as SQL_C_CHAR
    Then String representation matches "yyyy-mm-dd" format
    When leap year non-leap day DATE is fetched as SQL_C_CHAR
    Then String representation matches "yyyy-mm-dd" format
    When non-leap year Feb 28 DATE is fetched as SQL_C_CHAR
    Then String representation matches "yyyy-mm-dd" format

  @odbc_e2e
  Scenario: DATE to SQL_C_CHAR exact buffer fit
    Given Snowflake client is logged in
    When A DATE value is fetched into an 11-byte buffer
    Then SQL_SUCCESS is returned with indicator 10

  @odbc_e2e
  Scenario: DATE to SQL_C_CHAR buffer too small
    # BD#41: Old driver returns 07006 instead of 22003 for undersized date buffer
    Given Snowflake client is logged in
    When A DATE value is fetched into a buffer smaller than 11 bytes
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: DATE to SQL_C_CHAR far future
    Given Snowflake client is logged in
    When The maximum DATE value 9999-12-31 is fetched as SQL_C_CHAR
    Then String representation is "9999-12-31"

  @odbc_e2e
  Scenario: DATE to SQL_C_CHAR far past
    Given Snowflake client is logged in
    When The minimum DATE value 0001-01-01 is fetched as SQL_C_CHAR
    Then String representation is "0001-01-01"

  @odbc_e2e
  Scenario: DATE NULL to SQL_C_CHAR
    Given Snowflake client is logged in
    When A NULL DATE value is queried
    Then Indicator returns SQL_NULL_DATA

  @odbc_e2e
  Scenario: DATE to SQL_C_WCHAR
    Given Snowflake client is logged in
    When basic DATE is fetched as SQL_C_WCHAR
    Then Wide string representation matches "yyyy-mm-dd" format
    When pre-epoch DATE is fetched as SQL_C_WCHAR
    Then Wide string representation matches "yyyy-mm-dd" format
    When leap day DATE is fetched as SQL_C_WCHAR
    Then Wide string representation matches "yyyy-mm-dd" format
    When epoch DATE is fetched as SQL_C_WCHAR
    Then Wide string representation matches "yyyy-mm-dd" format

  @odbc_e2e
  Scenario: DATE to SQL_C_WCHAR exact buffer fit
    Given Snowflake client is logged in
    When A DATE value is fetched into a WCHAR buffer of exactly 11 characters
    Then SQL_SUCCESS is returned with the correct wide string

  @odbc_e2e
  Scenario: DATE to SQL_C_WCHAR buffer too small
    # BD#41: Old driver returns 07006 instead of 22003 for undersized date buffer
    Given Snowflake client is logged in
    When A DATE value is fetched into a WCHAR buffer smaller than 11 characters
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: DATE NULL to SQL_C_WCHAR
    Given Snowflake client is logged in
    When A NULL DATE value is queried
    Then Indicator returns SQL_NULL_DATA
