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
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.function.Executable;

@RequiresBrowser
class NativeOktaTests implements WithQueryUtils, WithConnect {

  private static final String OKTA_URL = TestParameters.get("SNOWFLAKE_TEST_OKTA_URL");
  private static final String OKTA_USER = TestParameters.get("SNOWFLAKE_TEST_OKTA_USER");
  private static final String OKTA_PASSWORD = TestParameters.get("SNOWFLAKE_TEST_OKTA_PASSWORD");

  @Test
  void shouldAuthenticateUsingNativeOkta() throws Exception {
    // Given Okta authentication is configured with valid credentials
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", OKTA_URL);
    props.setProperty("user", OKTA_USER);
    props.setProperty("password", OKTA_PASSWORD);

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldFailNativeOktaAuthenticationWithWrongCredentials() {
    // Given Okta authentication is configured with wrong password
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", OKTA_URL);
    props.setProperty("user", OKTA_USER);
    props.setProperty("password", "wrong_password_12345");

    // When Trying to Connect
    Executable connect = () -> connect(props);

    // Then Connection fails with authentication error
    SQLException exception = assertThrows(SQLException.class, connect);
    String msg = exception.getMessage().toLowerCase();
    if (isNewDriver()) {
      assertTrue(
          msg.contains("okta"),
          () -> "Expected error to mention Okta, got: " + exception.getMessage());
    } else {
      // Legacy JDBC surfaces Okta's 401 as a communication error rather than naming Okta.
      assertTrue(
          msg.contains("okta") || msg.contains("http status=401"),
          () -> "Expected Okta auth failure or HTTP 401, got: " + exception.getMessage());
    }
  }

  @Test
  void shouldFailNativeOktaAuthenticationWithWrongOktaUrl() {
    // Given Okta authentication is configured with invalid okta url
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "https://invalid.okta.com");
    props.setProperty("user", OKTA_USER);
    props.setProperty("password", OKTA_PASSWORD);

    // When Trying to Connect
    Executable connect = () -> connect(props);

    // Then Connection fails with authentication error
    SQLException exception = assertThrows(SQLException.class, connect);
    String msg = exception.getMessage().toLowerCase();
    assertTrue(
        msg.contains("not accepted") || msg.contains("does not match"),
        () ->
            "Expected authenticator rejection or IdP URL mismatch, got: " + exception.getMessage());
  }
}
