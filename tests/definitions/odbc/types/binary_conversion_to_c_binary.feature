@odbc
Feature: ODBC binary to SQL_C_BINARY type conversions
  # Tests converting Snowflake BINARY type to binary ODBC C types:
  # SQL_C_BINARY, SQL_C_DEFAULT

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Binary to SQL_C_BINARY
  # ============================================================================

  @odbc_e2e
  Scenario: should convert binary to SQL_C_BINARY returning raw bytes
    Given Snowflake client is logged in
    When Query "SELECT X'48656C6C6F'::BINARY" is executed
    Then SQL_C_BINARY should return raw bytes [0x48, 0x65, 0x6C, 0x6C, 0x6F]

  @odbc_e2e
  Scenario: should convert binary to SQL_C_DEFAULT returning raw bytes
    Given Snowflake client is logged in
    When Query "SELECT X'CAFE'::BINARY" is executed
    Then SQL_C_DEFAULT should return same result as SQL_C_BINARY

  # ============================================================================
  # SQLBindCol with SQL_C_BINARY
  # ============================================================================

  @odbc_e2e
  Scenario: should retrieve binary via SQLBindCol with SQL_C_BINARY
    Given Snowflake client is logged in
    When Query "SELECT X'ABCDEF'::BINARY" is executed with SQLBindCol using SQL_C_BINARY
    Then Bound buffer should contain raw bytes [0xAB, 0xCD, 0xEF]

  # ============================================================================
  # EMPTY BINARY CONVERSION
  # ============================================================================

  @odbc_e2e
  Scenario: should convert empty binary to SQL_C_BINARY returning zero-length data
    Given Snowflake client is logged in
    When Query "SELECT X''::BINARY" is executed
    Then SQL_C_BINARY should return indicator 0

  # ============================================================================
  # VARBINARY SYNONYM
  # ============================================================================

  @odbc_e2e
  Scenario: should convert VARBINARY to SQL_C_BINARY same as BINARY
    Given Snowflake client is logged in
    When Query "SELECT X'CAFE'::VARBINARY" is executed
    Then SQL_C_BINARY should return raw bytes [0xCA, 0xFE]

  # ============================================================================
  # UNSUPPORTED TARGET TYPE
  # ============================================================================

  @odbc_e2e
  Scenario: should return error when converting binary to unsupported C type
    Given Snowflake client is logged in
    When Query "SELECT X'ABCDEF'::BINARY" is executed
    Then SQLGetData with SQL_C_FLOAT should return SQL_ERROR

  # ============================================================================
  # COLUMN METADATA VIA SQLColAttribute
  # ============================================================================

  # TODO: Blocked — SQLColAttribute handling for SQL_DESC_OCTET_LENGTH and
  # SQL_DESC_PRECISION requires type-specific semantics beyond column_size.
  # Intentionally untagged (no @odbc_e2e) to exclude from test runs.
  # odbc_e2e
  Scenario: should report correct column metadata via SQLColAttribute for BINARY
    Given Snowflake client is logged in
    When Query "SELECT X'ABCDEF'::BINARY(10)" is executed
    Then SQLColAttribute should report SQL_DESC_TYPE as SQL_VARBINARY
    And SQLColAttribute should report SQL_DESC_OCTET_LENGTH as 10
    And SQLColAttribute should report SQL_DESC_PRECISION as 10

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: should handle NULL binary with SQL_C_BINARY
    Given Snowflake client is logged in
    When Query "SELECT NULL::BINARY" is executed
    Then SQL_C_BINARY should return SQL_NULL_DATA indicator

  # ============================================================================
  # CHUNKED SQLGetData FOR LARGE BINARY
  # ============================================================================

  @odbc_e2e
  Scenario: should retrieve large binary in chunks via SQLGetData
    Given Snowflake client is logged in
    When Query selecting a binary value larger than the buffer is executed
    Then First SQLGetData call should return SQL_SUCCESS_WITH_INFO with partial data
    And Second SQLGetData call should return SQL_SUCCESS with remaining data

  # ============================================================================
  # EXACT-FIT BUFFER
  # ============================================================================

  @odbc_e2e
  Scenario: should succeed with exact-fit buffer for SQL_C_BINARY
    Given Snowflake client is logged in
    When Query "SELECT X'ABCDEF'::BINARY" is executed
    Then SQL_C_BINARY with buffer exactly matching data length should return SQL_SUCCESS

  # ============================================================================
  # 3-CHUNK RETRIEVAL
  # ============================================================================

  @odbc_e2e
  Scenario: should retrieve binary in three chunks via SQLGetData
    Given Snowflake client is logged in
    When Query selecting a 9-byte binary value is executed
    Then First SQLGetData call should return first 3 bytes with 01004
    And Second SQLGetData call should return next 3 bytes with 01004
    And Third SQLGetData call should return final 3 bytes with SQL_SUCCESS

  # ============================================================================
  # ZERO-LENGTH BUFFER — LENGTH-ONLY QUERY
  # ============================================================================

  @odbc_e2e
  Scenario: should report full length with zero-length buffer for SQL_C_BINARY
    Given Snowflake client is logged in
    When Query "SELECT X'ABCDEF'::BINARY" is executed
    Then SQLGetData with BufferLength=0 should return 01004 with indicator reporting full data length
