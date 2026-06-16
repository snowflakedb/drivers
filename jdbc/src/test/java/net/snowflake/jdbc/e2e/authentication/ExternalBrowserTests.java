package net.snowflake.jdbc.e2e.authentication;

import static net.snowflake.jdbc.e2e.authentication.AuthTestUtils.browserLoginFuture;
import static net.snowflake.jdbc.e2e.authentication.AuthTestUtils.cleanBrowserProcesses;
import static net.snowflake.jdbc.utils.TestParameters.buildJdbcUrl;
import static net.snowflake.jdbc.utils.TestParameters.loadConnectionProperties;

import java.sql.Connection;
import java.sql.DriverManager;
import java.util.Properties;
import java.util.concurrent.CompletableFuture;
import net.snowflake.jdbc.utils.RequiresBrowser;
import net.snowflake.jdbc.utils.TestParameters;
import net.snowflake.jdbc.utils.WithQueryUtils;
import org.junit.jupiter.api.Test;

/**
 * External browser authentication E2E test.
 *
 * <p>Requires the snowdrivers-test-external-browser-universal-driver Docker container (headless
 * Chromium + /externalbrowser/provideBrowserCredentials.js).
 *
 * <p>Run locally:
 *
 * <pre>./tests/auth/run_auth_browser.sh jdbc</pre>
 */
@RequiresBrowser
class ExternalBrowserTests implements WithQueryUtils {

  @Test
  void shouldAuthenticateWithExternalBrowserViaOktaIdp() throws Exception {
    cleanBrowserProcesses();
    try {
      // Given External browser authentication is configured with valid Okta user
      String login = TestParameters.get("SNOWFLAKE_TEST_OKTA_USER");
      String password = TestParameters.get("SNOWFLAKE_TEST_OKTA_PASSWORD");

      Properties props = loadConnectionProperties();
      props.setProperty("authenticator", "EXTERNALBROWSER");
      props.setProperty("user", login);
      String url = buildJdbcUrl(props);

      // When Trying to Connect with headless browser providing valid credentials
      CompletableFuture<Void> browser = browserLoginFuture(login, password);

      try (Connection conn = DriverManager.getConnection(url, props)) {
        // Then Login is successful and simple query can be executed
        assertSimpleQuerySucceeds(conn);
      } finally {
        browser.join();
      }
    } finally {
      cleanBrowserProcesses();
    }
  }
}
