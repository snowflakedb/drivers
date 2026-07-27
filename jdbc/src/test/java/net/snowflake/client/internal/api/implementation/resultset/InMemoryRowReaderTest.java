package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.nio.charset.StandardCharsets;
import java.sql.Date;
import java.sql.SQLException;
import java.sql.Time;
import java.sql.Timestamp;
import org.junit.jupiter.api.Test;

class InMemoryRowReaderTest {

  // --- Cursor navigation ---

  @Test
  void shouldStartBeforeFirst() {
    InMemoryRowReader reader = twoRowReader();
    assertTrue(reader.isBeforeFirst());
    assertFalse(reader.isAfterLast());
    assertFalse(reader.isFirst());
  }

  @Test
  void shouldAdvanceThroughAllRows() throws SQLException {
    InMemoryRowReader reader = twoRowReader();

    assertTrue(reader.next());
    assertTrue(reader.isFirst());

    assertTrue(reader.next());
    assertFalse(reader.isFirst());

    assertFalse(reader.next());
    assertTrue(reader.isAfterLast());
  }

  @Test
  void shouldReportIsLastOnFinalRow() throws SQLException {
    InMemoryRowReader reader = twoRowReader();
    assertFalse(reader.isLast());

    assertTrue(reader.next());
    assertFalse(reader.isLast());

    assertTrue(reader.next());
    assertTrue(reader.isLast());

    assertFalse(reader.next());
    assertFalse(reader.isLast());
  }

  @Test
  void shouldReportIsLastBeforeFirstRowOfEmptyResult() {
    InMemoryRowReader empty = new InMemoryRowReader(new String[] {"C"}, new Object[0][]);
    // Parity with snowflake-jdbc: before-first cursor of an empty result is reported as last.
    assertTrue(empty.isLast());
    assertFalse(empty.next());
    assertFalse(empty.isLast());
  }

  @Test
  void shouldReturnZeroRowsWhenBackedByEmptyArray() {
    InMemoryRowReader empty = new InMemoryRowReader(new String[] {"C"}, new Object[0][]);
    assertTrue(empty.isBeforeFirst());
    assertFalse(empty.next());
    assertTrue(empty.isAfterLast());
  }

  @Test
  void shouldMarkClosedAfterClose() {
    InMemoryRowReader reader = twoRowReader();
    reader.close();
    assertTrue(reader.isClosed());
  }

  // --- Column metadata ---

  @Test
  void shouldExposeColumnMetadata() throws SQLException {
    InMemoryRowReader reader =
        new InMemoryRowReader(new String[] {"A", "B"}, new Object[][] {{"x", "y"}});
    assertEquals(2, reader.getColumnCount());
    assertEquals("A", reader.getColumnName(1));
    assertEquals("B", reader.getColumnName(2));
  }

  @Test
  void shouldThrowOnOutOfBoundsColumnIndex() throws SQLException {
    InMemoryRowReader reader = new InMemoryRowReader(new String[] {"C"}, new Object[][] {{"v"}});
    reader.next();
    assertThrows(SQLException.class, () -> reader.getString(0));
    assertThrows(SQLException.class, () -> reader.getString(2));
  }

  // --- Access guards ---

  @Test
  void shouldThrowOnColumnAccessBeforeNext() {
    InMemoryRowReader reader = twoRowReader();
    assertThrows(SQLException.class, () -> reader.getString(1));
  }

  @Test
  void shouldThrowOnColumnAccessAfterLast() throws SQLException {
    InMemoryRowReader reader = twoRowReader();
    while (reader.next()) {}
    assertThrows(SQLException.class, () -> reader.getString(1));
  }

  // --- Typed accessors: happy path ---

  @Test
  void shouldReturnNullForNullCell() throws SQLException {
    InMemoryRowReader reader = new InMemoryRowReader(new String[] {"C"}, new Object[][] {{null}});
    reader.next();
    assertNull(reader.getString(1));
    assertNull(reader.getObject(1));
  }

  @Test
  void shouldReturnString() throws SQLException {
    InMemoryRowReader reader = readerWith("hello");
    assertEquals("hello", reader.getString(1));
    assertEquals("hello", reader.getObject(1));
  }

  @Test
  void shouldReturnBoolean() throws SQLException {
    InMemoryRowReader reader = readerWith(true);
    assertTrue(reader.getBoolean(1));
  }

  @Test
  void shouldReturnByte() throws SQLException {
    InMemoryRowReader reader = readerWith((byte) 42);
    assertEquals((byte) 42, reader.getByte(1));
  }

  @Test
  void shouldReturnShort() throws SQLException {
    InMemoryRowReader reader = readerWith((short) 1000);
    assertEquals((short) 1000, reader.getShort(1));
  }

  @Test
  void shouldReturnInt() throws SQLException {
    InMemoryRowReader reader = readerWith(99);
    assertEquals(99, reader.getInt(1));
  }

