package net.snowflake.jdbc.e2e.session;

import static net.snowflake.jdbc.utils.DriverCompatibility.isOldDriver;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.net.URI;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.jdbc.wiremock.BaseWiremockTest;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

class ProxyTests extends BaseWiremockTest {

  private static final String TARGET_JDBC_URL = "jdbc:snowflake://proxy-target.snowflake.com:8090/";

  @BeforeAll
  void loadDriver() throws Exception {
    Class.forName(SnowflakeDriver.class.getName());
  }

  @Test
  void shouldRouteRequestThroughProxyWhenProxyHostAndPortAreConfigured() throws Exception {
    // Given a forward-proxy WireMock serving a canned login response
    wiremock.addMapping("auth/login_success_any.json");
    wiremock.addMapping("session/logout_success.json");

    // When the driver connects with the proxy host and port pointing at the proxy
    Properties properties = connectionProperties();
    properties.setProperty("useProxy", "on");
    properties.setProperty("proxyHost", proxyHost());
    properties.setProperty("proxyPort", Integer.toString(proxyPort()));
    try (Connection connection = DriverManager.getConnection(TARGET_JDBC_URL, properties)) {
      assertFalse(connection.isClosed());
    }

    // Then the connect succeeds and the proxy received the login request
    wiremock.verifyRequestCount(1, "/session/v1/login-request");
  }

  @Test
  void shouldBypassProxyWhenTheTargetHostIsExcludedFromProxying() {
    // Given a forward-proxy WireMock serving a canned login response
    wiremock.addMapping("auth/login_success_any.json");

    // When the driver connects with a proxy and the target host excluded from proxying
    Properties properties = connectionProperties();
    properties.setProperty("useProxy", "true");
    properties.setProperty("proxyHost", proxyHost());
    properties.setProperty("proxyPort", Integer.toString(proxyPort()));
    properties.setProperty("nonProxyHosts", "*.snowflake.com");
    properties.setProperty("loginTimeout", "2");
    assertThrows(
        SQLException.class,
        () -> {
          try (Connection ignored = DriverManager.getConnection(TARGET_JDBC_URL, properties)) {}
        });

    // Then the connect fails and the proxy received no requests
    wiremock.verifyRequestCount(0, "/session/v1/login-request");
  }

  private Properties connectionProperties() {
    Properties properties = new Properties();
    properties.setProperty("account", "test_account");
    properties.setProperty("user", "test_user");
    properties.setProperty("password", "test_password");
    if (isOldDriver()) {
      properties.setProperty("ssl", "off");
    } else {
      properties.setProperty("protocol", "http");
    }
    return properties;
  }

  private String proxyHost() {
    return URI.create(wiremock.httpUrl()).getHost();
  }

  private int proxyPort() {
    return URI.create(wiremock.httpUrl()).getPort();
  }
}
