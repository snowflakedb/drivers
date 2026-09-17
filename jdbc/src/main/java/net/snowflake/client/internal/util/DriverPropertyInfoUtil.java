package net.snowflake.client.internal.util;

import static lombok.AccessLevel.PRIVATE;
import static net.snowflake.client.internal.util.StringUtil.isNullOrEmpty;

import java.security.PrivateKey;
import java.sql.DriverPropertyInfo;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Properties;
import lombok.NoArgsConstructor;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.connection.ConnectionString;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;

/** Resolves and validates JDBC driver property information for {@code Driver.getPropertyInfo}. */
@NoArgsConstructor(access = PRIVATE)
public final class DriverPropertyInfoUtil {
  public static DriverPropertyInfo[] getPropertyInfo(String url, Properties info) {
    if (isNullOrEmpty(url)) {
      return new DriverPropertyInfo[] {
        propertyInfo(
            "serverURL",
            "server URL in form of <protocol>://<host or domain>:<port number>/<path of resource>")
      };
    }

    Properties resolvedProperties = resolve(url, info);
    validate(resolvedProperties);
    return findMissingProperties(resolvedProperties);
  }

  static DriverPropertyInfo[] findMissingProperties(Properties resolved) {
    List<DriverPropertyInfo> missing = new ArrayList<>();
    String authenticator = stringProperty(resolved, "authenticator");
    if (requiresPasswordCredentials(authenticator)
        && !usesPrivateKeyCredentials(resolved, authenticator)) {
      if (isMissing(resolved, "user")) {
        missing.add(propertyInfo("user", "username for account"));
      }
      if (isMissing(resolved, "password")) {
        missing.add(propertyInfo("password", "password for account"));
      }
    }
    if (booleanProperty(resolved, "useProxy", "use_proxy")) {
      if (!hasProperty(resolved, "proxyHost", "proxy_host")) {
        missing.add(propertyInfo("proxyHost", "proxy host name"));
      }
      if (!hasProperty(resolved, "proxyPort", "proxy_port")) {
        missing.add(propertyInfo("proxyPort", "proxy port; should be an integer"));
      }
    }
    return missing.toArray(new DriverPropertyInfo[0]);
  }

  static Properties resolve(String url, Properties info) {
    ConnectionString parsed = ConnectionString.parse(url, info);
    if (!parsed.isValid()) {
      throw SFSQLException.fromErrorCode(ErrorCode.INVALID_CONNECTION_STRING);
    }
    Properties resolved = new Properties();
    resolved.putAll(parsed.getParameters());
    return resolved;
  }

  static void validate(Properties resolved) {
    validatePropertyType(resolved, "user", String.class);
    validatePropertyType(resolved, "password", String.class);
    validatePropertyType(resolved, "authenticator", String.class);
    validateBooleanProperty(resolved, "useProxy", "use_proxy");
    validatePropertyType(resolved, "proxyHost", String.class);
    validatePropertyType(resolved, "proxy_host", String.class);
    validatePropertyType(resolved, "proxyPort", String.class);
    validatePropertyType(resolved, "proxy_port", String.class);
    validatePropertyType(resolved, "privateKey", PrivateKey.class);
    validateCanonicalPrivateKeyProperty(resolved);
    validatePropertyType(resolved, "private_key_file", String.class);
    validatePropertyType(resolved, "private_key_base64", String.class);
  }

  static boolean requiresPasswordCredentials(String authenticator) {
    return isNullOrEmpty(authenticator)
        || authenticator.equalsIgnoreCase("snowflake")
        || authenticator.equalsIgnoreCase("username_password_mfa")
        || authenticator.regionMatches(true, 0, "https://", 0, "https://".length());
  }

  static boolean usesPrivateKeyCredentials(Properties properties, String authenticator) {
    return isNullOrEmpty(authenticator)
        && hasProperty(
            properties, "privateKey", "private_key", "private_key_file", "private_key_base64");
  }

  private static DriverPropertyInfo propertyInfo(String name, String description) {
    DriverPropertyInfo property = new DriverPropertyInfo(name, null);
    property.description = description;
    return property;
  }

  private static boolean booleanProperty(Properties properties, String... names) {
    return Boolean.parseBoolean(stringProperty(properties, names));
  }

  private static boolean isMissing(Properties properties, String... names) {
    return isNullOrEmpty(stringProperty(properties, names));
  }

  private static boolean hasProperty(Properties properties, String... names) {
    for (Map.Entry<Object, Object> entry : properties.entrySet()) {
      for (String name : names) {
        if (entry.getKey().toString().equalsIgnoreCase(name)) {
          return true;
        }
      }
    }
    return false;
  }

  private static void validatePropertyType(
      Properties properties, String name, Class<?> expectedType) {
    Object value = propertyValue(properties, name);
    if (value != null && !expectedType.isInstance(value)) {
      throw invalidPropertyType(value, expectedType.getName());
    }
  }

  private static void validateBooleanProperty(Properties properties, String... names) {
    Object value = propertyValue(properties, names);
    if (value != null && !(value instanceof String) && !(value instanceof Boolean)) {
      throw invalidPropertyType(value, Boolean.class.getName());
    }
  }

  private static void validateCanonicalPrivateKeyProperty(Properties properties) {
    Object value = propertyValue(properties, "private_key");
    if (value != null && !(value instanceof String) && !(value instanceof PrivateKey)) {
      throw invalidPropertyType(
          value, String.class.getName() + " or " + PrivateKey.class.getName());
    }
  }

  private static Object propertyValue(Properties properties, String... names) {
    for (String name : names) {
      for (Map.Entry<Object, Object> entry : properties.entrySet()) {
        if (entry.getKey().toString().equalsIgnoreCase(name)) {
          return entry.getValue();
        }
      }
    }
    return null;
  }

  private static String stringProperty(Properties properties, String... names) {
    Object value = propertyValue(properties, names);
    return value == null ? null : value.toString();
  }

  private static SFSQLException invalidPropertyType(Object value, String expectedType) {
    return SFSQLException.fromErrorCode(
        ErrorCode.INVALID_PARAMETER_TYPE, value.getClass().getName(), expectedType);
  }
}
