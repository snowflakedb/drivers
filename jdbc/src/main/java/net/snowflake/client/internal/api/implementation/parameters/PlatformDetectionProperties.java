package net.snowflake.client.internal.api.implementation.parameters;

import static net.snowflake.client.internal.util.StringUtil.isBlank;

import java.util.Properties;
import lombok.AccessLevel;
import lombok.NoArgsConstructor;

/**
 * Fills the platform-detection connection properties from the JVM system properties when the
 * connection did not set them.
 *
 * <p>The connection properties are {@code platformDetectionTimeoutMs} and {@code
 * disablePlatformDetection}. The system properties are {@code
 * net.snowflake.jdbc.platformDetectionTimeoutMs} and {@code
 * net.snowflake.jdbc.disablePlatformDetection}.
 *
 * <p>A connection property that is present counts as set even when its value is unusable, so the
 * system property is not copied. Core rejects a timeout that is not an integer and ignores an
 * unrecognized disable value, which leaves {@code SNOWFLAKE_DISABLE_PLATFORM_DETECTION} in effect.
 */
@NoArgsConstructor(access = AccessLevel.PRIVATE)
public final class PlatformDetectionProperties {

  static final String TIMEOUT_PROPERTY = "platformDetectionTimeoutMs";

  static final String DISABLE_PROPERTY = "disablePlatformDetection";

  static final String TIMEOUT_SYSTEM_PROPERTY = "net.snowflake.jdbc.platformDetectionTimeoutMs";

  static final String DISABLE_SYSTEM_PROPERTY = "net.snowflake.jdbc.disablePlatformDetection";

  interface Environment {
    String getSystemProperty(String key);
  }

  public static void apply(Properties properties) {
    apply(properties, System::getProperty);
  }

  static void apply(Properties properties, Environment environment) {
    if (!hasConnectionTimeout(properties)) {
      String timeout = environment.getSystemProperty(TIMEOUT_SYSTEM_PROPERTY);
      if (isTimeout(timeout)) {
        properties.setProperty(TIMEOUT_PROPERTY, timeout.trim());
      }
    }
    if (!hasConnectionDisable(properties)) {
      String disable = environment.getSystemProperty(DISABLE_SYSTEM_PROPERTY);
      if (!isBlank(disable)) {
        properties.setProperty(DISABLE_PROPERTY, disable.trim());
      }
    }
  }

  private static boolean hasConnectionTimeout(Properties properties) {
    return hasKey(properties, TIMEOUT_PROPERTY)
        || hasKey(properties, "platform_detection_timeout_ms");
  }

  private static boolean hasConnectionDisable(Properties properties) {
    return hasKey(properties, DISABLE_PROPERTY) || hasKey(properties, "disable_platform_detection");
  }

  private static boolean hasKey(Properties properties, String key) {
    for (Object existing : properties.keySet()) {
      if (key.equalsIgnoreCase(String.valueOf(existing))) {
        return true;
      }
    }
    return false;
  }

  private static boolean isTimeout(String value) {
    if (value == null || isBlank(value)) {
      return false;
    }
    try {
      Long.parseLong(value.trim());
      return true;
    } catch (NumberFormatException e) {
      return false;
    }
  }
}
