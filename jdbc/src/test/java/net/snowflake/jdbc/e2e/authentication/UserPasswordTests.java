package net.snowflake.jdbc.e2e.authentication;

import static net.snowflake.jdbc.utils.DriverCompatibility.isNewDriver;
import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import java.sql.Connection;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.jdbc.utils.TestParameters;
import net.snowflake.jdbc.utils.WithConnect;
import net.snowflake.jdbc.utils.WithQueryUtils;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.function.Executable;

class UserPasswordTests implements WithQueryUtils, WithConnect {

  private static final String USER = TestParameters.get("SNOWFLAKE_TEST_USER");
  // null when the environment uses JWT-only auth (no SNOWFLAKE_TEST_PASSWORD configured)
  private static final String PASSWORD =
      TestParameters.has("SNOWFLAKE_TEST_PASSWORD")
          ? TestParameters.get("SNOWFLAKE_TEST_PASSWORD")
          : null;

  // Error 390197: account enforces MFA for password auth (PERSON-type users on accounts
  // with account-level MFA policy). LEGACY_SERVICE users are exempt; this guard handles
  // the transition period and any future account where enforcement is re-enabled.
  private static boolean isMfaEnforced(SQLException e) {
    return e.getMessage() != null && e.getMessage().contains("390197");
  }

  @Test
  void shouldAuthenticateUsingUsernameAndPassword() throws Exception {
    assumeTrue(
        PASSWORD != null,
        "Skipping: SNOWFLAKE_TEST_PASSWORD not configured (JWT-only environment)");
    // Given Authentication is set to default (snowflake) with valid username and password
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("user", USER);
    props.setProperty("password", PASSWORD);

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    } catch (SQLException e) {
      assumeTrue(!isMfaEnforced(e), "Skipping: account enforces MFA for password auth (390197)");
      throw e;
    }
  }

  @Test
  void shouldAuthenticateUsingExplicitSnowflakeAuthenticator() throws Exception {
    assumeTrue(
        PASSWORD != null,
        "Skipping: SNOWFLAKE_TEST_PASSWORD not configured (JWT-only environment)");
    // Given Authentication is explicitly set to snowflake with valid username and password
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "snowflake");
    props.setProperty("user", USER);
    props.setProperty("password", PASSWORD);

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    } catch (SQLException e) {
      assumeTrue(!isMfaEnforced(e), "Skipping: account enforces MFA for password auth (390197)");
      throw e;
    }
  }

  @Test
  void shouldFailAuthenticationWhenWrongPasswordIsProvided() {
    assumeTrue(
        PASSWORD != null,
        "Skipping: SNOWFLAKE_TEST_PASSWORD not configured (JWT-only environment)");
    // Given Authentication is set to default with valid username and wrong password
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("user", USER);
    props.setProperty("password", "wrong_password");

    // When Trying to Connect
    Executable connect = () -> connect(props);

    // Then There is error returned
    SQLException exception = assertThrows(SQLException.class, connect);
    // On MFA-enforced accounts wrong credentials return 390197 (MFA required) rather
    // than 390100 — still correctly rejected, just different error.
    if (!isMfaEnforced(exception)) {
      assertTrue(
          exception
              .getMessage()
              .toLowerCase()
              .contains("incorrect username or password was specified"));
      if (isNewDriver()) {
        assertTrue(exception.getMessage().contains("390100"));
      }
    }
  }
}
