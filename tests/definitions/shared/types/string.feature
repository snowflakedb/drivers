@odbc
Feature: String datatype handling
  # Snowflake String types: VARCHAR, CHAR, CHARACTER, NCHAR, STRING, TEXT, VARCHAR2, NVARCHAR, NVARCHAR2, CHAR VARYING, NCHAR VARYING
  # All are synonymous with VARCHAR and store Unicode UTF-8 characters.
  # Maximum length: 134,217,728 characters (default 16,777,216 if unspecified)
  # Maximum storage: 128 MB (134,217,728 bytes)
  # Reference: https://docs.snowflake.com/en/sql-reference/data-types-text

  # ============================================================================
  # SIMPLE SELECTS - LITERALS (Happy path, Corner cases, Generative)
  # ============================================================================

  @odbc_e2e
  Scenario: should select hardcoded string literals
    Given Snowflake client is logged in
    When Query "SELECT 'hello' AS str1, 'Hello World' AS str2, 'Snowflake Driver Test' AS str3" is executed
    Then the result should contain:
      | str1  | str2        | str3                  |
      | hello | Hello World | Snowflake Driver Test |

  @odbc_e2e
  Scenario: should select string literals with corner case values
    # Corner cases: empty string, single character, whitespace-only, unicode characters, escape sequences
    Given Snowflake client is logged in
    When Query selecting corner case string literals is executed
    # Corner cases include:
    #   - Empty string: ''
    #   - Single character: 'X'
    #   - Whitespace only: '   '
    #   - Tab character: '\t'
    #   - Newline: '\n'
    #   - Unicode snowman: '\u26c4' (⛄)
    #   - Unicode characters: '日本語テスト' (Japanese)
    #   - Escaped single quote: '\''
    #   - Escaped backslash: '\\'
    #   - NULL value
    Then the result should contain expected corner case string values

  # ============================================================================
  # SIMPLE SELECTS - FROM TABLE (Happy path, Corner cases, Generative)
  # ============================================================================

  @odbc_e2e
  Scenario: should select hardcoded string values from table
    Given Snowflake client is logged in
    And A temporary table with VARCHAR column is created
    And The table is populated with string values
    When Query "SELECT * FROM {table}" is executed
    Then the result should contain the inserted hardcoded string values

  @odbc_e2e
  Scenario: should select corner case string values from table
    Given Snowflake client is logged in
    And A temporary table with VARCHAR column is created
    And The table is populated with corner case string values
    # Corner cases: empty string, max length string, unicode, special characters
    When Query "SELECT * FROM {table}" is executed
    Then the result should contain the inserted corner case string values

  @odbc_e2e
  Scenario: should select generative random string values from table
    Given Snowflake client is logged in
    And A random seed is initialized and logged
    And A temporary table with VARCHAR column is created
    And The table is populated with 100 randomly generated string values
    When Query "SELECT * FROM {table} ORDER BY id" is executed
    Then all returned string values should match the generated values in order

  # ============================================================================
  # SIMPLE INSERT WITH BINDING (Simple, Generative)
  # ============================================================================

  @odbc_e2e
  Scenario: should insert and select back hardcoded string values using parameter binding
    Given Snowflake client is logged in
    And A temporary table with VARCHAR column is created
    When String value 'Test binding value 日本語' is inserted using parameter binding
    And Query "SELECT * FROM {table}" is executed
    Then the result should contain the bound string value 'Test binding value 日本語'

  @odbc_e2e
  Scenario: should insert and select back generative string values using parameter binding
    Given Snowflake client is logged in
    And A random seed is initialized and logged
    And A temporary table with VARCHAR column is created
    When 100 randomly generated string values are inserted using parameter binding
    And Query "SELECT * FROM {table} ORDER BY id" is executed
    Then all returned string values should match the generated values in order

  # ============================================================================
  # MULTIPLE CHUNKS DOWNLOADING
  # ============================================================================

  @odbc_e2e
  Scenario: should download string data in multiple chunks
    # This test ensures proper handling of large result sets that span multiple chunks
    # ~10^6 values ensures data is downloaded in at least two chunks
    Given Snowflake client is logged in
    And A random seed is initialized and logged
    And A temporary table with VARCHAR column is created
    And The table is populated with 1000000 randomly generated string values
    When Query "SELECT * FROM {table} ORDER BY id" is executed
    Then there are 1000000 rows returned
    And all returned string values should match the generated values in order
