@odbc
Feature: ODBC binary to character type conversions
  # Tests converting Snowflake BINARY type to character ODBC C types:
  # SQL_C_CHAR, SQL_C_WCHAR
  # Binary values are converted to uppercase hexadecimal string representation

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Binary to SQL_C_CHAR
  # ============================================================================

  @odbc_e2e
  Scenario: should convert binary to SQL_C_CHAR returning uppercase hex
    Given Snowflake client is logged in
    When Query "SELECT X'48656C6C6F'::BINARY, X'ABCDEF'::BINARY" is executed
    Then SQL_C_CHAR should return "48656C6C6F" and "ABCDEF" in uppercase

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Binary to SQL_C_WCHAR
  # ============================================================================

  @odbc_e2e
  Scenario: should convert binary to SQL_C_WCHAR returning uppercase hex
    Given Snowflake client is logged in
    When Query "SELECT X'ABCDEF'::BINARY" is executed
    Then SQL_C_WCHAR should return "ABCDEF" as wide string

  # ============================================================================
  # SQLBindCol with SQL_C_CHAR and SQL_C_WCHAR
  # ============================================================================

  @odbc_e2e
  Scenario: should retrieve binary via SQLBindCol with SQL_C_CHAR
    Given Snowflake client is logged in
    When Query "SELECT X'ABCDEF'::BINARY" is executed with SQLBindCol using SQL_C_CHAR
    Then Bound buffer should contain uppercase hex string "ABCDEF"

  @odbc_e2e
  Scenario: should retrieve binary via SQLBindCol with SQL_C_WCHAR
    Given Snowflake client is logged in
    When Query "SELECT X'ABCDEF'::BINARY" is executed with SQLBindCol using SQL_C_WCHAR
    Then Bound wide buffer should contain uppercase hex string "ABCDEF"

  # ============================================================================
  # EMPTY BINARY CONVERSION
  # ============================================================================

  @odbc_e2e
  Scenario: should convert empty binary to SQL_C_CHAR returning empty string
    Given Snowflake client is logged in
    When Query "SELECT X''::BINARY" is executed
    Then SQL_C_CHAR should return empty string with null terminator and indicator 0

  @odbc_e2e
  Scenario: should convert empty binary to SQL_C_WCHAR returning empty wide string
    Given Snowflake client is logged in
    When Query "SELECT X''::BINARY" is executed
    Then SQL_C_WCHAR should return empty wide string with null terminator and indicator 0

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: should handle NULL binary with character C types
    Given Snowflake client is logged in
    When Query "SELECT NULL::BINARY" is executed
    Then SQL_C_CHAR should return SQL_NULL_DATA indicator
    And SQL_C_WCHAR should return SQL_NULL_DATA indicator

  # ============================================================================
  # CHUNKED SQLGetData FOR LARGE HEX OUTPUT
  # ============================================================================

  @odbc_e2e
  Scenario: should retrieve large binary as hex in chunks via SQLGetData with SQL_C_CHAR
    Given Snowflake client is logged in
    When Query selecting a binary value whose hex representation exceeds buffer size is executed
    Then First SQLGetData call with SQL_C_CHAR should return SQL_SUCCESS_WITH_INFO with partial hex
    And Second SQLGetData call should return SQL_SUCCESS with remaining hex

  @odbc_e2e
  Scenario: should retrieve large binary as hex in chunks via SQLGetData with SQL_C_WCHAR
    Given Snowflake client is logged in
    When Query selecting a binary value whose hex representation exceeds wide buffer size is executed
    Then First SQLGetData call with SQL_C_WCHAR should return SQL_SUCCESS_WITH_INFO with truncated data
    And Second SQLGetData call with SQL_C_WCHAR should return SQL_SUCCESS with remaining wide hex
