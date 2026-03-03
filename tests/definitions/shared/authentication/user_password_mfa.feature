@core_e2e
Feature: Username Password MFA Authentication

  @core_e2e 
  Scenario: should authenticate using username password and DUO push
    Given Authentication is set to username_password_mfa and user, password are provided and DUO push is enabled
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e 
  Scenario: should authenticate using username password and TOTP passcode
    Given Authentication is set to username_password_mfa and user, password and passcode are provided
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e 
  Scenario: should authenticate using username password with appended TOTP passcode
    Given Authentication is set to username_password_mfa and user, password with appended passcode are provided and passcodeInPassword is set
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e
  Scenario: should fail authentication when wrong password is provided
    Given Authentication is set to username_password_mfa and user is provided but password is skipped or invalid
    When Trying to Connect
    Then There is error returned

  @core_int
  Scenario: should fail authentication when user is not provided
    Given Authentication is set to username_password_mfa and user is not provided
    When Trying to Connect
    Then There is error returned

  @core_int
  Scenario: should fail authentication when password is not provided
    Given Authentication is set to username_password_mfa and password is not provided
    When Trying to Connect
    Then There is error returned

  @core_int
  Scenario: should fail authentication when passcodeInPassword is not set but passcode is appended to password
    Given Authentication is set to username_password_mfa and user, password with appended passcode are provided and passcodeInPassword is not set 
    When Trying to Connect
    Then There is error returned

  @core_int
  Scenario: should fail authentication when passcodeInPassword is not set but passcode is appended to password
    Given Authentication is set to username_password_mfa and user, password with appended passcode are provided and passcodeInPassword is not set 
    When Trying to Connect
    Then There is error returned

  @core_int
  Scenario: should fail authentication when passcodeInPassword is set but passcode is not appended to password
    Given Authentication is set to username_password_mfa and user, password are provided and passcodeInPassword is set but passcode is not appended to password 
    When Trying to Connect
    Then There is error returned

