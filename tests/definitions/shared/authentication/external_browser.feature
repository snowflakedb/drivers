@core @python @odbc @jdbc
Feature: External Browser Authentication

  External browser SSO: user configures authenticator as EXTERNALBROWSER
  and UD opens the default browser for IdP login, then captures the token
  via a localhost callback to obtain a Snowflake session.

  # =============================================================================
  # E2E Tests - Real External Browser Authentication (headless browser container)
  # =============================================================================

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should authenticate with external browser via Okta IdP
    Given External browser authentication is configured with valid Okta user
    When Trying to Connect with headless browser providing valid credentials
    Then Login is successful and simple query can be executed

  # =============================================================================
  # Integration Tests - Mocked External Browser Authentication
  # =============================================================================

  @core_int @python_int @odbc_int @jdbc_int
  Scenario: should login with external browser using simulated callback
    Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
    And Login endpoint returns success
    When Trying to Connect with simulated browser callback delivering a token
    Then Login is successful
    And Login request contains EXTERNALBROWSER authenticator, token, proof key, and login name

  @core_int @python_int @odbc_int @jdbc_int
  Scenario: should fail when authenticator-request returns forbidden
    Given Wiremock returns HTTP 403 for authenticator-request
    When Trying to Connect
    Then Connection fails with authenticator error

  @core_int @python_int @odbc_int @jdbc_int
  Scenario: should fail when authenticator-request returns logical failure
    Given Wiremock returns success false for authenticator-request
    When Trying to Connect
    Then Connection fails with authenticator error

  @core_int @python_int @odbc_int @jdbc_int
  Scenario: should fail with timeout when no browser callback arrives
    Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
    And Authentication timeout is set to 2 seconds
    When Trying to Connect without any browser callback
    Then Connection fails with timeout or browser error

  @core_int @python_int @odbc_int @jdbc_int
  Scenario: should fail when login request is rejected after browser callback
    Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
    And Login endpoint returns failure
    When Trying to Connect with simulated browser callback delivering a token
    Then Connection fails with login error

  # =============================================================================
  # Integration Tests - SSO URL validation (SNOW-3649282)
  # =============================================================================

  @core_int
  Scenario: should reject malicious sso url before opening browser
    Given Wiremock returns an ssoUrl containing a shell metacharacter
    When Trying to Connect
    Then Connection fails before the browser is opened

  @core_int
  Scenario: should login when sso url has multi param query string
    Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
    And Login endpoint returns success
    When Trying to Connect with simulated browser callback delivering a token
    Then Login is successful
