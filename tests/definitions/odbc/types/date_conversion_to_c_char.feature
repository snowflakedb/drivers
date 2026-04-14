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
  Scenario: DATE to SQL_C_CHAR truncation
    # BD#41: Old driver returns error instead of 01004 truncation
    Given Snowflake client is logged in
    When A DATE value is fetched into a buffer smaller than 11 bytes
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  @odbc_e2e
  Scenario: DATE to SQL_C_CHAR chunked retrieval
    # BD#41: Old driver returns error instead of 01004 truncation
    Given Snowflake client is logged in
    When A DATE value is fetched via two sequential SQLGetData calls with a 6-byte buffer
    Then The first call returns partial data with 01004 and the second call returns the remainder

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
  Scenario: DATE to SQL_C_WCHAR truncation
    # BD#41: Old driver returns error instead of 01004 truncation
    Given Snowflake client is logged in
    When A DATE value is fetched into a WCHAR buffer smaller than the date string
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  @odbc_e2e
  Scenario: DATE to SQL_C_WCHAR chunked retrieval
    # BD#39: Old driver returns error instead of 01004 truncation
    Given Snowflake client is logged in
    When A DATE value is fetched via two sequential SQLGetData calls with a 6-character WCHAR buffer
    Then The first call returns partial data with 01004 and the second call returns the remainder

  @odbc_e2e
  Scenario: DATE NULL to SQL_C_WCHAR
    Given Snowflake client is logged in
    When A NULL DATE value is queried
    Then Indicator returns SQL_NULL_DATA
