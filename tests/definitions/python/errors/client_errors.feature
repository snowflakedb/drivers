@python
Feature: Client-side errors
  Errors raised by the connector for client-side issues: closed connections,
  closed cursors, invalid parameters, unsupported operations, and
  configuration problems.

  # --- Closed Connection / Cursor ---

  @python_e2e
  Scenario: should raise DatabaseError when creating cursor on closed connection
    Given A connection that has been closed
    When The user attempts to create a cursor
    Then DatabaseError is raised with message matching "Connection is closed"

  @python_e2e
  Scenario: should raise InterfaceError when executing on closed cursor
    Given A cursor that has been closed
    When The user calls execute with "SELECT 1"
    Then InterfaceError is raised with message matching "Cursor is closed"

  @python_e2e
  Scenario: should raise error when executing on cursor after connection closed
    Given A connection with an open cursor
    When The connection is closed
    And The user calls execute on the cursor
    Then Error is raised with message matching "closed"

  # --- Invalid Connection Parameters ---

  @python_e2e
  Scenario: should raise ProgrammingError for invalid authenticator value
    When The user connects with authenticator "INVALID_AUTH_METHOD"
    Then ProgrammingError is raised with errno 251007

  @python_e2e
  Scenario: should raise ProgrammingError for malformed private key
    When The user connects with SNOWFLAKE_JWT and invalid private_key bytes
    Then ProgrammingError is raised with message matching "private key"

  # --- executemany Errors ---

  @python_e2e
  Scenario: should raise InterfaceError for executemany with non-rewritable INSERT
    Given A temporary table with schema "val INT"
    When executemany is called with "INSERT INTO t (SELECT 1)" and [[1], [2]]
    Then InterfaceError is raised with message matching "Failed to rewrite multi-row insert"

  @python_e2e
  Scenario: should raise InterfaceError for executemany with inconsistent row sizes
    Given A temporary table with schema "val INT" and qmark paramstyle
    When executemany is called with "INSERT INTO t VALUES (?)" and [[1], [1, 2]]
    Then InterfaceError is raised with message matching "Bulk data size don't match"

  # --- Data Conversion Errors ---

  @python_e2e
  Scenario: should raise InterfaceError for timestamp with out-of-range year
    When The user executes "SELECT '12345-01-02'::TIMESTAMP_NTZ" and calls fetchone
    Then InterfaceError is raised with message matching "Failed to convert"
