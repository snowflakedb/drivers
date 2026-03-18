@odbc
Feature: ODBC REAL (FLOAT/DOUBLE) to integer C type conversions
  # Tests conversion of Snowflake FLOAT/DOUBLE/REAL SQL types to integer C types
  # (SQL_C_LONG, SQL_C_SLONG, SQL_C_ULONG, SQL_C_SHORT, SQL_C_SSHORT, SQL_C_USHORT,
  #  SQL_C_TINYINT, SQL_C_STINYINT, SQL_C_UTINYINT, SQL_C_SBIGINT, SQL_C_UBIGINT).
  # Covers fractional truncation (01S07), overflow (22003), boundary values,
  # unsigned types, NaN, and Infinity.

  @odbc_e2e
  Scenario: REAL explicit integer conversions truncate fractional part
    Given A Snowflake connection is established
    When A fractional FLOAT value 123.789 is fetched as each integer C type
    Then All integer C types return 123 with fractional truncation

  @odbc_e2e
  Scenario: REAL explicit integer conversions - negative value
    Given A Snowflake connection is established
    When A negative fractional FLOAT value -42.9 is fetched as signed integer C types
    Then All signed integer C types return -42 with fractional truncation

  @odbc_e2e
  Scenario: REAL explicit SQL_C_SBIGINT with large values
    Given A Snowflake connection is established
    When The largest integer exactly representable as f64 (2^53) is fetched as SQL_C_SBIGINT
    Then The value 9007199254740992 is returned without truncation

  @odbc_e2e
  Scenario: REAL fractional truncation returns 01S07
    Given A Snowflake connection is established
    When Fractional FLOAT values are fetched as integer C types
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01S07

  @odbc_e2e
  Scenario: REAL overflow returns 22003
    Given A Snowflake connection is established
    When Out-of-range FLOAT values are fetched as narrow integer C types
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: REAL explicit unsigned integer conversions
    Given A Snowflake connection is established
    When Positive FLOAT values are fetched as unsigned integer C types
    Then Exact values succeed, fractional values truncate with 01S07, zero succeeds

  @odbc_e2e
  Scenario: REAL integer boundary values for overflow
    Given A Snowflake connection is established
    When FLOAT values at or just past integer type boundaries are fetched
    Then Values at boundaries succeed, values past boundaries return 22003

  @odbc_e2e
  Scenario: REAL negative fraction to unsigned integer types
    Given A Snowflake connection is established
    When Negative fractional FLOAT values are fetched as unsigned integer C types
    Then Either 01S07 or 22003 is returned depending on implementation

  @odbc_e2e
  Scenario: REAL NaN to integer types returns error
    Given A Snowflake connection is established
    When NaN FLOAT value is fetched as integer C types
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: REAL Infinity to integer types returns 22003
    Given A Snowflake connection is established
    When Infinity FLOAT values are fetched as integer C types
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: REAL NULL to SQL_C_INTEGER types
    Given A Snowflake connection is established
    When NULL FLOAT values are queried
    Then NULL FLOAT values return SQL_NULL_DATA
