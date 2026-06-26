package net.snowflake.client.api.metadata.reference;

import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.Statement;
import java.sql.Time;
import java.sql.Timestamp;
import java.sql.Types;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.Callable;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.ThreadLocalRandom;
import java.util.function.Consumer;
import net.snowflake.client.internal.util.SnowflakeTypeHelper;

final class TestUtil {
  static final String GENERATED_SCHEMA_PREFIX = "GENERATED_";
  static final String ESCAPED_GENERATED_SCHEMA_PREFIX =
      GENERATED_SCHEMA_PREFIX.replaceAll("_", "\\\\_");

  private static final List<String> SCHEMA_GENERATED_IN_TESTS_PREFIXES =
      Arrays.asList(GENERATED_SCHEMA_PREFIX, "GITHUB_", "GH_JOB_", "JDBCPERF", "SCHEMA_");

  private static final char[] ALPHANUMERIC =
      "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz".toCharArray();

  private TestUtil() {}

  interface ThrowingFunction<T, R, E extends Throwable> {
    R apply(T x) throws E;
  }

  @FunctionalInterface
  interface ThrowingRunnable {
    void run() throws Exception;
  }

  @FunctionalInterface
  interface ThrowingConsumer<T> {
    void accept(T value) throws Exception;
  }

  @FunctionalInterface
  interface MethodRaisesSQLException {
    void run() throws SQLException;
  }

  static String randomAlphaNumeric(int count) {
    StringBuilder sb = new StringBuilder(count);
    ThreadLocalRandom random = ThreadLocalRandom.current();
    for (int i = 0; i < count; i++) {
      sb.append(ALPHANUMERIC[random.nextInt(ALPHANUMERIC.length)]);
    }
    return sb.toString();
  }

  static String randomDatabaseName(String prefix) {
    return prefix + "_" + randomAlphaNumeric(8).toUpperCase();
  }

  static String javaTypeToClassName(int type) throws SQLException {
    switch (type) {
      case Types.VARCHAR:
      case Types.CHAR:
      case Types.STRUCT:
      case Types.ARRAY:
        return String.class.getName();

      case Types.BINARY:
        return SnowflakeTypeHelper.BINARY_CLASS_NAME;

      case Types.INTEGER:
        return Integer.class.getName();

      case Types.DECIMAL:
        return BigDecimal.class.getName();

      case Types.DOUBLE:
        return Double.class.getName();

      case Types.TIMESTAMP:
      case Types.TIMESTAMP_WITH_TIMEZONE:
        return Timestamp.class.getName();

      case Types.DATE:
        return java.sql.Date.class.getName();

      case Types.TIME:
        return Time.class.getName();

      case Types.BOOLEAN:
        return Boolean.class.getName();

      case Types.BIGINT:
        return Long.class.getName();

      case Types.SMALLINT:
        return Short.class.getName();

      default:
        throw new SQLFeatureNotSupportedException(
            String.format("No corresponding Java type is found for java.sql.Type: %d", type));
    }
  }

  static boolean isSchemaGeneratedInTests(String schema) {
    return SCHEMA_GENERATED_IN_TESTS_PREFIXES.stream().anyMatch(schema::startsWith);
  }

  static void withSchema(Statement statement, String schemaName, ThrowingRunnable action)
      throws Exception {
    try {
      statement.execute("CREATE OR REPLACE SCHEMA " + schemaName);
      action.run();
    } finally {
      statement.execute("DROP SCHEMA " + schemaName);
    }
  }

  static void withRandomSchema(Statement statement, ThrowingConsumer<String> action)
      throws Exception {
    String customSchema = GENERATED_SCHEMA_PREFIX + randomAlphaNumeric(5).toUpperCase();
    try {
      statement.execute("CREATE OR REPLACE SCHEMA " + customSchema);
      action.accept(customSchema);
    } finally {
      statement.execute("DROP SCHEMA " + customSchema);
    }
  }

  static <T> CompletableFuture<Void> asyncAssert(
      ExecutorService executor, Callable<T> supplier, Consumer<T> assertion) {
    return CompletableFuture.supplyAsync(
            () -> {
              try {
                return supplier.call();
              } catch (Exception e) {
                throw new RuntimeException(e);
              }
            },
            executor)
        .thenAccept(assertion);
  }

  static String escapeUnderscore(String input) {
    return input.replace("_", "\\_");
  }

  static void expectFeatureNotSupportedException(MethodRaisesSQLException callback) {
    SQLException exception = assertThrows(SQLException.class, callback::run);
    assertTrue(exception instanceof SQLFeatureNotSupportedException);
  }

  static List<String> getInfoBySQL(Connection connection, String sqlCmd) throws SQLException {
    List<String> result = new ArrayList<>();
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sqlCmd)) {
      while (resultSet.next()) {
        result.add(resultSet.getString(1));
      }
    }
    return result;
  }

  static boolean isJavaTypeSigned(int type) {
    return type == Types.INTEGER || type == Types.DECIMAL || type == Types.DOUBLE;
  }
}
