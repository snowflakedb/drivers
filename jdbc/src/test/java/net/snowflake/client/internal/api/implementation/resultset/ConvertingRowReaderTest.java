package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.math.BigDecimal;
import java.nio.charset.StandardCharsets;
import java.sql.Date;
import java.sql.Time;
import java.sql.Timestamp;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class ConvertingRowReaderTest {

  private RowReader delegate;
  private String[] columnNames;
  private ConvertingRowReader reader;

  @BeforeEach
  void setUp() {
    delegate = mock(RowReader.class);
    columnNames = new String[] {"ID", "NAME", "ACTIVE"};
  }

  private ConvertingRowReader readerWithPassthrough() {
    return new ConvertingRowReader(
        delegate,
        columnNames,
        row -> new Object[] {row.getInt(1), row.getString(2), row.getBoolean(3)});
  }

  // --- cursor navigation ---

  @Test
  void shouldDelegateNextToUnderlyingReader() {
    reader = readerWithPassthrough();
    when(delegate.next()).thenReturn(true, true, false);
    when(delegate.getInt(1)).thenReturn(1, 2);
    when(delegate.getString(2)).thenReturn("a", "b");
    when(delegate.getBoolean(3)).thenReturn(true, false);

    assertTrue(reader.next());
    assertEquals(0, reader.getCurrentRow());
    assertTrue(reader.next());
    assertEquals(1, reader.getCurrentRow());
    assertFalse(reader.next());
  }

  @Test
  void shouldTransitionCursorState() {
    reader = readerWithPassthrough();
    when(delegate.next()).thenReturn(true, false);
    when(delegate.getInt(1)).thenReturn(1);
    when(delegate.getString(2)).thenReturn("x");
    when(delegate.getBoolean(3)).thenReturn(true);

    assertTrue(reader.isBeforeFirst());
    assertFalse(reader.isAfterLast());
    assertFalse(reader.isFirst());

    assertTrue(reader.next());
    assertFalse(reader.isBeforeFirst());
    assertFalse(reader.isAfterLast());
    assertTrue(reader.isFirst());

    assertFalse(reader.next());
    assertFalse(reader.isBeforeFirst());
    assertTrue(reader.isAfterLast());
    assertFalse(reader.isFirst());
  }

  // --- filtering ---

  @Test
  void shouldSkipRowWhenConverterReturnsNull() {
    int[] callCount = {0};
    reader =
        new ConvertingRowReader(
            delegate,
            columnNames,
            row -> {
              callCount[0]++;
              if (callCount[0] == 1) {
                return null;
              }
              return new Object[] {42, "kept", true};
            });
    when(delegate.next()).thenReturn(true, true, false);

    assertTrue(reader.next());
    assertEquals(0, reader.getCurrentRow());
    assertEquals("kept", reader.getString(2));

    assertFalse(reader.next());
  }

  @Test
  void shouldReturnExhaustedWhenAllRowsFiltered() {
    reader = new ConvertingRowReader(delegate, columnNames, row -> null);
    when(delegate.next()).thenReturn(true, true, false);

    assertFalse(reader.next());
    assertTrue(reader.isAfterLast());
  }

  // --- isLast (one-row look-ahead) ---

  @Test
  void shouldReportIsLastOnlyOnFinalProjectedRow() {
    reader = readerWithPassthrough();
    when(delegate.next()).thenReturn(true, true, false);
    when(delegate.getInt(1)).thenReturn(1, 2);
    when(delegate.getString(2)).thenReturn("a", "b");
    when(delegate.getBoolean(3)).thenReturn(true, false);

    assertTrue(reader.next());
    assertFalse(reader.isLast());

    assertTrue(reader.next());
    assertTrue(reader.isLast());

    assertFalse(reader.next());
    assertFalse(reader.isLast());
  }

  @Test
  void shouldReportIsLastForSingleRowResult() {
    reader = new ConvertingRowReader(delegate, new String[] {"V"}, row -> new Object[] {1});
    when(delegate.next()).thenReturn(true, false);

    assertTrue(reader.next());
    assertTrue(reader.isLast());
  }

  @Test
  void shouldReportIsLastBeforeFirstRowOfEmptyResult() {
    reader = readerWithPassthrough();
    when(delegate.next()).thenReturn(false);

    // Parity with snowflake-jdbc: before-first cursor of an empty projected result is last.
    assertTrue(reader.isLast());
    assertFalse(reader.next());
    assertFalse(reader.isLast());
  }

  @Test
  void shouldNotReportIsLastBeforeFirstRowOfNonEmptyResult() {
    reader =
        new ConvertingRowReader(delegate, new String[] {"V"}, row -> new Object[] {row.getInt(1)});
    when(delegate.next()).thenReturn(true, false);
    when(delegate.getInt(1)).thenReturn(7);

    assertFalse(reader.isLast());

    // The peek did not consume the row: next() still returns it.
    assertTrue(reader.next());
    assertEquals(7, reader.getInt(1));
    assertTrue(reader.isLast());
  }

  @Test
  void shouldAccountForFilteredTrailingRowsInIsLast() {
    int[] callCount = {0};
    reader =
        new ConvertingRowReader(
            delegate,
            columnNames,
            row -> {
              callCount[0]++;
              return callCount[0] == 1 ? new Object[] {42, "kept", true} : null;
            });
    // Delegate still has two rows after the kept one, but the converter drops both.
    when(delegate.next()).thenReturn(true, true, true, false);

    assertTrue(reader.next());
    assertEquals(0, reader.getCurrentRow());
    // The kept row is last even though the delegate is not yet exhausted.
    assertTrue(reader.isLast());
    assertFalse(reader.next());
  }

  @Test
  void shouldNotReportIsLastWhenAKeptRowFollowsFilteredRows() {
    int[] callCount = {0};
    reader =
        new ConvertingRowReader(
            delegate,
            columnNames,
            row -> {
              callCount[0]++;
              if (callCount[0] == 2) {
                return null; // drop the middle delegate row
              }
              return new Object[] {callCount[0], "row", true};
            });
    when(delegate.next()).thenReturn(true, true, true, false);

    assertTrue(reader.next());
    assertEquals(1, reader.getInt(1));
    // A later kept row still follows across the dropped one, so this is not last.
    assertFalse(reader.isLast());

    assertTrue(reader.next());
    assertEquals(3, reader.getInt(1));
    assertTrue(reader.isLast());
  }

  @Test
  void shouldReturnSameIsLastOnRepeatedCallsWithoutAdvancing() {
    reader = readerWithPassthrough();
    when(delegate.next()).thenReturn(true, false);
    when(delegate.getInt(1)).thenReturn(1);
    when(delegate.getString(2)).thenReturn("a");
    when(delegate.getBoolean(3)).thenReturn(true);

    assertTrue(reader.next());
    assertTrue(reader.isLast());
    // A second call reuses the buffered peek rather than advancing the delegate again.
    assertTrue(reader.isLast());
    assertFalse(reader.next());
  }

  // --- column access: typed getters ---

  @Test
  void shouldReturnToStringOfObjectFromGetString() {
    reader = new ConvertingRowReader(delegate, new String[] {"V"}, row -> new Object[] {123});
    when(delegate.next()).thenReturn(true);

    reader.next();
    assertEquals("123", reader.getString(1));
  }

  @Test
  void shouldReturnNullFromGetStringForNullValue() {
    reader = new ConvertingRowReader(delegate, new String[] {"V"}, row -> new Object[] {null});
    when(delegate.next()).thenReturn(true);

    reader.next();
    assertNull(reader.getString(1));
    assertTrue(reader.wasNull());
  }

  @Test
  void shouldReturnBooleanFromBooleanObject() {
    reader = new ConvertingRowReader(delegate, new String[] {"B"}, row -> new Object[] {true});
    when(delegate.next()).thenReturn(true);
    reader.next();
    assertTrue(reader.getBoolean(1));
  }

  @Test
  void shouldReturnBooleanFromNumber() {
    reader = new ConvertingRowReader(delegate, new String[] {"B"}, row -> new Object[] {1});
    when(delegate.next()).thenReturn(true);
    reader.next();
    assertTrue(reader.getBoolean(1));
  }

  @Test
  void shouldReturnBooleanFromStringOne() {
    reader = new ConvertingRowReader(delegate, new String[] {"B"}, row -> new Object[] {"1"});
    when(delegate.next()).thenReturn(true);
    reader.next();
    assertTrue(reader.getBoolean(1));
  }

  @Test
  void shouldReturnFalseFromGetBooleanForNull() {
    reader = new ConvertingRowReader(delegate, new String[] {"B"}, row -> new Object[] {null});
    when(delegate.next()).thenReturn(true);
    reader.next();
    assertFalse(reader.getBoolean(1));
    assertTrue(reader.wasNull());
  }

  @Test
  void shouldReturnCorrectTypesFromNumericGetters() {
    reader = new ConvertingRowReader(delegate, new String[] {"V"}, row -> new Object[] {42});
    when(delegate.next()).thenReturn(true);
    reader.next();

    assertEquals((byte) 42, reader.getByte(1));
    assertEquals((short) 42, reader.getShort(1));
    assertEquals(42, reader.getInt(1));
    assertEquals(42L, reader.getLong(1));
    assertEquals(42.0f, reader.getFloat(1));
    assertEquals(42.0, reader.getDouble(1));
    assertEquals(new BigDecimal("42"), reader.getBigDecimal(1));
  }

  @Test
  void shouldReturnZeroForNullNumericGetters() {
    reader = new ConvertingRowReader(delegate, new String[] {"V"}, row -> new Object[] {null});
    when(delegate.next()).thenReturn(true);
    reader.next();

    assertEquals(0, reader.getByte(1));
    assertTrue(reader.wasNull());
    assertEquals(0, reader.getShort(1));
    assertTrue(reader.wasNull());
    assertEquals(0, reader.getInt(1));
    assertTrue(reader.wasNull());
    assertEquals(0L, reader.getLong(1));
    assertTrue(reader.wasNull());
    assertEquals(0.0f, reader.getFloat(1));
    assertTrue(reader.wasNull());
    assertEquals(0.0, reader.getDouble(1));
    assertTrue(reader.wasNull());
    assertNull(reader.getBigDecimal(1));
    assertTrue(reader.wasNull());
  }

  @Test
  void shouldParseNumericGettersFromString() {
    reader = new ConvertingRowReader(delegate, new String[] {"V"}, row -> new Object[] {"7"});
    when(delegate.next()).thenReturn(true);
    reader.next();

    assertEquals((byte) 7, reader.getByte(1));
    assertEquals((short) 7, reader.getShort(1));
    assertEquals(7, reader.getInt(1));
    assertEquals(7L, reader.getLong(1));
    assertEquals(7.0f, reader.getFloat(1));
    assertEquals(7.0, reader.getDouble(1));
  }

  @Test
  void shouldReturnUtf8BytesFromGetBytes() {
    reader = new ConvertingRowReader(delegate, new String[] {"V"}, row -> new Object[] {"hello"});
    when(delegate.next()).thenReturn(true);
    reader.next();

    assertArrayEquals("hello".getBytes(StandardCharsets.UTF_8), reader.getBytes(1));
  }

  @Test
  void shouldReturnNullFromGetBytesForNull() {
    reader = new ConvertingRowReader(delegate, new String[] {"V"}, row -> new Object[] {null});
    when(delegate.next()).thenReturn(true);
    reader.next();

    assertNull(reader.getBytes(1));
    assertTrue(reader.wasNull());
  }

  @Test
  void shouldPassThroughTypedObjectsFromDateTimestampTimeGetters() {
    Date date = Date.valueOf("2025-01-15");
    Time time = Time.valueOf("13:45:30");
    Timestamp ts = Timestamp.valueOf("2025-01-15 13:45:30");

    reader =
        new ConvertingRowReader(
            delegate, new String[] {"D", "T", "TS"}, row -> new Object[] {date, time, ts});
    when(delegate.next()).thenReturn(true);
    reader.next();

    assertEquals(date, reader.getDate(1));
    assertEquals(time, reader.getTime(2));
    assertEquals(ts, reader.getTimestamp(3));
  }

  @Test
  void shouldReturnNullFromDateTimestampTimeGettersForNull() {
    reader =
        new ConvertingRowReader(
            delegate, new String[] {"D", "T", "TS"}, row -> new Object[] {null, null, null});
    when(delegate.next()).thenReturn(true);
    reader.next();

    assertNull(reader.getDate(1));
    assertTrue(reader.wasNull());
    assertNull(reader.getTime(2));
    assertTrue(reader.wasNull());
    assertNull(reader.getTimestamp(3));
    assertTrue(reader.wasNull());
  }

  @Test
  void shouldReturnRawValueFromGetObject() {
    Object obj = new Object();
    reader = new ConvertingRowReader(delegate, new String[] {"V"}, row -> new Object[] {obj});
    when(delegate.next()).thenReturn(true);
    reader.next();

    assertEquals(obj, reader.getObject(1));
  }

  // --- column metadata ---

  @Test
  void shouldReflectProjectedNamesInGetColumnCount() {
    reader = readerWithPassthrough();
    assertEquals(3, reader.getColumnCount());
  }

  @Test
  void shouldReturnOneBasedNameFromGetColumnName() {
    reader = readerWithPassthrough();
    assertEquals("ID", reader.getColumnName(1));
    assertEquals("NAME", reader.getColumnName(2));
    assertEquals("ACTIVE", reader.getColumnName(3));
  }

  @Test
  void shouldThrowOnGetColumnNameOutOfRange() {
    reader = readerWithPassthrough();
    assertThrows(IllegalArgumentException.class, () -> reader.getColumnName(0));
    assertThrows(IllegalArgumentException.class, () -> reader.getColumnName(4));
  }

  // --- error conditions ---

  @Test
  void shouldThrowOnGetObjectWhenNoCurrentRow() {
    reader = readerWithPassthrough();
    assertThrows(IllegalStateException.class, () -> reader.getObject(1));
  }

  @Test
  void shouldThrowOnGetObjectForInvalidColumnIndex() {
    reader = new ConvertingRowReader(delegate, new String[] {"V"}, row -> new Object[] {"x"});
    when(delegate.next()).thenReturn(true);
    reader.next();

    assertThrows(IllegalArgumentException.class, () -> reader.getObject(0));
    assertThrows(IllegalArgumentException.class, () -> reader.getObject(2));
  }

  // --- close ---

  @Test
  void shouldDelegateCloseToUnderlyingReader() {
    reader = readerWithPassthrough();
    assertFalse(reader.isClosed());

    reader.close();

    assertTrue(reader.isClosed());
    verify(delegate).close();
  }
}
