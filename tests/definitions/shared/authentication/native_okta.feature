@core @odbc @python
Feature: Native Okta Authentication

  Native SSO through Okta (SAML): user configures authenticator as an Okta URL
  and UD performs the Okta/SAML exchange to obtain a Snowflake session.

  # =============================================================================
  # E2E Tests - Real Okta Authentication
  # =============================================================================

  @core_e2e @odbc_e2e @python_e2e
  Scenario: should authenticate using native okta
    Given Okta authentication is configured with valid credentials
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e @odbc_e2e @python_e2e
  Scenario: should fail native okta authentication with wrong credentials
    Given Okta authentication is configured with wrong password
    When Trying to Connect
    Then Connection fails with authentication error

  @core_e2e @odbc_e2e @python_e2e
  Scenario: should fail native okta authentication with wrong okta url
    Given Okta authentication is configured with invalid okta url
    When Trying to Connect
    Then Connection fails with authentication error
