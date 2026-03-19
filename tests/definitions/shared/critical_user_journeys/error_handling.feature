@python
Feature: Error Handling

  Structured error information (errno, sqlstate, message) for error handling
  and retry logic. Every consumer relies on this.

  # ============================================================================
  # SQL ERRORS
  # ============================================================================

  @python_e2e
  Scenario: should return structured error for SQL syntax error
    Given Snowflake client is logged in
    When Invalid SQL "SELCT INVALID SYNTAX" is executed
    Then A programming error should be raised with sqlstate "42000"
    And The error should have a non-empty errno
    And The error should have a non-empty message

  @python_e2e
  Scenario: should return error for non-existent table
    Given Snowflake client is logged in
    When "SELECT * FROM this_table_does_not_exist_e2e_xyz" is executed
    Then An error should be raised with errno 2003
    And The error message should contain the table name

  @python_e2e
  Scenario: should return error for non-existent database
    Given Snowflake client is logged in
    When "USE DATABASE this_db_does_not_exist_e2e_xyz" is executed
    Then An error should be raised

  @python_e2e
  Scenario: should succeed silently for DROP IF EXISTS on non-existent table
    Given Snowflake client is logged in
    When "DROP TABLE IF EXISTS this_table_does_not_exist_e2e_xyz" is executed
    Then No error should be raised

  @python_e2e
  Scenario: should raise interface error on operations on closed cursor
    Given Snowflake client is logged in
    And A cursor is created and used and then closed
    When execute() is called on the closed cursor
    Then InterfaceError should be raised

  @python_e2e
  Scenario: should maintain correct exception hierarchy
    Given The snowflake.connector error module is imported
    When The exception classes are inspected
    Then ProgrammingError should be a subclass of DatabaseError
    And DatabaseError should be a subclass of Error
    And InterfaceError should be a subclass of Error
    And InterfaceError should NOT be a subclass of DatabaseError
