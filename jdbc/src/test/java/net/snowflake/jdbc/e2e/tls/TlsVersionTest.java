package net.snowflake.jdbc.e2e.tls;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.net.URI;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.jdbc.utils.SkipOldDriver;
import net.snowflake.jdbc.wiremock.WiremockClient;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.function.Executable;

@SkipOldDriver("Old driver does not support MIN_TLS_VERSION and MAX_TLS_VERSION")
class TlsVersionTest {
  // snowflake-jdbc expects TLSv1.2, TLSv1.3 (not tls12, tls13), we use those values
  // to verify that this behavior is still respected (new jdbc accepts both formats)

  private static final String OFFLINE_JDBC_URL = "jdbc:snowflake://localhost/";

  @BeforeAll
  static void loadDriver() throws Exception {
    Assumptions.assumeTrue(
        WiremockClient.requiresJava11OrHigher(),
        "WireMock 3.x requires JRE 11+ to launch; skipping TLS wiremock tests on Java 8");
    Class.forName(SnowflakeDriver.class.getName());
  }

  @Test
  void shouldNegotiateTlsWhenTheServerOffersAVersionInsideTheWindow() throws Exception {
    // Given a TLS server that offers only TLS 1.3
    try (WiremockClient wiremock = new WiremockClient("tls13")) {
      wiremock.start();
      wiremock.addMapping("auth/login_success_any.json");
      // And a client configured with min_tls_version tls12 and max_tls_version tls13
      Properties props = httpsConnectionProps();
      props.setProperty("MIN_TLS_VERSION", "TLSv1.2");
      props.setProperty("MAX_TLS_VERSION", "TLSv1.3");
      // When a request is sent to the server
      try (Connection conn = DriverManager.getConnection(jdbcUrlFor(wiremock), props)) {
        // Then the handshake succeeds
        assertFalse(conn.isClosed());
      }
    }
  }

  @Test
  void shouldFailTheHandshakeWhenTheServerOnlyOffersAVersionBelowTheMinimum() throws Exception {
    // Given a TLS server that offers only TLS 1.2
    try (WiremockClient wiremock = new WiremockClient("tls12")) {
      wiremock.start();
      wiremock.addMapping("auth/login_success_any.json");
      // And a client configured with min_tls_version tls13
      Properties props = httpsConnectionProps();
      props.setProperty("min_tls_version", "TLSv1.3");
      // When a request is sent to the server
      Executable connect = () -> DriverManager.getConnection(jdbcUrlFor(wiremock), props);
      // Then the handshake fails
      SQLException exception = assertThrows(SQLException.class, connect);
      assertTrue(
          exception.getMessage().toLowerCase().contains("protocolversion"),
          () ->
              "Expected a TLS protocol-version handshake failure, but got: "
                  + exception.getMessage());
      // Positive control: same server succeeds with a permissive window
      Properties permissive = httpsConnectionProps();
      permissive.setProperty("min_tls_version", "TLSv1.2");
      permissive.setProperty("max_tls_version", "TLSv1.3");
      try (Connection conn = DriverManager.getConnection(jdbcUrlFor(wiremock), permissive)) {
        assertFalse(conn.isClosed(), "Same TLS 1.2 server must succeed with a permissive window");
      }
    }
  }

  @Test
  void shouldRejectTheConfigurationWhenTheMinimumExceedsTheMaximum() {
    // Given settings with min_tls_version tls13 and max_tls_version tls12
    Properties props = new Properties();
    props.setProperty("account", "test_account");
    props.setProperty("user", "test_user");
    props.setProperty("password", "test_password");
    props.setProperty("protocol", "http");
    props.setProperty("min_tls_version", "TLSv1.3");
    props.setProperty("max_tls_version", "TLSv1.2");
    // When the TLS configuration is built from settings
    SQLException exception =
        assertThrows(
            SQLException.class, () -> DriverManager.getConnection(OFFLINE_JDBC_URL, props));
    // Then a configuration error is returned
    assertTrue(exception.getMessage().toLowerCase().contains("max_tls_version"));
  }

  private static String jdbcUrlFor(WiremockClient wiremock) {
    URI uri = URI.create(wiremock.httpsUrl());
    return "jdbc:snowflake://" + uri.getHost() + ":" + uri.getPort() + "/";
  }

  private static Properties httpsConnectionProps() {
    Properties props = new Properties();
    props.setProperty("account", "test_account");
    props.setProperty("user", "test_user");
    props.setProperty("password", "test_password");
    props.setProperty("ssl", "true");
    props.setProperty(
        "custom_root_store_path", WiremockClient.wiremockCaPemPath().toAbsolutePath().toString());
    return props;
  }
}
