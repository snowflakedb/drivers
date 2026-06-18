package net.snowflake.jdbc.e2e.authentication;

import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.sql.Connection;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.jdbc.utils.TestParameters;
import net.snowflake.jdbc.utils.WithConnect;
import net.snowflake.jdbc.utils.WithQueryUtils;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.function.Executable;

class OauthTests implements WithQueryUtils, WithConnect {

  private static final String USER = TestParameters.get("SNOWFLAKE_TEST_USER");

  @Disabled("TODO: SNOW-2872392 - requires SNOWFLAKE_TEST_OAUTH_* parameters in parameters.json")
  @Test
  void oauthShouldAuthenticateWithPreAcquiredAccessToken() throws Exception {
    // Given Authentication is set to legacy OAUTH and a pre-acquired OAuth access token is supplied
    // via `token=`
    Properties props = oauthConnectionProperties("OAUTH");
    props.setProperty("token", TestParameters.get("SNOWFLAKE_TEST_OAUTH_ACCESS_TOKEN"));

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and a simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void oauthShouldFailLegacyAuthenticationWithInvalidToken() {
    // Given Authentication is set to legacy OAUTH and an invalid OAuth access token is supplied
    Properties props = oauthConnectionProperties("OAUTH");
    props.setProperty("token", "invalid_oauth_token_12345");

    // When Trying to Connect
    Executable connect = () -> connect(props);

    // Then Connection fails with an authentication / login error
    assertThrows(SQLException.class, connect);
  }

  @Disabled("TODO: SNOW-2872392 - OAuth authorization code E2E spawns a real OS browser")
  @Test
  void oauthShouldAuthenticateUsingAuthorizationCodeFlow() throws Exception {
    // Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a valid client id / secret.
    // `oauth_authorization_url` and `oauth_token_request_url` are forwarded from parameters when
    // present (otherwise the driver falls back to the Snowflake-IdP defaults
    // `https://{host}/oauth/authorize` and `https://{host}/oauth/token-request`).
    // `client_store_temporary_credential=true` lets the AC flow short-circuit on subsequent runs by
    // re-using the cached access / refresh token (AC state machine: cache → refresh → interactive).
    Properties props = oauthConnectionProperties("OAUTH_AUTHORIZATION_CODE");
    props.setProperty("oauth_client_id", TestParameters.get("SNOWFLAKE_TEST_OAUTH_CLIENT_ID"));
    props.setProperty(
        "oauth_client_secret", TestParameters.get("SNOWFLAKE_TEST_OAUTH_CLIENT_SECRET"));
    props.setProperty("clientStoreTemporaryCredential", "true");
    props.setProperty(
        "oauth_authorization_url", TestParameters.get("SNOWFLAKE_TEST_OAUTH_AUTHORIZATION_URL"));
    props.setProperty(
        "oauth_token_request_url", TestParameters.get("SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL"));
    props.setProperty(
        "oauth_redirect_uri", TestParameters.get("SNOWFLAKE_TEST_OAUTH_REDIRECT_URI"));
    props.setProperty("oauth_scope", TestParameters.get("SNOWFLAKE_TEST_OAUTH_SCOPE"));

    // When Trying to Connect (this will spawn the local-loopback HTTP listener and
    // `xdg-open`/`open`/`ShellExecute` the IdP login URL unless a previously cached access token
    // short-circuits the leg)
    try (Connection conn = connect(props)) {
      // Then Login is successful and a simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Disabled("TODO: SNOW-2872392 - requires SNOWFLAKE_TEST_OAUTH_* parameters in parameters.json")
  @Test
  void oauthShouldAuthenticateUsingClientCredentialsFlow() throws Exception {
    // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id / secret and
    // an external IdP token URL. Snowflake's GS does not mint CC tokens, so
    // `oauth_token_request_url` is required up-front.
    Properties props = oauthConnectionProperties("OAUTH_CLIENT_CREDENTIALS");
    props.setProperty("oauth_client_id", TestParameters.get("SNOWFLAKE_TEST_OAUTH_CLIENT_ID"));
    props.setProperty(
        "oauth_client_secret", TestParameters.get("SNOWFLAKE_TEST_OAUTH_CLIENT_SECRET"));
    props.setProperty(
        "oauth_token_request_url", TestParameters.get("SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL"));
    props.setProperty("oauth_scope", TestParameters.get("SNOWFLAKE_TEST_OAUTH_SCOPE"));

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and a simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Disabled("TODO: SNOW-2872392 - OAuth authorization code E2E spawns a real OS browser")
  @Test
  void oauthShouldFailAuthorizationCodeFlowWithBadClientSecret() {
    // Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a valid client id but a
    // deliberately invalid client secret. The IdP token-exchange step must reject the credentials
    // and the driver must surface an authentication / login error.
    Properties props = oauthConnectionProperties("OAUTH_AUTHORIZATION_CODE");
    props.setProperty("oauth_client_id", TestParameters.get("SNOWFLAKE_TEST_OAUTH_CLIENT_ID"));
    props.setProperty("oauth_client_secret", "invalid_client_secret_12345");
    props.setProperty("clientStoreTemporaryCredential", "false");
    props.setProperty(
        "oauth_authorization_url", TestParameters.get("SNOWFLAKE_TEST_OAUTH_AUTHORIZATION_URL"));
    props.setProperty(
        "oauth_token_request_url", TestParameters.get("SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL"));
    props.setProperty(
        "oauth_redirect_uri", TestParameters.get("SNOWFLAKE_TEST_OAUTH_REDIRECT_URI"));
    props.setProperty("oauth_scope", TestParameters.get("SNOWFLAKE_TEST_OAUTH_SCOPE"));

    // When Trying to Connect
    Executable connect = () -> connect(props);

    // Then Connection fails with an authentication / login error
    assertThrows(SQLException.class, connect);
  }

  @Disabled("TODO: SNOW-2872392 - requires SNOWFLAKE_TEST_OAUTH_* parameters in parameters.json")
  @Test
  void oauthShouldAuthenticateUsingLowercaseOauthAuthenticator() throws Exception {
    // Given Authentication is set to lowercase oauth and a valid pre-acquired OAuth access token is
    // supplied via TOKEN
    Properties props = oauthConnectionProperties("oauth");
    props.setProperty("token", TestParameters.get("SNOWFLAKE_TEST_OAUTH_ACCESS_TOKEN"));

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and a simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Disabled("TODO: SNOW-2872392 - requires SNOWFLAKE_TEST_OAUTH_* parameters in parameters.json")
  @Test
  void oauthShouldFailClientCredentialsFlowWithBadClientSecret() {
    // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id, an invalid
    // client secret and a valid token_request_url
    Properties props = oauthConnectionProperties("OAUTH_CLIENT_CREDENTIALS");
    props.setProperty("oauth_client_id", TestParameters.get("SNOWFLAKE_TEST_OAUTH_CLIENT_ID"));
    props.setProperty("oauth_client_secret", "invalid_client_secret_12345");
    props.setProperty(
        "oauth_token_request_url", TestParameters.get("SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL"));
    props.setProperty("oauth_scope", TestParameters.get("SNOWFLAKE_TEST_OAUTH_SCOPE"));

    // When Trying to Connect
    Executable connect = () -> connect(props);

    // Then Connection fails with an authentication / login error
    assertThrows(SQLException.class, connect);
  }

  private Properties oauthConnectionProperties(String authenticator) {
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", authenticator);
    props.setProperty("user", USER);
    return props;
  }
}
