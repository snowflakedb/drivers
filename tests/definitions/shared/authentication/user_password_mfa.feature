@core @python @odbc
Feature: Username Password MFA Authentication

  @core_e2e @python_e2e
  Scenario: should authenticate using username password and DUO push
    Given Authentication is set to username_password_mfa and user, password are provided and DUO push is enabled
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should authenticate using username password and TOTP passcode
    Given Authentication is set to username_password_mfa and user, password and passcode are provided
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @python_e2e @odbc_e2e
  Scenario: should authenticate using username password with appended TOTP passcode
    Given Authentication is set to username_password_mfa and user, password with appended passcode are provided and passcodeInPassword is set
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @python_e2e
  Scenario: should fail authentication when wrong password is provided
    Given Authentication is set to username_password_mfa and user is provided but password is skipped or invalid
    When Trying to Connect
    Then There is error returned

  @python_e2e
  Scenario: should cache MFA token on first connection
    Given Authentication is set to username_password_mfa with client_store_temporary_credential enabled
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @python_e2e
  Scenario: should reuse cached MFA token without passcode
    Given Authentication is set to username_password_mfa and MFA token has been cached from a previous connection
    When Trying to Connect without passcode
    Then Login is successful and simple query can be executed

