package net.snowflake.client.api.statement;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import org.junit.jupiter.api.Test;

/** Tests for executing queries through the Snowflake JDBC Driver */
public class SnowflakeStatementTest extends SnowflakeIntegrationTestBase {

  @Test
  public void testSimpleSelect() throws Exception {
    Connection conn = getDefaultConnection();
    // Create and execute statement
    try (Statement stmt = conn.createStatement()) {
      try (ResultSet rs = stmt.executeQuery("SELECT 1")) {

        // Verify result
        assertNotNull(rs, "ResultSet should not be null");
        assertTrue(rs.next(), "ResultSet should have one row");
        assertEquals(1, rs.getInt(1), "Result should be 1");
      }
    }
  }

  @Test
  public void testStatementExecuteWithDecfloatResultSet() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement();
        ResultSet rs = stmt.executeQuery("SELECT 123.456::DECFLOAT")) {
      assertNotNull(rs, "ResultSet should be available after executeQuery");
      assertTrue(rs.next(), "Expected one row");
      assertEquals(new BigDecimal("123.456"), rs.getBigDecimal(1));
      assertFalse(rs.next(), "Expected exactly one row");
    }
  }

  @Test
  public void testStatementExecuteSelectTracksCurrentResultSet() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement()) {
      assertTrue(stmt.execute("SELECT 1"), "SELECT should report a ResultSet");

      ResultSet rs = stmt.getResultSet();
      assertNotNull(rs, "Statement should expose the current ResultSet");
      assertEquals(-1, stmt.getUpdateCount(), "SELECT should not expose an update count");
      assertNotNull(((SnowflakeStatement) stmt).getQueryID(), "Query ID should be captured");

      assertTrue(rs.next(), "Expected one row");
      assertEquals(1, rs.getInt(1), "Result should be 1");
      assertFalse(rs.next(), "Expected exactly one row");
    }
  }

  @Test
  public void testStatementExecuteInsertTracksUpdateCount() throws Exception {
    Connection conn = getDefaultConnection();
    String tableName = createTempTable(conn, "ud_stmt_exec_", "v INTEGER");

    try (Statement stmt = conn.createStatement()) {
      assertFalse(stmt.execute("INSERT INTO " + tableName + " VALUES (7)"));
      assertNull(stmt.getResultSet(), "INSERT should not expose a ResultSet");
      assertEquals(1, stmt.getUpdateCount(), "INSERT should report affected rows");
      assertNotNull(((SnowflakeStatement) stmt).getQueryID(), "Query ID should be captured");
    }
  }

  @Test
  public void testStatementExecuteDdlReturnsZeroUpdateCount() throws Exception {
    Connection conn = getDefaultConnection();
    String tableName = createTempTable(conn, "ud_stmt_exec_", "v INTEGER");

    try (Statement stmt = conn.createStatement()) {
      assertFalse(stmt.execute("CREATE OR REPLACE TEMPORARY TABLE " + tableName + " (v INTEGER)"));
      assertNull(stmt.getResultSet(), "DDL should not expose a ResultSet");
      assertEquals(0, stmt.getUpdateCount(), "DDL should report zero affected rows");
    }
  }

  @Test
  public void testStatementExecuteQueryAllowsInsertForLegacyCompatibility() throws Exception {
    Connection conn = getDefaultConnection();
    String tableName = createTempTable(conn, "ud_stmt_exec_", "v INTEGER");

    try (Statement stmt = conn.createStatement()) {
      ResultSet rs =
          assertDoesNotThrow(() -> stmt.executeQuery("INSERT INTO " + tableName + " VALUES (7)"));
      if (rs != null) {
        rs.close();
      }
    }

    try (Statement verify = conn.createStatement();
        ResultSet rs = verify.executeQuery("SELECT v FROM " + tableName)) {
      assertTrue(rs.next(), "INSERT should still write a row");
      assertEquals(7, rs.getInt(1), "Inserted value should be persisted");
      assertFalse(rs.next(), "Expected exactly one row");
    }
  }

  @Test
  public void testStatementExecuteUpdateRejectsSelect() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement()) {
      assertThrows(SQLException.class, () -> stmt.executeUpdate("SELECT 1"));
    }
  }

  @Test
  public void testStatementExecuteReplacesPreviousResultSet() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement()) {
      assertTrue(stmt.execute("SELECT 1"));
      ResultSet firstResultSet = stmt.getResultSet();
      assertNotNull(firstResultSet);

      assertTrue(stmt.execute("SELECT 2"));
      assertFalse(firstResultSet.isClosed(), "Previous ResultSet should remain open");
      assertTrue(firstResultSet.next(), "Previous ResultSet should remain readable");
      assertEquals(1, firstResultSet.getInt(1), "First ResultSet should retain its data");
      assertFalse(firstResultSet.next(), "Expected exactly one row in the first ResultSet");

      ResultSet secondResultSet = stmt.getResultSet();
      assertNotNull(secondResultSet, "Statement should expose the latest ResultSet");
      assertTrue(secondResultSet.next(), "Expected one row from the second query");
      assertEquals(2, secondResultSet.getInt(1), "Result should be 2");
      assertFalse(secondResultSet.next(), "Expected exactly one row");
    }
  }

  @Test
  public void testStatementExecuteOpenResultSetsMirrorJdbcParity() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement()) {
      SnowflakeStatementImpl statementImpl = stmt.unwrap(SnowflakeStatementImpl.class);
      for (int i = 0; i < 10; i++) {
        assertTrue(stmt.execute("SELECT 1"));
        assertNotNull(stmt.getResultSet(), "Statement should expose the latest ResultSet");
      }

      assertEquals(9, statementImpl.getOpenResultSets().size());
    }

    try (Statement stmt = getDefaultConnection().createStatement()) {
      SnowflakeStatementImpl statementImpl = stmt.unwrap(SnowflakeStatementImpl.class);
      for (int i = 0; i < 10; i++) {
        assertTrue(stmt.execute("SELECT 1"));
        ResultSet rs = stmt.getResultSet();
        assertNotNull(rs, "Statement should expose the latest ResultSet");
        rs.close();
      }

      assertEquals(0, statementImpl.getOpenResultSets().size());
    }
  }
}
