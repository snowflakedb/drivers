@odbc
Feature: ODBC SQLSetPos function behavior
  # Tests for SQLSetPos based on ODBC specification

  @odbc_e2e
  Scenario: should return IM001 when SQLSetPos is called
    Given a query is executed and a row is fetched
    When SQLSetPos is called
    Then the driver should report that it does not support this function
