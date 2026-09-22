package net.snowflake.client.internal.api.implementation.parameters;

import static net.snowflake.client.internal.util.StringUtil.isBlank;

import java.util.Locale;
import java.util.Map;
import java.util.Properties;
import java.util.function.Function;
import lombok.AccessLevel;
import lombok.NoArgsConstructor;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.connection.AutoConnectionUrl;
import net.snowflake.client.internal.api.implementation.connection.ConnectionString;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;

@NoArgsConstructor(access = AccessLevel.PRIVATE)
public final class ConnectionOptionsResolver {

  public static Properties resolve(String url, Properties properties) {
    return resolve(url, properties, System::getenv);
  }

  /** Resolves connection options using the supplied environment lookup. */
  public static Properties resolve(
      String url, Properties properties, Function<String, String> environment) {
    Properties resolved = new Properties();
    if (properties != null) {
      resolved.putAll(properties);
    }

    String effectiveUrl = effectiveUrl(url, properties);
    if (effectiveUrl != null) {
      if (AutoConnectionUrl.isAutoConnectionUrl(effectiveUrl)) {
        copyInheritedDefaults(properties, resolved);
        populateFromAutoConnectionUrl(effectiveUrl, resolved, environment);
        return resolved;
      }
      resolved.setProperty("url", effectiveUrl);
      populateFromConnectionString(effectiveUrl, resolved);
    }
    return resolved;
  }

  public static boolean usesDefaultAutoProfile(
      String url, Properties properties, Properties resolvedProperties) {
    return isAutoConnection(url, properties) && !resolvedProperties.containsKey("connection_name");
  }

  public static boolean isAutoConnection(String url, Properties properties) {
    return AutoConnectionUrl.isAutoConnectionUrl(effectiveUrl(url, properties));
  }

  static String effectiveUrl(String url, Properties properties) {
    Properties hashtableView = new Properties();
    if (properties != null) {
      hashtableView.putAll(properties);
    }
    return firstNonBlank(url, hashtableView.getProperty("url"));
  }

  private static void copyInheritedDefaults(Properties source, Properties resolved) {
    if (source == null) {
      return;
    }
    for (String name : source.stringPropertyNames()) {
      if (!hasDirectCanonicalKey(source, name)) {
        resolved.setProperty(name, source.getProperty(name));
      }
    }
  }

  private static boolean hasDirectCanonicalKey(Properties properties, String inheritedKey) {
    String inheritedCanonicalKey = AutoConnectionUrl.canonicalKey(inheritedKey);
    for (Object key : properties.keySet()) {
      if (key instanceof String
          && AutoConnectionUrl.canonicalKey((String) key).equals(inheritedCanonicalKey)) {
        return true;
      }
    }
    return false;
  }

  private static void populateFromAutoConnectionUrl(
      String jdbcUrl, Properties resolved, Function<String, String> environment) {
    Properties propertyOverrides = canonicalizeProperties(resolved);
    resolved.clear();

    Map<String, String> parameters = AutoConnectionUrl.parseParameters(jdbcUrl);
    for (Map.Entry<String, String> entry : parameters.entrySet()) {
      if (AutoConnectionUrl.isConnectionName(entry.getKey())) {
        continue;
      }
      String normalizedKey = AutoConnectionUrl.canonicalKey(entry.getKey());
      resolved.setProperty(normalizedKey, entry.getValue());
    }
    resolved.putAll(propertyOverrides);

    resolved.remove("url");
    resolved.remove("connection_name");
    String connectionName = selectConnectionName(propertyOverrides, parameters, environment);
    if (connectionName != null) {
      resolved.setProperty("connection_name", connectionName);
    }
  }

  private static Properties canonicalizeProperties(Properties properties) {
    Properties canonical = new Properties();
    properties.forEach(
        (key, value) -> {
          if (key instanceof String) {
            if (AutoConnectionUrl.isConnectionName((String) key) && !(value instanceof String)) {
              throw new SFSQLException(
                  ErrorCode.INVALID_PARAMETER_VALUE,
                  "JDBC connection Properties selector must be a String");
            }
            String normalizedKey = AutoConnectionUrl.canonicalKey((String) key);
            Object previous = canonical.get(normalizedKey);
            if (previous != null && !previous.equals(value)) {
              throw new SFSQLException(
                  ErrorCode.INVALID_PARAMETER_VALUE,
                  "Conflicting JDBC connection Properties aliases for parameter: " + normalizedKey);
            }
            canonical.put(normalizedKey, value);
          }
        });
    return canonical;
  }

  private static String selectConnectionName(
      Properties properties,
      Map<String, String> urlParameters,
      Function<String, String> environment) {
    String fromProperties = properties.getProperty("connection_name");
    if (!isBlank(fromProperties)) {
      return fromProperties;
    }
    return AutoConnectionUrl.selectConfiguredConnectionName(urlParameters, environment);
  }

  private static void populateFromConnectionString(String jdbcUrl, Properties resolved) {
    ConnectionString parsed = ConnectionString.parse(jdbcUrl, resolved);
    if (!parsed.isValid()) {
      return;
    }

    setIfAbsent(resolved, "host", parsed.getHost());
    if (parsed.getPort() > 0) {
      setIfAbsentInt(resolved, "port", parsed.getPort());
    }

    if (parsed.getAccount() != null) {
      setIfAbsent(resolved, "account", parsed.getAccount());
    }

    for (Map.Entry<String, Object> entry : parsed.getParameters().entrySet()) {
      String normalizedKey = entry.getKey().toLowerCase(Locale.ROOT);
      if (normalizedKey.trim().isEmpty()) {
        continue;
      }
      Object value = entry.getValue();
      setIfAbsent(resolved, normalizedKey, value.toString());
    }

    // Derive "protocol" from the scheme only when "ssl" is absent: sf_core derives the scheme from
    // ssl itself and rejects ssl + protocol set together (ConflictingParameters).
    if (!containsKeyIgnoreCase(resolved, "ssl")) {
      setIfAbsent(resolved, "protocol", parsed.getScheme());
    }
  }

  private static void setIfAbsent(Properties resolved, String key, String value) {
    if (value != null && !value.isEmpty() && !containsKeyIgnoreCase(resolved, key)) {
      resolved.setProperty(key, value);
    }
  }

  private static boolean containsKeyIgnoreCase(Properties props, String key) {
    for (Object k : props.keySet()) {
      if (k instanceof String && ((String) k).equalsIgnoreCase(key)) {
        return true;
      }
    }
    return false;
  }

  private static void setIfAbsentInt(Properties resolved, String key, int value) {
    if (!resolved.containsKey(key)) {
      resolved.put(key, value);
    }
  }

  private static String firstNonBlank(String first, String second) {
    if (!isBlank(first)) {
      return first;
    }
    if (!isBlank(second)) {
      return second;
    }
    return null;
  }
}
