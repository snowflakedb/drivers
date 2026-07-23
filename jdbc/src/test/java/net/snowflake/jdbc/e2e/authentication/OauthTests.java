package net.snowflake.jdbc.e2e.authentication;

import static net.snowflake.jdbc.utils.DriverCompatibility.isNewDriver;
import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.jdbc.utils.RequiresBrowser;
import net.snowflake.jdbc.utils.TestParameters;
import net.snowflake.jdbc.utils.WithConnect;
import net.snowflake.jdbc.utils.WithQueryUtils;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.function.Executable;

@RequiresBrowser
class OauthTests implements WithQueryUtils, WithConnect, WithOauthAccessToken {

  // ===========================================================================
  // Legacy AUTHENTICATOR=OAUTH (pre-acquired access token)
  //
  // A fresh OAuth access token is minted from the Okta IdP and passed via
  // `token=`; it is presented to Snowflake as-is.
  // ===========================================================================

  @Nested
  class LegacyOauth {

    private final String TOKEN_URL = TestParameters.get("SNOWFLAKE_TEST_OKTA_OAUTH_TOKEN_URL");
    private final String CLIENT_ID = TestParameters.get("SNOWFLAKE_TEST_OKTA_OAUTH_CLIENT_ID");
    private final String CLIENT_SECRET =
        TestParameters.get("SNOWFLAKE_TEST_OKTA_OAUTH_CLIENT_SECRET");
    private final String USER = TestParameters.get("SNOWFLAKE_TEST_OKTA_USER");
    private final String PASSWORD = TestParameters.get("SNOWFLAKE_TEST_OKTA_PASSWORD");
    private final String ROLE = TestParameters.get("SNOWFLAKE_TEST_ROLE");

    @Test
    void oauthShouldAuthenticateWithPreAcquiredAccessToken() throws Exception {
      // Given Authentication is set to legacy OAUTH and a pre-acquired OAuth access token is
      // supplied via `token=`
      Properties props = loadDefaultConnectionProperties();
      props.setProperty("authenticator", "OAUTH");
      props.setProperty("user", USER);
      props.setProperty(
          "token",
          retrieveOauthAccessToken(TOKEN_URL, CLIENT_ID, CLIENT_SECRET, USER, PASSWORD, ROLE));

      // When Trying to Connect
      try (Connection conn = connect(props)) {
        // Then Login is successful and a simple query can be executed
        assertSimpleQuerySucceeds(conn);
      }
    }

    @Test
    void oauthShouldAuthenticateUsingLowercaseOauthAuthenticator() throws Exception {
      // Given Authentication is set to lowercase oauth and a valid pre-acquired OAuth access token
      // is supplied via TOKEN
      Properties props = loadDefaultConnectionProperties();
      props.setProperty("authenticator", "oauth");
      props.setProperty("user", USER);
      props.setProperty(
          "token",
          retrieveOauthAccessToken(TOKEN_URL, CLIENT_ID, CLIENT_SECRET, USER, PASSWORD, ROLE));

      // When Trying to Connect
      try (Connection conn = connect(props)) {
        // Then Login is successful and a simple query can be executed
        assertSimpleQuerySucceeds(conn);
      }
    }

    @Test
    @Disabled("Bad-secret tests cause pipeline flakiness by blocking the test account")
    void oauthShouldFailLegacyAuthenticationWithInvalidToken() {
      // Given Authentication is set to legacy OAUTH and an invalid OAuth access token is supplied
      Properties props = loadDefaultConnectionProperties();
      props.setProperty("authenticator", "OAUTH");
      props.setProperty("user", USER);
      props.setProperty("token", "invalid_oauth_token_12345");

      // When Trying to Connect
      Executable connect = () -> connect(props);

      // Then Connection fails with an authentication / login error
      SQLException exception = assertThrows(SQLException.class, connect);
      if (isNewDriver()) {
        assertTrue(exception.getMessage().toLowerCase().contains("invalid oauth access token"));
        assertTrue(exception.getMessage().contains("390303"));
      }
    }
  }

  // ===========================================================================
  // OAuth Authorization Code (AC) flow
  //
  // An interactive, user-based flow that authenticates a real user through a
  // browser login leg. The connect thread spawns Chromium via the OS browser
  // opener; the browser thread drives the Snowflake IdP login over Chromium's
  // remote-debugging port.
  // ===========================================================================

  @Nested
  class AuthorizationCodeFlow implements WithBrowserAutomation {

    private final String USER = TestParameters.get("SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_USER");
    private final String PASSWORD = TestParameters.get("SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_PASSWORD");
    private final String CLIENT_ID = TestParameters.get("SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_CLIENT_ID");
    private final String CLIENT_SECRET =
        TestParameters.get("SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_CLIENT_SECRET");
    private final String REDICTED_URI =
        TestParameters.get("SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_REDIRECT_URI");

