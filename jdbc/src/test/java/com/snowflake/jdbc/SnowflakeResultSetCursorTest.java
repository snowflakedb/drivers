package com.snowflake.jdbc;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.Properties;
import org.junit.jupiter.api.Test;

public class SnowflakeResultSetCursorTest extends SnowflakeIntegrationTestBase {

  @Test
  public void testCursorPosition() throws Exception {
    String tableName = "RS_CURSOR_TEST_" + System.currentTimeMillis();
    try (Connection conn = openConnection();
        Statement stmt = conn.createStatement()) {
      ensureDatabaseAndSchema(conn);
      stmt.execute("create or replace temporary table " + tableName + " (id int)");
      stmt.execute("insert into " + tableName + " values (1), (2), (3)");

      try (ResultSet rs = stmt.executeQuery("select id from " + tableName + " order by id")) {
        assertTrue(rs.isBeforeFirst());
        assertEquals(0, rs.getRow());

        assertTrue(rs.next());
        assertTrue(rs.isFirst());
        assertEquals(1, rs.getRow());

        assertTrue(rs.next());
        assertFalse(rs.isFirst());
        assertFalse(rs.isLast());
        assertEquals(2, rs.getRow());

        assertTrue(rs.next());
        assertTrue(rs.isLast());
        assertEquals(3, rs.getRow());

        assertFalse(rs.next());
        assertTrue(rs.isAfterLast());
        assertEquals(0, rs.getRow());
      }
    }
  }

  @Test
  public void testIsLastSingleRow() throws Exception {
    try (Connection conn = openConnection();
        Statement stmt = conn.createStatement()) {
      ensureDatabaseAndSchema(conn);
      try (ResultSet rs = stmt.executeQuery("select 1")) {
        assertTrue(rs.isBeforeFirst());
        assertFalse(rs.isFirst());

        assertTrue(rs.next());
        assertTrue(rs.isFirst());
        assertTrue(rs.isLast());

        assertFalse(rs.next());
        assertTrue(rs.isAfterLast());
        assertFalse(rs.isLast());
      }
    }
  }

  @Test
  public void testCloseAfterIsLastPrefetch() throws Exception {
    try (Connection conn = openConnection();
        Statement stmt = conn.createStatement()) {
      ensureDatabaseAndSchema(conn);
      try (ResultSet rs =
          stmt.executeQuery("select 1 as id union all select 2 as id order by id")) {
        assertTrue(rs.next());
        assertTrue(rs.next());
        assertTrue(rs.isLast());
        rs.close();
        assertTrue(rs.isClosed());
      }
    }
  }

  @Test
  public void testNextAfterCloseReturnsFalse() throws Exception {
    try (Connection conn = openConnection();
        Statement stmt = conn.createStatement()) {
      ensureDatabaseAndSchema(conn);
      ResultSet rs = stmt.executeQuery("select 1");
      try {
        assertFalse(rs.isClosed());
        rs.close();
        assertTrue(rs.isClosed());
        assertFalse(rs.next());
      } finally {
        rs.close();
      }
    }
  }

  @Test
  public void testGettersAfterCloseThrow() throws Exception {
    try (Connection conn = openConnection();
        Statement stmt = conn.createStatement()) {
      ensureDatabaseAndSchema(conn);
      try (ResultSet rs = stmt.executeQuery("select 1")) {
        assertTrue(rs.next());
        rs.close();
        assertFalse(rs.next());
        assertThrows(Exception.class, rs::getMetaData);
        assertThrows(Exception.class, () -> rs.getString(1));
        assertThrows(Exception.class, () -> rs.getInt(1));
        assertThrows(Exception.class, () -> rs.findColumn("COL1"));
      }
    }
  }

  private void ensureDatabaseAndSchema(Connection conn) throws Exception {
    Properties props = loadConnectionProperties();
    String database = props.getProperty("db");
    String schema = props.getProperty("schema");
    try (Statement stmt = conn.createStatement()) {
      if (database != null && !database.isEmpty()) {
        stmt.execute("use database " + database);
      }
      if (schema != null && !schema.isEmpty()) {
        stmt.execute("use schema " + schema);
      }
    }
  }
}
