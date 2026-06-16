package net.snowflake.jdbc.e2e.authentication;

import static net.snowflake.jdbc.utils.TestParameters.buildJdbcUrl;
import static net.snowflake.jdbc.utils.TestParameters.loadConnectionProperties;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;

/**
 * End-to-end {@code DriverManager}-based tests for {@code USERNAME_PASSWORD_MFA} authentication.
 * Mirrors the Gherkin scenarios in {@code
 * tests/definitions/shared/authentication/user_password_mfa.feature}.
 */
@Disabled("TODO: SNOW-2872399 - not yet implemented")
class UserPasswordMfaTests extends SnowflakeIntegrationTestBase {

  private Properties baseConnectionProperties() throws Exception {
    Properties props = loadConnectionProperties();
    props.setProperty("authenticator", "USERNAME_PASSWORD_MFA");
    return props;
  }

  @Test
  void shouldAuthenticateUsingUsernamePasswordAndDuoPush() throws Exception {
    // Given Authentication is set to username_password_mfa and user, password are provided and DUO
    // push is enabled
    Properties props = baseConnectionProperties();

    // When Trying to Connect
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingUsernamePasswordAndTotpPasscode() throws Exception {
    // Given Authentication is set to username_password_mfa and user, password and passcode are
    // provided
    Properties props = baseConnectionProperties();
    props.setProperty("passcode", requireMfaEnv("SNOWFLAKE_TEST_MFA_PASSCODE"));

    // When Trying to Connect
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingUsernamePasswordWithAppendedTotpPasscode() throws Exception {
    // Given Authentication is set to username_password_mfa and user, password with appended
    // passcode are provided and passcodeInPassword is set
    Properties props = baseConnectionProperties();
    props.setProperty(
        "password", props.getProperty("password") + requireMfaEnv("SNOWFLAKE_TEST_MFA_PASSCODE"));
    props.setProperty("passcodeInPassword", "true");

    // When Trying to Connect
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldFailAuthenticationWhenWrongPasswordIsProvided() throws Exception {
    // Given Authentication is set to username_password_mfa and user is provided but password is
    // skipped or invalid
    Properties props = baseConnectionProperties();
    props.setProperty("password", "wrong_password");
    props.setProperty("passcode", requireMfaEnv("SNOWFLAKE_TEST_MFA_PASSCODE"));

    // When Trying to Connect
    String url = buildJdbcUrl(props);

    // Then There is error returned
    assertThrows(SQLException.class, () -> DriverManager.getConnection(url, props));
  }

  @Test
  void shouldReuseCachedMfaTokenWithoutPasscode() throws Exception {
    // Given Authentication is set to username_password_mfa and MFA token has been cached from a
    // previous connection
    Properties first = baseConnectionProperties();
    first.setProperty("passcode", requireMfaEnv("SNOWFLAKE_TEST_MFA_PASSCODE"));
    first.setProperty("clientStoreTemporaryCredential", "true");
    String url = buildJdbcUrl(first);
    try (Connection conn = DriverManager.getConnection(url, first)) {
      assertSimpleQuerySucceeds(conn);
    }

    Properties second = baseConnectionProperties();
    second.setProperty("clientStoreTemporaryCredential", "true");

    // When Trying to Connect without passcode
    try (Connection conn = DriverManager.getConnection(url, second)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  private static String requireMfaEnv(String name) {
    String value = System.getenv(name);
    if (value == null || value.isEmpty()) {
      throw new IllegalStateException("Missing required MFA test env var: " + name);
    }
    return value;
  }
}
