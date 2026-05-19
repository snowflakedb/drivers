package net.snowflake.jdbc.e2e.query;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.sql.Types;
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
  public void shouldFailWhenMultistatementSqlIsSentWithoutMultiStatementCount() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Multistatement SQL is executed without configuring multi_statement_count
    assertThrows(
        SQLException.class,
        () -> {
          // Then an error is returned indicating multi-statement is not enabled
          try (Statement statement = connection.createStatement()) {
            statement.execute("SELECT 1; SELECT 2; SELECT 3");
          }
        });
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

  @Test
  public void shouldExecuteMultistatementDmlWithPositionalParameters() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with column (id NUMBER) exists
    String tableName = "ms_bind_dml";
    try (Statement setup = connection.createStatement()) {
      setup.execute("CREATE OR REPLACE TEMPORARY TABLE " + tableName + "(id NUMBER)");
    }

    // When Multistatement INSERT chain is executed with 3 positional parameters
    String sql =
        "INSERT INTO " + tableName + " VALUES(?);" + " INSERT INTO " + tableName + " VALUES(?),(?)";
    try (PreparedStatement ps = connection.prepareStatement(sql)) {
      ps.unwrap(SnowflakeStatement.class).setParameter("MULTI_STATEMENT_COUNT", 2);
      ps.setInt(1, 10);
      ps.setInt(2, 20);
      ps.setInt(3, 30);
      ps.execute();

      // Then 2 result sets are returned
      assertEquals(null, ps.getResultSet(), "First INSERT should not produce a result set");

      // And the first result set reports update count 1
      assertEquals(1, ps.getUpdateCount(), "First INSERT should affect 1 row");

      // And the second result set reports update count 2
      assertFalse(ps.getMoreResults());
      assertEquals(2, ps.getUpdateCount(), "Second INSERT should affect 2 rows");
      assertFalse(ps.getMoreResults());
      assertEquals(-1, ps.getUpdateCount(), "No more results");
    }

    // And the table contains rows [10, 20, 30]
    try (Statement check = connection.createStatement();
        ResultSet rs = check.executeQuery("SELECT id FROM " + tableName + " ORDER BY id")) {
      assertTrue(rs.next());
      assertEquals(10, rs.getInt(1));
      assertTrue(rs.next());
      assertEquals(20, rs.getInt(1));
      assertTrue(rs.next());
      assertEquals(30, rs.getInt(1));
      assertFalse(rs.next(), "Table should contain exactly 3 rows");
    }
  }

  @Test
  public void shouldExecuteMultistatementSelectWithPositionalParameters() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Multistatement SELECT chain is executed with 6 positional parameters
    try (PreparedStatement ps =
        connection.prepareStatement("SELECT ?; SELECT ?, ?; SELECT ?, ?, ?")) {
      ps.unwrap(SnowflakeStatement.class).setParameter("MULTI_STATEMENT_COUNT", 3);
      ps.setInt(1, 10);
      ps.setInt(2, 20);
      ps.setInt(3, 30);
      ps.setInt(4, 40);
      ps.setInt(5, 50);
      ps.setInt(6, 60);
      boolean hasResultSet = ps.execute();

      // Then 3 result sets are returned
      assertTrue(hasResultSet, "First SELECT should produce a result set");

      // And the first result set contains row [10]
      try (ResultSet rs1 = ps.getResultSet()) {
        assertTrue(rs1.next());
        assertEquals(10, rs1.getInt(1));
        assertFalse(rs1.next());
      }

      // And the second result set contains row [20, 30]
      assertTrue(ps.getMoreResults());
      try (ResultSet rs2 = ps.getResultSet()) {
        assertTrue(rs2.next());
        assertEquals(20, rs2.getInt(1));
        assertEquals(30, rs2.getInt(2));
        assertFalse(rs2.next());
      }

      // And the third result set contains row [40, 50, 60]
      assertTrue(ps.getMoreResults());
      try (ResultSet rs3 = ps.getResultSet()) {
        assertTrue(rs3.next());
        assertEquals(40, rs3.getInt(1));
        assertEquals(50, rs3.getInt(2));
        assertEquals(60, rs3.getInt(3));
        assertFalse(rs3.next());
      }

      assertFalse(ps.getMoreResults(), "No more results after third statement");
      assertEquals(-1, ps.getUpdateCount(), "No more results");
    }
  }

  @Test
  public void shouldFailWhenMultistatementQueryHasTooFewParameters() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Multistatement SELECT requires 3 parameters but only 1 is bound
    assertThrows(
        SQLException.class,
        () -> {
          // Then an error is returned indicating parameter count mismatch
          try (PreparedStatement ps = connection.prepareStatement("SELECT ?; SELECT ?, ?")) {
            ps.unwrap(SnowflakeStatement.class).setParameter("MULTI_STATEMENT_COUNT", 2);
            ps.setInt(1, 10);
            ps.execute();
          }
        });
  }

  @Test
  public void shouldFailWhenNullPositionalParametersAreUsedInMultistatementQuery()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Multistatement SELECT is executed with NULL positional parameters
    SQLException ex =
        assertThrows(
            SQLException.class,
            () -> {
              // Then an error is returned indicating NULL bindings are not supported
              try (PreparedStatement ps = connection.prepareStatement("SELECT ?; SELECT ?, ?")) {
                ps.unwrap(SnowflakeStatement.class).setParameter("MULTI_STATEMENT_COUNT", 2);
                ps.setNull(1, Types.INTEGER);
                ps.setInt(2, 10);
                ps.setNull(3, Types.INTEGER);
                ps.execute();
              }
            });
    // Server surfaces "Bind variable ? not set" — match loosely on "bind".
    assertTrue(
        ex.getMessage().toLowerCase().contains("bind"),
        "Expected error to mention bind variables, got: " + ex.getMessage());
  }
}
