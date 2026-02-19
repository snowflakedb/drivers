package net.snowflake.client.api.statement;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.sql.Types;
import java.util.UUID;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class SnowflakePreparedStatementBindingTest extends SnowflakeIntegrationTestBase {

  @Test
  public void testSetNull() throws Exception {
    String tableName = buildTempTableName();
    Connection conn = getDefaultConnection();
    createTempTable(conn, tableName, "(id INTEGER)");

    try (PreparedStatement insert =
        conn.prepareStatement("INSERT INTO " + tableName + " (id) VALUES (?)")) {
      insert.setNull(1, Types.INTEGER);
      insert.execute();
    }

    try (PreparedStatement verify =
            conn.prepareStatement("SELECT COUNT(*), COUNT(id) FROM " + tableName);
        ResultSet rs = verify.executeQuery()) {
      assertTrue(rs.next(), "Verification query should return one row");
      assertEquals(1, rs.getInt(1), "Exactly one row should be inserted");
      assertEquals(0, rs.getInt(2), "Null column should not contribute to COUNT(column)");
    }
  }

  @Test
  public void testSetString() throws Exception {
    String tableName = buildTempTableName();
    Connection conn = getDefaultConnection();
    createTempTable(conn, tableName, "(txt STRING)");

    try (PreparedStatement insert =
        conn.prepareStatement("INSERT INTO " + tableName + " (txt) VALUES (?)")) {
      insert.setString(1, "hello");
      insert.execute();
    }

    try (PreparedStatement verify =
            conn.prepareStatement("SELECT COUNT(*), MIN(txt) FROM " + tableName);
        ResultSet rs = verify.executeQuery()) {
      assertTrue(rs.next(), "Verification query should return one row");
      assertEquals(1, rs.getInt(1), "Exactly one row should be inserted");
      assertEquals("hello", rs.getString(2), "Inserted string value should match");
    }
  }

  @Test
  public void testSetBoolean() throws Exception {
    String tableName = buildTempTableName();
    Connection conn = getDefaultConnection();
    createTempTable(conn, tableName, "(flag BOOLEAN)");

    try (PreparedStatement insert =
        conn.prepareStatement("INSERT INTO " + tableName + " (flag) VALUES (?)")) {
      insert.setBoolean(1, true);
      insert.execute();
    }

    try (PreparedStatement verify =
            conn.prepareStatement("SELECT COUNT(*), MIN(flag) FROM " + tableName);
        ResultSet rs = verify.executeQuery()) {
      assertTrue(rs.next(), "Verification query should return one row");
      assertEquals(1, rs.getInt(1), "Exactly one row should be inserted");
      assertTrue(rs.getBoolean(2), "Inserted boolean value should match");
    }
  }

  @Test
  public void testSetByte() throws Exception {
    String tableName = buildTempTableName();
    Connection conn = getDefaultConnection();
    createTempTable(conn, tableName, "(v INTEGER)");

    try (PreparedStatement insert =
        conn.prepareStatement("INSERT INTO " + tableName + " (v) VALUES (?)")) {
      insert.setByte(1, (byte) 7);
      insert.execute();
    }

    try (PreparedStatement verify =
            conn.prepareStatement("SELECT COUNT(*), MIN(v) FROM " + tableName);
        ResultSet rs = verify.executeQuery()) {
      assertTrue(rs.next(), "Verification query should return one row");
      assertEquals(1, rs.getInt(1), "Exactly one row should be inserted");
      assertEquals((byte) 7, rs.getByte(2), "Inserted byte value should match");
    }
  }

  @Test
  public void testSetShort() throws Exception {
    String tableName = buildTempTableName();
    Connection conn = getDefaultConnection();
    createTempTable(conn, tableName, "(v INTEGER)");

    try (PreparedStatement insert =
        conn.prepareStatement("INSERT INTO " + tableName + " (v) VALUES (?)")) {
      insert.setShort(1, (short) 11);
      insert.execute();
    }

    try (PreparedStatement verify =
            conn.prepareStatement("SELECT COUNT(*), MIN(v) FROM " + tableName);
        ResultSet rs = verify.executeQuery()) {
      assertTrue(rs.next(), "Verification query should return one row");
      assertEquals(1, rs.getInt(1), "Exactly one row should be inserted");
      assertEquals((short) 11, rs.getShort(2), "Inserted short value should match");
    }
  }

  @Test
  public void testSetInt() throws Exception {
    String tableName = buildTempTableName();
    Connection conn = getDefaultConnection();
    createTempTable(conn, tableName, "(v INTEGER)");

    try (PreparedStatement insert =
        conn.prepareStatement("INSERT INTO " + tableName + " (v) VALUES (?)")) {
      insert.setInt(1, 42);
      insert.execute();
    }

    try (PreparedStatement verify =
            conn.prepareStatement("SELECT COUNT(*), MIN(v) FROM " + tableName);
        ResultSet rs = verify.executeQuery()) {
      assertTrue(rs.next(), "Verification query should return one row");
      assertEquals(1, rs.getInt(1), "Exactly one row should be inserted");
      assertEquals(42, rs.getInt(2), "Inserted int value should match");
    }
  }

  @Test
  public void testSetLong() throws Exception {
    String tableName = buildTempTableName();
    Connection conn = getDefaultConnection();
    createTempTable(conn, tableName, "(v INTEGER)");

    try (PreparedStatement insert =
        conn.prepareStatement("INSERT INTO " + tableName + " (v) VALUES (?)")) {
      insert.setLong(1, 123456789L);
      insert.execute();
    }

    try (PreparedStatement verify =
            conn.prepareStatement("SELECT COUNT(*), MIN(v) FROM " + tableName);
        ResultSet rs = verify.executeQuery()) {
      assertTrue(rs.next(), "Verification query should return one row");
      assertEquals(1, rs.getInt(1), "Exactly one row should be inserted");
      assertEquals(123456789L, rs.getLong(2), "Inserted long value should match");
    }
  }

  @Test
  public void testSetFloat() throws Exception {
    String tableName = buildTempTableName();
    Connection conn = getDefaultConnection();
    createTempTable(conn, tableName, "(v FLOAT)");

    try (PreparedStatement insert =
        conn.prepareStatement("INSERT INTO " + tableName + " (v) VALUES (?)")) {
      insert.setFloat(1, 1.25f);
      insert.execute();
    }

    try (PreparedStatement verify =
            conn.prepareStatement("SELECT COUNT(*), MIN(v) FROM " + tableName);
        ResultSet rs = verify.executeQuery()) {
      assertTrue(rs.next(), "Verification query should return one row");
      assertEquals(1, rs.getInt(1), "Exactly one row should be inserted");
      assertEquals(1.25f, rs.getFloat(2), 0.0001f, "Inserted float value should match");
    }
  }

  @Test
  public void testSetDouble() throws Exception {
    String tableName = buildTempTableName();
    Connection conn = getDefaultConnection();
    createTempTable(conn, tableName, "(v FLOAT)");

    try (PreparedStatement insert =
        conn.prepareStatement("INSERT INTO " + tableName + " (v) VALUES (?)")) {
      insert.setDouble(1, 2.5d);
      insert.execute();
    }

    try (PreparedStatement verify =
            conn.prepareStatement("SELECT COUNT(*), MIN(v) FROM " + tableName);
        ResultSet rs = verify.executeQuery()) {
      assertTrue(rs.next(), "Verification query should return one row");
      assertEquals(1, rs.getInt(1), "Exactly one row should be inserted");
      assertEquals(2.5d, rs.getDouble(2), 0.0001d, "Inserted double value should match");
    }
  }

  @Test
  public void testSetBigDecimal() throws Exception {
    String tableName = buildTempTableName();
    Connection conn = getDefaultConnection();
    createTempTable(conn, tableName, "(v NUMBER(18,2))");

    try (PreparedStatement insert =
        conn.prepareStatement("INSERT INTO " + tableName + " (v) VALUES (?)")) {
      insert.setBigDecimal(1, new BigDecimal("12345.67"));
      insert.execute();
    }

    try (PreparedStatement verify =
            conn.prepareStatement("SELECT COUNT(*), MIN(v) FROM " + tableName);
        ResultSet rs = verify.executeQuery()) {
      assertTrue(rs.next(), "Verification query should return one row");
      assertEquals(1, rs.getInt(1), "Exactly one row should be inserted");
      assertEquals(
          0,
          new BigDecimal("12345.67").compareTo(rs.getBigDecimal(2)),
          "Inserted BigDecimal value should match");
    }
  }

  @Test
  public void testSetBytes() throws Exception {
    String tableName = buildTempTableName();
    byte[] value = new byte[] {0x01, 0x02, 0x03};
    Connection conn = getDefaultConnection();
    createTempTable(conn, tableName, "(v BINARY)");

    try (PreparedStatement insert =
        conn.prepareStatement("INSERT INTO " + tableName + " (v) VALUES (?)")) {
      insert.setBytes(1, value);
      insert.execute();
    }

    try (PreparedStatement verify =
            conn.prepareStatement("SELECT COUNT(*), MIN(v) FROM " + tableName);
        ResultSet rs = verify.executeQuery()) {
      assertTrue(rs.next(), "Verification query should return one row");
      assertEquals(1, rs.getInt(1), "Exactly one row should be inserted");
      assertArrayEquals(value, rs.getBytes(2), "Inserted byte[] value should match");
    }
  }

  @Test
  public void testAllSupportedSetters() throws Exception {
    String tableName = buildTempTableName();
    byte[] bytesValue = new byte[] {0x0A, 0x0B, 0x0C};
    Connection conn = getDefaultConnection();
    createTempTable(
        conn,
        tableName,
        "(n INTEGER, s STRING, b BOOLEAN, byte_col INTEGER, short_col INTEGER, i INTEGER, l INTEGER,"
            + " f FLOAT, d FLOAT, bd NUMBER(18,2), bin BINARY)");

    try (PreparedStatement insert =
        conn.prepareStatement(
            "INSERT INTO "
                + tableName
                + " (n, s, b, byte_col, short_col, i, l, f, d, bd, bin) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")) {
      insert.setNull(1, Types.INTEGER);
      insert.setString(2, "all");
      insert.setBoolean(3, true);
      insert.setByte(4, (byte) 12);
      insert.setShort(5, (short) 320);
      insert.setInt(6, 1234);
      insert.setLong(7, 987654321L);
      insert.setFloat(8, 3.25f);
      insert.setDouble(9, 6.5d);
      insert.setBigDecimal(10, new BigDecimal("77.88"));
      insert.setBytes(11, bytesValue);
      insert.execute();
    }

    try (PreparedStatement verify =
            conn.prepareStatement(
                "SELECT COUNT(*), COUNT(n), MIN(s), MIN(b), MIN(byte_col), MIN(short_col),"
                    + " MIN(i), MIN(l), MIN(f), MIN(d), MIN(bd), MIN(bin) FROM "
                    + tableName);
        ResultSet rs = verify.executeQuery()) {
      assertTrue(rs.next(), "Verification query should return one row");
      assertEquals(1, rs.getInt(1), "Exactly one row should be inserted");
      assertEquals(0, rs.getInt(2), "setNull column should remain NULL");
      assertEquals("all", rs.getString(3), "setString value should match");
      assertTrue(rs.getBoolean(4), "setBoolean value should match");
      assertEquals((byte) 12, rs.getByte(5), "setByte value should match");
      assertEquals((short) 320, rs.getShort(6), "setShort value should match");
      assertEquals(1234, rs.getInt(7), "setInt value should match");
      assertEquals(987654321L, rs.getLong(8), "setLong value should match");
      assertEquals(3.25f, rs.getFloat(9), 0.0001f, "setFloat value should match");
      assertEquals(6.5d, rs.getDouble(10), 0.0001d, "setDouble value should match");
      assertEquals(
          0,
          new BigDecimal("77.88").compareTo(rs.getBigDecimal(11)),
          "setBigDecimal value should match");
      assertArrayEquals(bytesValue, rs.getBytes(12), "setBytes value should match");
    }
  }

  @Test
  public void testSetObject() throws Exception {
    String tableName = buildTempTableName();
    byte[] bytesValue = new byte[] {0x01, 0x02, 0x03};
    Connection conn = getDefaultConnection();
    createTempTable(
        conn,
        tableName,
        "(n INTEGER, s STRING, b BOOLEAN, i INTEGER, l INTEGER, f FLOAT, d FLOAT,"
            + " bd NUMBER(18,2), bin BINARY)");

    try (PreparedStatement insert =
        conn.prepareStatement(
            "INSERT INTO "
                + tableName
                + " (n, s, b, i, l, f, d, bd, bin) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")) {
      insert.setObject(1, null);
      insert.setObject(2, "obj");
      insert.setObject(3, true);
      insert.setObject(4, 123);
      insert.setObject(5, 987654321L);
      insert.setObject(6, 1.25f);
      insert.setObject(7, 2.5d);
      insert.setObject(8, new BigDecimal("77.88"));
      insert.setObject(9, bytesValue);
      insert.execute();
    }

    try (PreparedStatement verify =
            conn.prepareStatement(
                "SELECT COUNT(*), COUNT(n), MIN(s), MIN(b), MIN(i), MIN(l), MIN(f), MIN(d),"
                    + " MIN(bd), MIN(bin) FROM "
                    + tableName);
        ResultSet rs = verify.executeQuery()) {
      assertTrue(rs.next(), "Verification query should return one row");
      assertEquals(1, rs.getInt(1), "Exactly one row should be inserted");
      assertEquals(0, rs.getInt(2), "setObject(null) should keep INTEGER column NULL");
      assertEquals("obj", rs.getString(3), "setObject(String) value should match");
      assertTrue(rs.getBoolean(4), "setObject(Boolean) value should match");
      assertEquals(123, rs.getInt(5), "setObject(Integer) value should match");
      assertEquals(987654321L, rs.getLong(6), "setObject(Long) value should match");
      assertEquals(1.25f, rs.getFloat(7), 0.0001f, "setObject(Float) value should match");
      assertEquals(2.5d, rs.getDouble(8), 0.0001d, "setObject(Double) value should match");
      assertEquals(
          0,
          new BigDecimal("77.88").compareTo(rs.getBigDecimal(9)),
          "setObject(BigDecimal) value should match");
      assertArrayEquals(bytesValue, rs.getBytes(10), "setObject(byte[]) value should match");
    }
  }

  @Test
  public void testSetObjectWithTargetSqlTypeAndNull() throws Exception {
    String tableName = buildTempTableName();
    Connection conn = getDefaultConnection();
    createTempTable(conn, tableName, "(id INTEGER, txt STRING)");

    try (PreparedStatement insert =
        conn.prepareStatement("INSERT INTO " + tableName + " (id, txt) VALUES (?, ?)")) {
      insert.setObject(1, null, Types.INTEGER);
      insert.setObject(2, "typed", Types.VARCHAR);
      insert.execute();
    }

    try (PreparedStatement verify =
            conn.prepareStatement("SELECT COUNT(*), COUNT(id), MIN(txt) FROM " + tableName);
        ResultSet rs = verify.executeQuery()) {
      assertTrue(rs.next(), "Verification query should return one row");
      assertEquals(1, rs.getInt(1), "Exactly one row should be inserted");
      assertEquals(0, rs.getInt(2), "setObject(null, Types.INTEGER) should remain NULL");
      assertEquals("typed", rs.getString(3), "setObject with sqlType should delegate correctly");
    }
  }

  @Test
  public void testSetObjectWithTargetSqlTypeUsesTargetBindingType() throws Exception {
    Connection conn = getDefaultConnection();
    try (PreparedStatement stmt =
            conn.prepareStatement("SELECT SYSTEM$TYPEOF(?), SYSTEM$TYPEOF(?)");
        ResultSet rs = executeWithTargetTypes(stmt)) {
      assertTrue(rs.next(), "TYPEOF query should return one row");
      assertTrue(
          rs.getString(1).toUpperCase().contains("VARCHAR"),
          "setObject(..., Types.VARCHAR) should bind as text type");
      assertTrue(
          rs.getString(2).toUpperCase().contains("NUMBER"),
          "setObject(..., Types.INTEGER) should bind as numeric type");
    }
  }

  private ResultSet executeWithTargetTypes(PreparedStatement stmt) throws SQLException {
    stmt.setObject(1, 123, Types.VARCHAR);
    stmt.setObject(2, "456", Types.INTEGER);
    return stmt.executeQuery();
  }

  @Test
  public void testSetObjectUnsupportedTypeThrowsSQLException() throws Exception {
    String tableName = buildTempTableName();
    Connection conn = getDefaultConnection();
    createTempTable(conn, tableName, "(v STRING)");

    try (PreparedStatement insert =
        conn.prepareStatement("INSERT INTO " + tableName + " (v) VALUES (?)")) {
      assertThrows(SQLException.class, () -> insert.setObject(1, new Object()));
    }
  }

  @Test
  public void testClearParametersAndPartialRebindFailsDeterministically() throws Exception {
    String tableName = buildTempTableName();
    Connection conn = getDefaultConnection();
    createTempTable(conn, tableName, "(id INTEGER, txt STRING)");

    try (PreparedStatement insert =
        conn.prepareStatement("INSERT INTO " + tableName + " (id, txt) VALUES (?, ?)")) {
      insert.setInt(1, 1);
      insert.setString(2, "before-clear");
      insert.clearParameters();
      insert.setInt(1, 2);

      assertThrows(SQLException.class, insert::execute);
    }
  }

  @Test
  public void testSetNullWithRepresentativeSqlTypes() throws Exception {
    String tableName = buildTempTableName();
    Connection conn = getDefaultConnection();
    createTempTable(conn, tableName, "(b BOOLEAN, i INTEGER, bin BINARY)");

    try (PreparedStatement insert =
        conn.prepareStatement("INSERT INTO " + tableName + " (b, i, bin) VALUES (?, ?, ?)")) {
      insert.setNull(1, Types.BOOLEAN);
      insert.setNull(2, Types.INTEGER);
      insert.setNull(3, Types.BINARY);
      insert.execute();
    }

    try (PreparedStatement verify =
            conn.prepareStatement(
                "SELECT COUNT(*), COUNT(b), COUNT(i), COUNT(bin) FROM " + tableName);
        ResultSet rs = verify.executeQuery()) {
      assertTrue(rs.next(), "Verification query should return one row");
      assertEquals(1, rs.getInt(1), "Exactly one row should be inserted");
      assertEquals(0, rs.getInt(2), "BOOLEAN NULL should not contribute to COUNT");
      assertEquals(0, rs.getInt(3), "INTEGER NULL should not contribute to COUNT");
      assertEquals(0, rs.getInt(4), "BINARY NULL should not contribute to COUNT");
    }
  }

  private String buildTempTableName() {
    return "JDBC_PS_BINDING_" + UUID.randomUUID().toString().replace("-", "");
  }

  private void createTempTable(Connection conn, String tableName, String schema) throws Exception {
    try (Statement stmt = conn.createStatement()) {
      stmt.execute("ALTER SESSION SET JDBC_QUERY_RESULT_FORMAT='JSON'");
      stmt.execute("CREATE OR REPLACE TEMPORARY TABLE " + tableName + " " + schema);
    }
  }
}
