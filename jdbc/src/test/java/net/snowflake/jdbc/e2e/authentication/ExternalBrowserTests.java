package net.snowflake.jdbc.e2e.authentication;

import static net.snowflake.jdbc.utils.DriverCompatibility.isOldDriver;
import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;

import java.sql.Connection;
import java.util.Properties;
import net.snowflake.jdbc.utils.RequiresBrowser;
import net.snowflake.jdbc.utils.TestParameters;
import net.snowflake.jdbc.utils.WithConnect;
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
class ExternalBrowserTests implements WithQueryUtils, WithConnect, WithBrowserAutomation {

  @Test
  void shouldAuthenticateWithExternalBrowserViaOktaIdp() throws Exception {
    cleanBrowserProcesses();
    try {
      // Given External browser authentication is configured with valid Okta user
      String login = TestParameters.get("SNOWFLAKE_TEST_OKTA_USER");
      String password = TestParameters.get("SNOWFLAKE_TEST_OKTA_PASSWORD");

      Properties props = loadDefaultConnectionProperties();
      props.setProperty("authenticator", "EXTERNALBROWSER");
      props.setProperty("user", login);
      props.setProperty("clientStoreTemporaryCredential", "false");

      // When Trying to Connect with headless browser providing valid credentials
      try (Connection conn =
          connectWithBrowserAutomation(() -> connect(props), "success", login, password)) {
        // Then Login is successful and simple query can be executed
        assertSimpleQuerySucceeds(conn);
      }
    } finally {
      cleanBrowserProcesses();
    }
  }

  @Test
  void shouldReuseCachedIdTokenWithoutBrowserInteraction() throws Exception {
    cleanBrowserProcesses();
    try {
      // Given External browser authentication is configured with caching enabled and a token has
      // been cached from a previous connection
      String login = TestParameters.get("SNOWFLAKE_TEST_OKTA_USER");
      String password = TestParameters.get("SNOWFLAKE_TEST_OKTA_PASSWORD");

      Properties props = loadDefaultConnectionProperties();
      props.setProperty("authenticator", "EXTERNALBROWSER");
      props.setProperty("user", login);
      props.setProperty("clientStoreTemporaryCredential", "true");
      if (isOldDriver()) {
        // BD#42: legacy snowflake-jdbc on Linux gates id-token cache on this session parameter.
        props.put("CLIENT_STORE_TEMPORARY_CREDENTIAL", Boolean.TRUE);
      }

      try (Connection first =
          connectWithBrowserAutomation(() -> connect(props), "success", login, password)) {
        assertSimpleQuerySucceeds(first);
      }

      // When Trying to Connect without browser interaction
      try (Connection second = connect(props)) {
        // Then Login is successful and simple query can be executed
        assertSimpleQuerySucceeds(second);
      }
    } finally {
      cleanBrowserProcesses();
    }
  }
}
