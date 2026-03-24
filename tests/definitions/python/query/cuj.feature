@python
Feature: Critical user journeys

  @python_e2e
  Scenario: desc command
    When DESC SCHEMA command is executed
    Then Schema properties are returned with correct types

  @python_e2e
  Scenario: show command
    When SHOW SCHEMAS command is executed
    Then Result contains INFORMATION_SCHEMA and PUBLIC schemas
