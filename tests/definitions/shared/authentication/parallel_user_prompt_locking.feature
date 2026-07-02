@core @odbc @python
Feature: Parallel User Prompt Locking

  When a connection pool opens multiple connections concurrently and interactive
  authentication is required (external browser, MFA, OAuth authorization code),
  the driver must serialize the prompts so the user sees only one prompt rather
  than one per concurrent connection.

  Locking is engaged when `clientStoreTemporaryCredential=true` and
  `DISABLE_PARALLEL_USER_PROMPT=true` (the default).

  @core_int @odbc_int @python_e2e
  Scenario: should show only one external browser prompt when multiple connections authenticate concurrently
    Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is true
    And Wiremock returns valid ssoUrl and proofKey for authenticator-request
    And Login endpoint returns success
    When Multiple connections attempt external browser login concurrently
    Then Only one authenticator-request is sent to the server
    And All connections succeed

  @core_int @odbc_int @python_e2e
  Scenario: should show only one MFA prompt when multiple connections authenticate concurrently
    Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is true
    And Wiremock returns successful login with MFA token for the first connection
    When Multiple connections attempt username_password_mfa login concurrently
    Then Only one interactive MFA login-request is sent to the server
    And All connections succeed using the cached MFA token

  @core_int @odbc_int @python_e2e
  Scenario: should show independent prompts when DISABLE_PARALLEL_USER_PROMPT is false
    Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is false
    And Wiremock returns valid ssoUrl and proofKey for each authenticator-request
    And Login endpoint returns success
    When Multiple connections attempt external browser login concurrently
    Then Each connection sends its own authenticator-request to the server
    And All connections succeed independently

  @core_int @odbc_int @python_e2e
  Scenario: should show independent prompts when clientStoreTemporaryCredential is false
    Given clientStoreTemporaryCredential is disabled and DISABLE_PARALLEL_USER_PROMPT is true
    And Wiremock returns valid ssoUrl and proofKey for each authenticator-request
    And Login endpoint returns success
    When Multiple connections attempt external browser login concurrently
    Then Each connection sends its own authenticator-request to the server
    And All connections succeed independently

  @core_int
  Scenario: should release the lock when the first connection login fails so the waiting connection can authenticate independently
    Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is true
    And Wiremock returns valid ssoUrl and proofKey for each authenticator-request
    And Login endpoint returns failure for the first connection's browser token
    And Login endpoint returns success for the second connection's browser token
    When Multiple connections attempt external browser login concurrently
    Then The first connection fails with an authentication error
    And The second connection acquires the released lock and succeeds
    And Two authenticator-requests were sent to the server

  @core_int
  Scenario: should release the lock when the browser callback times out so the waiting connection can authenticate independently
    Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is true
    And authentication_timeout is configured to a short duration
    And Wiremock returns valid ssoUrl and proofKey for each authenticator-request
    And Login endpoint returns success
    When Multiple connections attempt external browser login concurrently
    And The browser callback is never delivered to the first connection
    Then The first connection fails with a timeout error
    And The second connection acquires the released lock and succeeds
    And Two authenticator-requests were sent to the server

  @core_int
  Scenario: should show only one OAuth authorization code IdP exchange when multiple connections authenticate concurrently
    Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is true
    And A refresh token is seeded in the cache to bypass the interactive browser leg
    And IdP token endpoint returns a fresh access token on refresh_token exchange
    And Snowflake login endpoint returns success for OAuth
    When Multiple connections attempt OAuth authorization code login concurrently
    Then Only one IdP token exchange is performed
    And All connections succeed using the cached access token
