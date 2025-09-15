@core @python
Feature: Personal Access Token Authentication

  @core @python
  Scenario: should authenticate using PAT as password
    Given Authentication is set to password and valid PAT token is provided
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core @python
  Scenario: should authenticate using PAT as token
    Given Authentication is set to Programmatic Access Token and valid PAT token is provided
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core @python
  Scenario: should fail PAT authentication when invalid token provided
    Given Authentication is set to Programmatic Access Token and invalid PAT token is provided
    When Trying to Connect
    Then There is error returned