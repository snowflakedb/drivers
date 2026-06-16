package net.snowflake.client.api.datasource;

import static net.snowflake.jdbc.utils.TestParameters.buildJdbcUrl;
import static net.snowflake.jdbc.utils.TestParameters.loadConnectionProperties;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.sql.Connection;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import net.snowflake.jdbc.utils.TestParameters;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;

class OauthTests extends SnowflakeIntegrationTestBase {

  private Properties props;
  private String jdbcUrl;

  @BeforeAll
  void setUp() throws Exception {
    props = loadConnectionProperties();
    jdbcUrl = buildJdbcUrl(props);
  }

  @Disabled("TODO: SNOW-2872392 - requires SNOWFLAKE_TEST_OAUTH_* parameters in parameters.json")
  @Test
  void oauthShouldAuthenticateWithPreAcquiredAccessToken() throws Exception {
    // Given Authentication is set to legacy OAUTH and a pre-acquired OAuth access token is supplied
    // via `token=`
    SnowflakeDataSource ds = createDataSource();
    ds.setAuthenticator("OAUTH");
    ds.setToken(TestParameters.get("SNOWFLAKE_TEST_OAUTH_ACCESS_TOKEN"));

    // When Trying to Connect
    try (Connection conn = ds.getConnection()) {
      // Then Login is successful and a simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void oauthShouldFailLegacyAuthenticationWithInvalidToken() {
    // Given Authentication is set to legacy OAUTH and an invalid OAuth access token is supplied
    SnowflakeDataSource ds = createDataSource();
    ds.setAuthenticator("OAUTH");
    ds.setToken("invalid_oauth_token_12345");

    // When Trying to Connect
    // Then Connection fails with an authentication / login error
    assertThrows(SQLException.class, ds::getConnection);
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
    SnowflakeDataSource ds = createDataSource();
    ds.setAuthenticator("OAUTH_AUTHORIZATION_CODE");
    ds.setOauthClientId(TestParameters.get("SNOWFLAKE_TEST_OAUTH_CLIENT_ID"));
    ds.setOauthClientSecret(TestParameters.get("SNOWFLAKE_TEST_OAUTH_CLIENT_SECRET"));
    ds.setClientStoreTemporaryCredential(true);
    ds.setOauthAuthorizationUrl(TestParameters.get("SNOWFLAKE_TEST_OAUTH_AUTHORIZATION_URL"));
    ds.setOauthTokenRequestUrl(TestParameters.get("SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL"));
    ds.setOauthRedirectUri(TestParameters.get("SNOWFLAKE_TEST_OAUTH_REDIRECT_URI"));
    ds.setOauthScope(TestParameters.get("SNOWFLAKE_TEST_OAUTH_SCOPE"));

    // When Trying to Connect (this will spawn the local-loopback HTTP listener and
    // `xdg-open`/`open`/`ShellExecute` the IdP login URL unless a previously cached access token
    // short-circuits the leg)
    try (Connection conn = ds.getConnection()) {
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
    SnowflakeDataSource ds = createDataSource();
    ds.setAuthenticator("OAUTH_CLIENT_CREDENTIALS");
    ds.setOauthClientId(TestParameters.get("SNOWFLAKE_TEST_OAUTH_CLIENT_ID"));
    ds.setOauthClientSecret(TestParameters.get("SNOWFLAKE_TEST_OAUTH_CLIENT_SECRET"));
    ds.setOauthTokenRequestUrl(TestParameters.get("SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL"));
    ds.setOauthScope(TestParameters.get("SNOWFLAKE_TEST_OAUTH_SCOPE"));

    // When Trying to Connect
    try (Connection conn = ds.getConnection()) {
      // Then Login is successful and a simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Disabled("TODO: SNOW-2872392 - OAuth authorization code E2E spawns a real OS browser")
  @Test
  void oauthShouldFailAuthorizationCodeFlowWithBadClientSecret() throws Exception {
    // Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a valid client id but a
    // deliberately invalid client secret. The IdP token-exchange step must reject the credentials
    // and the driver must surface an authentication / login error.
    SnowflakeDataSource ds = createDataSource();
    ds.setAuthenticator("OAUTH_AUTHORIZATION_CODE");
    ds.setOauthClientId(TestParameters.get("SNOWFLAKE_TEST_OAUTH_CLIENT_ID"));
    ds.setOauthClientSecret("invalid_client_secret_12345");
    ds.setClientStoreTemporaryCredential(false);
    ds.setOauthAuthorizationUrl(TestParameters.get("SNOWFLAKE_TEST_OAUTH_AUTHORIZATION_URL"));
    ds.setOauthTokenRequestUrl(TestParameters.get("SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL"));
    ds.setOauthRedirectUri(TestParameters.get("SNOWFLAKE_TEST_OAUTH_REDIRECT_URI"));
    ds.setOauthScope(TestParameters.get("SNOWFLAKE_TEST_OAUTH_SCOPE"));

    // When Trying to Connect
    // Then Connection fails with an authentication / login error
    assertThrows(SQLException.class, ds::getConnection);
  }

  @Disabled("TODO: SNOW-2872392 - requires SNOWFLAKE_TEST_OAUTH_* parameters in parameters.json")
  @Test
  void oauthShouldAuthenticateUsingLowercaseOauthAuthenticator() throws Exception {
    // Given Authentication is set to lowercase oauth and a valid pre-acquired OAuth access token is
    // supplied via TOKEN
    SnowflakeDataSource ds = createDataSource();
    ds.setAuthenticator("oauth");
    ds.setToken(TestParameters.get("SNOWFLAKE_TEST_OAUTH_ACCESS_TOKEN"));

    // When Trying to Connect
    try (Connection conn = ds.getConnection()) {
      // Then Login is successful and a simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Disabled("TODO: SNOW-2872392 - requires SNOWFLAKE_TEST_OAUTH_* parameters in parameters.json")
  @Test
  void oauthShouldFailClientCredentialsFlowWithBadClientSecret() throws Exception {
    // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id, an invalid
    // client secret and a valid token_request_url
    SnowflakeDataSource ds = createDataSource();
    ds.setAuthenticator("OAUTH_CLIENT_CREDENTIALS");
    ds.setOauthClientId(TestParameters.get("SNOWFLAKE_TEST_OAUTH_CLIENT_ID"));
    ds.setOauthClientSecret("invalid_client_secret_12345");
    ds.setOauthTokenRequestUrl(TestParameters.get("SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL"));
    ds.setOauthScope(TestParameters.get("SNOWFLAKE_TEST_OAUTH_SCOPE"));

    // When Trying to Connect
    // Then Connection fails with an authentication / login error
    assertThrows(SQLException.class, ds::getConnection);
  }

  private SnowflakeDataSource createDataSource() {
    SnowflakeDataSource ds = SnowflakeDataSourceFactory.createDataSource();
    ds.setUrl(jdbcUrl);
    ds.setUser(props.getProperty("user"));
    ds.setAccount(props.getProperty("account"));
    return ds;
  }
}
