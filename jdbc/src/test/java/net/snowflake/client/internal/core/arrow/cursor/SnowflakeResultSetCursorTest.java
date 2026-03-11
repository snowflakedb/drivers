package net.snowflake.client.internal.core.arrow.cursor;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.Statement;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class SnowflakeResultSetCursorTest extends SnowflakeIntegrationTestBase {

  @Test
  public void testCursorPosition() throws Exception {
    Connection conn = getDefaultConnection();
    try (Statement stmt = conn.createStatement()) {
      try (ResultSet rs = stmt.executeQuery("select * from values (1), (2), (3);")) {
        assertTrue(rs.isBeforeFirst());
        assertEquals(0, rs.getRow());

        assertTrue(rs.next());
        assertTrue(rs.isFirst());
        assertEquals(1, rs.getRow());

        assertTrue(rs.next());
        assertFalse(rs.isFirst());
        assertEquals(2, rs.getRow());

        assertTrue(rs.next());
        assertEquals(3, rs.getRow());

        assertFalse(rs.next());
        assertTrue(rs.isAfterLast());
        assertEquals(0, rs.getRow());
      }
    }
  }

  @Test
  public void testNextAfterCloseReturnsFalse() throws Exception {
    Connection conn = getDefaultConnection();
    try (Statement stmt = conn.createStatement()) {
      try (ResultSet rs = stmt.executeQuery("select 1")) {
        assertFalse(rs.isClosed());
        rs.close();
        assertTrue(rs.isClosed());
        assertFalse(rs.next());
      }
    }
  }

  @Test
  public void testGettersAfterCloseThrow() throws Exception {
    Connection conn = getDefaultConnection();
    try (Statement stmt = conn.createStatement()) {
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

  @Test
  public void testWasNullUpdatesAfterReads() throws Exception {
    Connection conn = getDefaultConnection();
    try (Statement stmt = conn.createStatement()) {
      try (ResultSet rs = stmt.executeQuery("select null as n, 1 as v")) {
        assertTrue(rs.next());
        assertNull(rs.getString(1));
        assertTrue(rs.wasNull());
        assertEquals(1, rs.getInt(2));
        assertFalse(rs.wasNull());
      }
    }
  }

  @Test
  public void testGettersBeforeFirstAndAfterLastThrow() throws Exception {
    Connection conn = getDefaultConnection();
    try (Statement stmt = conn.createStatement()) {
      try (ResultSet rs = stmt.executeQuery("select 1")) {
        assertThrows(SQLException.class, () -> rs.getInt(1));
        assertTrue(rs.next());
        assertEquals(1, rs.getInt(1));
        assertFalse(rs.next());
        assertThrows(SQLException.class, () -> rs.getInt(1));
      }
    }
  }

  @Test
  public void testUnsupportedGettersThrow() throws Exception {
    Connection conn = getDefaultConnection();
    try (Statement stmt = conn.createStatement()) {
      try (ResultSet rs = stmt.executeQuery("select 1")) {
        assertTrue(rs.next());
        assertThrows(SQLFeatureNotSupportedException.class, () -> rs.getDate(1));
        assertThrows(SQLFeatureNotSupportedException.class, () -> rs.getTime(1));
        assertThrows(SQLFeatureNotSupportedException.class, () -> rs.getTimestamp(1));
      }
    }
  }

  @Test
  public void testFindColumnCaseInsensitive() throws Exception {
    Connection conn = getDefaultConnection();
    try (Statement stmt = conn.createStatement()) {
      try (ResultSet rs = stmt.executeQuery("select 1 as FooBar")) {
        assertTrue(rs.next());
        assertEquals(1, rs.findColumn("foobar"));
        assertEquals(1, rs.findColumn("FOOBAR"));
      }
    }
  }
}
