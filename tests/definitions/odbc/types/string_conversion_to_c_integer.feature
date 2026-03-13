@odbc
Feature: ODBC string to integer type conversions
  # Tests converting Snowflake VARCHAR/STRING type to integer ODBC C types:
  # SQL_C_LONG, SQL_C_SLONG, SQL_C_ULONG, SQL_C_SHORT, SQL_C_SSHORT, SQL_C_USHORT,
  # SQL_C_TINYINT, SQL_C_STINYINT, SQL_C_UTINYINT, SQL_C_SBIGINT, SQL_C_UBIGINT,
  # SQL_C_BIT, SQL_C_NUMERIC

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - String to Signed Integer Types
  # ============================================================================

  @odbc_e2e
  Scenario Outline: should convert string literals to signed <c_type>
    Given Snowflake client is logged in
    When Query selecting string literals representing integers is executed
    Then <c_type> conversions should work

    Examples:
      | c_type         |
      | SQL_C_LONG     |
      | SQL_C_SLONG    |
      | SQL_C_SHORT    |
      | SQL_C_TINYINT  |
      | SQL_C_STINYINT |
      | SQL_C_SBIGINT  |

  @odbc_e2e
  Scenario Outline: should convert string literals to unsigned <c_type>
    Given Snowflake client is logged in
    When Query selecting string literals representing unsigned integers is executed
    Then <c_type> conversions should work

    Examples:
      | c_type         |
      | SQL_C_ULONG    |
      | SQL_C_USHORT   |
      | SQL_C_UTINYINT |
      | SQL_C_UBIGINT  |
      | SQL_C_SSHORT   |

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - String to BIT Type
  # ============================================================================

  @odbc_e2e
  Scenario: should convert string literals to SQL_C_BIT
    Given Snowflake client is logged in
    When Query selecting string literals representing boolean values is executed
    Then the string values should be correctly converted to SQL_C_BIT

  # ============================================================================
  # FAILING CONVERSIONS - String to BIT Type
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting string literals with to SQL_C_BIT
    Given Snowflake client is logged in
    When Query selecting string literals with leading/trailing whitespace is executed
    Then the string values should fail to convert to SQL_C_BIT

  # ============================================================================
  # TRUNCATION TESTS
  # ============================================================================

  @odbc_e2e
  Scenario: should truncate decimal string literals with fractional part when converting to integer types
    Given Snowflake client is logged in
    When Query selecting string literals with decimal parts is executed
    Then the string values should be truncated when converted to integer types

  @odbc_e2e
  Scenario: should truncate decimal string literals without fractional part when converting to integer types
    Given Snowflake client is logged in
    When Query selecting string literals without fractional part is executed
    Then the string values should be truncated when converted to integer types

  # ============================================================================
  # CONVERSION WITH SQLBindCol - Integer types
  # ============================================================================

  @odbc_e2e
  Scenario: should convert strings to integer types using SQLBindCol
    # Test successful SQL_C_LONG binding
    # Test failed binding for invalid string
    Given Snowflake client is logged in
    When Query selecting string numeric value is executed with SQLBindCol for SQL_C_LONG
    Then the bound integer value should match the string representation
    And invalid string should fail binding with SQLSTATE 22018