  @Test
  void shouldReturnLong() throws SQLException {
    InMemoryRowReader reader = readerWith(123456789L);
    assertEquals(123456789L, reader.getLong(1));
  }

  @Test
  void shouldReturnFloat() throws SQLException {
    InMemoryRowReader reader = readerWith(1.5f);
    assertEquals(1.5f, reader.getFloat(1));
  }

  @Test
  void shouldReturnDouble() throws SQLException {
    InMemoryRowReader reader = readerWith(3.14);
    assertEquals(3.14, reader.getDouble(1));
  }

  @Test
  void shouldReturnBigDecimal() throws SQLException {
    BigDecimal val = new BigDecimal("12345.678");
    InMemoryRowReader reader = readerWith(val);
    assertEquals(val, reader.getBigDecimal(1));
  }

  @Test
  void shouldReturnBytes() throws SQLException {
    byte[] val = {1, 2, 3};
    InMemoryRowReader reader = readerWith(val);
    assertArrayEquals(val, reader.getBytes(1));
  }

  @Test
  void shouldReturnDate() throws SQLException {
    Date val = Date.valueOf("2024-01-15");
    InMemoryRowReader reader = readerWith(val);
    assertEquals(val, reader.getDate(1));
    assertEquals(val, reader.getDate(1, null));
  }

  @Test
  void shouldReturnTime() throws SQLException {
    Time val = Time.valueOf("10:30:00");
    InMemoryRowReader reader = readerWith(val);
    assertEquals(val, reader.getTime(1));
  }

  @Test
  void shouldReturnTimestamp() throws SQLException {
    Timestamp val = Timestamp.valueOf("2024-01-15 10:30:00");
    InMemoryRowReader reader = readerWith(val);
    assertEquals(val, reader.getTimestamp(1));
  }

  // --- Null handling: numeric types return 0, boolean returns false ---

  @Test
  void shouldReturnZeroForNullNumericCells() throws SQLException {
    InMemoryRowReader reader = readerWith(null);
    assertEquals(0, reader.getByte(1));
    reader = readerWith(null);
    assertEquals(0, reader.getShort(1));
    reader = readerWith(null);
    assertEquals(0, reader.getInt(1));
    reader = readerWith(null);
    assertEquals(0L, reader.getLong(1));
    reader = readerWith(null);
    assertEquals(0f, reader.getFloat(1));
    reader = readerWith(null);
    assertEquals(0d, reader.getDouble(1));
  }

  @Test
  void shouldReturnFalseForNullBoolean() throws SQLException {
    InMemoryRowReader reader = readerWith(null);
    assertFalse(reader.getBoolean(1));
  }

  @Test
  void shouldReturnNullForNullBigDecimal() throws SQLException {
    InMemoryRowReader reader = readerWith(null);
    assertNull(reader.getBigDecimal(1));
  }

  @Test
  void shouldThrowForNullBytes() throws SQLException {
    InMemoryRowReader reader = readerWith(null);
    assertThrows(SQLException.class, () -> reader.getBytes(1));
  }

  // --- wasNull tracking ---

  @Test
  void shouldReportWasNullAfterNullCell() throws SQLException {
    InMemoryRowReader reader = readerWith(null);
    reader.getString(1);
    assertTrue(reader.wasNull());
  }

  @Test
  void shouldReportWasNotNullAfterNonNullCell() throws SQLException {
    InMemoryRowReader reader = readerWith("hello");
    reader.getString(1);
    assertFalse(reader.wasNull());
  }

  // --- Numeric cross-type conversions ---

  @Test
  void shouldReadIntegerAsShort() throws SQLException {
    InMemoryRowReader reader = readerWith(1);
    assertEquals((short) 1, reader.getShort(1));
  }

  @Test
  void shouldReadShortAsInt() throws SQLException {
    InMemoryRowReader reader = readerWith((short) 42);
    assertEquals(42, reader.getInt(1));
  }

  @Test
  void shouldReadIntegerAsLong() throws SQLException {
    InMemoryRowReader reader = readerWith(100);
    assertEquals(100L, reader.getLong(1));
  }

  @Test
  void shouldReadLongAsDouble() throws SQLException {
    InMemoryRowReader reader = readerWith(7L);
    assertEquals(7.0, reader.getDouble(1));
  }

  @Test
  void shouldReadIntegerAsByte() throws SQLException {
    InMemoryRowReader reader = readerWith(127);
    assertEquals((byte) 127, reader.getByte(1));
  }

  @Test
  void shouldReadDoubleAsFloat() throws SQLException {
    InMemoryRowReader reader = readerWith(1.5);
    assertEquals(1.5f, reader.getFloat(1), 0.001f);
  }

  // --- String-to-number parsing ---

