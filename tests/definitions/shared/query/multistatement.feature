@core @jdbc @odbc
Feature: Multistatement query execution

  # ============================================================================
  # MULTIPLE SELECTS
  # ============================================================================

  @core_e2e @jdbc_e2e @odbc_e2e
  Scenario: should execute multiple SELECT statements
    Given Snowflake client is logged in
    When Multistatement query with 3 SELECTs is executed
    Then 3 result sets are returned
    And each result set contains correct data

  # ============================================================================
  # MULTIPLE DML STATEMENTS
  # ============================================================================

  @core_e2e @jdbc_e2e @odbc_e2e
  Scenario: should execute multiple DML statements
    Given Snowflake client is logged in
    When Multistatement query with CREATE TABLE, INSERT, and DROP is executed
    Then 3 result sets are returned

  # ============================================================================
  # MIXED STATEMENT TYPES
  # ============================================================================

  @core_e2e @jdbc_e2e @odbc_e2e
  Scenario: should execute mixed statement types
    Given Snowflake client is logged in
    When Multistatement query with various types is executed
    Then 5 result sets are returned
    And the SELECT result contains expected data

  # ============================================================================
  # ERROR HANDLING
  # ============================================================================

  @core_e2e @jdbc_e2e @odbc_e2e
  Scenario: should succeed when multistatement SQL is sent without multi_statement_count
    Given Snowflake client is logged in
    When Multistatement SQL is executed without configuring multi_statement_count
    Then the statement succeeds with MULTI_STATEMENT_COUNT defaulting to 0 (unlimited)

  @core_e2e @jdbc_e2e @odbc_e2e
  Scenario: should fail when multi_statement_count does not match actual statement count
    Given Snowflake client is logged in
    When Single SELECT is executed with multi_statement_count set to 3
    Then an error is returned indicating statement count mismatch
