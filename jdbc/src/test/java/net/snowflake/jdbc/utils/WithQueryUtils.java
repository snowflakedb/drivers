package net.snowflake.jdbc.utils;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.UUID;

public interface WithQueryUtils {

  default void execute(Connection connection, String sql) throws Exception {
    try (Statement statement = connection.createStatement()) {
      statement.execute(sql);
    }
  }

  default String createTempTable(Connection connection, String tablePrefix, String columns)
      throws Exception {
    String tableName = tablePrefix + UUID.randomUUID().toString().replace("-", "");
    execute(connection, "CREATE TEMPORARY TABLE " + tableName + " (" + columns + ")");
    return tableName;
  }

  @FunctionalInterface
  interface ResultSetConsumer {

    void accept(ResultSet resultSet) throws Exception;
  }

  default void withQueryResult(Connection connection, String sql, ResultSetConsumer consumer)
      throws Exception {
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      consumer.accept(resultSet);
    }
  }

  @FunctionalInterface
  interface PreparedStatementSetup {

    void accept(PreparedStatement preparedStatement) throws Exception;
  }

  default void withPreparedQueryResult(
      Connection connection, String sql, PreparedStatementSetup setup, ResultSetConsumer consumer)
      throws Exception {
    try (PreparedStatement preparedStatement = connection.prepareStatement(sql)) {
      setup.accept(preparedStatement);
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        consumer.accept(resultSet);
      }
    }
  }

  default int getSizeOfResultSet(ResultSet resultSet) throws SQLException {
    int count = 0;
    while (resultSet.next()) {
      count++;
    }
    return count;
  }

  default void assertSimpleQuerySucceeds(Connection conn) throws SQLException {
    try (Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT 1")) {
      assertTrue(rs.next(), "Result set should have at least one row");
      assertEquals(1, rs.getInt(1), "SELECT 1 should return 1");
    }
  }
}
