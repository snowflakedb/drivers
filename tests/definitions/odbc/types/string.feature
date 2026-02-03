@odbc
Feature: ODBC-specific string datatype handling
  # ODBC-specific string tests that use SQLBindCol, SQLGetData truncation, and other ODBC-specific APIs

  @odbc_e2e
  Scenario: should select hardcoded string literals using SQLBindCol
    Given Snowflake client is logged in
    When Query "SELECT 'hello' AS str1, 'Hello World' AS str2, 'Snowflake Driver Test' AS str3" is executed
    And Columns are bound using SQLBindCol
    And SQLFetch is called
    Then the result should contain:
      | str1  | str2        | str3                  |
      | hello | Hello World | Snowflake Driver Test |

  @odbc_e2e
  Scenario: should convert UTF-16 to ASCII with 0x1a substitution when using SQL_C_CHAR
    # ODBC-specific: When reading UTF-16 data using SQL_C_CHAR target type,
    # non-ASCII characters (> 0x7F) should be replaced with 0x1a (SUB character)
    Given Snowflake client is logged in
    When Query selecting strings with non-ASCII Unicode characters is executed
    Then Japanese characters should be replaced with 0x1a (SUB) when reading as SQL_C_CHAR
    And Mixed string should have ASCII preserved and non-ASCII replaced with 0x1a
    And Emojis should all be replaced with 0x1a
    And Greek letters should be replaced with 0x1a
    And Pure ASCII string should remain unchanged
    And Combined string should have ASCII preserved and non-ASCII replaced with 0x1a

  @odbc_e2e
  Scenario: should download string data in multiple chunks using SQLBindCol
    # Generative test: Large result set using SQLBindCol for data retrieval
    # ~10^6 values ensures data is downloaded in at least two chunks
    Given Snowflake client is logged in
    And Expected row count is defined
    When Query "SELECT seq8() AS id, TO_VARCHAR(seq8()) AS str_val FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v ORDER BY 1" is executed
    And Columns are bound using SQLBindCol
    Then there are 10000 rows returned
    And all returned string values should match the generated values in order
