@odbc
Feature: ODBC incompatible numeric C types to SQL_TYPE_TIMESTAMP conversions

  @odbc_e2e
  Scenario: should reject SQL_C_SLONG bound to SQL_TYPE_TIMESTAMP
    Given Snowflake client is logged in
    When SQL_C_SLONG is bound to SQL_TYPE_TIMESTAMP and executed
    Then the driver rejects the incompatible conversion with an error

  @odbc_e2e
  Scenario: should reject SQL_C_DOUBLE bound to SQL_TYPE_TIMESTAMP
    Given Snowflake client is logged in
    When SQL_C_DOUBLE is bound to SQL_TYPE_TIMESTAMP and executed
    Then the driver rejects the incompatible conversion with an error

  @odbc_e2e
  Scenario: should reject SQL_C_FLOAT bound to SQL_TYPE_TIMESTAMP
    Given Snowflake client is logged in
    When SQL_C_FLOAT is bound to SQL_TYPE_TIMESTAMP and executed
    Then the driver rejects the incompatible conversion with an error

  @odbc_e2e
  Scenario: should reject SQL_C_BIT bound to SQL_TYPE_TIMESTAMP
    Given Snowflake client is logged in
    When SQL_C_BIT is bound to SQL_TYPE_TIMESTAMP and executed
    Then the driver rejects the incompatible conversion with an error

  @odbc_e2e
  Scenario: should reject SQL_C_NUMERIC bound to SQL_TYPE_TIMESTAMP
    Given Snowflake client is logged in
    When SQL_C_NUMERIC is bound to SQL_TYPE_TIMESTAMP and executed
    Then the driver rejects the incompatible conversion with an error

  @odbc_e2e
  Scenario: should reject SQL_C_SBIGINT bound to SQL_TYPE_TIMESTAMP
    Given Snowflake client is logged in
    When SQL_C_SBIGINT is bound to SQL_TYPE_TIMESTAMP and executed
    Then the driver rejects the incompatible conversion with an error
