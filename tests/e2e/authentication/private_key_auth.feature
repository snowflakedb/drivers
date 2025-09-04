@core @odbc
Feature: Private Key Authentication

  @core @odbc
  Scenario: should authenticate using private file with password
    Given Authentication is set to JWT
    And Private file with password is provided
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core @odbc
  Scenario: should fail JWT authentication when no private file provided
    Given Authentication is set to JWT
    When Trying to Connect with no private file provided
    Then There is error returned
