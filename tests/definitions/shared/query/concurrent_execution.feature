@core @odbc
Feature: Concurrent execution

  Concurrent queries return independent results.

  @core_e2e @odbc_e2e
  Scenario: should return independent correct results for overlapping queries on one connection
    Given Snowflake client is logged in
    When 8 queries with distinct markers are executed concurrently on the same connection
    Then each query returns its own marker

  @core_e2e @odbc_e2e
  Scenario: should return independent correct results for overlapping queries on distinct connections
    Given 8 Snowflake clients are logged in
    When 8 queries with distinct markers are executed concurrently on distinct connections
    Then each query returns its own marker

  # ODBC has no execute-hook to assert that two SQLExecDirect calls overlap in
  # time. Independent results for concurrent statements on one connection are
  # covered by the e2e scenario above.
  @core_int @odbc_not_needed
  Scenario: should overlap execute on distinct statements of one connection
    Given Snowflake client is logged in
    When two statements execute concurrently on the same connection
    Then both executions succeed
    And the calls overlap

  # Concurrent SQLExecute on one statement handle is not a supported ODBC
  # application pattern. The driver does not serialize two threads on one HSTMT
  # as a public contract.
  @core_int @odbc_not_needed
  Scenario: should serialize concurrent execute calls on the same statement
    Given Snowflake client is logged in
    When the same statement is executed from 2 threads
    Then both executions succeed
    And the calls run one after another
