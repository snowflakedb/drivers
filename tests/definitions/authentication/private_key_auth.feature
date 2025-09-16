@core_e2e @odbc_e2e
Feature: Private Key Authentication

  @core_e2e @odbc_e2e @python_e2e
  Scenario: should authenticate using private file with password
    Given Authentication is set to JWT and private file with password is provided
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @odbc_e2e @python_e2e
  Scenario: should fail JWT authentication when invalid private key provided
    Given Authentication is set to JWT and invalid private key file is provided
    When Trying to Connect
    Then There is error returned

  @core_int @odbc_int @python_int
  Scenario: should fail JWT authentication when no private file provided
    Given Authentication is set to JWT
    When Trying to Connect with no private file provided
    Then There is error returned