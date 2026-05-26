package net.snowflake.client.api.statement;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.Statement;
import java.sql.Types;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

/**
 * End-to-end coverage for {@code addBatch} / {@code executeBatch} on Statement and
 * PreparedStatement.
 */
public class SnowflakeBatchExecutionTest extends SnowflakeIntegrationTestBase {

  @Test
  public void testStatementBatchExecutesEachSqlAndReturnsCounts() throws Exception {
    Connection conn = getDefaultConnection();
    String tableName = createTempTable(conn, "ud_stmt_batch_", "v INTEGER");

    try (Statement stmt = conn.createStatement()) {
      stmt.addBatch("INSERT INTO " + tableName + " VALUES (1)");
      stmt.addBatch("INSERT INTO " + tableName + " VALUES (2), (3)");
      stmt.addBatch("INSERT INTO " + tableName + " VALUES (4), (5), (6)");

      int[] counts = stmt.executeBatch();
      assertArrayEquals(new int[] {1, 2, 3}, counts);
    }

    try (Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT COUNT(*), SUM(v) FROM " + tableName)) {
      assertTrue(rs.next());
      assertEquals(6, rs.getInt(1));
      assertEquals(21, rs.getInt(2));
    }
  }

  @Test
  public void testStatementClearBatchPreventsExecution() throws Exception {
    Connection conn = getDefaultConnection();
    String tableName = createTempTable(conn, "ud_stmt_clear_", "v INTEGER");

    try (Statement stmt = conn.createStatement()) {
      stmt.addBatch("INSERT INTO " + tableName + " VALUES (1)");
      stmt.addBatch("INSERT INTO " + tableName + " VALUES (2)");
      stmt.clearBatch();

      int[] counts = stmt.executeBatch();
      assertEquals(0, counts.length);
    }

    try (Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT COUNT(*) FROM " + tableName)) {
      assertTrue(rs.next());
      assertEquals(0, rs.getInt(1));
    }
  }

  @Test
  public void testPreparedStatementBatchInsertExpandsToPerRowCounts() throws Exception {
    Connection conn = getDefaultConnection();
    String tableName = createTempTable(conn, "ud_ps_batch_", "n INTEGER, s STRING");

    try (PreparedStatement ps =
        conn.prepareStatement("INSERT INTO " + tableName + " (n, s) VALUES (?, ?)")) {
      ps.setInt(1, 1);
      ps.setString(2, "one");
      ps.addBatch();

      ps.setInt(1, 2);
      ps.setString(2, "two");
      ps.addBatch();

      ps.setInt(1, 3);
      ps.setString(2, "three");
      ps.addBatch();

      int[] counts = ps.executeBatch();
      // Snowflake aggregates inserts; the driver expands the count when it equals batchSize.
      assertArrayEquals(new int[] {1, 1, 1}, counts);
    }

    try (Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT n, s FROM " + tableName + " ORDER BY n")) {
      assertTrue(rs.next());
      assertEquals(1, rs.getInt(1));
      assertEquals("one", rs.getString(2));
      assertTrue(rs.next());
      assertEquals(2, rs.getInt(1));
      assertEquals("two", rs.getString(2));
      assertTrue(rs.next());
      assertEquals(3, rs.getInt(1));
      assertEquals("three", rs.getString(2));
    }
  }

  @Test
  public void testPreparedStatementBatchInsertWithNullValuesAcrossRows() throws Exception {
    Connection conn = getDefaultConnection();
    String tableName = createTempTable(conn, "ud_ps_batch_null_", "n INTEGER, s STRING");

    try (PreparedStatement ps =
        conn.prepareStatement("INSERT INTO " + tableName + " (n, s) VALUES (?, ?)")) {
      ps.setNull(1, Types.INTEGER);
      ps.setString(2, "first-null");
      ps.addBatch();

      ps.setInt(1, 7);
      ps.setNull(2, Types.VARCHAR);
      ps.addBatch();

      ps.setInt(1, 8);
      ps.setString(2, "third");
      ps.addBatch();

      int[] counts = ps.executeBatch();
      assertEquals(3, counts.length);
    }

    try (Statement stmt = conn.createStatement();
        ResultSet rs =
            stmt.executeQuery("SELECT n, s FROM " + tableName + " ORDER BY n NULLS FIRST")) {
      assertTrue(rs.next());
      rs.getInt(1);
      assertTrue(rs.wasNull());
      assertEquals("first-null", rs.getString(2));

      assertTrue(rs.next());
      assertEquals(7, rs.getInt(1));
      rs.getString(2);
      assertTrue(rs.wasNull());

      assertTrue(rs.next());
      assertEquals(8, rs.getInt(1));
      assertEquals("third", rs.getString(2));
    }
  }

  @Test
  public void testPreparedStatementClearBatchResetsAccumulatedRows() throws Exception {
    Connection conn = getDefaultConnection();
    String tableName = createTempTable(conn, "ud_ps_clear_", "n INTEGER");

    try (PreparedStatement ps = conn.prepareStatement("INSERT INTO " + tableName + " VALUES (?)")) {
      ps.setInt(1, 1);
      ps.addBatch();
      ps.setInt(1, 2);
      ps.addBatch();
      ps.clearBatch();

      int[] counts = ps.executeBatch();
      assertEquals(0, counts.length);

      ps.setInt(1, 99);
      ps.addBatch();
      counts = ps.executeBatch();
      assertEquals(1, counts.length);
    }

    try (Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT n FROM " + tableName)) {
      assertTrue(rs.next());
      assertEquals(99, rs.getInt(1));
      assertNotNull(rs);
    }
  }
}
