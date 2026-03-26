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
  # VARBINARY SYNONYM
  # ============================================================================

  @odbc_e2e
  Scenario: should convert VARBINARY to SQL_C_CHAR and SQL_C_WCHAR same as BINARY
    Given Snowflake client is logged in
    When Query "SELECT X'ABCDEF'::VARBINARY" is executed
    Then SQL_C_CHAR should return uppercase hex "ABCDEF"
    And SQL_C_WCHAR should return uppercase hex u"ABCDEF"

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

  # ============================================================================
  # EXACT-FIT BUFFER
  # ============================================================================

  @odbc_e2e
  Scenario: should succeed with exact-fit buffer for SQL_C_CHAR
    Given Snowflake client is logged in
    When Query "SELECT X'ABCDEF'::BINARY" is executed (3 bytes -> hex "ABCDEF" = 6 chars)
    Then SQL_C_CHAR with buffer = 7 (6 hex chars + null) should return SQL_SUCCESS

  @odbc_e2e
  Scenario: should succeed with exact-fit buffer for SQL_C_WCHAR
    Given Snowflake client is logged in
    When Query "SELECT X'CAFE'::BINARY" is executed (2 bytes -> hex "CAFE" = 4 wide chars)
    Then SQL_C_WCHAR with buffer = 5 * sizeof(SQLWCHAR) (4 chars + null) should return SQL_SUCCESS

  # ============================================================================
  # TRUNCATION EDGE CASES
  # ============================================================================

  @odbc_e2e
  Scenario: should truncate binary hex with one-byte-short buffer for SQL_C_CHAR
    # BD#30: Old driver writes only complete hex pairs on even BufferLength
    # ("ABCD\0", last byte unused). New driver fills all available space ("ABCDE\0").
    Given Snowflake client is logged in
    When Query "SELECT X'ABCDEF'::BINARY" is executed (3 bytes -> hex "ABCDEF" = 6 chars, needs 7)
    Then SQL_C_CHAR with buffer = 6 (one short of the 7 needed) should return 01004
    And Truncated output should be null-terminated with valid hex prefix

  @odbc_e2e
  Scenario: should handle buffer size 1 for SQL_C_CHAR with binary
    Given Snowflake client is logged in
    When Query "SELECT X'ABCDEF'::BINARY" is executed
    Then SQL_C_CHAR with buffer = 1 should return 01004 with indicator = 6 and only null terminator

  # ============================================================================
  # 3-CHUNK RETRIEVAL FOR SQL_C_CHAR
  # ============================================================================

  @odbc_e2e
  Scenario: should retrieve binary hex in three chunks via SQLGetData with SQL_C_CHAR
    Given Snowflake client is logged in
    When Query selecting a 6-byte binary value (hex = 12 chars) is executed
    Then First SQLGetData call should return first 4 hex chars with 01004
    And Second SQLGetData call should return next 4 hex chars with 01004
    And Third SQLGetData call should return final 4 hex chars with SQL_SUCCESS
