@odbc_e2e @python_e2e
Feature: crl_enabled

  Scenario: connect and select with CRL enabled
    Given Snowflake client is logged in
    When Query "SELECT 1" is executed
    Then the request attempt should complete


