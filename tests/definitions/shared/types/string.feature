@odbc @python
Feature: String datatype handling
  # Snowflake String types: VARCHAR, CHAR, CHARACTER, NCHAR, STRING, TEXT, VARCHAR2, NVARCHAR, NVARCHAR2, CHAR VARYING, NCHAR VARYING
  # All are synonymous with VARCHAR and store Unicode UTF-8 characters.
  # Maximum length: 134,217,728 characters (default 16,777,216 if unspecified)
  # Maximum storage: 128 MB (134,217,728 bytes)
  # Reference: https://docs.snowflake.com/en/sql-reference/data-types-text

  # ============================================================================
  # TYPE CASTING
  # ============================================================================

  @python_e2e
  Scenario: should cast string values to appropriate type for string and synonyms
    # Python: Values should be cast to 'str' type
    Given Snowflake client is logged in
    When Query "SELECT 'hello'::<type>, 'Hello World'::<type>, '日本語テスト'::<type>" is executed
    Then All values should be returned as appropriate type

  # ============================================================================
  # SIMPLE SELECTS - LITERALS (Happy path, Corner cases)
  # ============================================================================

  @odbc_e2e @python_e2e
  Scenario: should select hardcoded string literals
    Given Snowflake client is logged in
    When Query "SELECT 'hello' AS str1, 'Hello World' AS str2, 'Snowflake Driver Test' AS str3" is executed
    Then the result should contain:
      | str1  | str2        | str3                  |
      | hello | Hello World | Snowflake Driver Test |

  @odbc_e2e @python_e2e
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
    #   - Combined character: 'y̆es' (character with combining diacritical mark)
    #   - Surrogate pair: '\U0001D11E' (𝄞 musical G clef)
    Then the result should contain expected corner case string values

  # ============================================================================
  # SIMPLE SELECTS - FROM TABLE (Happy path, Corner cases)
  # ============================================================================

  @odbc_e2e @python_e2e
  Scenario: should select hardcoded string values from table
    Given Snowflake client is logged in
    And A temporary table with VARCHAR column is created
    And The table is populated with string values
    When Query "SELECT * FROM {table}" is executed
    Then the result should contain the inserted hardcoded string values

  @odbc_e2e @python_e2e
  Scenario: should select corner case string values from table
    Given Snowflake client is logged in
    And A temporary table with VARCHAR column is created
    And The table is populated with corner case string values
    # Corner cases (same as literal scenario):
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
    #   - Combined character: 'y̆es' (character with combining diacritical mark)
    #   - Surrogate pair: '\U0001D11E' (𝄞 musical G clef)
    When Query "SELECT * FROM {table}" is executed
    Then the result should contain the inserted corner case string values

  # ============================================================================
  # BINDING TESTS
  # ============================================================================

  @odbc_e2e @python_e2e
  Scenario: should insert and select back hardcoded string values using parameter binding
    Given Snowflake client is logged in
    And A temporary table with VARCHAR column is created
    When String value 'Test binding value 日本語' is inserted using parameter binding
    And Query "SELECT * FROM {table}" is executed
    Then the result should contain the bound string value 'Test binding value 日本語'

  
  @odbc_e2e @python_e2e
  Scenario: should select string literals using parameter binding
    # SELECT binding test: Uses SELECT ?::VARCHAR to bind string values
    Given Snowflake client is logged in
    When Query "SELECT ?::VARCHAR, ?::VARCHAR, ?::VARCHAR" is executed with bound string values ['hello', 'Hello World', '日本語テスト']
    Then the result should contain:
      | col1  | col2        | col3       |
      | hello | Hello World | 日本語テスト |

  @odbc_e2e @python_e2e
  Scenario: should select corner case string values using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::VARCHAR" is executed with each corner case string value bound
    # Corner cases (same as literal scenario):
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
    #   - Combined character: 'y̆es' (character with combining diacritical mark)
    #   - Surrogate pair: '\U0001D11E' (𝄞 musical G clef)
    Then the result should match the bound corner case value


  # ============================================================================
  # MULTIPLE CHUNKS DOWNLOADING
  # ============================================================================

  @odbc_e2e @python_e2e
  Scenario: should download string data in multiple chunks
    # This test ensures proper handling of large result sets that span multiple chunks
    # ~10000 values ensures data is downloaded in at least two chunks
    Given Snowflake client is logged in
    When Query "SELECT seq8() AS id, TO_VARCHAR(seq8()) AS str_val FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v ORDER BY id" is executed
    Then there are 10000 rows returned
    And all returned string values should match the generated values in order
