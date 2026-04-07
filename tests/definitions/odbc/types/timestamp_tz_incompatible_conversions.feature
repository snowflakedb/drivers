@odbc
Feature: ODBC TIMESTAMP_TZ incompatible C type conversions
  # Per the ODBC spec, SQL_TYPE_TIMESTAMP can only be converted to
  # SQL_C_CHAR, SQL_C_WCHAR, SQL_C_BINARY, SQL_C_TYPE_DATE,
  # SQL_C_TYPE_TIME, SQL_C_TYPE_TIMESTAMP (and SQL_C_DEFAULT).
  # All other C types are incompatible.
  # Expected SQLSTATE: 07006 (Restricted data type attribute violation)

  @odbc_e2e
  Scenario: should fail converting TIMESTAMP_TZ to numeric C types
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 14:30:45 +00:00'::TIMESTAMP_TZ" is executed
    Then SQL_C_SLONG conversion should fail with SQLSTATE 07006
    And SQL_C_DOUBLE conversion should fail with SQLSTATE 07006
    And SQL_C_FLOAT conversion should fail with SQLSTATE 07006
    And SQL_C_NUMERIC conversion should fail with SQLSTATE 07006
    And SQL_C_BIT conversion should fail with SQLSTATE 07006

  @odbc_e2e
  Scenario: should fail converting TIMESTAMP_TZ to additional numeric C types
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 14:30:45 +00:00'::TIMESTAMP_TZ" is executed
    Then SQL_C_STINYINT conversion should fail with SQLSTATE 07006
    And SQL_C_SSHORT conversion should fail with SQLSTATE 07006
    And SQL_C_SBIGINT conversion should fail with SQLSTATE 07006
    And SQL_C_UTINYINT conversion should fail with SQLSTATE 07006
    And SQL_C_USHORT conversion should fail with SQLSTATE 07006
    And SQL_C_ULONG conversion should fail with SQLSTATE 07006
    And SQL_C_UBIGINT conversion should fail with SQLSTATE 07006

  @odbc_e2e
  Scenario: should fail converting TIMESTAMP_TZ to single-component interval C types
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 14:30:45 +00:00'::TIMESTAMP_TZ" is executed
    Then SQL_C_INTERVAL_YEAR conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_MONTH conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_DAY conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_HOUR conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_MINUTE conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_SECOND conversion should fail with SQLSTATE 07006

  @odbc_e2e
  Scenario: should fail converting TIMESTAMP_TZ to compound interval C types
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 14:30:45 +00:00'::TIMESTAMP_TZ" is executed
    Then SQL_C_INTERVAL_YEAR_TO_MONTH conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_DAY_TO_HOUR conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_DAY_TO_MINUTE conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_DAY_TO_SECOND conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_HOUR_TO_MINUTE conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_HOUR_TO_SECOND conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_MINUTE_TO_SECOND conversion should fail with SQLSTATE 07006

  @odbc_e2e
  Scenario: should fail converting TIMESTAMP_TZ to SQL_C_GUID
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 14:30:45 +00:00'::TIMESTAMP_TZ" is executed
    Then SQL_C_GUID conversion should fail with SQLSTATE 07006 (or HYC00 on Windows)
