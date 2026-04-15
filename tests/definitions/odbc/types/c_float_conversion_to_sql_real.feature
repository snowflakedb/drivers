@odbc
Feature: ODBC C float types to SQL real conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE to SQL_DOUBLE and read back
    Given Snowflake client is logged in
    When A double value is bound with SQL_C_DOUBLE and SQL_DOUBLE and inserted
    Then The value should be read back correctly as SQL_C_DOUBLE

  @odbc_e2e
  Scenario: should bind SQL_C_FLOAT to SQL_REAL and read back
    Given Snowflake client is logged in
    When A float value is bound with SQL_C_FLOAT and SQL_REAL and inserted
    Then The value should be read back as SQL_C_DOUBLE

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE negative zero
    Given Snowflake client is logged in
    When Negative zero is bound as SQL_C_DOUBLE and inserted
    Then The fetched value should be floating-point zero

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE large value
    Given Snowflake client is logged in
    When A large double near DBL_MAX is bound and inserted
    Then The value should round-trip within floating-point precision

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE small value
    Given Snowflake client is logged in
    When A very small positive double is bound and inserted
    Then The value should round-trip within floating-point precision

  @odbc_e2e
  Scenario: should bind SQL_C_FLOAT max value
    Given Snowflake client is logged in
    When FLT_MAX is bound with SQL_C_FLOAT and SQL_REAL and inserted
    Then The value should read back matching FLT_MAX

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE with NULL indicator
    Given Snowflake client is logged in
    When SQL_NULL_DATA is used for the SQL_C_DOUBLE parameter
    Then The column value should be NULL

  @odbc_e2e
  Scenario: should bind SQL_C_DEFAULT to SQL_DOUBLE and read back
    Given Snowflake client is logged in
    When A double value is bound with SQL_C_DEFAULT and SQL_DOUBLE and inserted
    Then The value should be read back correctly as SQL_C_DOUBLE

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE zero
    Given Snowflake client is logged in
    When Zero is bound as SQL_C_DOUBLE and inserted
    Then The fetched value should be zero
