package net.snowflake.client.api.resultset;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.Statement;
import net.snowflake.client.api.connection.SnowflakeConnection;
import net.snowflake.client.api.statement.SnowflakePreparedStatement;
import net.snowflake.client.api.statement.SnowflakeStatement;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

class AsyncQueryTests extends SnowflakeIntegrationTestBase {

  @Test
  void shouldExecuteAsyncQueryAndFetchResults() throws Exception {
    Connection connection = getDefaultConnection();

    try (Statement statement = connection.createStatement()) {
      ResultSet resultSet =
          statement.unwrap(SnowflakeStatement.class).executeAsyncQuery("SELECT 42 AS value");

      String queryId = resultSet.unwrap(SnowflakeResultSet.class).getQueryID();
      assertNotNull(queryId, "Async query should return a query ID");
      assertFalse(queryId.isEmpty(), "Query ID should not be empty");

      assertTrue(resultSet.next(), "Expected one row");
      assertEquals(42, resultSet.getInt(1));
      assertFalse(resultSet.next(), "Expected exactly one row");

      resultSet.close();
    }
  }

  @Test
  void shouldGetQueryStatusViaConnectionWhenSuccess() throws Exception {
    Connection connection = getDefaultConnection();
    String sql = "SELECT 1";

    String queryId;
    try (Statement statement = connection.createStatement()) {
      ResultSet resultSet = statement.unwrap(SnowflakeStatement.class).executeAsyncQuery(sql);
      queryId = resultSet.unwrap(SnowflakeResultSet.class).getQueryID();

      assertTrue(resultSet.next());
      resultSet.close();
    }

    QueryStatus status = connection.unwrap(SnowflakeConnection.class).getQueryStatus(queryId);
    assertTrue(status.isSuccess(), "Expected SUCCESS for completed query");
    assertEquals(queryId, status.getId());
    assertEquals(sql, status.getSqlText());
    assertTrue(status.getStartTime() > 0, "Expected startTime to be populated");
    assertTrue(status.getEndTime() >= status.getStartTime(), "endTime should be >= startTime");
    assertTrue(status.getTotalDuration() >= 0, "totalDuration should be non-negative");
    assertTrue(status.getSessionId() > 0, "Expected sessionId to be populated");
    assertNotNull(status.getWarehouseName(), "Expected warehouseName to be populated");
    assertFalse(status.getWarehouseName().isEmpty(), "warehouseName should not be empty");
  }

  @Test
  void shouldGetQueryStatusViaConnectionWhenRunning() throws Exception {
    Connection connection = getDefaultConnection();
    String sql = "SELECT SYSTEM$WAIT(5)";

    String queryId;
    try (Statement statement = connection.createStatement()) {
      ResultSet resultSet = statement.unwrap(SnowflakeStatement.class).executeAsyncQuery(sql);
      queryId = resultSet.unwrap(SnowflakeResultSet.class).getQueryID();

      QueryStatus status = connection.unwrap(SnowflakeConnection.class).getQueryStatus(queryId);

      assertTrue(status.isStillRunning(), "Expected query to still be running");
      assertFalse(status.isSuccess(), "Query should not be SUCCESS yet");
      assertEquals(queryId, status.getId());
      assertEquals(sql, status.getSqlText());
      assertTrue(status.getStartTime() > 0, "Expected startTime to be populated");
      assertTrue(status.getSessionId() > 0, "Expected sessionId to be populated");
      assertNotNull(status.getWarehouseName(), "Expected warehouseName to be populated");

      assertTrue(resultSet.next(), "Expected result after waiting");
      resultSet.close();
    }
  }

  @Test
  void shouldFetchResultsViaCreateResultSet() throws Exception {
    Connection connection = getDefaultConnection();

    String queryId;
    try (Statement statement = connection.createStatement()) {
      ResultSet asyncRs =
          statement.unwrap(SnowflakeStatement.class).executeAsyncQuery("SELECT 99 AS value");
      queryId = asyncRs.unwrap(SnowflakeResultSet.class).getQueryID();

      assertTrue(asyncRs.next());
      asyncRs.close();
    }

    ResultSet resultSet = connection.unwrap(SnowflakeConnection.class).createResultSet(queryId);

    assertTrue(resultSet.next(), "Expected one row");
    assertEquals(99, resultSet.getInt(1));
    assertFalse(resultSet.next(), "Expected exactly one row");
    resultSet.close();
  }

  @Test
  void shouldExecuteAsyncQueryWithPreparedStatement() throws Exception {
    Connection connection = getDefaultConnection();

    try (PreparedStatement preparedStatement = connection.prepareStatement("SELECT ? AS value")) {
      preparedStatement.setInt(1, 77);
      ResultSet resultSet =
          preparedStatement.unwrap(SnowflakePreparedStatement.class).executeAsyncQuery();

      assertNotNull(
          resultSet.unwrap(SnowflakeResultSet.class).getQueryID(),
          "Async prepared statement should return a query ID");
      assertTrue(resultSet.next(), "Expected one row");
      assertEquals(77, resultSet.getInt(1));
      assertFalse(resultSet.next(), "Expected exactly one row");

      resultSet.close();
    }
  }

  @Test
  void shouldReturnCorrectPositionBeforeMaterialization() throws Exception {
    Connection connection = getDefaultConnection();

    try (Statement statement = connection.createStatement()) {
      ResultSet resultSet =
          statement.unwrap(SnowflakeStatement.class).executeAsyncQuery("SELECT 1");

      assertTrue(resultSet.isBeforeFirst(), "Should be before first before next()");
      assertFalse(resultSet.isAfterLast(), "Should not be after last before next()");
      assertFalse(resultSet.isFirst(), "Should not be first before next()");
      assertEquals(0, resultSet.getRow(), "Row should be 0 before next()");

      resultSet.close();
    }
  }
}
