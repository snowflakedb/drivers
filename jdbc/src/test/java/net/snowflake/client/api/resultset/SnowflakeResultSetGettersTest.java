package net.snowflake.client.api.resultset;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.BufferedReader;
import java.math.BigDecimal;
import java.nio.ByteBuffer;
import java.sql.Clob;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.time.LocalTime;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
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

  // TIME getBytes/getBoolean parity with snowflake-jdbc. Running in this package (vs. e2e/types)
  // exercises the same assertions against both universal-driver and the legacy driver under the
  // referenceTest task, without the Gherkin/feature-file alignment the e2e packages require.

  @Test
  public void shouldReturnRawBigEndianBytesForIntBackedTime() throws Exception {
    // TIME(0) is SB4-encoded (IntVector), so getBytes() exposes the raw 4-byte big-endian image of
    // the seconds-since-midnight value, matching snowflake-jdbc's IntToTimeConverter.toBytes.
    int secondsSinceMidnight = (int) (LocalTime.of(12, 34, 56).toNanoOfDay() / 1_000_000_000L);
    byte[] expected = ByteBuffer.allocate(Integer.BYTES).putInt(secondsSinceMidnight).array();
    try (Statement stmt = getDefaultConnection().createStatement();
        ResultSet rs = stmt.executeQuery("SELECT '12:34:56'::TIME(0)")) {
      assertTrue(rs.next());
      assertArrayEquals(expected, rs.getBytes(1), "getBytes mismatch for INT-backed TIME");
      assertFalse(rs.wasNull());
    }
  }

  @Test
  public void shouldThrowWhenGettingBytesFromBigIntBackedTime() throws Exception {
    // TIME(9) is SB8-encoded (BigIntVector); snowflake-jdbc's BigIntToTimeConverter does not
    // implement toBytes, so getBytes falls through to the unsupported-conversion error.
    try (Statement stmt = getDefaultConnection().createStatement();
        ResultSet rs = stmt.executeQuery("SELECT '12:34:56.123456789'::TIME(9)")) {
      assertTrue(rs.next());
      assertThrows(
          SQLException.class, () -> rs.getBytes(1), "getBytes should fail for BIGINT-backed TIME");
    }
  }

  @Test
  public void shouldReturnFalseForGetBooleanOnNullTimeAndThrowOtherwise() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement();
        ResultSet rs = stmt.executeQuery("SELECT NULL::TIME, '10:30:00'::TIME")) {
      assertTrue(rs.next());

      // getBoolean on a SQL NULL TIME returns false (matching the legacy converter).
      assertFalse(rs.getBoolean(1), "getBoolean of NULL TIME should be false");
      assertTrue(rs.wasNull());

      // getBoolean on a non-null TIME is an unsupported conversion on both drivers.
      assertThrows(
          SQLException.class, () -> rs.getBoolean(2), "getBoolean of a non-null TIME should fail");
    }
  }

  @Test
  public void shouldReadClobByIndexAndLabel() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement();
        ResultSet rs = stmt.executeQuery("SELECT 'hello' AS C")) {
      assertTrue(rs.next());

      Clob byIndex = rs.getClob(1);
      assertNotNull(byIndex);
      assertEquals(5L, byIndex.length());
      assertEquals("hello", readClob(byIndex));

      Clob byLabel = rs.getClob("C");
      assertNotNull(byLabel);
      assertEquals("hello", readClob(byLabel));
    }
  }

  private static String readClob(Clob clob) throws Exception {
    StringBuilder sb = new StringBuilder();
    try (BufferedReader reader = new BufferedReader(clob.getCharacterStream())) {
      int c;
      while ((c = reader.read()) != -1) {
        sb.append((char) c);
      }
    }
    return sb.toString();
  }

  @Test
  public void shouldReturnNullClobForSqlNull() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement();
        ResultSet rs = stmt.executeQuery("SELECT NULL::VARCHAR")) {
      assertTrue(rs.next());
      assertNull(rs.getClob(1));
      assertTrue(rs.wasNull());
    }
  }
}
