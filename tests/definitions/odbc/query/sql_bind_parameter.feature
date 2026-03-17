@odbc
Feature: ODBC SQLBindParameter function behavior
  # Tests for SQLBindParameter based on ODBC specification:
  # https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindparameter-function
  # E2E round-trip tests: bind parameter -> execute -> fetch -> verify result

  # ============================================================================
  # Integer Types
  # ============================================================================

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_SLONG integer and round-trips through SELECT.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared
    And an SQL_C_SLONG parameter is bound with value 42
    Then executing and fetching should return 42

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_SHORT integer and round-trips through SELECT.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared
    And an SQL_C_SHORT parameter is bound with value 12345
    Then executing and fetching should return 12345

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_SBIGINT and round-trips through SELECT.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared
    And an SQL_C_SBIGINT parameter is bound with a large value
    Then executing and fetching should return the large value

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_STINYINT and round-trips through SELECT.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared
    And an SQL_C_STINYINT parameter is bound with value 127
    Then executing and fetching should return 127

  @odbc_e2e
  Scenario: SQLBindParameter binds negative integer and round-trips through SELECT.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared
    And an SQL_C_SLONG parameter is bound with value -42
    Then executing and fetching should return -42

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_UTINYINT and round-trips through SELECT.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared
    And an SQL_C_UTINYINT parameter is bound with value 255
    Then executing and fetching should return 255

  # ============================================================================
  # Decimal / Numeric Types
  # ============================================================================

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_CHAR to SQL_DECIMAL and round-trips through INSERT/SELECT.
    Given Snowflake client is logged in
    And a temporary table with a DECIMAL column exists
    When a parameterized INSERT is prepared
    And an SQL_C_CHAR parameter is bound with a decimal string value
    And the INSERT is executed
    Then selecting the value should return 12345.67

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_CHAR to SQL_NUMERIC and round-trips through SELECT.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared with SQL_NUMERIC parameter type
    And an SQL_C_CHAR parameter is bound with a numeric string value
    Then executing and fetching should return the value

  # ============================================================================
  # Float / Double Types
  # ============================================================================

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_DOUBLE and round-trips with precision.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared
    And an SQL_C_DOUBLE parameter is bound with value 3.14159265358979
    Then executing and fetching should return the double value with precision

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_FLOAT and round-trips through SELECT.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared
    And an SQL_C_FLOAT parameter is bound with value 2.5
    Then executing and fetching should return the float value

  # ============================================================================
  # String Types
  # ============================================================================

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_CHAR with SQL_NTS and round-trips through SELECT.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared
    And an SQL_C_CHAR parameter is bound with null-terminated string
    Then executing and fetching should return the string

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_CHAR with explicit length.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared
    And an SQL_C_CHAR parameter is bound with explicit length
    Then executing and fetching should return the substring defined by the length

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_CHAR with empty string.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared
    And an SQL_C_CHAR parameter is bound with an empty string
    Then executing and fetching should return an empty string

  # ============================================================================
  # Boolean Types
  # ============================================================================

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_BIT true and round-trips through SELECT.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared
    And an SQL_C_BIT parameter is bound with value 1
    Then executing and fetching should return true

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_BIT false and round-trips through SELECT.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared
    And an SQL_C_BIT parameter is bound with value 0
    Then executing and fetching should return false

  # ============================================================================
  # Binary Types
  # ============================================================================

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_BINARY and round-trips through INSERT/SELECT.
    Given Snowflake client is logged in
    And a temporary table with a BINARY column exists
    When a parameterized INSERT is prepared
    And an SQL_C_BINARY parameter is bound with binary data
    And the INSERT is executed
    Then selecting the data should return the original binary content

  # ============================================================================
  # Date Types
  # ============================================================================

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_TYPE_DATE struct and round-trips through INSERT/SELECT.
    Given Snowflake client is logged in
    And a temporary table with a DATE column exists
    When a parameterized INSERT is prepared
    And an SQL_C_TYPE_DATE parameter is bound with date 2025-03-15
    And the INSERT is executed
    Then selecting the date should return 2025-03-15

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_TYPE_DATE with epoch date.
    Given Snowflake client is logged in
    And a temporary table with a DATE column exists
    When a parameterized INSERT is prepared
    And an SQL_C_TYPE_DATE parameter is bound with date 1970-01-01
    And the INSERT is executed
    Then selecting the date should return 1970-01-01

  @odbc_e2e
  Scenario: SQLBindParameter binds date as SQL_C_CHAR string to SQL_TYPE_DATE.
    Given Snowflake client is logged in
    And a temporary table with a DATE column exists
    When a parameterized INSERT is prepared
    And a date string is bound as SQL_C_CHAR to SQL_TYPE_DATE
    And the INSERT is executed
    Then selecting the date should return 2025-03-15

  # ============================================================================
  # Time Types
  # ============================================================================

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_TYPE_TIME struct and round-trips through INSERT/SELECT.
    Given Snowflake client is logged in
    And a temporary table with a TIME column exists
    When a parameterized INSERT is prepared
    And an SQL_C_TYPE_TIME parameter is bound with time 10:30:45
    And the INSERT is executed
    Then selecting the time should return 10:30:45

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_TYPE_TIME with midnight.
    Given Snowflake client is logged in
    And a temporary table with a TIME column exists
    When a parameterized INSERT is prepared
    And an SQL_C_TYPE_TIME parameter is bound with time 00:00:00
    And the INSERT is executed
    Then selecting the time should return 00:00:00

  @odbc_e2e
  Scenario: SQLBindParameter binds time as SQL_C_CHAR string to SQL_TYPE_TIME.
    Given Snowflake client is logged in
    And a temporary table with a TIME column exists
    When a parameterized INSERT is prepared
    And a time string is bound as SQL_C_CHAR to SQL_TYPE_TIME
    And the INSERT is executed
    Then selecting the time should return 14:30:00

  # ============================================================================
  # Timestamp Types
  # ============================================================================

  @odbc_e2e
  Scenario: SQLBindParameter binds SQL_C_TYPE_TIMESTAMP to TIMESTAMP_NTZ and round-trips.
    Given Snowflake client is logged in
    And a temporary table with a TIMESTAMP_NTZ column exists
    When a parameterized INSERT is prepared
    And an SQL_C_TYPE_TIMESTAMP parameter is bound with fractional seconds
    And the INSERT is executed
    Then selecting the timestamp should return the expected components

  @odbc_e2e
  Scenario: SQLBindParameter binds timestamp as SQL_C_CHAR string.
    Given Snowflake client is logged in
    And a temporary table with a TIMESTAMP_NTZ column exists
    When a parameterized INSERT is prepared
    And an SQL_C_CHAR parameter is bound with a timestamp string
    And the INSERT is executed
    Then selecting the timestamp should return the expected value

  # ============================================================================
  # NULL Handling
  # ============================================================================

  @odbc_e2e
  Scenario: SQLBindParameter binds NULL via SQL_NULL_DATA indicator.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared
    And a parameter is bound with SQL_NULL_DATA indicator
    Then executing and fetching should return NULL

  @odbc_e2e
  Scenario: SQLBindParameter mixes NULL and non-NULL in sequential executions.
    Given Snowflake client is logged in
    And a temporary table with an INTEGER column exists
    When a parameterized INSERT is prepared
    And a non-NULL integer is inserted followed by a NULL and another non-NULL
    Then selecting all rows should return the expected values with one NULL

  # ============================================================================
  # Multi-Parameter and Rebinding
  # ============================================================================

  @odbc_e2e
  Scenario: SQLBindParameter binds multiple typed parameters in one statement.
    Given Snowflake client is logged in
    When a SELECT with two parameter markers is prepared
    And an integer and a string parameter are bound
    Then executing and fetching should return both values

  @odbc_e2e
  Scenario: SQLBindParameter re-executes prepared statement with changed bound value.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared and bound with value 10
    And the statement is executed and the result verified
    And the cursor is closed and the bound variable changed to 20
    Then re-executing should return 20

  @odbc_e2e
  Scenario: SQLFreeStmt SQL_RESET_PARAMS clears bindings and allows re-binding.
    Given Snowflake client is logged in
    When a parameterized SELECT is prepared and an integer is bound
    And the statement is executed and the integer result is verified
    And all parameter bindings are reset with SQL_RESET_PARAMS
    And a new string parameter is bound to the same position
    Then re-executing should return the new string value

  @odbc_e2e
  Scenario: SQLExecDirect with bound parameter executes without SQLPrepare.
    Given Snowflake client is logged in
    When a parameter is bound before calling SQLExecDirect
    And SQLExecDirect is called with a parameterized query
    Then executing and fetching should return the bound parameter value
