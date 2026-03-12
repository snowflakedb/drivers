package net.snowflake.client.api.resultset;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class SnowflakeResultSetGettersTest extends SnowflakeIntegrationTestBase {

  @Test
  public void testGetInt() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement();
        ResultSet rs = stmt.executeQuery("SELECT 42")) {
      assertTrue(rs.next());
      assertEquals(42, rs.getInt(1));
    }
  }

  @Test
  public void testGetFloat() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement();
        ResultSet rs = stmt.executeQuery("SELECT 12.5::FLOAT")) {
      assertTrue(rs.next());
      assertEquals(12.5f, rs.getFloat(1), 0.0001f);
    }
  }

  @Test
  public void testGetDouble() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement();
        ResultSet rs = stmt.executeQuery("SELECT 12345.6789::DOUBLE")) {
      assertTrue(rs.next());
      assertEquals(12345.6789, rs.getDouble(1), 0.0000001);
    }
  }

  @Test
  public void testGetString() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement();
        ResultSet rs = stmt.executeQuery("SELECT 'hello'")) {
      assertTrue(rs.next());
      assertEquals("hello", rs.getString(1));
    }
  }

  @Test
  public void testGetObject() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement();
        ResultSet rs = stmt.executeQuery("SELECT 7::NUMBER(10,0)")) {
      assertTrue(rs.next());
      Object value = rs.getObject(1);
      assertNotNull(value);
      assertInstanceOf(Long.class, value);
      assertEquals(7L, value);
    }
  }

  @Test
  public void testGetBytes() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement();
        ResultSet rs = stmt.executeQuery("SELECT TO_BINARY('0102', 'HEX')")) {
      assertTrue(rs.next());
      assertArrayEquals(new byte[] {0x01, 0x02}, rs.getBytes(1));
    }
  }

  @Test
  public void testGetBigDecimal() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement();
        ResultSet rs = stmt.executeQuery("SELECT 123.45::NUMBER(10,2)")) {
      assertTrue(rs.next());
      BigDecimal value = rs.getBigDecimal(1);
      assertNotNull(value);
      assertEquals(0, value.compareTo(new BigDecimal("123.45")));
    }
  }

  @Test
  public void testFloatSpecialValues() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement();
        ResultSet rs = stmt.executeQuery("SELECT 'inf'::FLOAT, '-inf'::FLOAT, 'nan'::FLOAT")) {
      assertTrue(rs.next());
      float posInf = rs.getFloat(1);
      float negInf = rs.getFloat(2);
      float nanVal = rs.getFloat(3);
      assertTrue(Float.isInfinite(posInf) && posInf > 0);
      assertTrue(Float.isInfinite(negInf) && negInf < 0);
      assertTrue(Float.isNaN(nanVal));
    }
  }

  @Test
  public void testDecfloatIntegerGetterOverflowBehavior() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement();
        ResultSet rs =
            stmt.executeQuery(
                "SELECT 123::DECFLOAT, 2147483648::DECFLOAT, 9223372036854775808::DECFLOAT")) {
      assertTrue(rs.next());

      assertEquals(123, rs.getInt(1));
      assertEquals(123L, rs.getLong(1));
      assertEquals((short) 123, rs.getShort(1));

      assertThrows(SQLException.class, () -> rs.getInt(2));
      assertThrows(SQLException.class, () -> rs.getShort(2));
      assertEquals(2147483648L, rs.getLong(2));

      assertThrows(SQLException.class, () -> rs.getLong(3));
      assertThrows(SQLException.class, () -> rs.getInt(3));
      assertThrows(SQLException.class, () -> rs.getShort(3));
    }
  }

  @Test
  public void testDecfloatWasNullAcrossMultipleGetters() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement();
        ResultSet rs = stmt.executeQuery("SELECT NULL::DECFLOAT")) {
      assertTrue(rs.next());

      assertNull(rs.getBigDecimal(1));
      assertTrue(rs.wasNull());

      assertNull(rs.getObject(1));
      assertTrue(rs.wasNull());

      assertEquals(0.0d, rs.getDouble(1), 0.0d);
      assertTrue(rs.wasNull());

      assertNull(rs.getString(1));
      assertTrue(rs.wasNull());

      assertFalse(rs.next());
    }
  }
}
