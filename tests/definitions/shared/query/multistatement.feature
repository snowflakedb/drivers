@core @jdbc @odbc @python
Feature: Multistatement query execution

  # ============================================================================
  # MULTIPLE SELECTS
  # ============================================================================

  @core_e2e @jdbc_e2e @odbc_e2e @python_e2e
  Scenario: should execute multiple SELECT statements
    Given Snowflake client is logged in
    When Multistatement query with 3 SELECTs is executed
    Then 3 result sets are returned
    And each result set contains correct data

  # ============================================================================
  # MULTIPLE DML STATEMENTS
  # ============================================================================

  @core_e2e @jdbc_e2e @odbc_e2e @python_e2e
  Scenario: should execute multiple DML statements
    Given Snowflake client is logged in
    When Multistatement query with CREATE TABLE, INSERT, and DROP is executed
    Then 3 result sets are returned

  # ============================================================================
  # MIXED STATEMENT TYPES
  # ============================================================================

  @core_e2e @jdbc_e2e @odbc_e2e @python_e2e
  Scenario: should execute mixed statement types
    Given Snowflake client is logged in
    When Multistatement query with various types is executed
    Then 5 result sets are returned
    And the SELECT result contains expected data

  # ============================================================================
  # ERROR HANDLING
  # ============================================================================

  @core_e2e @jdbc_e2e @odbc_e2e @python_e2e
  Scenario: should fail when multistatement SQL is sent without multi_statement_count
    Given Snowflake client is logged in
    When Multistatement SQL is executed without configuring multi_statement_count
    Then an error is returned indicating multi-statement is not enabled

  @core_e2e @jdbc_e2e @odbc_e2e @python_e2e
  Scenario: should fail when multi_statement_count does not match actual statement count
    Given Snowflake client is logged in
    When Single SELECT is executed with multi_statement_count set to 3
    Then an error is returned indicating statement count mismatch

  # ============================================================================
  # POSITIONAL PARAMETER BINDING
  # ============================================================================
  # Combines MULTI_STATEMENT_COUNT with parameterized SQL. Bindings are
  # positional and flatten across `;`-separated statements in source order —
  # same contract as legacy snowflake-jdbc PreparedStatement, snowflake-odbc
  # SQLBindParameter, and snowflake-connector-python qmark paramstyle.

  @jdbc_e2e @odbc_e2e @python_e2e
  Scenario: should execute multistatement DML with positional parameters
    Given Snowflake client is logged in
    And A temporary table with column (id NUMBER) exists
    When Multistatement query "INSERT INTO {table} VALUES(?); INSERT INTO {table} VALUES(?),(?)" is executed with positional parameters [10, 20, 30] and multi_statement_count=2
    Then 2 result sets are returned
    And the first result set reports update count 1
    And the second result set reports update count 2
    And the table contains rows [10, 20, 30]

  @jdbc_e2e @odbc_e2e @python_e2e
  Scenario: should execute multistatement SELECT with positional parameters
    Given Snowflake client is logged in
    When Multistatement query "SELECT ?; SELECT ?, ?; SELECT ?, ?, ?" is executed with positional parameters [10, 20, 30, 40, 50, 60] and multi_statement_count=3
    Then 3 result sets are returned
    And the first result set contains row [10]
    And the second result set contains row [20, 30]
    And the third result set contains row [40, 50, 60]

  @jdbc_e2e @odbc_e2e @python_e2e
  Scenario: should fail when multistatement query has too few parameters
    Given Snowflake client is logged in
    When Multistatement query "SELECT ?; SELECT ?, ?" is executed with positional parameters [10] and multi_statement_count=2
    Then an error is returned indicating parameter count mismatch

  @jdbc_e2e @odbc_e2e @python_e2e
  Scenario: should handle NULL positional parameters in multistatement query
    Given Snowflake client is logged in
    When Multistatement query "SELECT ?; SELECT ?, ?" is executed with positional parameters [NULL, 10, NULL] and multi_statement_count=2
    Then 2 result sets are returned
    And the first result set contains row [NULL]
    And the second result set contains row [10, NULL]
