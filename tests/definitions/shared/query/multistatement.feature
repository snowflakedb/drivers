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
  # PER-CALL SCOPE
  # ============================================================================
  # Pins the server-observable consequence of per-call MULTI_STATEMENT_COUNT
  # scoping: after a multistatement execute with num_statements=3, a
  # follow-up single-statement execute on the same client must succeed.
  # GS validates the declared count and rejects mismatches (see scenarios
  # above), so a leak between executes would surface as a clean GS error,
  # not a silent hang.

  @core_e2e
  Scenario: should not persist num_statements across executes
    Given Snowflake client is logged in
    When A multistatement query with num_statements=3 is executed
    And On the same client, a single-statement query is executed
    Then The single SELECT returns its row cleanly

  @jdbc_e2e @odbc_e2e @python_e2e
  Scenario: should fail when multistatement query has too few parameters
    Given Snowflake client is logged in
    When Multistatement SELECT requires 3 parameters but only 1 is bound
    Then an error is returned indicating parameter count mismatch

  @jdbc_e2e @odbc_e2e @python_e2e
  Scenario: should fail when NULL positional parameters are used in multistatement query
    # Snowflake's SYSTEM$MULTISTMT server-side dispatcher rejects NULL bindings
    # with "Bind variable ? not set" — confirmed against legacy snowflake-jdbc
    # and legacy snowflake-odbc; the universal-driver inherits the same behavior.
    Given Snowflake client is logged in
    When Multistatement SELECT is executed with NULL positional parameters
    Then an error is returned indicating NULL bindings are not supported

  # ============================================================================
  # POSITIONAL PARAMETER BINDING
  # ============================================================================
  # Combines MULTI_STATEMENT_COUNT with parameterized SQL. Bindings are
  # positional and flattened across `;`-separated statements in source order —
  # same contract as legacy snowflake-jdbc PreparedStatement, snowflake-odbc
  # SQLBindParameter, and snowflake-connector-python qmark paramstyle.
  #
  # "Then N result sets are returned" is asserted implicitly by walking the
  # result-set chain to its terminator (-1 update count in JDBC, SQL_NO_DATA in
  # ODBC, nextset() returning None in Python) — there is no separate count
  # check.

  @jdbc_e2e @odbc_e2e @python_e2e
  Scenario: should execute multistatement DML with positional parameters
    Given Snowflake client is logged in
    And A temporary table with column (id NUMBER) exists
    When Multistatement INSERT chain is executed with 3 positional parameters
    Then 2 result sets are returned
    And the first result set reports update count 1
    And the second result set reports update count 2
    And the table contains rows [10, 20, 30]

  @jdbc_e2e @odbc_e2e @python_e2e
  Scenario: should execute multistatement SELECT with positional parameters
    Given Snowflake client is logged in
    When Multistatement SELECT chain is executed with 6 positional parameters
    Then 3 result sets are returned
    And the first result set contains row [10]
    And the second result set contains row [20, 30]
    And the third result set contains row [40, 50, 60]
