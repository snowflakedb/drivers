package net.snowflake.jdbc.integration.authentication;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.List;
import java.util.Locale;
import java.util.Properties;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.jdbc.utils.SkipOldDriver;
import net.snowflake.jdbc.wiremock.BaseWiremockTest;
import org.json.JSONObject;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

/**
 * WireMock-based integration tests for {@code EXTERNALBROWSER} authentication. The browser opener
 * is suppressed via {@code SF_TEST_BROWSER_OPENER=noop} (set on the Gradle test task) so sf_core
 * never launches a real browser; instead the test simulates the browser callback by connecting a
 * raw socket to sf_core's localhost listener and delivering a token.
 *
 * <p>Mirrors {@code odbc_tests/tests/integration/authentication/external_browser.cpp} and {@code
 * python/tests/integ/authentication/test_external_browser.py}. The full external-browser flow
 * against a real headless Chrome is exercised by the e2e test in {@code
 * net.snowflake.jdbc.e2e.authentication.ExternalBrowserTests}.
 */
class ExternalBrowserTests extends BaseWiremockTest {

  @BeforeAll
  void loadDriver() throws Exception {
    Class.forName(SnowflakeDriver.class.getName());
  }

  @Test
  @SkipOldDriver("New-driver-only: external browser auth via WireMock")
  void shouldLoginWithExternalBrowserUsingSimulatedCallback() throws Exception {
    // Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
    wiremock.addMapping("auth/external_browser_authenticator_request.json");

    // And Login endpoint returns success
    wiremock.addMapping("auth/login_success_external_browser.json");

    // When Trying to Connect with simulated browser callback delivering a token
    String token = "browser_sso_token_12345";
    Thread callbackThread = new Thread(() -> simulateBrowserCallback(token), "browser-callback");
    callbackThread.start();
    try (Connection conn = DriverManager.getConnection(wiremockJdbcUrl(), connectionProps())) {
      callbackThread.join();

      // Then Login is successful
      assertFalse(conn.isClosed());

      // And Login request contains EXTERNALBROWSER authenticator, token, proof key, and login name
      List<JSONObject> loginRequests = wiremock.getRequests("/session/v1/login-request.*");
      assertTrue(loginRequests.size() >= 1, "Expected at least one login-request");
      JSONObject data =
          new JSONObject(loginRequests.get(0).getString("body")).getJSONObject("data");
      assertEquals("EXTERNALBROWSER", data.getString("AUTHENTICATOR"));
      assertEquals(token, data.getString("TOKEN"));
      assertEquals("mock_proof_key_abc123", data.getString("PROOF_KEY"));
      assertEquals("test_user", data.getString("LOGIN_NAME"));
    } finally {
      callbackThread.join();
    }
  }

  @Test
  @SkipOldDriver("New-driver-only: external browser auth via WireMock")
  void shouldFailWhenAuthenticatorRequestReturnsForbidden() throws Exception {
    // Given Wiremock returns HTTP 403 for authenticator-request
    wiremock.addMapping("auth/external_browser_authenticator_request_forbidden.json");

    // When Trying to Connect
    SQLException exception = assertThrows(SQLException.class, () -> attemptConnect());

    // Then Connection fails with authenticator error
    String message = exception.getMessage().toLowerCase(Locale.ROOT);
    assertTrue(
        message.contains("403")
            || message.contains("forbidden")
            || message.contains("authenticator"),
        "Unexpected error message: " + exception.getMessage());
  }

  @Test
  @SkipOldDriver("New-driver-only: external browser auth via WireMock")
  void shouldFailWhenAuthenticatorRequestReturnsLogicalFailure() throws Exception {
    // Given Wiremock returns success false for authenticator-request
    wiremock.addMapping("auth/external_browser_authenticator_request_logical_failure.json");

    // When Trying to Connect
    SQLException exception = assertThrows(SQLException.class, () -> attemptConnect());

    // Then Connection fails with authenticator error
    String message = exception.getMessage().toLowerCase(Locale.ROOT);
    assertTrue(
        message.contains("not enabled") || message.contains("authenticator"),
        "Unexpected error message: " + exception.getMessage());
  }

