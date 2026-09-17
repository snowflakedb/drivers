package net.snowflake.client.internal.api.implementation.parameters;

import static org.junit.jupiter.api.Assertions.assertAll;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.Collections;
import java.util.HashMap;
import java.util.Map;
import java.util.Properties;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import org.junit.jupiter.api.Test;

class ProxyOptionsResolverTest {

  @Test
  void shouldTranslateLegacyConnectionProxyToCoreSettings() {
    Properties properties = new Properties();
    properties.setProperty("useProxy", "on");
    properties.setProperty("proxyHost", "proxy.example.com");
    properties.setProperty("proxyPort", "8443");
    properties.setProperty("proxyUser", "proxy-user");
    properties.setProperty("proxyPassword", "proxy-password");
    properties.setProperty("nonProxyHosts", "*.example.com|localhost");
    properties.setProperty("proxyProtocol", "https");

    Properties resolved =
        ProxyOptionsResolver.resolve(
            properties, new MapEnvironment(Collections.emptyMap(), Collections.emptyMap()));

    assertAll(
        () -> assertEquals("proxy.example.com", resolved.getProperty("proxy_host")),
        () -> assertEquals(8443, resolved.get("proxy_port")),
        () -> assertEquals("proxy-user", resolved.getProperty("proxy_user")),
        () -> assertEquals("proxy-password", resolved.getProperty("proxy_password")),
        () -> assertEquals("*.example.com|localhost", resolved.getProperty("no_proxy")),
        () -> assertFalse(resolved.containsKey("useProxy")),
        () -> assertFalse(resolved.containsKey("proxyProtocol")));
  }

  @Test
  void shouldIgnoreLegacyConnectionProxyWhenUseProxyIsNotEnabled() {
    Properties properties = new Properties();
    properties.setProperty("proxyHost", "proxy.example.com");
    properties.setProperty("proxyPort", "8080");
    properties.setProperty("disableSocksProxy", "true");

    Properties resolved =
        ProxyOptionsResolver.resolve(
            properties, new MapEnvironment(Collections.emptyMap(), Collections.emptyMap()));

    assertAll(
        () -> assertFalse(resolved.containsKey("proxy_host")),
        () -> assertFalse(resolved.containsKey("proxyHost")),
        () -> assertFalse(resolved.containsKey("disableSocksProxy")));
  }

  @Test
  void shouldUseJvmHttpsProxyWhenConnectionProxyIsNotEnabled() {
    Properties properties = new Properties();
    properties.setProperty("useProxy", "false");
    Map<String, String> systemProperties = new HashMap<>();
    systemProperties.put("http.useProxy", "true");
    systemProperties.put("http.proxyProtocol", "https");
    systemProperties.put("https.proxyHost", "secure-proxy.example.com");
    systemProperties.put("https.proxyPort", "9443");
    systemProperties.put("https.proxyUser", "proxy-user");
    systemProperties.put("https.proxyPassword", "proxy-password");
    systemProperties.put("http.nonProxyHosts", "localhost|*.internal");
    Map<String, String> environmentVariables = new HashMap<>();
    environmentVariables.put("NO_PROXY", "metadata.example.com");

    Properties resolved =
        ProxyOptionsResolver.resolve(
            properties, new MapEnvironment(systemProperties, environmentVariables));

    assertAll(
        () -> assertEquals("secure-proxy.example.com", resolved.getProperty("proxy_host")),
        () -> assertEquals(9443, resolved.get("proxy_port")),
        () -> assertEquals("proxy-user", resolved.getProperty("proxy_user")),
        () -> assertEquals("proxy-password", resolved.getProperty("proxy_password")),
        () ->
            assertEquals(
                "localhost|*.internal|metadata.example.com", resolved.getProperty("no_proxy")));
  }

