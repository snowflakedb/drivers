package net.snowflake.client.api.datasource;

import static org.junit.jupiter.api.Assertions.assertThrows;

import java.sql.Connection;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;

/**
 * Integration tests for {@code USERNAME_PASSWORD_MFA} authentication via {@link
 * SnowflakeDataSource}. Mirrors the Gherkin scenarios in {@code
 * tests/definitions/shared/authentication/user_password_mfa.feature}.
 */
@Disabled("TODO: SNOW-2872399 - not yet implemented")
class UserPasswordMfaTests extends SnowflakeIntegrationTestBase {

  private Properties props;
  private String jdbcUrl;

  @BeforeAll
  void setUp() throws Exception {
    props = loadConnectionProperties();
    jdbcUrl = buildJdbcUrl(props);
  }

  private SnowflakeDataSource createDataSource(String password) {
    SnowflakeDataSource ds = SnowflakeDataSourceFactory.createDataSource();
    ds.setUrl(jdbcUrl);
    ds.setUser(props.getProperty("user"));
    ds.setPassword(password);
    ds.setAccount(props.getProperty("account"));
    ds.setAuthenticator("USERNAME_PASSWORD_MFA");
    return ds;
  }

  @Test
  void shouldAuthenticateUsingUsernamePasswordAndDuoPush() throws Exception {
    // Given Authentication is set to username_password_mfa and user, password are provided and DUO
    // push is enabled
    SnowflakeDataSource ds = createDataSource(props.getProperty("password"));

    // When Trying to Connect
    try (Connection conn = ds.getConnection()) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingUsernamePasswordAndTotpPasscode() throws Exception {
    // Given Authentication is set to username_password_mfa and user, password and passcode are
    // provided
    SnowflakeDataSource ds = createDataSource(props.getProperty("password"));
    ds.setPasscode(requireMfaEnv("SNOWFLAKE_TEST_MFA_PASSCODE"));

    // When Trying to Connect
    try (Connection conn = ds.getConnection()) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingUsernamePasswordWithAppendedTotpPasscode() throws Exception {
    // Given Authentication is set to username_password_mfa and user, password with appended
    // passcode are provided and passcodeInPassword is set
    SnowflakeDataSource ds =
        createDataSource(
            props.getProperty("password") + requireMfaEnv("SNOWFLAKE_TEST_MFA_PASSCODE"));
    ds.setPasscodeInPassword(true);

    // When Trying to Connect
    try (Connection conn = ds.getConnection()) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldFailAuthenticationWhenWrongPasswordIsProvided() throws Exception {
    // Given Authentication is set to username_password_mfa and user is provided but password is
    // skipped or invalid
    SnowflakeDataSource ds = createDataSource("wrong_password");
    ds.setPasscode(requireMfaEnv("SNOWFLAKE_TEST_MFA_PASSCODE"));

    // When Trying to Connect
    // Then There is error returned
    assertThrows(SQLException.class, ds::getConnection);
  }

  @Test
  void shouldReuseCachedMfaTokenWithoutPasscode() throws Exception {
    // Given Authentication is set to username_password_mfa and MFA token has been cached from a
    // previous connection
    SnowflakeDataSource first = createDataSource(props.getProperty("password"));
    first.setPasscode(requireMfaEnv("SNOWFLAKE_TEST_MFA_PASSCODE"));
    first.setClientStoreTemporaryCredential(true);
    try (Connection conn = first.getConnection()) {
      assertSimpleQuerySucceeds(conn);
    }

    SnowflakeDataSource second = createDataSource(props.getProperty("password"));
    second.setClientStoreTemporaryCredential(true);

    // When Trying to Connect without passcode
    try (Connection conn = second.getConnection()) {
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
