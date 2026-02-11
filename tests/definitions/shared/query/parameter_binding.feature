@python
Feature: Parameter binding

  # Core-specific parameter binding tests live in core/query/parameters_bind.feature.

  # =========================================================================== #
  #                        Basic type binding                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should bind basic types with positional parameters
    Given Snowflake client is logged in
    When Query "SELECT ?, ?, ?, ?, ?" is executed with positional parameters [42, 3.14, "hello", True, None]
    Then Result should contain values matching the bound parameters

  @python_e2e
  Scenario: should bind positional parameters with numeric placeholders
    Given Snowflake client is logged in
    When Query "SELECT :1, :2, :3" is executed with positional parameters [100, "test", True]
    Then Result should contain values in order [100, "test", True]

  # =========================================================================== #
  #                         Table operations                                   #
  # =========================================================================== #

  @python_e2e
  Scenario: should insert single row with parameter binding
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR, active BOOLEAN) exists
    When Row with values [1, "Alice", True] is inserted using parameter binding
    And Query "SELECT * FROM table" is executed
    Then Result should contain the inserted row [1, "Alice", True]

  @python_e2e
  Scenario: should insert multiple rows sequentially with parameter binding
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    When Rows [1, "Alice"], [2, "Bob"], [3, "Charlie"] are inserted sequentially using parameter binding
    And Query "SELECT * FROM table ORDER BY id" is executed
    Then Result should contain 3 rows with correct values

  @python_e2e
  Scenario: should update row with parameter binding
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    And Row [1, "Alice"] is inserted
    When Query "UPDATE table SET name = ? WHERE id = ?" is executed with parameters ["Alice Updated", 1]
    And Query "SELECT * FROM table" is executed
    Then Result should contain [1, "Alice Updated"]

  @python_e2e
  Scenario: should delete row with parameter binding
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    And Rows [1, "Alice"] and [2, "Bob"] are inserted
    When Query "DELETE FROM table WHERE id = ?" is executed with parameter [1]
    And Query "SELECT * FROM table" is executed
    Then Result should contain only [2, "Bob"]

  @python_e2e
  Scenario: should select with WHERE clause parameter binding
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR, age NUMBER) exists
    And Rows [1, "Alice", 30], [2, "Bob", 25], [3, "Charlie", 35] are inserted
    When Query "SELECT * FROM table WHERE age > ?" is executed with parameter [28]
    Then Result should contain rows for "Alice" and "Charlie"

  # =========================================================================== #
  #                            Edge cases                                      #
  # =========================================================================== #

  @python_e2e
  Scenario: should handle NULL values in parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?, ?, ?" is executed with parameters [None, 42, None]
    Then Result should contain [NULL, 42, NULL]

  @python_e2e
  Scenario: should handle special characters in string binding
    Given Snowflake client is logged in
    When Query "SELECT ?::VARCHAR" is executed with parameter containing special characters
    # Special characters include:
    #   - SQL injection attempt: "'; DROP TABLE test; --"
    #   - XSS attempt: "<script>alert('xss')</script>"
    #   - Multiple newlines: "Line1\nLine2\nLine3"
    #   - Multiple tabs: "Tab\t\tSeparated\t\tValues"
    #   - Mixed quotes: "Quote'Within\"String"
    #   - Escaped sequences as literal: "\\n\\t\\r\\\\"
    Then Result should contain the exact special character string

  @python_e2e
  Scenario: should handle Unicode characters in parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::VARCHAR, ?::VARCHAR" is executed with parameters ["日本語", "⛄"]
    Then Result should contain Unicode strings ["日本語", "⛄"]

  @python_e2e
  Scenario: should bind zero values
    Given Snowflake client is logged in
    When Query "SELECT ?, ?::FLOAT, ?::VARCHAR" is executed with parameters [0, 0.0, ""]
    Then Result should contain zero and empty values [0, 0.0, ""]

  @python_e2e
  Scenario: should handle mixed type casting with parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::NUMBER, ?::VARCHAR, ?::BOOLEAN" is executed with parameters [42, "hello", True]
    Then Result should match the type-casted parameters [42, "hello", True]

  @python_e2e
  Scenario: should raise error when placeholder count mismatches argument count
    Given Snowflake client is logged in
    When Query with 2 placeholders is executed with 3 arguments
    Then Query should successfully execute
    When Query with 3 placeholders is executed with 1 argument
    Then Error should be raised for too few arguments

  # =========================================================================== #
  #                        Multirow binding                                   #
  # =========================================================================== #

  @python_e2e
  Scenario: should insert multiple rows using multirow binding
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    When Rows [[1, "Alice"], [2, "Bob"], [3, "Charlie"]] are inserted using multirow binding
    And Query "SELECT * FROM table ORDER BY id" is executed
    Then Result should contain 3 rows with correct values

  @python_e2e
  Scenario: should handle empty sequence in multirow binding
    Given Snowflake client is logged in
    When Multirow binding is called with empty sequence
    Then No error should be raised

  @python_e2e
  Scenario: should validate parameter length in multirow binding
    Given Snowflake client is logged in
    When Multirow binding is called with inconsistent parameter lengths [(1, "a"), (2, "b", "extra")]
    Then Error should be raised indicating parameter sequence length mismatch

  @python_e2e
  Scenario: should handle NULL values in multirow binding
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, value VARCHAR) exists
    When Rows [[1, NULL], [2, "value"], [3, NULL]] are inserted using multirow binding
    And Query "SELECT * FROM table ORDER BY id" is executed
    Then Result should contain [[1, NULL], [2, "value"], [3, NULL]]

  # =========================================================================== #
  #                        Complex scenarios                                   #
  # =========================================================================== #

  @python_e2e
  Scenario: should bind many parameters
    Given Snowflake client is logged in
    When Query with 20 positional parameters is executed with values [0..19]
    Then Result should contain all 20 values in order

  @python_e2e
  Scenario: should bind parameters with OR clause for multiple value matching
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    And Rows [1, "Alice"], [2, "Bob"], [3, "Charlie"], [4, "David"], [5, "Eve"] are inserted
    When Query "SELECT FROM {table_name} WHERE id = ? OR id = ? OR id = ? ORDER BY id" is executed with parameters [1, 3, 5]
    Then Result should contain [("Alice"), ("Charlie"), ("Eve")]