  @Test
  void shouldUseJvmHttpProxyWhenConnectionProxyIsNotEnabled() {
    Properties properties = new Properties();
    properties.setProperty("useProxy", "false");
    Map<String, String> systemProperties = new HashMap<>();
    systemProperties.put("http.useProxy", "true");
    systemProperties.put("http.proxyHost", "proxy.example.com");
    systemProperties.put("http.proxyPort", "8080");
    systemProperties.put("http.proxyUser", "proxy-user");
    systemProperties.put("http.proxyPassword", "proxy-password");

    Properties resolved =
        ProxyOptionsResolver.resolve(
            properties, new MapEnvironment(systemProperties, Collections.emptyMap()));

    assertAll(
        () -> assertEquals("proxy.example.com", resolved.getProperty("proxy_host")),
        () -> assertEquals(8080, resolved.get("proxy_port")),
        () -> assertEquals("proxy-user", resolved.getProperty("proxy_user")),
        () -> assertEquals("proxy-password", resolved.getProperty("proxy_password")));
  }

  @Test
  void shouldPreserveCanonicalCoreProxySettings() {
    Properties properties = new Properties();
    properties.setProperty("proxy_host", "canonical-proxy.example.com");
    properties.setProperty("proxy_port", "8080");
    properties.setProperty("useProxy", "false");

    Properties resolved =
        ProxyOptionsResolver.resolve(
            properties, new MapEnvironment(Collections.emptyMap(), Collections.emptyMap()));

    assertAll(
        () -> assertEquals("canonical-proxy.example.com", resolved.getProperty("proxy_host")),
        () -> assertEquals("8080", resolved.getProperty("proxy_port")),
        () -> assertFalse(resolved.containsKey("useProxy")));
  }

  @Test
  void shouldRejectLegacyProxyWithoutBothHostAndPort() {
    Properties properties = new Properties();
    properties.setProperty("useProxy", "true");
    properties.setProperty("proxyHost", "proxy.example.com");

    SFSQLException exception =
        assertThrows(
            SFSQLException.class,
            () ->
                ProxyOptionsResolver.resolve(
                    properties,
                    new MapEnvironment(Collections.emptyMap(), Collections.emptyMap())));

    assertAll(
        () -> assertEquals(ErrorCode.INVALID_PROXY_PROPERTIES, exception.getErrorCode()),
        () ->
            assertEquals(
                ErrorCode.INVALID_PROXY_PROPERTIES.getSqlState(),
                exception.toSQLException().getSQLState()),
        () ->
            assertEquals(
                ErrorCode.INVALID_PROXY_PROPERTIES.getMessageCode(),
                exception.toSQLException().getErrorCode()));
  }

  @Test
  void shouldRejectLegacyProxyWhenPortIsNotNumeric() {
    Properties properties = new Properties();
    properties.setProperty("useProxy", "true");
    properties.setProperty("proxyHost", "proxy.example.com");
    properties.setProperty("proxyPort", "not-a-port");

    SFSQLException exception =
        assertThrows(
            SFSQLException.class,
            () ->
                ProxyOptionsResolver.resolve(
                    properties,
                    new MapEnvironment(Collections.emptyMap(), Collections.emptyMap())));

    assertAll(
        () -> assertEquals(ErrorCode.INVALID_PROXY_PROPERTIES, exception.getErrorCode()),
        () ->
            assertEquals(
                ErrorCode.INVALID_PROXY_PROPERTIES.getSqlState(),
                exception.toSQLException().getSQLState()),
        () ->
            assertEquals(
                ErrorCode.INVALID_PROXY_PROPERTIES.getMessageCode(),
                exception.toSQLException().getErrorCode()));
  }

  private static final class MapEnvironment implements ProxyOptionsResolver.Environment {
    private final Map<String, String> systemProperties;
    private final Map<String, String> environmentVariables;

    private MapEnvironment(
        Map<String, String> systemProperties, Map<String, String> environmentVariables) {
      this.systemProperties = new HashMap<>(systemProperties);
      this.environmentVariables = new HashMap<>(environmentVariables);
    }

    @Override
    public String getSystemProperty(String key) {
      return systemProperties.get(key);
    }

    @Override
    public String getEnvironmentVariable(String key) {
      return environmentVariables.get(key);
    }
  }
}
