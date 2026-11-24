package net.snowflake.jdbc.e2e.authentication;

import static net.snowflake.jdbc.e2e.authentication.ExternalBrowserTests.cachingExternalBrowserProperties;
import static net.snowflake.jdbc.e2e.authentication.ExternalBrowserTests.externalBrowserProperties;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.SQLException;
import java.util.Locale;
import java.util.Properties;
import net.snowflake.jdbc.utils.RequiresBrowser;
import net.snowflake.jdbc.utils.TestParameters;
import net.snowflake.jdbc.utils.WithConnect;
import net.snowflake.jdbc.utils.WithQueryUtils;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

/**
 * JDBC-only external-browser cases (IdP rejection, user mismatch, cache-off). Shared scenarios live
 * in {@link ExternalBrowserTests}.
 */
@RequiresBrowser
class ExternalBrowserJdbcTests implements WithQueryUtils, WithConnect, WithBrowserAutomation {

  private static final String BROWSER_RESPONSE_TIMEOUT_SECONDS = "5";
  private static final int IDP_USER_MISMATCH_ERROR_CODE = 390191;

  @BeforeEach
  void cleanBrowserBeforeTest() {
    cleanBrowserProcesses();
  }

  @AfterEach
  void cleanBrowserAfterTest() {
    cleanBrowserProcesses();
  }

  @Test
  void shouldFailWithTimeoutWhenTheIdpRejectsTheCredentials() {
    // Given External browser authentication is configured with valid Okta user and a short timeout
    String login = TestParameters.get("SNOWFLAKE_TEST_OKTA_USER");

    Properties props = externalBrowserProperties(login);
    props.setProperty("BROWSER_RESPONSE_TIMEOUT", BROWSER_RESPONSE_TIMEOUT_SECONDS);

    // When Trying to Connect with headless browser providing invalid credentials
    SQLException exception =
        connectExpectingFailureWithBrowserAutomation(
            () -> connect(props), "fail", login, "wrong_password_12345");

    // Then Connection fails with timeout error
    assertTimeoutError(exception);
  }

  @Test
  void shouldFailWhenTheIdpUserDiffersFromTheConnectionUser() {
    // Given External browser authentication is configured with a connection user that differs from
    // the Okta user
    String oktaLogin = TestParameters.get("SNOWFLAKE_TEST_OKTA_USER");
    String oktaPassword = TestParameters.get("SNOWFLAKE_TEST_OKTA_PASSWORD");
    String connectionUser = TestParameters.get("SNOWFLAKE_TEST_USER");
    assertNotEquals(
        oktaLogin.toLowerCase(Locale.ROOT),
        connectionUser.toLowerCase(Locale.ROOT),
        "SNOWFLAKE_TEST_USER must differ from SNOWFLAKE_TEST_OKTA_USER for the mismatch to occur");

    Properties props = externalBrowserProperties(connectionUser);

    // When Trying to Connect with headless browser providing valid credentials
    SQLException exception =
        connectExpectingFailureWithBrowserAutomation(
            () -> connect(props), "success", oktaLogin, oktaPassword);

    // Then Connection fails with error 390191
    assertEquals(IDP_USER_MISMATCH_ERROR_CODE, exception.getErrorCode());
    assertEquals("08001", exception.getSQLState());
    assertTrue(
        exception
            .getMessage()
            .toLowerCase(Locale.ROOT)
            .contains("differs from the user currently logged in at the idp"),
        () -> "Expected an IdP user mismatch message, got: " + exception.getMessage());
  }

  @Test
  void shouldNotReuseCachedIdTokenWhenCredentialCachingIsDisabled() throws Exception {
    // Given A token has been cached by a previous external browser connection with caching enabled
    String login = TestParameters.get("SNOWFLAKE_TEST_OKTA_USER");
    String password = TestParameters.get("SNOWFLAKE_TEST_OKTA_PASSWORD");

    Properties cachingProps = cachingExternalBrowserProperties(login);

    try (Connection first =
        connectWithBrowserAutomation(() -> connect(cachingProps), "success", login, password)) {
      assertSimpleQuerySucceeds(first);
    }

    // And External browser authentication is configured with caching disabled and a short timeout
    Properties props = externalBrowserProperties(login);
    props.setProperty("BROWSER_RESPONSE_TIMEOUT", BROWSER_RESPONSE_TIMEOUT_SECONDS);

    // When Trying to Connect without browser interaction
    SQLException exception = assertThrows(SQLException.class, () -> connect(props));

    // Then Connection fails with timeout error
    assertTimeoutError(exception);
  }

  private static void assertTimeoutError(SQLException exception) {
    String message = exception.getMessage().toLowerCase(Locale.ROOT);
    assertTrue(
        message.contains("timeout") || message.contains("deadline exceeded"),
        () -> "Expected an external browser timeout, got: " + exception.getMessage());
  }
}
