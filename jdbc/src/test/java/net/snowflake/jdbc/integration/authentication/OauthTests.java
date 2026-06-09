package net.snowflake.jdbc.integration.authentication;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.jdbc.utils.SkipOldDriver;
import net.snowflake.jdbc.wiremock.BaseWiremockTest;
import org.junit.jupiter.api.Test;

class OauthTests extends BaseWiremockTest {

  private static final String SECRET_LITERAL = "ZZ_JDBC_SECRET_NEEDLE_OAUTH_CC_ZZ";

  @Test
  @SkipOldDriver("BD#6")
  void shouldFailOauthClientCredentialsWhenClientIdIsMissing() throws Exception {
    // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS without oauth_client_id
    Properties props = offlineOauthProperties("OAUTH_CLIENT_CREDENTIALS");
    props.setProperty("oauth_client_secret", "test-client-secret");
    props.setProperty("oauth_token_request_url", "https://idp.example.com/oauth/token");

    // When Trying to Connect
    SQLException exception = assertThrows(SQLException.class, () -> attemptConnect(props));

    // Then Connection fails with a missing-parameter error citing oauth_client_id
    assertTrue(exception.getMessage().toLowerCase().contains("oauth_client_id"));
  }

  @Test
  @SkipOldDriver("BD#6")
  void shouldFailOauthClientCredentialsWhenClientSecretIsMissing() throws Exception {
    // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS without oauth_client_secret
    Properties props = offlineOauthProperties("OAUTH_CLIENT_CREDENTIALS");
    props.setProperty("oauth_client_id", "test-client-id");
    props.setProperty("oauth_token_request_url", "https://idp.example.com/oauth/token");

    // When Trying to Connect
    SQLException exception = assertThrows(SQLException.class, () -> attemptConnect(props));

    // Then Connection fails with a missing-parameter error citing oauth_client_secret
    assertTrue(exception.getMessage().toLowerCase().contains("oauth_client_secret"));
  }

  @Test
  @SkipOldDriver("BD#6")
  void shouldFailOauthClientCredentialsWhenTokenRequestUrlIsMissing() throws Exception {
    // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS without oauth_token_request_url
    Properties props = offlineOauthProperties("OAUTH_CLIENT_CREDENTIALS");
    props.setProperty("oauth_client_id", "test-client-id");
    props.setProperty("oauth_client_secret", "test-client-secret");

    // When Trying to Connect
    SQLException exception = assertThrows(SQLException.class, () -> attemptConnect(props));

    // Then Connection fails with a missing-parameter error citing oauth_token_request_url
    assertTrue(exception.getMessage().toLowerCase().contains("oauth_token_request_url"));
  }

  @Test
  void shouldForwardAuthenticatorOauthWithTokenToCore() throws Exception {
    // Given Authentication is set to legacy OAUTH with a pre-acquired access token
    Properties props = offlineOauthProperties("OAUTH");
    props.setProperty("token", "fake.jwt.token");

    // When Trying to Connect
    try {
      openOfflineConnection(props);
    } catch (SQLException exception) {
      // Then The wrapper forwards the token to sf_core without raising a missing-parameter error
      // for it
      assertFalse(
          exception.getMessage().toLowerCase().contains("missing required parameter 'token'"));
    }
  }

  @Test
  void shouldFailAuthenticatorOauthWhenTokenIsMissing() throws Exception {
    // Given Authentication is set to legacy OAUTH without a TOKEN
    Properties props = offlineOauthProperties("OAUTH");

    // When Trying to Connect
    SQLException exception = assertThrows(SQLException.class, () -> attemptConnect(props));

    // Then Connection fails with a missing-parameter error citing token
    assertTrue(exception.getMessage().toLowerCase().contains("token"));
  }

  @Test
  void shouldAcceptLowercaseOauthAuthenticatorValue() throws Exception {
    // Given Authentication is set to lowercase oauth with a TOKEN
    Properties props = offlineOauthProperties("oauth");
    props.setProperty("token", "fake.jwt.token");

    // When Trying to Connect
    try {
      openOfflineConnection(props);
    } catch (SQLException exception) {
      // Then The wrapper does not reject the AUTHENTICATOR value as unknown
      String message = exception.getMessage().toLowerCase();
      assertFalse(message.contains("invalid authenticator"));
      assertFalse(message.contains("unknown authenticator"));
    }
  }

  @Test
  @SkipOldDriver("Hangs on the old driver when connecting with an unknown OAuth-like authenticator")
  void shouldFailWhenAuthenticatorIsAnUnknownOAuthLikeValue() throws Exception {
    // Given Authentication is set to a typo of an OAuth flow name
    Properties props = offlineOauthProperties("OAUTH_AUTHORIZATION_TYPO");
    props.setProperty("oauth_client_id", "test-client-id");

    // When Trying to Connect
    SQLException exception = assertThrows(SQLException.class, () -> attemptConnect(props));

    // Then Connection fails with an authenticator-related error
    assertTrue(exception.getMessage().toLowerCase().contains("authenticator"));
  }

  @Test
  void shouldNotEchoOauthClientSecretInDiagnostics() throws Exception {
    // Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a distinctive client secret
    // literal
    Properties props = offlineOauthProperties("OAUTH_CLIENT_CREDENTIALS");
    props.setProperty("oauth_client_id", "test-client-id");
    props.setProperty("oauth_client_secret", SECRET_LITERAL);

    // When Trying to Connect
    SQLException exception = assertThrows(SQLException.class, () -> attemptConnect(props));

    // Then No diagnostic record contains the literal client secret
    assertFalse(exception.getMessage().contains(SECRET_LITERAL));
  }

  private Properties offlineOauthProperties(String authenticator) {
    Properties props = new Properties();
    props.setProperty("account", "test_account");
    props.setProperty("user", "test_user");
    props.setProperty("protocol", "http");
    props.setProperty("authenticator", authenticator);
    return props;
  }

  private void attemptConnect(Properties props) throws Exception {
    openOfflineConnection(props);
  }

  private void openOfflineConnection(Properties props) throws Exception {
    Class.forName(SnowflakeDriver.class.getName());
    try (java.sql.Connection ignored = DriverManager.getConnection(wiremockJdbcUrl(), props)) {}
  }
}
