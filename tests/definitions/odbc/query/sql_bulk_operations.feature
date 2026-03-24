@odbc
Feature: ODBC SQLBulkOperations function behavior
  # Tests for SQLBulkOperations based on ODBC specification

  @odbc_e2e
  Scenario: should return IM001 when SQLBulkOperations is called
    Given a query is executed and a row is fetched
    When SQLBulkOperations is called
    Then the driver should report that it does not support this function