  @Test
  @SkipOldDriver("New-driver-only: external browser auth via WireMock")
  void shouldFailWithTimeoutWhenNoBrowserCallbackArrives() throws Exception {
    // Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
    wiremock.addMapping("auth/external_browser_authenticator_request.json");

    // And Authentication timeout is set to 2 seconds
    Properties props = connectionProps();
    props.setProperty("authentication_timeout", "2");

    // When Trying to Connect without any browser callback
    SQLException exception =
        assertThrows(
            SQLException.class,
            () -> {
              try (Connection ignored = DriverManager.getConnection(wiremockJdbcUrl(), props)) {}
            });

    // Then Connection fails with timeout or browser error
    String message = exception.getMessage().toLowerCase(Locale.ROOT);
    assertTrue(
        message.contains("timeout") || message.contains("browser"),
        "Unexpected error message: " + exception.getMessage());
  }

  @Test
  @SkipOldDriver("New-driver-only: external browser auth via WireMock")
  void shouldFailWhenLoginRequestIsRejectedAfterBrowserCallback() throws Exception {
    // Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
    wiremock.addMapping("auth/external_browser_authenticator_request.json");

    // And Login endpoint returns failure
    wiremock.addMapping("auth/login_failure_external_browser.json");

    // When Trying to Connect with simulated browser callback delivering a token
    String token = "browser_sso_token_rejected";
    Thread callbackThread = new Thread(() -> simulateBrowserCallback(token), "browser-callback");
    callbackThread.start();
    try {
      SQLException exception =
          assertThrows(
              SQLException.class,
              () -> {
                try (Connection ignored =
                    DriverManager.getConnection(wiremockJdbcUrl(), connectionProps())) {}
              });

      // Then Connection fails with login error
      String message = exception.getMessage().toLowerCase(Locale.ROOT);
      assertTrue(
          message.contains("invalid credentials") || message.contains("login"),
          "Unexpected error message: " + exception.getMessage());
    } finally {
      callbackThread.join();
    }
  }

  private Properties connectionProps() {
    Properties props = new Properties();
    props.setProperty("account", "test_account");
    props.setProperty("user", "test_user");
    props.setProperty("protocol", "http");
    props.setProperty("authenticator", "EXTERNALBROWSER");
    return props;
  }

  private void attemptConnect() throws SQLException {
    try (Connection ignored = DriverManager.getConnection(wiremockJdbcUrl(), connectionProps())) {}
  }

  /**
   * Poll WireMock for the authenticator-request, extract sf_core's redirect port, then deliver a
   * token to its localhost callback listener over a raw socket — standing in for the browser's
   * redirect back to {@code http://localhost:<port>/?token=...}.
   */
  private void simulateBrowserCallback(String token) {
    long deadline = System.nanoTime() + 10_000L * 1_000_000L;
    while (System.nanoTime() < deadline) {
      List<JSONObject> requests = wiremock.getRequests("/session/authenticator-request.*");
      if (!requests.isEmpty()) {
        JSONObject body = new JSONObject(requests.get(0).getString("body"));
        int port =
            Integer.parseInt(body.getJSONObject("data").getString("BROWSER_MODE_REDIRECT_PORT"));
        try (Socket sock = new Socket()) {
          sock.connect(new InetSocketAddress("127.0.0.1", port), 5_000);
          OutputStream out = sock.getOutputStream();
          String httpRequest = "GET /?token=" + token + " HTTP/1.1\r\nHost: localhost\r\n\r\n";
          out.write(httpRequest.getBytes(StandardCharsets.UTF_8));
          out.flush();
          // Drain the driver's HTTP response so it can complete the handshake cleanly.
          sock.getInputStream().read(new byte[4096]);
        } catch (Exception e) {
          throw new RuntimeException("Failed to deliver browser callback to port " + port, e);
        }
        return;
      }
      try {
        Thread.sleep(200);
      } catch (InterruptedException e) {
        Thread.currentThread().interrupt();
        return;
      }
    }
    throw new RuntimeException("authenticator-request never arrived at WireMock");
  }
}
