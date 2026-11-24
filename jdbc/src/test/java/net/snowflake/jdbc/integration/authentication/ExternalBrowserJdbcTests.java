package net.snowflake.jdbc.integration.authentication;

import static net.snowflake.jdbc.utils.JsonTestUtils.parseJson;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.List;
import java.util.Properties;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.jdbc.utils.SkipOldDriver;
import net.snowflake.jdbc.wiremock.BaseWiremockTest;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

/**
 * JDBC-only WireMock coverage for an IdP user mismatch ({@code 390191}). Shared EXTERNALBROWSER
 * scenarios live in {@link ExternalBrowserTests}.
 */
class ExternalBrowserJdbcTests extends BaseWiremockTest {

  @BeforeAll
  void loadDriver() throws Exception {
    Class.forName(SnowflakeDriver.class.getName());
  }

  @Test
  @SkipOldDriver("New-driver-only: external browser auth via WireMock")
  void shouldFailWhenLoginRequestReportsAnIdpUserMismatch() throws Exception {
    wiremock.addMapping("auth/external_browser_authenticator_request.json");
    wiremock.addMapping("auth/login_failure_external_browser_usernames_mismatch.json");

    Thread callbackThread =
        new Thread(
            () -> simulateBrowserCallback("browser_sso_token_mismatched_user"), "browser-callback");
    callbackThread.start();
    try {
      SQLException exception =
          assertThrows(
              SQLException.class,
              () -> {
                try (Connection ignored =
                    DriverManager.getConnection(wiremockJdbcUrl(), connectionProps())) {}
              });

      assertEquals(390191, exception.getErrorCode());
      assertEquals("08001", exception.getSQLState());
      assertTrue(
          exception.getMessage().contains("differs from the user currently logged in at the IDP"),
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

  private void simulateBrowserCallback(String token) {
    long deadline = System.nanoTime() + 10_000L * 1_000_000L;
    while (System.nanoTime() < deadline) {
      List<JsonNode> requests = wiremock.getRequests("/session/authenticator-request.*");
      if (!requests.isEmpty()) {
        JsonNode body = parseJson(requests.get(0).get("body").asText());
        int port = Integer.parseInt(body.get("data").get("BROWSER_MODE_REDIRECT_PORT").asText());
        try (Socket sock = new Socket()) {
          sock.connect(new InetSocketAddress("127.0.0.1", port), 5_000);
          OutputStream out = sock.getOutputStream();
          String httpRequest = "GET /?token=" + token + " HTTP/1.1\r\nHost: localhost\r\n\r\n";
          out.write(httpRequest.getBytes(StandardCharsets.UTF_8));
          out.flush();
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
