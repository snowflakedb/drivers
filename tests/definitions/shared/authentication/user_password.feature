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

  @core_int
  Scenario: should authenticate with password via wiremock
    Given Wiremock is running and has password login success mapping
    And Snowflake client is configured for password authentication
    When Trying to Connect
    Then Login is successful

  @core_int
  Scenario: should fail authentication when user is not provided
    Given Wiremock is running and has password login success mapping
    And Snowflake client is configured for password authentication without user
    When Trying to Connect
    Then There is error returned with missing parameter

  @core_int
  Scenario: should fail authentication when password is not provided
    Given Wiremock is running and has password login success mapping
    And Snowflake client is configured for password authentication without password
    When Trying to Connect
    Then There is error returned with missing parameter

  @core_int
  Scenario: should fail authentication when password is empty
    Given Wiremock is running and has password login success mapping
    And Snowflake client is configured for password authentication with empty password
    When Trying to Connect
    Then There is error returned with missing parameter

  @core_int
  Scenario: should fail authentication when wrong credentials are provided
    Given Wiremock is running and has password login failure mapping for wrong credentials
    And Snowflake client is configured for password authentication with wrong password
    When Trying to Connect
    Then There is error returned
