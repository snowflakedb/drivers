@core
Feature: Native Okta Authentication

  Native SSO through Okta (SAML): user configures authenticator as an Okta URL
  and UD performs the Okta/SAML exchange to obtain a Snowflake session.

  # =============================================================================
  # E2E Tests - Real Okta Authentication
  # =============================================================================

  @core_e2e
  Scenario: should authenticate using native okta
    Given Okta authentication is configured with valid credentials
    When Trying to Connect
    Then Login is successful and simple query can be executed

  @core_e2e
  Scenario: should fail native okta authentication with wrong credentials
    Given Okta authentication is configured with wrong password
    When Trying to Connect
    Then Connection fails with authentication error

  @core_e2e
  Scenario: should fail native okta authentication with wrong okta url
    Given Okta authentication is configured with invalid okta url
    When Trying to Connect
    Then Connection fails with authentication error

  # =============================================================================
  # Basic Authentication Flow - WireMock Integration
  # =============================================================================

  @core_int
  Scenario: should login with native okta using saml flow
    Given Wiremock is running
    And Wiremock has Snowflake and Okta mappings
    And Snowflake client is configured for native Okta
    And TLS certificate verification is disabled for the Okta HTTPS mock
    When Trying to Connect
    Then Login is successful

  # =============================================================================
  # Error Handling - Invalid Credentials
  # =============================================================================

  @core_int
  Scenario: should fail with bad credentials when okta returns 401
    Given Wiremock is running
    And Wiremock has Snowflake authenticator-request mapping
    And Wiremock has Okta token endpoint returning 401 Unauthorized
    And Snowflake client is configured for native Okta
    And TLS certificate verification is disabled for the Okta HTTPS mock
    When Trying to Connect
    Then Connection fails with bad credentials error

  @core_int
  Scenario: should fail with bad credentials when okta returns 403
    Given Wiremock is running
    And Wiremock has Snowflake authenticator-request mapping
    And Wiremock has Okta token endpoint returning 403 Forbidden
    And Snowflake client is configured for native Okta
    And TLS certificate verification is disabled for the Okta HTTPS mock
    When Trying to Connect
    Then Connection fails with bad credentials error

  # =============================================================================
  # Error Handling - MFA Required
  # =============================================================================

  @core_int
  Scenario: should fail when okta returns MFA_REQUIRED status
    Given Wiremock is running
    And Wiremock has Snowflake authenticator-request mapping
    And Wiremock has Okta token endpoint returning MFA_REQUIRED status
    And Snowflake client is configured for native Okta
    And TLS certificate verification is disabled for the Okta HTTPS mock
    When Trying to Connect
    Then Connection fails with MFA required error

  # =============================================================================
  # IdP URL Validation
  # =============================================================================

  @core_int
  Scenario: should fail when tokenUrl does not match configured okta url origin
    Given Wiremock is running
    And Wiremock has Snowflake authenticator-request with mismatched tokenUrl
    And Snowflake client is configured for native Okta
    And TLS certificate verification is disabled for the Okta HTTPS mock
    When Trying to Connect
    Then Connection fails with IdP URL mismatch error

  @core_int
  Scenario: should fail when ssoUrl does not match configured okta url origin
    Given Wiremock is running
    And Wiremock has Snowflake authenticator-request with mismatched ssoUrl
    And Snowflake client is configured for native Okta
    And TLS certificate verification is disabled for the Okta HTTPS mock
    When Trying to Connect
    Then Connection fails with IdP URL mismatch error

  # =============================================================================
  # SAML Postback Validation
  # =============================================================================

  @core_int
  Scenario: should fail when saml postback url does not match snowflake server
    Given Wiremock is running
    And Wiremock has Snowflake authenticator-request mapping
    And Wiremock has Okta token success mapping
    And Wiremock has Okta SSO returning SAML with mismatched postback URL
    And Snowflake client is configured for native Okta
    And TLS certificate verification is disabled for the Okta HTTPS mock
    When Trying to Connect
    Then Connection fails with SAML destination mismatch error

  @core_int
  Scenario: should succeed with mismatched postback when disable_saml_url_check is true
    Given Wiremock is running
    And Wiremock has Snowflake authenticator-request mapping
    And Wiremock has Okta token success mapping
    And Wiremock has Okta SSO returning SAML with mismatched postback URL
    And Wiremock has Snowflake login success for Okta
    And Snowflake client is configured for native Okta with disable_saml_url_check
    And TLS certificate verification is disabled for the Okta HTTPS mock
    When Trying to Connect
    Then Login is successful

  @core_int
  Scenario: should fail when saml html is missing form action
    Given Wiremock is running
    And Wiremock has Snowflake authenticator-request mapping
    And Wiremock has Okta token success mapping
    And Wiremock has Okta SSO returning SAML HTML without form action
    And Snowflake client is configured for native Okta
    And TLS certificate verification is disabled for the Okta HTTPS mock
    When Trying to Connect
    Then Connection fails with missing SAML postback error

  # =============================================================================
  # Token Handling
  # =============================================================================

  @core_int
  Scenario: should use cookieToken when sessionToken is missing
    Given Wiremock is running
    And Wiremock has Snowflake authenticator-request mapping
    And Wiremock has Okta token endpoint returning cookieToken instead of sessionToken
    And Wiremock has Okta SSO success mapping
    And Wiremock has Snowflake login success for Okta
    And Snowflake client is configured for native Okta
    And TLS certificate verification is disabled for the Okta HTTPS mock
    When Trying to Connect
    Then Login is successful

  # =============================================================================
  # Retry Behavior - Token Refresh on Transient Errors
  # =============================================================================

  @core_int
  Scenario: should retry saml fetch with fresh token on transient error
    Given Wiremock is running
    And Wiremock has Snowflake authenticator-request mapping
    And Wiremock has Okta token success mapping
    And Wiremock has Okta SSO returning 503 on first attempt
    And Wiremock has Okta SSO returning success on retry
    And Wiremock has Snowflake login success for Okta
    And Snowflake client is configured for native Okta
    And TLS certificate verification is disabled for the Okta HTTPS mock
    When Trying to Connect
    Then Login is successful
