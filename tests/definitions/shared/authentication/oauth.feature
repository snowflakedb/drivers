@core @odbc
Feature: OAuth Authentication

  OAuth 2.0 authentication for Snowflake drivers, covering the three
  flows (Authorization Code with PKCE, Client Credentials, and legacy
  pre-acquired access token). Behaviour
  parity with snowflake-jdbc and snowflake-connector-python is the goal;
  the feature description requires the `oauth2` crate as the underlying
  primitive in sf_core.

  # ===========================================================================
  # ODBC integration tests -- offline; exercise the wrapper's token
  # forwarding for legacy AUTHENTICATOR=OAUTH and OAUTH_CLIENT_SECRET
  # redaction in diagnostics without contacting an IdP or Snowflake.
  # Implemented in odbc_tests/tests/integration/authentication/oauth.cpp.
  # ===========================================================================

  @odbc_int
  Scenario: should forward AUTHENTICATOR=OAUTH with TOKEN to core
    Given Authentication is set to legacy OAUTH with a pre-acquired access token
    When Trying to Connect
    Then The wrapper forwards the token to sf_core without raising a missing-parameter error for it

  @odbc_int
  Scenario: should fail AUTHENTICATOR=OAUTH when TOKEN is missing
    Given Authentication is set to legacy OAUTH without a TOKEN
    When Trying to Connect
    Then Connection fails with a missing-parameter error citing token

  @odbc_int
  Scenario: should not echo OAUTH_CLIENT_SECRET in diagnostics
    Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a distinctive client secret literal
    When Trying to Connect
    Then No diagnostic record contains the literal client secret

  # ===========================================================================
  # E2E tests -- require real Snowflake / IdP credentials. Implemented in
  # sf_core/tests/e2e/authentication/oauth.rs and (where applicable to the
  # ODBC wrapper) odbc_tests/tests/e2e/authentication/oauth.cpp.
  #
  # Scenario step text matches the existing sf_core comments verbatim so a
  # single Gherkin definition validates both the Rust and ODBC test
  # methods. The Authorization Code happy path additionally requires a
  # real OS browser; in ODBC we gate it behind SNOWFLAKE_OAUTH_E2E_BROWSER=1.
  # The cached-access-token short-circuit scenario depends on the OS
  # keyring helper that lives in sf_core only -- ODBC does not implement
  # it.
  # ===========================================================================

  @core_e2e @odbc_e2e
  Scenario: oauth should authenticate with pre acquired access token
    Given Authentication is set to legacy OAUTH and a pre-acquired OAuth access token is supplied via `token=`
    When Trying to Connect
    Then Login is successful and a simple query can be executed

  @core_e2e @odbc_e2e
  Scenario: oauth should fail legacy authentication with invalid token
    Given Authentication is set to legacy OAUTH and an invalid OAuth access token is supplied
    When Trying to Connect
    Then Connection fails with an authentication / login error

  @core_e2e @odbc_e2e
  Scenario: oauth should authenticate using authorization code flow
    Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a valid client id / secret. `oauth_authorization_url` and `oauth_token_request_url` are forwarded from parameters when present (otherwise the driver falls back to the Snowflake-IdP defaults `https://{host}/oauth/authorize` and `https://{host}/oauth/token-request`). `client_store_temporary_credential=true` lets the AC flow short-circuit on subsequent runs by re-using the cached access / refresh token (AC state machine: cache → refresh → interactive).
    When Trying to Connect (this will spawn the local-loopback HTTP listener and `xdg-open`/`open`/`ShellExecute` the IdP login URL unless a previously cached access token short-circuits the leg)
    Then Login is successful and a simple query can be executed

  @core_e2e
  Scenario: oauth should short circuit authorization code flow with cached access token
    Given Authentication is set to OAUTH_AUTHORIZATION_CODE and a valid OAuth access token is pre-seeded in the OS keyring under the (host, user, OAUTH_ACCESS_TOKEN) cache key. The host is derived from `oauth_token_request_url` — falling back to the Snowflake server URL — exactly like `host_from_token_url` in production code (prefers IdP token URL host, falls back to Snowflake host).
    When Trying to Connect — should NOT spawn a browser; the pre-seeded access token must satisfy the AC short-circuit.
    Then Login is successful and a simple query can be executed

  @core_e2e @odbc_e2e
  Scenario: oauth should authenticate using client credentials flow
    Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id / secret and an external IdP token URL. Snowflake's GS does not mint CC tokens, so `oauth_token_request_url` is required up-front.
    When Trying to Connect
    Then Login is successful and a simple query can be executed

  @core_e2e @odbc_e2e
  Scenario: oauth should fail authorization code flow with bad client secret
    Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a valid client id but a deliberately invalid client secret. The IdP token-exchange step must reject the credentials and the driver must surface an authentication / login error.
    When Trying to Connect
    Then Connection fails with an authentication / login error

  # ODBC-only E2E scenarios -- not implemented in sf_core because the
  # Rust e2e harness covers different cases (keyring short-circuit).

  @odbc_e2e
  Scenario: oauth should fail client credentials flow with bad client secret
    Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id, an invalid client secret and a valid token_request_url
    When Trying to Connect
    Then Connection fails with an authentication / login error
