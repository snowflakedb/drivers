@odbc @python @jdbc
Feature: Basic execute query

  # ============================================================================
  # SELECT QUERIES
  # ============================================================================

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should execute simple SELECT returning single value
    When Query "SELECT 1 AS value" is executed
    Then the result should contain value 1

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should execute SELECT returning multiple columns
    When Query "SELECT 1 AS col1, 'hello' AS col2, '3.14' AS col3" is executed
    Then the result should contain:
      | col1 | col2  | col3 |
      | 1    | hello | 3.14 |

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should execute SELECT returning multiple rows
    When Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 5)) v ORDER BY id" is executed
    Then there are 5 numbered sequentially rows returned

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should execute SELECT returning empty result set
    When Query "SELECT 1 WHERE 1=0" is executed
    Then the result set should be empty

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should execute SELECT returning NULL values
    When Query "SELECT NULL AS col1, 42 AS col2, NULL AS col3" is executed
    Then the result should contain NULL for col1 and col3 and 42 for col2

  # ============================================================================
  # DDL STATEMENTS
  # ============================================================================

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should execute CREATE and DROP TABLE statements
    When CREATE TABLE statement is executed
    Then the table should be created successfully
    And DROP TABLE statement should complete successfully

  # ============================================================================
  # DML STATEMENTS
  # ============================================================================

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should execute INSERT and retrieve inserted data
    And A temporary table is created
    When INSERT statement is executed to add rows
    And Query "SELECT id, value FROM {table} ORDER BY id" is executed
    Then the inserted data should be correctly returned

  # ============================================================================
  # ERROR HANDLING
  # ============================================================================

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should return error for invalid SQL syntax
    When Invalid SQL "SELCT INVALID SYNTAX" is executed
    Then An error should be returned

  # ============================================================================
  # SEQUENTIAL EXECUTION
  # ============================================================================

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should execute multiple queries sequentially on same connection
    When Multiple queries are executed sequentially
    Then each query should return correct results
