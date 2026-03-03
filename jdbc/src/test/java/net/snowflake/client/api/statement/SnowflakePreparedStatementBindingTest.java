package net.snowflake.client.api.statement;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Types;
import java.util.stream.Stream;
import lombok.ToString;
import lombok.Value;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.MethodSource;

public class SnowflakePreparedStatementBindingTest extends SnowflakeIntegrationTestBase {

  @ParameterizedTest(name = "{index} => {0}")
  @MethodSource("singleSetterCases")
  public void testSingleSetterBindings(SingleSetterCase testCase) throws Exception {
    Connection conn = getDefaultConnection();
    String tableName = createTempTable(conn, "ud_bindings_", testCase.getSchema());

    executeInsert(conn, "INSERT INTO " + tableName + " (v) VALUES (?)", testCase.getBinder());

    verifySingleValue(conn, "SELECT v FROM " + tableName, testCase.getVerifier());
  }

  @Test
  public void testAllSupportedSetters() throws Exception {
    Connection conn = getDefaultConnection();
    String tableName =
        createTempTable(
            conn,
            "ud_bindings_",
            "n INTEGER, s STRING, b BOOLEAN, byte_col INTEGER, short_col INTEGER, i INTEGER, l INTEGER, f FLOAT, d FLOAT, bd NUMBER(18,2), bin BINARY");
    byte[] bytesValue = new byte[] {0x0A, 0x0B, 0x0C};
    BigDecimal expectedDecimal = new BigDecimal("77.88");

    executeInsert(
        conn,
        "INSERT INTO "
            + tableName
            + " (n, s, b, byte_col, short_col, i, l, f, d, bd, bin) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        insert -> {
          insert.setNull(1, Types.INTEGER);
          insert.setString(2, "all");
          insert.setBoolean(3, true);
          insert.setByte(4, (byte) 12);
          insert.setShort(5, (short) 320);
          insert.setInt(6, 1234);
          insert.setLong(7, 987654321L);
          insert.setFloat(8, 3.25f);
          insert.setDouble(9, 6.5d);
          insert.setBigDecimal(10, expectedDecimal);
          insert.setBytes(11, bytesValue);
        });

