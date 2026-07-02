@core @python @odbc @jdbc
Feature: Personal Access Token Authentication

  @core_e2e @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should authenticate using PAT as password
    Given Authentication is set to password and valid PAT token is provided
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should authenticate using PAT as token
    Given Authentication is set to Programmatic Access Token and valid PAT token is provided
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @odbc_e2e @jdbc_e2e
  Scenario: should authenticate using PAT as token with lowercase authenticator
    Given Authentication is set to lowercase programmatic_access_token and valid PAT token is provided
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should fail PAT authentication when invalid token provided
    Given Authentication is set to Programmatic Access Token and invalid PAT token is provided
    When Trying to Connect
    Then There is error returned

  @odbc_e2e
  Scenario: should handle ALTER USER PAT result set: new driver returns token, old driver returns cursor state error
    Given ALTER USER ADD PROGRAMMATIC ACCESS TOKEN is executed
    When SQLFetch is called on the ALTER USER result
    Then The old driver returns invalid cursor state, the new driver returns the token

  # SNOW-3647715: token-based authenticators must not require `user` --
  # the principal is encoded in the token and resolved by GS at login
  # time. This wrapper-level scenario asserts the connector does not
  # reject the connect when `user` is omitted; it currently only exists
  # for Python (other drivers track parity work separately).

  @python_e2e
  Scenario: should authenticate using PAT as token without user
    Given Authentication is set to Programmatic Access Token and valid PAT token is provided
    When Trying to Connect without user
    Then Login is successful and simple query can be executed