    @Test
    void oauthShouldAuthenticateUsingAuthorizationCodeFlow() throws Exception {
      // Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a valid client id / secret.
      // `oauth_authorization_url` and `oauth_token_request_url` are forwarded from parameters when
      // present (otherwise the driver falls back to the Snowflake-IdP defaults
      // `https://{host}/oauth/authorize` and `https://{host}/oauth/token-request`).
      // `client_store_temporary_credential=true` lets the AC flow short-circuit on subsequent runs
      // by re-using the cached access / refresh token (AC state machine: cache → refresh →
      // interactive).
      Properties props = loadDefaultConnectionProperties();
      props.setProperty("authenticator", "OAUTH_AUTHORIZATION_CODE");
      props.setProperty("user", USER);
      props.setProperty("oauthClientId", CLIENT_ID);
      props.setProperty("oauthClientSecret", CLIENT_SECRET);
      props.setProperty("oauthRedirectUri", REDICTED_URI);

      cleanBrowserProcesses();
      try {
        // When Trying to Connect (this will spawn the local-loopback HTTP listener and
        // `xdg-open`/`open`/`ShellExecute` the IdP login URL unless a previously cached access
        // token short-circuits the leg)
        try (Connection conn =
            connectWithBrowserAutomation(
                () -> connect(props), "internalOauthSnowflakeSuccess", USER, PASSWORD)) {
          // Then Login is successful and a simple query can be executed
          assertSimpleQuerySucceeds(conn);
        }
      } finally {
        cleanBrowserProcesses();
      }
    }

    @Test
    @Disabled("Bad-secret tests cause pipeline flakiness by blocking the test account")
    void oauthShouldFailAuthorizationCodeFlowWithBadClientSecret() {
      // Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a valid client id but a
      // deliberately invalid client secret. The IdP token-exchange step must reject the credentials
      // and the driver must surface an authentication / login error.
      Properties props = loadDefaultConnectionProperties();
      props.setProperty("authenticator", "OAUTH_AUTHORIZATION_CODE");
      props.setProperty("user", USER);
      props.setProperty("oauthClientId", CLIENT_ID);
      props.setProperty("oauthClientSecret", "invalid_client_secret_12345");
      props.setProperty("oauthRedirectUri", REDICTED_URI);

      cleanBrowserProcesses();
      try {
        // When Trying to Connect
        Executable connect =
            () ->
                connectWithBrowserAutomation(
                    () -> connect(props), "internalOauthSnowflakeSuccess", USER, PASSWORD);

        // Then Connection fails with an authentication / login error
        SQLException exception = assertThrows(SQLException.class, connect);
        if (isNewDriver()) {
          assertTrue(exception.getMessage().toLowerCase().contains("invalid_client"));
        }
      } finally {
        cleanBrowserProcesses();
      }
    }
  }

  // ===========================================================================
  // OAuth Client Credentials (CC) flow
  //
  // A non-interactive, machine-to-machine flow where an external IdP mints the
  // token from a client id / secret. Snowflake's GS does not mint CC tokens, so
  // `oauth_token_request_url` is required up-front.
  // ===========================================================================

  @Nested
  class ClientCredentialsFlow {

    private final String CLIENT_ID =
        TestParameters.get("SNOWFLAKE_TEST_OKTA_OAUTH_EXTERNAL_CLIENT_ID");
    private final String CLIENT_SECRET =
        TestParameters.get("SNOWFLAKE_TEST_OKTA_OAUTH_EXTERNAL_CLIENT_SECRET");
    private final String TOKEN_URL = TestParameters.get("SNOWFLAKE_TEST_OKTA_OAUTH_TOKEN_URL");

    @Test
    void oauthShouldAuthenticateUsingClientCredentialsFlow() throws Exception {
      // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id / secret and
      // an external IdP token URL. Snowflake's GS does not mint CC tokens, so
      // `oauth_token_request_url` is required up-front.
      Properties props = loadDefaultConnectionProperties();
      props.setProperty("authenticator", "OAUTH_CLIENT_CREDENTIALS");
      props.setProperty("user", CLIENT_ID);
      props.setProperty("oauthClientId", CLIENT_ID);
      props.setProperty("oauthClientSecret", CLIENT_SECRET);
      props.setProperty("oauthTokenRequestUrl", TOKEN_URL);

      // When Trying to Connect
      try (Connection conn = connect(props)) {
        // Then Login is successful and a simple query can be executed
        assertSimpleQuerySucceeds(conn);
      }
    }

    @Test
    @Disabled("Bad-secret tests cause pipeline flakiness by blocking the test account")
    void oauthShouldFailClientCredentialsFlowWithBadClientSecret() {
      // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id, an invalid
      // client secret and a valid token_request_url
      Properties props = loadDefaultConnectionProperties();
      props.setProperty("authenticator", "OAUTH_CLIENT_CREDENTIALS");
      props.setProperty("user", CLIENT_ID);
      props.setProperty("oauthClientId", CLIENT_ID);
      props.setProperty("oauthClientSecret", "invalid_client_secret_12345");
      props.setProperty("oauthTokenRequestUrl", TOKEN_URL);

      // When Trying to Connect
      Executable connect = () -> connect(props);

      // Then Connection fails with an authentication / login error
      SQLException exception = assertThrows(SQLException.class, connect);
      if (isNewDriver()) {
        assertTrue(exception.getMessage().toLowerCase().contains("invalid_client"));
      }
    }
  }
}
