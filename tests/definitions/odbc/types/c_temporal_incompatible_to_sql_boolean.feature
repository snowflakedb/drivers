@odbc
Feature: ODBC incompatible temporal/GUID C types to SQL boolean conversions

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_DATE bound to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_TYPE_DATE is bound to SQL_BIT and executed
    Then the driver rejects the incompatible conversion with an error

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_TIME bound to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_TYPE_TIME is bound to SQL_BIT and executed
    Then the driver rejects the incompatible conversion with an error

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_TIMESTAMP bound to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_TYPE_TIMESTAMP is bound to SQL_BIT and executed
    Then the driver rejects the incompatible conversion with an error

  @odbc_e2e
  Scenario: should reject SQL_C_GUID bound to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_GUID is bound to SQL_BIT and executed
    Then the driver rejects the incompatible conversion with an error
