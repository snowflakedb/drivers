package net.snowflake.client.internal.api.implementation.connection;

import java.util.Locale;
import java.util.Map;
import java.util.Properties;

final class ConnectionOptionsResolver {
  private ConnectionOptionsResolver() {}

  static Properties resolve(String url, Properties properties) {
    Properties resolved = new Properties();
    if (properties != null) {
      resolved.putAll(properties);
    }

    String effectiveUrl = firstNonBlank(url, resolved.getProperty("url"));
    if (effectiveUrl != null) {
      populateFromConnectionString(effectiveUrl, resolved);
    }
    return resolved;
  }

  private static void populateFromConnectionString(String jdbcUrl, Properties resolved) {
    ConnectionString parsed = ConnectionString.parse(jdbcUrl, resolved);
    if (!parsed.isValid()) {
      return;
    }

    setIfAbsent(resolved, "host", parsed.getHost());
    setIfAbsent(resolved, "protocol", parsed.getScheme());
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
  }

  private static void setIfAbsent(Properties resolved, String key, String value) {
    if (value != null && !value.isEmpty() && !resolved.containsKey(key)) {
      resolved.setProperty(key, value);
    }
  }

  private static void setIfAbsentInt(Properties resolved, String key, int value) {
    if (!resolved.containsKey(key)) {
      resolved.put(key, value);
    }
  }

  private static String firstNonBlank(String first, String second) {
    if (first != null && !first.trim().isEmpty()) {
      return first;
    }
    if (second != null && !second.trim().isEmpty()) {
      return second;
    }
    return null;
  }
}
