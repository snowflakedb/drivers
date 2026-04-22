package net.snowflake.jdbc.e2e.query;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import net.snowflake.client.api.statement.SnowflakeStatement;
import org.junit.jupiter.api.Test;

public class MultistatementTests extends SnowflakeIntegrationTestBase {

  @Test
  public void shouldExecuteMultipleSelectStatements() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Multistatement query with 3 SELECTs is executed
    try (Statement statement = connection.createStatement()) {
      statement.unwrap(SnowflakeStatement.class).setParameter("MULTI_STATEMENT_COUNT", 3);
      boolean hasResultSet = statement.execute("SELECT 1 AS a; SELECT 2 AS b; SELECT 3 AS c");

      // Then 3 result sets are returned
      assertTrue(hasResultSet, "First statement should produce a result set");
      // And each result set contains correct data
      try (ResultSet rs1 = statement.getResultSet()) {
        assertTrue(rs1.next(), "First result set should have a row");
        assertEquals(1, rs1.getInt(1), "First result set should contain 1");
        assertFalse(rs1.next(), "First result set should have exactly one row");
      }

      assertTrue(statement.getMoreResults(), "Second statement should produce a result set");
      try (ResultSet rs2 = statement.getResultSet()) {
        assertTrue(rs2.next(), "Second result set should have a row");
        assertEquals(2, rs2.getInt(1), "Second result set should contain 2");
        assertFalse(rs2.next(), "Second result set should have exactly one row");
      }

      assertTrue(statement.getMoreResults(), "Third statement should produce a result set");
      try (ResultSet rs3 = statement.getResultSet()) {
        assertTrue(rs3.next(), "Third result set should have a row");
        assertEquals(3, rs3.getInt(1), "Third result set should contain 3");
        assertFalse(rs3.next(), "Third result set should have exactly one row");
      }

      assertFalse(statement.getMoreResults(), "No more results after third statement");
    }
  }

  @Test
  public void shouldExecuteMultipleDmlStatements() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Multistatement query with CREATE TABLE, INSERT, and DROP is executed
    String sql =
        "CREATE OR REPLACE TEMPORARY TABLE ms_dml_test(id INT);"
            + " INSERT INTO ms_dml_test VALUES (1),(2),(3);"
            + " DROP TABLE ms_dml_test";
    try (Statement statement = connection.createStatement()) {
      statement.unwrap(SnowflakeStatement.class).setParameter("MULTI_STATEMENT_COUNT", 3);
      statement.execute(sql);

      // Then 3 result sets are returned
      assertEquals(null, statement.getResultSet(), "CREATE should not produce a result set");
      // First result: CREATE TABLE (update count = 0)
      assertEquals(0, statement.getUpdateCount(), "CREATE should return update count 0");

      // Second result: INSERT (update count = 3 rows)
      assertTrue(statement.getMoreResults(), "Should have more results");
      assertEquals(null, statement.getResultSet(), "INSERT should not produce a result set");
      assertEquals(3, statement.getUpdateCount(), "INSERT should affect 3 rows");

      // Third result: DROP TABLE (update count = 0)
      assertFalse(statement.getMoreResults(), "Last DML statement returns false");
      assertEquals(null, statement.getResultSet(), "DROP should not produce a result set");
      assertEquals(0, statement.getUpdateCount(), "DROP should return update count 0");

      // No more results
      assertFalse(statement.getMoreResults());
      assertEquals(-1, statement.getUpdateCount(), "No more results");
    }
  }

  @Test
  public void shouldExecuteMixedStatementTypes() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Multistatement query with various types is executed
    String sql =
        "ALTER SESSION SET TIMEZONE='UTC';"
            + " CREATE OR REPLACE TEMPORARY TABLE ms_mix_test(val TEXT);"
            + " INSERT INTO ms_mix_test VALUES ('hello');"
            + " SELECT val FROM ms_mix_test;"
            + " DROP TABLE ms_mix_test";
    try (Statement statement = connection.createStatement()) {
      statement.unwrap(SnowflakeStatement.class).setParameter("MULTI_STATEMENT_COUNT", 5);
      statement.execute(sql);

      // Then 5 result sets are returned
      assertEquals(null, statement.getResultSet(), "ALTER SESSION should not produce a result set");
      // First result: ALTER SESSION (update count = 0)
      assertEquals(0, statement.getUpdateCount(), "ALTER SESSION should return update count 0");

      // Second result: CREATE TABLE (update count = 0)
      assertTrue(statement.getMoreResults(), "Should have more results");
      assertEquals(null, statement.getResultSet(), "CREATE should not produce a result set");
      assertEquals(0, statement.getUpdateCount(), "CREATE should return update count 0");

      // Third result: INSERT (update count = 1)
      assertTrue(statement.getMoreResults(), "Should have more results");
      assertEquals(null, statement.getResultSet(), "INSERT should not produce a result set");
      assertEquals(1, statement.getUpdateCount(), "INSERT should affect 1 row");

      // Fourth result: SELECT (result set with data)
      // And the SELECT result contains expected data
      assertTrue(statement.getMoreResults(), "Should have more results");
      try (ResultSet rs = statement.getResultSet()) {
        assertTrue(rs.next(), "SELECT should return a row");
        assertEquals("hello", rs.getString(1), "SELECT should return 'hello'");
        assertFalse(rs.next(), "SELECT should return exactly one row");
      }

      // Fifth result: DROP TABLE (update count = 0)
      assertFalse(statement.getMoreResults(), "Last DML statement returns false");
      assertEquals(null, statement.getResultSet(), "DROP should not produce a result set");
      assertEquals(0, statement.getUpdateCount(), "DROP should return update count 0");

      // No more results
      assertFalse(statement.getMoreResults());
      assertEquals(-1, statement.getUpdateCount(), "No more results");
    }
  }

  @Test
  public void shouldSucceedWhenMultistatementSqlIsSentWithoutMultiStatementCount() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Multistatement SQL is executed without configuring multi_statement_count
    // the driver transparently sends MULTI_STATEMENT_COUNT=0 (unlimited).
    try (Statement statement = connection.createStatement()) {
      // Then the statement succeeds
      assertTrue(statement.execute("SELECT 1; SELECT 2; SELECT 3"));
    }
  }

  @Test
  public void shouldFailWhenMultiStatementCountDoesNotMatchActualStatementCount() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Single SELECT is executed with multi_statement_count set to 3
    assertThrows(
        SQLException.class,
        () -> {
          // Then an error is returned indicating statement count mismatch
          try (Statement statement = connection.createStatement()) {
            statement.unwrap(SnowflakeStatement.class).setParameter("MULTI_STATEMENT_COUNT", 3);
            statement.execute("SELECT 1");
          }
        });
  }
}
