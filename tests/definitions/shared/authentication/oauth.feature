@core @odbc @python @jdbc
Feature: OAuth Authentication

  OAuth 2.0 authentication for Snowflake drivers, covering the three
  flows (Authorization Code with PKCE, Client Credentials, and legacy
  pre-acquired access token). Behaviour
  parity with snowflake-jdbc and snowflake-connector-python is the goal;
  the feature description requires the `oauth2` crate as the underlying
  primitive in sf_core.

  # ===========================================================================
  # Wrapper integration tests -- offline; exercise the wrapper's
  # connection-string / kwarg parsing, OAuth-key forwarding,
  # required-parameter validation, and secret redaction without
  # contacting an IdP or Snowflake.
  #
  # Implemented in:
  #   * odbc_tests/tests/integration/authentication/oauth.cpp  (@odbc_int)
  #   * python/tests/integ/authentication/test_oauth.py        (@python_int)
  #   * jdbc/src/test/java/net/snowflake/jdbc/integration/authentication/OauthTests.java (@jdbc_int)
  # ===========================================================================

  @odbc_int @python_int @jdbc_int
  Scenario: should fail OAUTH_CLIENT_CREDENTIALS when client_id is missing
    Given Authentication is set to OAUTH_CLIENT_CREDENTIALS without oauth_client_id
    When Trying to Connect
    Then Connection fails with a missing-parameter error citing oauth_client_id

  @odbc_int @python_int @jdbc_int
  Scenario: should fail OAUTH_CLIENT_CREDENTIALS when client_secret is missing
    Given Authentication is set to OAUTH_CLIENT_CREDENTIALS without oauth_client_secret
    When Trying to Connect
    Then Connection fails with a missing-parameter error citing oauth_client_secret

  @odbc_int @python_int @jdbc_int
  Scenario: should fail OAUTH_CLIENT_CREDENTIALS when token_request_url is missing
    Given Authentication is set to OAUTH_CLIENT_CREDENTIALS without oauth_token_request_url
    When Trying to Connect
    Then Connection fails with a missing-parameter error citing oauth_token_request_url

  @odbc_int @python_int @jdbc_int
  Scenario: should forward AUTHENTICATOR=OAUTH with TOKEN to core
    Given Authentication is set to legacy OAUTH with a pre-acquired access token
    When Trying to Connect
    Then The wrapper forwards the token to sf_core without raising a missing-parameter error for it

  @odbc_int @python_int @jdbc_int
  Scenario: should fail AUTHENTICATOR=OAUTH when TOKEN is missing
    Given Authentication is set to legacy OAUTH without a TOKEN
    When Trying to Connect
    Then Connection fails with a missing-parameter error citing token

  # ===========================================================================
  # ODBC integration tests -- offline; exercise the wrapper's token
  # forwarding for legacy AUTHENTICATOR=OAUTH and OAUTH_CLIENT_SECRET
  # redaction in diagnostics without contacting an IdP or Snowflake.
  # Implemented in odbc_tests/tests/integration/authentication/oauth.cpp.
  # ===========================================================================

  @odbc_int @python_int @jdbc_int
  Scenario: should accept lowercase oauth authenticator value
    Given Authentication is set to lowercase oauth with a TOKEN
    When Trying to Connect
    Then The wrapper does not reject the AUTHENTICATOR value as unknown

  @odbc_int @python_int @jdbc_int
  Scenario: should fail when AUTHENTICATOR is an unknown OAuth-like value
    Given Authentication is set to a typo of an OAuth flow name
    When Trying to Connect
    Then Connection fails with an authenticator-related error

  @odbc_int @python_int @jdbc_int
  Scenario: should not echo OAUTH_CLIENT_SECRET in diagnostics
    Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a distinctive client secret literal
    When Trying to Connect
    Then No diagnostic record contains the literal client secret

  # Python-only OAuth wrapper behaviour (legacy OAUTH `token` literal
  # redaction, `oauth_token_url` / `oauth_socket_uri` alias rewrites, and
  # `oauth_enable_refresh_tokens` / `oauth_credentials_in_body`
  # deprecation warnings) lives in
  # python/tests/integ/authentication/test_oauth_python_specific.py.
  # These behaviours do not have shared Gherkin scenarios because they
  # are not part of the cross-driver OAuth contract.

  # ===========================================================================
  # E2E tests -- require real Snowflake / IdP credentials.
  #
  # Implemented in:
  #   * sf_core/tests/e2e/authentication/oauth.rs                   (@core_e2e)
  #   * odbc_tests/tests/e2e/authentication/oauth.cpp               (@odbc_e2e)
  #   * python/tests/e2e/authentication/test_oauth.py               (@python_e2e)
  #   * jdbc/src/test/java/net/snowflake/jdbc/e2e/authentication/OauthTests.java (@jdbc_e2e)
  #
  # Scenario step text matches the existing sf_core comments verbatim so a
  # single Gherkin definition validates the Rust, ODBC, Python, and JDBC test
  # methods. The Authorization Code happy path additionally requires a
  # real OS browser; in ODBC, Python, and JDBC we gate it behind
  # SNOWFLAKE_OAUTH_E2E_BROWSER=1. The cached-access-token short-circuit
  # scenario depends on the OS keyring helper that lives in sf_core only
  # -- the wrappers do not implement it.
  # ===========================================================================

  @core_e2e @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: oauth should authenticate with pre acquired access token
    Given Authentication is set to legacy OAUTH and a pre-acquired OAuth access token is supplied via `token=`
    When Trying to Connect
    Then Login is successful and a simple query can be executed

  @core_e2e @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: oauth should fail legacy authentication with invalid token
    Given Authentication is set to legacy OAUTH and an invalid OAuth access token is supplied
    When Trying to Connect
    Then Connection fails with an authentication / login error

  @core_e2e @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: oauth should authenticate using authorization code flow
    Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a valid client id / secret. `oauth_authorization_url` and `oauth_token_request_url` are forwarded from parameters when present (otherwise the driver falls back to the Snowflake-IdP defaults `https://{host}/oauth/authorize` and `https://{host}/oauth/token-request`). `client_store_temporary_credential=true` lets the AC flow short-circuit on subsequent runs by re-using the cached access / refresh token (AC state machine: cache → refresh → interactive).
    When Trying to Connect (this will spawn the local-loopback HTTP listener and `xdg-open`/`open`/`ShellExecute` the IdP login URL unless a previously cached access token short-circuits the leg)
    Then Login is successful and a simple query can be executed

  @core_e2e
  Scenario: oauth should short circuit authorization code flow with cached access token
    Given Authentication is set to OAUTH_AUTHORIZATION_CODE and a valid OAuth access token is pre-seeded in the OS keyring under the (host, user, OAUTH_ACCESS_TOKEN) cache key. The host is derived from `oauth_token_request_url` — falling back to the Snowflake server URL — exactly like `host_from_token_url` in production code (prefers IdP token URL host, falls back to Snowflake host).
    When Trying to Connect — should NOT spawn a browser; the pre-seeded access token must satisfy the AC short-circuit.
    Then Login is successful and a simple query can be executed

  @core_e2e @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: oauth should authenticate using client credentials flow
    Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id / secret and an external IdP token URL. Snowflake's GS does not mint CC tokens, so `oauth_token_request_url` is required up-front.
    When Trying to Connect
    Then Login is successful and a simple query can be executed

  @core_e2e @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: oauth should fail authorization code flow with bad client secret
    Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a valid client id but a deliberately invalid client secret. The IdP token-exchange step must reject the credentials and the driver must surface an authentication / login error.
    When Trying to Connect
    Then Connection fails with an authentication / login error

  # Wrapper-only E2E scenarios -- not implemented in sf_core because the
  # Rust e2e harness covers different cases (keyring short-circuit) and
  # the case-insensitive matching of OAUTH is exercised by sf_core unit
  # tests instead.

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: oauth should authenticate using lowercase oauth authenticator
    Given Authentication is set to lowercase oauth and a valid pre-acquired OAuth access token is supplied via TOKEN
    When Trying to Connect
    Then Login is successful and a simple query can be executed

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: oauth should fail client credentials flow with bad client secret
    Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id, an invalid client secret and a valid token_request_url
    When Trying to Connect
    Then Connection fails with an authentication / login error

  # SNOW-3647715: token-based authenticators must not require `user` --
  # the principal is encoded in the IdP-issued token and resolved by GS
  # at login time. These wrapper-level scenarios assert the connector
  # does not reject the connect when `user` is omitted; they currently
  # only exist for Python (other drivers track parity work separately).

  @python_e2e
  Scenario: should authenticate with pre acquired access token without user
    Given Authentication is set to legacy OAUTH and a pre-acquired OAuth access token is supplied via `token=` and user is omitted
    When Trying to Connect without user
    Then Login is successful and a simple query can be executed

  @python_e2e
  Scenario: should authenticate using client credentials flow without user
    Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id / secret and an external IdP token URL and user is omitted
    When Trying to Connect without user
    Then Login is successful and a simple query can be executed
