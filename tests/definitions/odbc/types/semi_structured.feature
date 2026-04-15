@odbc
Feature: ODBC-specific semi-structured type (VARIANT/OBJECT/ARRAY) handling
  # ODBC-specific tests for VARIANT/OBJECT/ARRAY types.
  # Tests ODBC metadata reporting, type conversions, and buffer handling.
  # ODBC reports semi-structured types as SQL_VARCHAR, and column_size follows
  # the session parameter `VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT`.
  # In the controlled ODBC E2E environment, that value is 134217728.

  # =========================================================================== #
  #                              TYPE CASTING                                   #
  # =========================================================================== #

  @odbc_e2e
  Scenario: should cast semi-structured values to SQL_VARCHAR
    Given Snowflake client is logged in
    When Query "SELECT PARSE_JSON('{\"a\":1}'), ARRAY_CONSTRUCT(1,2,3), OBJECT_CONSTRUCT('key','val')" is executed
    Then All columns should report SQL_VARCHAR with column_size 134217728 and decimal_digits 0

  # =========================================================================== #
  #                    SQLColAttribute - SQL_DESC_TYPE_NAME                      #
  # =========================================================================== #

  @odbc_e2e
  Scenario: should report SQL_DESC_TYPE_NAME for semi-structured columns
    Given Snowflake client is logged in
    When Query returning VARIANT, ARRAY, and OBJECT columns is executed
    Then SQL_DESC_TYPE_NAME should report VARIANT, ARRAY, and STRUCT respectively

  # =========================================================================== #
  #                   CONVERSION TO SQL_C_CHAR - TRUNCATION                     #
  # =========================================================================== #

  @odbc_e2e
  Scenario: should truncate variant data when buffer is too short
    Given Snowflake client is logged in
    When Query returning a VARIANT value is executed
    And Attempt to get data with a buffer smaller than the JSON string
    Then The function should return SQL_SUCCESS_WITH_INFO (truncation occurred)
    And The buffer should contain a truncated null-terminated string
    And The indicator should report SQL_NO_TOTAL or the full untruncated length

  # =========================================================================== #
  #                   CONVERSION TO SQL_C_WCHAR                                 #
  # =========================================================================== #

  @odbc_e2e
  Scenario: should retrieve variant data as SQL_C_WCHAR
    Given Snowflake client is logged in
    When Query returning a VARIANT value is executed
    Then Data should be retrievable as wide character string (SQL_C_WCHAR)

  @odbc_e2e
  Scenario: should handle JSON with unicode via SQL_C_WCHAR
    Given Snowflake client is logged in
    When Query returning JSON with unicode characters is executed
    Then Data should be retrievable as wide character string with unicode preserved

  @odbc_e2e
  Scenario: should truncate variant data as SQL_C_WCHAR when buffer is too short
    Given Snowflake client is logged in
    When Query returning a VARIANT value is executed
    And Attempt to get data with a wide-char buffer smaller than the JSON string
    Then The function should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004
    And The buffer should contain a null-terminated truncated wide string
    And The indicator should report SQL_NO_TOTAL or the full untruncated byte length

  # =========================================================================== #
  #                       CONVERSION TO SQL_C_BINARY                            #
  # =========================================================================== #

  @odbc_e2e
  Scenario: should retrieve variant data as SQL_C_BINARY
    Given Snowflake client is logged in
    When Query returning a VARIANT value is executed
    Then Data should be retrievable as raw bytes (SQL_C_BINARY)

  @odbc_e2e
  Scenario: should return SQL_NULL_DATA for NULL variant as SQL_C_BINARY
    Given Snowflake client is logged in
    When Query returning a NULL VARIANT is executed
    Then Indicator should be SQL_NULL_DATA
