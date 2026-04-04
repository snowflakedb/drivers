@odbc
Feature: Username Password MFA Authentication (ODBC-specific)

  @odbc_e2e
  Scenario: should authenticate using user password mfa with passcode in password
    Given MFA authentication is configured with valid credentials
    When Trying to Connect
    Then Login is successful

  @odbc_e2e
  Scenario: should authenticate using user password mfa with passcode explicit
    Given MFA authentication is configured with valid credentials
    When Trying to Connect
    Then Login is successful

  @odbc_int
  Scenario: should forward USERNAME_PASSWORD_MFA parameters to core
    Given Authentication is set to USERNAME_PASSWORD_MFA with user and password
    When Trying to Connect
    Then Connection reaches sf_core without a missing-parameter error for MFA fields

  @odbc_int
  Scenario: should forward PASSCODE parameter to core
    Given Authentication is set to USERNAME_PASSWORD_MFA with a TOTP passcode
    When Trying to Connect
    Then Connection reaches sf_core without a missing-parameter error

  @odbc_int
  Scenario: should forward PASSCODEINPASSWORD parameter to core
    Given Authentication is set to USERNAME_PASSWORD_MFA with passcode appended to password
    When Trying to Connect
    Then Connection reaches sf_core without a missing-parameter error

  @odbc_int
  Scenario: should fail MFA authentication when password is not provided
    Given Authentication is set to USERNAME_PASSWORD_MFA but password is omitted
    When Trying to Connect
    Then Connection fails with a missing-parameter error

  @odbc_int
  Scenario: should forward CLIENT_STORE_TEMPORARY_CREDENTIAL parameter to core
    Given Authentication is set to USERNAME_PASSWORD_MFA with token caching enabled
    When Trying to Connect
    Then Connection reaches sf_core without a missing-parameter error
