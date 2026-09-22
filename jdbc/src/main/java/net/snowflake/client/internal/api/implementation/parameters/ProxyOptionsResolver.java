package net.snowflake.client.internal.api.implementation.parameters;

import static net.snowflake.client.internal.util.StringUtil.isBlank;

import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Properties;
import lombok.AccessLevel;
import lombok.NoArgsConstructor;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;

@NoArgsConstructor(access = AccessLevel.PRIVATE)
public final class ProxyOptionsResolver {

  interface Environment {
    String getSystemProperty(String key);

    String getEnvironmentVariable(String key);
  }

  private static final Environment SYSTEM_ENVIRONMENT =
      new Environment() {
        @Override
        public String getSystemProperty(String key) {
          return System.getProperty(key);
        }

        @Override
        public String getEnvironmentVariable(String key) {
          return System.getenv(key);
        }
      };

  public static Properties resolve(Properties properties) {
    return resolve(properties, SYSTEM_ENVIRONMENT);
  }

  static Properties resolve(Properties properties, Environment environment) {
    Properties resolved = new Properties();
    resolved.putAll(properties);

    // TODO(SNOW-2872386): this walks proxy keys case-insensitively in parallel with
    // ParameterKeyNormalizer. The two stay separate until key normalization lives in core.
    Object useProxy = removeIgnoreCase(resolved, "useProxy");
    Object proxyHost = removeIgnoreCase(resolved, "proxyHost");
    Object proxyPort = removeIgnoreCase(resolved, "proxyPort");
    Object proxyUser = removeIgnoreCase(resolved, "proxyUser");
    Object proxyPassword = removeIgnoreCase(resolved, "proxyPassword");
    Object nonProxyHosts = removeIgnoreCase(resolved, "nonProxyHosts");
    Object proxyProtocol = removeIgnoreCase(resolved, "proxyProtocol");
    removeIgnoreCase(resolved, "disableSocksProxy");

    if (hasCanonicalProxyConfiguration(resolved)) {
      return resolved;
    }

    if (isLegacyEnabled(useProxy)) {
      applyProxy(
          resolved,
          stringValue(proxyHost),
          stringValue(proxyPort),
          stringValue(proxyUser),
          stringValue(proxyPassword),
          stringValue(nonProxyHosts),
          stringValue(proxyProtocol));
      return resolved;
    }

    applyJvmProxy(resolved, environment);
    return resolved;
  }

  private static void applyJvmProxy(Properties properties, Environment environment) {
    if (!Boolean.parseBoolean(environment.getSystemProperty("http.useProxy"))) {
      return;
    }

    String httpHost = environment.getSystemProperty("http.proxyHost");
    String httpPort = environment.getSystemProperty("http.proxyPort");
    String httpsHost = environment.getSystemProperty("https.proxyHost");
    String httpsPort = environment.getSystemProperty("https.proxyPort");
    String protocol = environment.getSystemProperty("http.proxyProtocol");

    if (isBlank(protocol)) {
      protocol =
          isPresent(httpsHost) && isPresent(httpsPort) && isBlank(httpHost) && isBlank(httpPort)
              ? "https"
              : "http";
    }

    String nonProxyHosts =
        combineNonProxyHosts(
            environment.getSystemProperty("http.nonProxyHosts"),
            environment.getEnvironmentVariable("NO_PROXY"));

    if ("https".equalsIgnoreCase(protocol) && isPresent(httpsHost) && isPresent(httpsPort)) {
      applyProxy(
          properties,
          httpsHost,
          httpsPort,
          environment.getSystemProperty("https.proxyUser"),
          environment.getSystemProperty("https.proxyPassword"),
          nonProxyHosts,
          protocol);
    } else if ("http".equalsIgnoreCase(protocol) && isPresent(httpHost) && isPresent(httpPort)) {
      applyProxy(
          properties,
          httpHost,
          httpPort,
          environment.getSystemProperty("http.proxyUser"),
          environment.getSystemProperty("http.proxyPassword"),
          nonProxyHosts,
          protocol);
    }
  }

  private static void applyProxy(
      Properties properties,
      String host,
      String portValue,
      String user,
      String password,
      String nonProxyHosts,
      String protocol) {
    if (isBlank(host) || isBlank(portValue)) {
      throw new SFSQLException(
          ErrorCode.INVALID_PROXY_PROPERTIES, "Both proxy host and port values are needed.");
    }

    int port;
    try {
      port = Integer.parseInt(portValue);
    } catch (NumberFormatException e) {
      throw new SFSQLException(
          ErrorCode.INVALID_PROXY_PROPERTIES, "Could not parse proxy port number.");
    }

    properties.setProperty("proxy_host", host.trim());
    properties.put("proxy_port", port);
    if (protocol != null && protocol.trim().equalsIgnoreCase("https")) {
      properties.setProperty("proxy_scheme", "https");
    }
    setIfPresent(properties, "proxy_user", user);
    setIfPresent(properties, "proxy_password", password);
    setIfPresent(properties, "no_proxy", nonProxyHosts);
  }

  private static String combineNonProxyHosts(String jvmValue, String environmentValue) {
    if (isBlank(jvmValue)) {
      return environmentValue;
    }
    if (isBlank(environmentValue)) {
      return jvmValue;
    }
    return jvmValue + "|" + environmentValue;
  }

  private static boolean hasCanonicalProxyConfiguration(Properties properties) {
    return containsIgnoreCase(properties, "proxy")
        || containsIgnoreCase(properties, "proxy_host")
        || containsIgnoreCase(properties, "use_proxy_env");
  }

  private static void setIfPresent(Properties properties, String key, String value) {
    if (value != null) {
      properties.setProperty(key, value);
    }
  }

  private static Object removeIgnoreCase(Properties properties, String key) {
    Object value = null;
    List<Object> matchingKeys = new ArrayList<>();
    for (Object propertyKey : properties.keySet()) {
      if (propertyKey instanceof String
          && ((String) propertyKey).toLowerCase(Locale.ROOT).equals(key.toLowerCase(Locale.ROOT))) {
        matchingKeys.add(propertyKey);
        value = properties.get(propertyKey);
      }
    }
    matchingKeys.forEach(properties::remove);
    return value;
  }

  private static boolean containsIgnoreCase(Properties properties, String key) {
    for (Object propertyKey : properties.keySet()) {
      if (propertyKey instanceof String && ((String) propertyKey).equalsIgnoreCase(key)) {
        return true;
      }
    }
    return false;
  }

  private static boolean isPresent(String value) {
    return !isBlank(value);
  }

  private static boolean isLegacyEnabled(Object value) {
    String stringValue = stringValue(value);
    return "true".equalsIgnoreCase(stringValue) || "on".equalsIgnoreCase(stringValue);
  }

  private static String stringValue(Object value) {
    return value == null ? null : value.toString();
  }
}
