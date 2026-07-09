package net.snowflake.client.internal.api.implementation.connection;

import static net.snowflake.client.api.exception.ErrorCode.CONNECTION_ESTABLISHED_WITH_DIFFERENT_PROP;

import java.sql.SQLWarning;
import java.util.Locale;
import java.util.Properties;
import lombok.experimental.UtilityClass;
import net.snowflake.client.api.exception.SFException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetInfoResponse;

/**
 * When the session was requested with a database/schema/role/warehouse that the server did not
 * honor, each mismatch is surfaced as a non-fatal {@link SQLWarning} (code 200041, SQLSTATE 01000).
 * The requested values come from the resolved connection properties; the actual session values come
 * from the core. Comparison is case-insensitive, and warnings are chained in the order Database,
 * Schema, Role, Warehouse to match the legacy driver.
 */
@UtilityClass
final class ConnectionEstablishedWarnings {
  /** Builds the warning chain for a freshly-established session. */
  static SQLWarning compute(Properties requestedProperties, ConnectionGetInfoResponse sessionInfo) {
    SQLWarning head = null;
    head =
        appendIfMismatch(
            head,
            "Database",
            requestedProperty(requestedProperties, "database", "db"),
            actualValue(sessionInfo, "database"));
    head =
        appendIfMismatch(
            head,
            "Schema",
            requestedProperty(requestedProperties, "schema"),
            actualValue(sessionInfo, "schema"));
    head =
        appendIfMismatch(
            head,
            "Role",
            requestedProperty(requestedProperties, "role"),
            actualValue(sessionInfo, "role"));
    head =
        appendIfMismatch(
            head,
            "Warehouse",
            requestedProperty(requestedProperties, "warehouse"),
            actualValue(sessionInfo, "warehouse"));
    return head;
  }

  private static SQLWarning appendIfMismatch(
      SQLWarning head, String property, String requested, String actual) {
    if (requested == null || requested.equalsIgnoreCase(actual)) {
      return head;
    }
    SFException e =
        new SFException(CONNECTION_ESTABLISHED_WITH_DIFFERENT_PROP, property, requested, actual);
    SQLWarning warning =
        new SQLWarning(
            e.getMessage(), e.getErrorCode().getSqlState(), e.getErrorCode().getMessageCode());
    if (head == null) {
      return warning;
    }
    // SQLWarning#setNextWarning appends to the end of the chain, preserving insertion order.
    head.setNextWarning(warning);
    return head;
  }

  /** Case-insensitive lookup of the first present requested value among the given property keys. */
  private static String requestedProperty(Properties requestedProperties, String... keys) {
    for (String key : keys) {
      for (Object k : requestedProperties.keySet()) {
        if (k instanceof String && ((String) k).equalsIgnoreCase(key)) {
          String value = requestedProperties.getProperty((String) k);
          if (value != null && !value.isEmpty()) {
            return value;
          }
        }
      }
    }
    return null;
  }

  private static String actualValue(ConnectionGetInfoResponse info, String property) {
    if (info == null) {
      return null;
    }
    switch (property.toLowerCase(Locale.ROOT)) {
      case "database":
        return info.hasDatabase() && !info.getDatabase().isEmpty() ? info.getDatabase() : null;
      case "schema":
        return info.hasSchema() && !info.getSchema().isEmpty() ? info.getSchema() : null;
      case "role":
        return info.hasRole() && !info.getRole().isEmpty() ? info.getRole() : null;
      case "warehouse":
        return info.hasWarehouse() && !info.getWarehouse().isEmpty() ? info.getWarehouse() : null;
      default:
        return null;
    }
  }
}
