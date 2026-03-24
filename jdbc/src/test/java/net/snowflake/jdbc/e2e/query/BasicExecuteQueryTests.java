package net.snowflake.jdbc.e2e.query;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.UUID;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import org.junit.jupiter.api.Test;

public class BasicExecuteQueryTests extends SnowflakeIntegrationTestBase {
  private static final SFLogger logger = SFLoggerFactory.getLogger(BasicExecuteQueryTests.class);

  @Test
  public void shouldExecuteSimpleSelectReturningSingleValue() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT 1 AS value" is executed
    String sql = "SELECT 1 AS value";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      // Then the result should contain value 1
      assertTrue(resultSet.next(), "Expected one row");
      assertEquals(1, resultSet.getInt(1), "Expected value 1");
      assertFalse(resultSet.wasNull(), "Expected non-NULL value");
      assertFalse(resultSet.next(), "Expected exactly one row");
    }
  }

  @Test
  public void shouldExecuteSelectReturningMultipleColumns() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT 1 AS col1, 'hello' AS col2, '3.14' AS col3" is executed
    String sql = "SELECT 1 AS col1, 'hello' AS col2, '3.14' AS col3";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      // Then the result should contain:
      assertTrue(resultSet.next(), "Expected one row");
      assertEquals(1, resultSet.getInt(1), "Unexpected first column value");
      assertFalse(resultSet.wasNull(), "First column should not be NULL");
      assertEquals("hello", resultSet.getString(2), "Unexpected second column value");
      assertFalse(resultSet.wasNull(), "Second column should not be NULL");
      assertEquals("3.14", resultSet.getString(3), "Unexpected third column value");
      assertFalse(resultSet.wasNull(), "Third column should not be NULL");
      assertFalse(resultSet.next(), "Expected exactly one row");
    }
  }

  @Test
  public void shouldExecuteSelectReturningMultipleRows() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 5)) v ORDER BY id" is
    // executed
    String sql = "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 5)) v ORDER BY id";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      // Then there are 5 numbered sequentially rows returned
      int expectedValue = 0;
      while (resultSet.next()) {
        assertEquals(
            expectedValue, resultSet.getInt(1), "Unexpected value at row " + expectedValue);
        assertFalse(
            resultSet.wasNull(), "Sequential value should not be NULL at row " + expectedValue);
        expectedValue++;
      }
      assertEquals(5, expectedValue, "Unexpected number of sequential rows");
    }
  }

  @Test
  public void shouldExecuteSelectReturningEmptyResultSet() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT 1 WHERE 1=0" is executed
    String sql = "SELECT 1 WHERE 1=0";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      // Then the result set should be empty
      assertFalse(resultSet.next(), "Expected an empty result set");
    }
  }

  @Test
  public void shouldExecuteSelectReturningNullValues() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT NULL AS col1, 42 AS col2, NULL AS col3" is executed
    String sql = "SELECT NULL AS col1, 42 AS col2, NULL AS col3";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      // Then the result should contain NULL for col1 and col3 and 42 for col2
      assertTrue(resultSet.next(), "Expected one row");
      assertNull(resultSet.getObject(1), "Expected NULL in first column");
      assertTrue(resultSet.wasNull(), "First column should set wasNull()");
      assertEquals(42, resultSet.getInt(2), "Unexpected second column value");
      assertFalse(resultSet.wasNull(), "Second column should not be NULL");
      assertNull(resultSet.getObject(3), "Expected NULL in third column");
      assertTrue(resultSet.wasNull(), "Third column should set wasNull()");
      assertFalse(resultSet.next(), "Expected exactly one row");
    }
  }

  @Test
  public void shouldExecuteCreateAndDropTableStatements() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();
    String tableName =
        ("ud_query_create_drop_" + UUID.randomUUID().toString().replace("-", "")).toUpperCase();

    try {
      // When CREATE TABLE statement is executed
      String createTableSql = "CREATE TEMPORARY TABLE " + tableName + " (id INT)";
      execute(connection, createTableSql);

      // Then the table should be created successfully
      String selectCountSql = "SELECT COUNT(*) FROM " + tableName;
      try (Statement statement = connection.createStatement();
          ResultSet resultSet = statement.executeQuery(selectCountSql)) {
        assertTrue(resultSet.next(), "Expected one row");
        assertEquals(0, resultSet.getInt(1), "Expected empty table");
        assertFalse(resultSet.wasNull(), "Count should not be NULL");
        assertFalse(resultSet.next(), "Expected exactly one row");
      }

      // And DROP TABLE statement should complete successfully
      String dropTableSql = "DROP TABLE " + tableName;
      execute(connection, dropTableSql);
      String droppedTableQuerySql = "SELECT COUNT(*) FROM " + tableName;
      assertThrows(
          SQLException.class,
          () -> {
            try (Statement statement = connection.createStatement()) {
              statement.executeQuery(droppedTableQuerySql);
            }
          },
          "Expected querying a dropped table to fail");
    } finally {
      try (Statement statement = connection.createStatement()) {
        statement.execute("DROP TABLE IF EXISTS " + tableName);
      } catch (SQLException cleanupException) {
        logger.warn("Failed to clean up test table: {}", tableName, cleanupException);
      }
    }
  }

  @Test
  public void shouldExecuteInsertAndRetrieveInsertedData() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table is created
    String tableName = createTempTable(connection, "ud_query_insert_", "id INT, value STRING");

    // When INSERT statement is executed to add rows
    String insertSql = "INSERT INTO " + tableName + " VALUES (1, 'first'), (2, 'second')";
    execute(connection, insertSql);

    // And Query "SELECT id, value FROM {table} ORDER BY id" is executed
    String selectSql = "SELECT id, value FROM " + tableName + " ORDER BY id";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(selectSql)) {
      // Then the inserted data should be correctly returned
      assertTrue(resultSet.next(), "Expected first row");
      assertEquals(1, resultSet.getInt(1), "Unexpected id in first row");
      assertFalse(resultSet.wasNull(), "First row id should not be NULL");
      assertEquals("first", resultSet.getString(2), "Unexpected value in first row");
      assertFalse(resultSet.wasNull(), "First row value should not be NULL");

      assertTrue(resultSet.next(), "Expected second row");
      assertEquals(2, resultSet.getInt(1), "Unexpected id in second row");
      assertFalse(resultSet.wasNull(), "Second row id should not be NULL");
      assertEquals("second", resultSet.getString(2), "Unexpected value in second row");
      assertFalse(resultSet.wasNull(), "Second row value should not be NULL");

      assertFalse(resultSet.next(), "Expected exactly two rows");
    }
  }

  @Test
  public void shouldReturnErrorForInvalidSqlSyntax() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Invalid SQL "SELCT INVALID SYNTAX" is executed
    String sql = "SELCT INVALID SYNTAX";
    SQLException exception =
        assertThrows(
            SQLException.class,
            () -> {
              try (Statement statement = connection.createStatement()) {
                statement.execute(sql);
              }
            });

    // Then An error should be returned
    assertNotNull(exception.getMessage(), "Expected SQL syntax error message");
    assertEquals("42000", exception.getSQLState(), "Expected SQLState for syntax error");
    assertEquals(1003, exception.getErrorCode(), "Expected Snowflake vendor code for syntax error");
  }

  @Test
  public void shouldExecuteMultipleQueriesSequentiallyOnSameConnection() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    try (Statement statement = connection.createStatement()) {
      // When Multiple queries are executed sequentially
      String firstSql = "SELECT 1";
      String secondSql = "SELECT 'hello'";
      String thirdSql = "SELECT 3";

      // Then each query should return correct results
      try (ResultSet resultSet = statement.executeQuery(firstSql)) {
        assertTrue(resultSet.next(), "Expected one row for first query");
        assertEquals(1, resultSet.getInt(1), "Unexpected first query result");
        assertFalse(resultSet.wasNull(), "First query result should not be NULL");
        assertFalse(resultSet.next(), "Expected exactly one row for first query");
      }

      try (ResultSet resultSet = statement.executeQuery(secondSql)) {
        assertTrue(resultSet.next(), "Expected one row for second query");
        assertEquals("hello", resultSet.getString(1), "Unexpected second query result");
        assertFalse(resultSet.wasNull(), "Second query result should not be NULL");
        assertFalse(resultSet.next(), "Expected exactly one row for second query");
      }

      try (ResultSet resultSet = statement.executeQuery(thirdSql)) {
        assertTrue(resultSet.next(), "Expected one row for third query");
        assertEquals(3, resultSet.getInt(1), "Unexpected third query result");
        assertFalse(resultSet.wasNull(), "Third query result should not be NULL");
        assertFalse(resultSet.next(), "Expected exactly one row for third query");
      }
    }
  }
}
