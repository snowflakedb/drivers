@core @python @odbc
Feature: Username and Password Authentication

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should authenticate using username and password
    Given Authentication is set to default (snowflake) with valid username and password
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @odbc_e2e
  Scenario: should authenticate using explicit snowflake authenticator
    Given Authentication is explicitly set to snowflake with valid username and password
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should fail authentication when wrong password is provided
    Given Authentication is set to default with valid username and wrong password
    When Trying to Connect
    Then There is error returned
