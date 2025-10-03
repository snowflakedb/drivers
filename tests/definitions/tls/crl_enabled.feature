@core @odbc @python
Feature: crl_enabled

  @core_e2e @odbc_e2e @python_e2e
  Scenario: Should connect and select with CRL enabled
    Given Snowflake client is logged in
    When Query "SELECT 1" is executed
    Then the request attempt should be successful