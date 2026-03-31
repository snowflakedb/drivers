@odbc
Feature: ODBC SQLMoreResults function behavior
  # Tests for SQLMoreResults based on ODBC specification:
  # https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlmoreresults-function

  @odbc_e2e
  Scenario: should return SQL_NO_DATA when there are no more result sets
    Given A query is executed and its result set is fetched
    When SQLMoreResults is called
    Then it should return SQL_NO_DATA (no additional result sets)

  @odbc_e2e
  Scenario: should close cursor so re-execution succeeds without explicit cursor close
    Given A prepared statement is executed and fetched
    And SQLMoreResults is called (which should close the cursor)
    When the same prepared statement is re-executed without explicit SQLCloseCursor / SQLFreeStmt(SQL_CLOSE)
    Then the result should be returned correctly
