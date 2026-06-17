package net.snowflake.jdbc.e2e.authentication;

import static net.snowflake.jdbc.e2e.authentication.MfaAuthHelpers.acquireTotpPasscode;
import static net.snowflake.jdbc.e2e.authentication.MfaAuthHelpers.connectWithTotpRetry;
import static net.snowflake.jdbc.e2e.authentication.MfaAuthHelpers.getMfaParam;
import static net.snowflake.jdbc.utils.TestParameters.buildJdbcUrl;
import static net.snowflake.jdbc.utils.TestParameters.loadConnectionProperties;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.jdbc.utils.RequiresBrowser;
import net.snowflake.jdbc.utils.WithQueryUtils;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;
import org.junit.jupiter.api.function.Executable;

/**
 * End-to-end {@code DriverManager}-based tests for {@code USERNAME_PASSWORD_MFA} authentication.
 * Mirrors the Gherkin scenarios in {@code
 * tests/definitions/shared/authentication/user_password_mfa.feature}.
 *
 * <p>Requires the snowdrivers-test-external-browser-universal-driver Docker container
 * (/externalbrowser/totpGenerator.js generates TOTP passcodes for the MFA test user).
 *
 * <p>Run locally:
 *
 * <pre>./tests/auth/run_auth_browser.sh jdbc</pre>
 */
@RequiresBrowser
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
class UserPasswordMfaTests implements WithQueryUtils {

  private String totpSeed;
  private String jdbcUrl;
  private Properties baseConnectionProps;

  @BeforeAll
  void setUp() throws Exception {
    String user = getMfaParam("SNOWFLAKE_TEST_MFA_USER");
    String password = getMfaParam("SNOWFLAKE_TEST_MFA_PASSWORD");
    totpSeed = getMfaParam("SNOWFLAKE_TEST_MFA_SEED");

    baseConnectionProps = loadConnectionProperties();
    baseConnectionProps.setProperty("user", user);
    baseConnectionProps.setProperty("password", password);
    baseConnectionProps.setProperty("authenticator", "USERNAME_PASSWORD_MFA");
    baseConnectionProps.setProperty("role", "PUBLIC");
    jdbcUrl = buildJdbcUrl(baseConnectionProps);
  }

  // -------------------------------------------------------------------------
  // Passcode flow
  // -------------------------------------------------------------------------

  @Test
  void shouldAuthenticateUsingUsernamePasswordAndTotpPasscode() throws Exception {
    // Given Authentication is set to username_password_mfa and user, password and passcode are
    // provided
    Properties props = new Properties();
    props.putAll(baseConnectionProps);

    // When Trying to Connect
    try (Connection conn = connectWithTotpRetry(jdbcUrl, props, totpSeed, false)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingUsernamePasswordWithAppendedTotpPasscode() throws Exception {
    // Given Authentication is set to username_password_mfa and user, password with appended
    // passcode are provided and passcodeInPassword is set
    Properties props = new Properties();
    props.putAll(baseConnectionProps);

    // When Trying to Connect
    try (Connection conn = connectWithTotpRetry(jdbcUrl, props, totpSeed, true)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  // -------------------------------------------------------------------------
  // Token caching flow
  // -------------------------------------------------------------------------

  @Test
  void shouldReuseCachedMfaTokenWithoutPasscode() throws Exception {
    // Given Authentication is set to username_password_mfa and MFA token has been cached from a
    // previous connection
    Properties firstProps = new Properties();
    firstProps.putAll(baseConnectionProps);
    firstProps.setProperty("clientStoreTemporaryCredential", "true");

    try (Connection first = connectWithTotpRetry(jdbcUrl, firstProps, totpSeed, false)) {
      assertSimpleQuerySucceeds(first);
    }

    // When Trying to Connect without passcode
    Properties secondProps = new Properties();
    secondProps.putAll(baseConnectionProps);
    secondProps.setProperty("clientStoreTemporaryCredential", "true");

    try (Connection second = DriverManager.getConnection(jdbcUrl, secondProps)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(second);
    }
  }

  // -------------------------------------------------------------------------
  // Error cases
  // -------------------------------------------------------------------------

  @Test
  void shouldFailAuthenticationWhenWrongPasswordIsProvided() throws Exception {
    // Given Authentication is set to username_password_mfa and user is provided but password is
    // skipped or invalid
    String passcode = acquireTotpPasscode(totpSeed);
    Properties props = new Properties();
    props.putAll(baseConnectionProps);
    props.setProperty("password", "wrong_password");
    props.setProperty("passcode", passcode);

    // When Trying to Connect
    Executable connect = () -> DriverManager.getConnection(jdbcUrl, props);

    // Then There is error returned
    assertThrows(SQLException.class, connect);
  }

  // -------------------------------------------------------------------------
  // DUO push flow
  // -------------------------------------------------------------------------

  @Test
  @Disabled("DUO push requires interactive device approval - run manually")
  void shouldAuthenticateUsingUsernamePasswordAndDuoPush() throws Exception {
    // Given Authentication is set to username_password_mfa and user, password are provided and DUO
    // push is enabled
    Properties props = new Properties();
    props.putAll(baseConnectionProps);

    // When Trying to Connect
    try (Connection conn = DriverManager.getConnection(jdbcUrl, props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }
}
