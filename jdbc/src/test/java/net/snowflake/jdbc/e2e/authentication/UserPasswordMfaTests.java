package net.snowflake.jdbc.e2e.authentication;

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
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.function.Executable;

@RequiresBrowser
class UserPasswordMfaTests implements WithQueryUtils, WithConnect, WithTotpCodes {

  private static final String USER = TestParameters.get("SNOWFLAKE_TEST_MFA_USER");
  private static final String PASSWORD = TestParameters.get("SNOWFLAKE_TEST_MFA_PASSWORD");
  private static final String TOTP_SEED = TestParameters.get("SNOWFLAKE_TEST_MFA_SEED");

  // -------------------------------------------------------------------------
  // Passcode flow
  // -------------------------------------------------------------------------

  @Test
  void shouldAuthenticateUsingUsernamePasswordAndTotpPasscode() throws Exception {
    // Given Authentication is set to username_password_mfa and user, password and passcode are
    // provided
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "USERNAME_PASSWORD_MFA");
    props.setProperty("user", USER);
    props.setProperty("password", PASSWORD);

    // When Trying to Connect
    try (Connection conn = connectWithTotpRetry(props, TOTP_SEED, false)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingUsernamePasswordWithAppendedTotpPasscode() throws Exception {
    // Given Authentication is set to username_password_mfa and user, password with appended
    // passcode are provided and passcodeInPassword is set
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "USERNAME_PASSWORD_MFA");
    props.setProperty("user", USER);
    props.setProperty("password", PASSWORD);

    // When Trying to Connect
    try (Connection conn = connectWithTotpRetry(props, TOTP_SEED, true)) {
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
    Properties firstProps = loadDefaultConnectionProperties();
    firstProps.setProperty("authenticator", "USERNAME_PASSWORD_MFA");
    firstProps.setProperty("user", USER);
    firstProps.setProperty("password", PASSWORD);
    firstProps.setProperty("clientStoreTemporaryCredential", "true");

    try (Connection first = connectWithTotpRetry(firstProps, TOTP_SEED, false)) {
      assertSimpleQuerySucceeds(first);
    }

    // When Trying to Connect without passcode
    Properties secondProps = loadDefaultConnectionProperties();
    secondProps.setProperty("authenticator", "USERNAME_PASSWORD_MFA");
    secondProps.setProperty("user", USER);
    secondProps.setProperty("password", PASSWORD);
    secondProps.setProperty("clientStoreTemporaryCredential", "true");

    try (Connection second = connect(secondProps)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(second);
    }
  }

  // -------------------------------------------------------------------------
  // Error cases
  // -------------------------------------------------------------------------

  @Test
  @Disabled("Bad-secret tests cause pipeline flakiness by blocking the test account")
  void shouldFailAuthenticationWhenWrongPasswordIsProvided() {
    // Given Authentication is set to username_password_mfa and user is provided but password is
    // skipped or invalid
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "USERNAME_PASSWORD_MFA");
    props.setProperty("user", USER);
    props.setProperty("password", "wrong_password");
    props.setProperty("passcode", acquireTotpPasscode(TOTP_SEED));

    // When Trying to Connect
    Executable connect = () -> connect(props);

    // Then There is error returned
    SQLException exception = assertThrows(SQLException.class, connect);
    assertTrue(
        exception
            .getMessage()
            .toLowerCase()
            .contains("incorrect username or password was specified"));
  }

  // -------------------------------------------------------------------------
  // DUO push flow
  // -------------------------------------------------------------------------

  @Test
  @Disabled("DUO push requires interactive device approval - run manually")
  void shouldAuthenticateUsingUsernamePasswordAndDuoPush() throws Exception {
    // Given Authentication is set to username_password_mfa and user, password are provided and DUO
    // push is enabled
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "USERNAME_PASSWORD_MFA");
    props.setProperty("user", USER);
    props.setProperty("password", PASSWORD);

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }
}
