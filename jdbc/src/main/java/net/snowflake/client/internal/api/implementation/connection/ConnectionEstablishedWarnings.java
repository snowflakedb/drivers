package net.snowflake.client.internal.api.implementation.connection;

import static net.snowflake.client.api.exception.ErrorCode.CONNECTION_ESTABLISHED_WITH_DIFFERENT_PROP;

import java.sql.SQLWarning;
import java.util.Properties;
import lombok.experimental.UtilityClass;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.api.implementation.parameters.ParameterKeyNormalizer;
import net.snowflake.client.internal.api.implementation.parameters.SessionProperty;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetInfoResponse;

/**
 * When the session was requested with a database/schema/role/warehouse that the server did not
 * honor, each mismatch is surfaced as a non-fatal {@link SQLWarning} (code 200041, SQLSTATE 01000).
 * The requested values come from the resolved connection properties; the actual session values come
 * from the core. Comparison is case-insensitive, and warnings are chained in the order Database,
 * Schema, Role, Warehouse to match the legacy driver.
 */
@UtilityClass
class ConnectionEstablishedWarnings {
  /** Builds the warning chain for a freshly-established session. */
  static SQLWarning compute(Properties requestedProperties, ConnectionGetInfoResponse sessionInfo) {
    SQLWarning head = null;
    head =
        appendIfMismatch(
            head,
            "Database",
            requestedProperty(requestedProperties, SessionProperty.DATABASE),
            actualValue(sessionInfo, SessionProperty.DATABASE));
    head =
        appendIfMismatch(
            head,
            "Schema",
            requestedProperty(requestedProperties, SessionProperty.SCHEMA),
            actualValue(sessionInfo, SessionProperty.SCHEMA));
    head =
        appendIfMismatch(
            head,
            "Role",
            requestedProperty(requestedProperties, SessionProperty.ROLE),
            actualValue(sessionInfo, SessionProperty.ROLE));
    head =
        appendIfMismatch(
            head,
            "Warehouse",
            requestedProperty(requestedProperties, SessionProperty.WAREHOUSE),
            actualValue(sessionInfo, SessionProperty.WAREHOUSE));
    return head;
  }

  private static SQLWarning appendIfMismatch(
      SQLWarning head, String property, String requested, String actual) {
    if (requested == null || requested.equalsIgnoreCase(actual)) {
      return head;
    }
    SFSQLException e =
        SFSQLException.fromErrorCode(
            CONNECTION_ESTABLISHED_WITH_DIFFERENT_PROP, property, requested, actual);
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

  /**
   * Case-insensitive lookup of the requested value for the given property. Legacy aliases (e.g.
   * {@code db} → {@code database}) are resolved through {@link ParameterKeyNormalizer} so the
   * canonical {@link SessionProperty} key matches whichever alias the caller supplied.
   */
  private static String requestedProperty(
      Properties requestedProperties, SessionProperty property) {
    String canonicalKey = property.getKey();
    for (Object k : requestedProperties.keySet()) {
      if (k instanceof String
          && ParameterKeyNormalizer.normalize((String) k).equalsIgnoreCase(canonicalKey)) {
        String value = requestedProperties.getProperty((String) k);
        if (value != null && !value.isEmpty()) {
          return value;
        }
      }
    }
    return null;
  }

  private static String actualValue(ConnectionGetInfoResponse info, SessionProperty property) {
    if (info == null) {
      return null;
    }
    switch (property) {
      case DATABASE:
        return info.hasDatabase() && !info.getDatabase().isEmpty() ? info.getDatabase() : null;
      case SCHEMA:
        return info.hasSchema() && !info.getSchema().isEmpty() ? info.getSchema() : null;
      case ROLE:
        return info.hasRole() && !info.getRole().isEmpty() ? info.getRole() : null;
      case WAREHOUSE:
        return info.hasWarehouse() && !info.getWarehouse().isEmpty() ? info.getWarehouse() : null;
      default:
        return null;
    }
  }
}