  @Test
  void shouldParseIntFromString() throws SQLException {
    InMemoryRowReader reader = readerWith("42");
    assertEquals(42, reader.getInt(1));
  }

  @Test
  void shouldParseShortFromString() throws SQLException {
    InMemoryRowReader reader = readerWith("1000");
    assertEquals((short) 1000, reader.getShort(1));
  }

  @Test
  void shouldParseLongFromString() throws SQLException {
    InMemoryRowReader reader = readerWith("9876543210");
    assertEquals(9876543210L, reader.getLong(1));
  }

  @Test
  void shouldParseFloatFromString() throws SQLException {
    InMemoryRowReader reader = readerWith("1.5");
    assertEquals(1.5f, reader.getFloat(1), 0.001f);
  }

  @Test
  void shouldParseDoubleFromString() throws SQLException {
    InMemoryRowReader reader = readerWith("3.14");
    assertEquals(3.14, reader.getDouble(1), 0.0001);
  }

  @Test
  void shouldParseByteFromString() throws SQLException {
    InMemoryRowReader reader = readerWith("127");
    assertEquals((byte) 127, reader.getByte(1));
  }

  @Test
  void shouldThrowOnUnparsableLongString() throws SQLException {
    InMemoryRowReader reader = readerWith("not-a-long");
    assertThrows(SQLException.class, () -> reader.getLong(1));
  }

  // --- Boolean conversions ---

  @Test
  void shouldReturnTrueForString1() throws SQLException {
    InMemoryRowReader reader = readerWith("1");
    assertTrue(reader.getBoolean(1));
  }

  @Test
  void shouldReturnFalseForNonOneString() throws SQLException {
    InMemoryRowReader reader = readerWith("true");
    assertFalse(reader.getBoolean(1));
    reader = readerWith("0");
    assertFalse(reader.getBoolean(1));
  }

  @Test
  void shouldReturnTrueForPositiveInteger() throws SQLException {
    InMemoryRowReader reader = readerWith(1);
    assertTrue(reader.getBoolean(1));
    reader = readerWith(42);
    assertTrue(reader.getBoolean(1));
  }

  @Test
  void shouldReturnFalseForZeroOrNegativeInteger() throws SQLException {
    InMemoryRowReader reader = readerWith(0);
    assertFalse(reader.getBoolean(1));
    reader = readerWith(-1);
    assertFalse(reader.getBoolean(1));
  }

  // --- BigDecimal from string and other numeric types ---

  @Test
  void shouldParseBigDecimalFromString() throws SQLException {
    InMemoryRowReader reader = readerWith("12345.678");
    assertEquals(new BigDecimal("12345.678"), reader.getBigDecimal(1));
  }

  @Test
  void shouldParseBigDecimalFromInteger() throws SQLException {
    InMemoryRowReader reader = readerWith(42);
    assertEquals(new BigDecimal("42"), reader.getBigDecimal(1));
  }

  // --- getBytes from String (UTF-8 encoding) ---

  @Test
  void shouldReturnBytesFromString() throws SQLException {
    InMemoryRowReader reader = readerWith("hello");
    assertArrayEquals("hello".getBytes(StandardCharsets.UTF_8), reader.getBytes(1));
  }

  // --- Typed accessors: type mismatch ---

  @Test
  void shouldThrowOnBooleanTypeMismatch() throws SQLException {
    InMemoryRowReader reader = readerWith(Date.valueOf("2024-01-01"));
    assertThrows(SQLException.class, () -> reader.getBoolean(1));
  }

  @Test
  void shouldThrowOnIntTypeMismatch() throws SQLException {
    InMemoryRowReader reader = readerWith("not-an-int");
    assertThrows(SQLException.class, () -> reader.getInt(1));
  }

  @Test
  void shouldThrowOnLongTypeMismatch() throws SQLException {
    InMemoryRowReader reader = readerWith("not-a-long");
    assertThrows(SQLException.class, () -> reader.getLong(1));
  }

  @Test
  void shouldThrowOnDateTypeMismatch() throws SQLException {
    InMemoryRowReader reader = readerWith("not-a-date");
    assertThrows(SQLException.class, () -> reader.getDate(1));
  }

  @Test
  void shouldThrowOnTimestampTypeMismatch() throws SQLException {
    InMemoryRowReader reader = readerWith("not-a-timestamp");
    assertThrows(SQLException.class, () -> reader.getTimestamp(1));
  }

  // --- Helpers ---

  private static InMemoryRowReader twoRowReader() {
    return new InMemoryRowReader(new String[] {"C"}, new Object[][] {{"first"}, {"second"}});
  }

  private static InMemoryRowReader readerWith(Object value) {
    InMemoryRowReader reader = new InMemoryRowReader(new String[] {"C"}, new Object[][] {{value}});
    reader.next();
    return reader;
  }
}