    verifySingleRow(
        conn,
        "SELECT COUNT(*), COUNT(n), MIN(s), MIN(b), MIN(byte_col), MIN(short_col),"
            + " MIN(i), MIN(l), MIN(f), MIN(d), MIN(bd), MIN(bin) FROM "
            + tableName,
        rs -> {
          assertEquals(0, rs.getInt(2));
          assertEquals("all", rs.getString(3));
          assertTrue(rs.getBoolean(4));
          assertEquals((byte) 12, rs.getByte(5));
          assertEquals((short) 320, rs.getShort(6));
          assertEquals(1234, rs.getInt(7));
          assertEquals(987654321L, rs.getLong(8));
          assertEquals(3.25f, rs.getFloat(9), 0.0001f);
          assertEquals(6.5d, rs.getDouble(10), 0.0001d);
          assertEquals(0, expectedDecimal.compareTo(rs.getBigDecimal(11)));
          assertArrayEquals(bytesValue, rs.getBytes(12));
        });
  }

  @Test
  public void testSetObject() throws Exception {
    Connection conn = getDefaultConnection();
    String tableName =
        createTempTable(
            conn,
            "ud_bindings_",
            "n INTEGER, s STRING, b BOOLEAN, i INTEGER, l INTEGER, f FLOAT, d FLOAT, bd NUMBER(18,2), bin BINARY");
    byte[] bytesValue = new byte[] {0x01, 0x02, 0x03};
    BigDecimal expectedDecimal = new BigDecimal("77.88");

    executeInsert(
        conn,
        "INSERT INTO "
            + tableName
            + " (n, s, b, i, l, f, d, bd, bin) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        insert -> {
          insert.setObject(1, null);
          insert.setObject(2, "obj");
          insert.setObject(3, true);
          insert.setObject(4, 123);
          insert.setObject(5, 987654321L);
          insert.setObject(6, 1.25f);
          insert.setObject(7, 2.5d);
          insert.setObject(8, expectedDecimal);
          insert.setObject(9, bytesValue);
        });

    verifySingleRow(
        conn,
        "SELECT COUNT(*), COUNT(n), MIN(s), MIN(b), MIN(i), MIN(l), MIN(f), MIN(d),"
            + " MIN(bd), MIN(bin) FROM "
            + tableName,
        rs -> {
          assertEquals(0, rs.getInt(2));
          assertEquals("obj", rs.getString(3));
          assertTrue(rs.getBoolean(4));
          assertEquals(123, rs.getInt(5));
          assertEquals(987654321L, rs.getLong(6));
          assertEquals(1.25f, rs.getFloat(7), 0.0001f);
          assertEquals(2.5d, rs.getDouble(8), 0.0001d);
          assertEquals(0, expectedDecimal.compareTo(rs.getBigDecimal(9)));
          assertArrayEquals(bytesValue, rs.getBytes(10));
        });
  }

  @Test
  public void testSetObjectWithTargetSqlTypeAndNull() throws Exception {
    Connection conn = getDefaultConnection();
    String tableName = createTempTable(conn, "ud_bindings_", "id INTEGER, txt STRING");

    executeInsert(
        conn,
        "INSERT INTO " + tableName + " (id, txt) VALUES (?, ?)",
        insert -> {
          insert.setObject(1, null, Types.INTEGER);
          insert.setObject(2, "typed", Types.VARCHAR);
        });

    verifySingleRow(
        conn,
        "SELECT COUNT(*), COUNT(id), MIN(txt) FROM " + tableName,
        rs -> {
          assertEquals(0, rs.getInt(2));
          assertEquals("typed", rs.getString(3));
        });
  }

  @Test
  public void testSetObjectWithTargetSqlTypeUsesTargetBindingType() throws Exception {
    Connection conn = getDefaultConnection();
    try (PreparedStatement stmt =
            conn.prepareStatement("SELECT SYSTEM$TYPEOF(?), SYSTEM$TYPEOF(?)");
        ResultSet rs = executeWithTargetTypes(stmt)) {
      assertTrue(rs.next());
      assertTrue(
          rs.getString(1).toUpperCase().contains("VARCHAR"),
          "Expected VARCHAR binding for setObject(..., Types.VARCHAR)");
      assertTrue(
          rs.getString(2).toUpperCase().contains("NUMBER"),
          "Expected NUMBER binding for setObject(..., Types.INTEGER)");
    }
  }

  private ResultSet executeWithTargetTypes(PreparedStatement stmt) throws SQLException {
    stmt.setObject(1, 123, Types.VARCHAR);
    stmt.setObject(2, "456", Types.INTEGER);
    return stmt.executeQuery();
  }

  @Test
  public void testSetObjectUnsupportedTypeThrowsSQLException() throws Exception {
    Connection conn = getDefaultConnection();
    String tableName = createTempTable(conn, "ud_bindings_", "v STRING");

    try (PreparedStatement insert =
        conn.prepareStatement("INSERT INTO " + tableName + " (v) VALUES (?)")) {
      assertThrows(SQLException.class, () -> insert.setObject(1, new Object()));
    }
  }

  @Test
  public void testClearParametersAndPartialRebindFailsDeterministically() throws Exception {
    Connection conn = getDefaultConnection();
    String tableName = createTempTable(conn, "ud_bindings_", "id INTEGER, txt STRING");

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
    Connection conn = getDefaultConnection();
    String tableName = createTempTable(conn, "ud_bindings_", "b BOOLEAN, i INTEGER, bin BINARY");

    executeInsert(
        conn,
        "INSERT INTO " + tableName + " (b, i, bin) VALUES (?, ?, ?)",
        insert -> {
          insert.setNull(1, Types.BOOLEAN);
          insert.setNull(2, Types.INTEGER);
          insert.setNull(3, Types.BINARY);
        });

    verifySingleRow(
        conn,
        "SELECT COUNT(*), COUNT(b), COUNT(i), COUNT(bin) FROM " + tableName,
        rs -> {
          assertEquals(0, rs.getInt(2));
          assertEquals(0, rs.getInt(3));
          assertEquals(0, rs.getInt(4));
        });
  }

  private static Stream<SingleSetterCase> singleSetterCases() {
    byte[] bytesValue = new byte[] {0x01, 0x02, 0x03};
    BigDecimal bigDecimalValue = new BigDecimal("12345.67");
    return Stream.of(
        new SingleSetterCase(
            "setNull",
            "v INTEGER",
            insert -> insert.setNull(1, Types.INTEGER),
            rs -> assertNull(rs.getObject(1))),
        new SingleSetterCase(
            "setString",
            "v STRING",
            insert -> insert.setString(1, "hello"),
            rs -> assertEquals("hello", rs.getString(1))),
        new SingleSetterCase(
            "setBoolean",
            "v BOOLEAN",
            insert -> insert.setBoolean(1, true),
            rs -> assertTrue(rs.getBoolean(1))),
        new SingleSetterCase(
            "setByte",
            "v INTEGER",
            insert -> insert.setByte(1, (byte) 7),
            rs -> assertEquals((byte) 7, rs.getByte(1))),
        new SingleSetterCase(
            "setShort",
            "v INTEGER",
            insert -> insert.setShort(1, (short) 11),
            rs -> assertEquals((short) 11, rs.getShort(1))),
        new SingleSetterCase(
            "setInt",
            "v INTEGER",
            insert -> insert.setInt(1, 42),
            rs -> assertEquals(42, rs.getInt(1))),
        new SingleSetterCase(
            "setLong",
            "v INTEGER",
            insert -> insert.setLong(1, 123456789L),
            rs -> assertEquals(123456789L, rs.getLong(1))),
        new SingleSetterCase(
            "setFloat",
            "v FLOAT",
            insert -> insert.setFloat(1, 1.25f),
            rs -> assertEquals(1.25f, rs.getFloat(1), 0.0001f)),
        new SingleSetterCase(
            "setDouble",
            "v FLOAT",
            insert -> insert.setDouble(1, 2.5d),
            rs -> assertEquals(2.5d, rs.getDouble(1), 0.0001d)),
        new SingleSetterCase(
            "setBigDecimal",
            "v NUMBER(18,2)",
            insert -> insert.setBigDecimal(1, bigDecimalValue),
            rs -> assertEquals(0, bigDecimalValue.compareTo(rs.getBigDecimal(1)))),
        new SingleSetterCase(
            "setBytes",
            "v BINARY",
            insert -> insert.setBytes(1, bytesValue),
            rs -> assertArrayEquals(bytesValue, rs.getBytes(1))));
  }

  private void executeInsert(Connection conn, String sql, SqlInsertBinder binder)
      throws SQLException {
    try (PreparedStatement insert = conn.prepareStatement(sql)) {
      binder.bind(insert);
      insert.execute();
    }
  }

  private void verifySingleRow(Connection conn, String sql, ResultSetVerifier verifier)
      throws SQLException {
    try (PreparedStatement verify = conn.prepareStatement(sql);
        ResultSet rs = verify.executeQuery()) {
      assertTrue(rs.next());
      assertEquals(1, rs.getInt(1));
      verifier.verify(rs);
    }
  }

  private void verifySingleValue(Connection conn, String sql, ResultSetVerifier verifier)
      throws SQLException {
    try (PreparedStatement verify = conn.prepareStatement(sql);
        ResultSet rs = verify.executeQuery()) {
      assertTrue(rs.next());
      verifier.verify(rs);
      assertFalse(rs.next());
    }
  }

  @FunctionalInterface
  private interface SqlInsertBinder {
    void bind(PreparedStatement insert) throws SQLException;
  }

  @FunctionalInterface
  private interface ResultSetVerifier {
    void verify(ResultSet rs) throws SQLException;
  }

  @Value
  @ToString(of = "name")
  private static class SingleSetterCase {
    String name;
    String schema;
    SqlInsertBinder binder;
    ResultSetVerifier verifier;
  }
}
