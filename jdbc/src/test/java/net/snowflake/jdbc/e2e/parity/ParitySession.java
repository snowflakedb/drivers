package net.snowflake.jdbc.e2e.parity;

import java.sql.Connection;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.TreeMap;

/**
 * Connection wrapper that memoizes the last-applied set of session parameters so we only emit an
 * ALTER SESSION when the desired state actually changes.
 *
 * <p>The memoization key is the full sorted (param, value) map. Two equivalent maps with different
 * insertion order or different ordering of overlay entries hit the same cache.
 */
final class ParitySession {

  private final Connection conn;
  private Map<String, String> currentState = new TreeMap<>();

  ParitySession(Connection conn) {
    this.conn = conn;
  }

  Connection connection() {
    return conn;
  }

  /**
   * Apply the given set of session parameters. Issues a single ALTER SESSION SET <k1>=<v1>, ... if
   * any entry differs from the last-applied state; otherwise no-op.
   */
  void apply(Map<String, String> desired) throws SQLException {
    TreeMap<String, String> sorted = new TreeMap<>(desired);
    if (sorted.equals(currentState)) {
      return;
    }
    StringBuilder sb = new StringBuilder("ALTER SESSION SET ");
    boolean first = true;
    for (Map.Entry<String, String> e : sorted.entrySet()) {
      if (!first) {
        sb.append(", ");
      }
      first = false;
      sb.append(e.getKey()).append(" = ").append(formatValue(e.getValue()));
    }
    try (Statement s = conn.createStatement()) {
      s.execute(sb.toString());
    }
    currentState = sorted;
  }

  /** Convenience helper: build a (tz, formatParam, formatValue) + overlay map and apply. */
  void applyWithOverlay(
      String tz, String formatParam, String formatValue, Map<String, String> overlay)
      throws SQLException {
    Map<String, String> combined = new LinkedHashMap<>();
    combined.put("TIMEZONE", tz);
    combined.put(formatParam, formatValue);
    combined.putAll(overlay);
    apply(combined);
  }

  private static String formatValue(String value) {
    // Boolean session params (e.g. JDBC_FORMAT_DATE_WITH_TIMEZONE) reject the quoted form
    // 'true'/'false' with "invalid value" — emit them as bare literals.
    if ("true".equalsIgnoreCase(value) || "false".equalsIgnoreCase(value)) {
      return value;
    }
    return "'" + value.replace("'", "''") + "'";
  }
}
