@odbc
Feature: ODBC TIMESTAMP client type mapping session parameters

  @odbc_e2e
  Scenario: CLIENT_TIMESTAMP_TYPE_MAPPING=TIMESTAMP_NTZ maps untyped timestamp to NTZ
    Given Snowflake client is logged in with NTZ timestamp mapping
    When An unqualified TIMESTAMP column is queried and described
    Then The value is fetched as SQL_C_TYPE_TIMESTAMP correctly

  @odbc_e2e
  Scenario: CLIENT_TIMESTAMP_TYPE_MAPPING=TIMESTAMP_LTZ maps untyped timestamp to LTZ
    Given Snowflake client is logged in with LTZ timestamp mapping and UTC timezone
    When An unqualified TIMESTAMP column is queried
    Then The value is fetched correctly (server interprets the literal in session timezone)

  @odbc_e2e
  Scenario: TIMESTAMP_TYPE_MAPPING changes column type for TIMESTAMP
    Given Snowflake client is logged in with NTZ type mapping
    When A TIMESTAMP column is described via SQLDescribeCol
    Then The SQL data type should be SQL_TYPE_TIMESTAMP
