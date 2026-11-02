package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.nio.charset.StandardCharsets;
import java.sql.Date;
import java.sql.SQLException;
import java.sql.Time;
import java.sql.Timestamp;
import java.time.Duration;
import java.time.Period;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.TimeZone;
import java.util.concurrent.TimeUnit;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.core.arrow.converters.DataConversionContext;
import net.snowflake.client.internal.core.arrow.cursor.ArrowResources;
import net.snowflake.client.internal.core.arrow.cursor.ColumnBatchListReader;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBatch;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

class ArrowRowReaderTest {

  private static final long UNKNOWN_ROW_COUNT = -1;

  private static ArrowResources buildResources(int[][] ids, String[][] names) {
    Map<String, String> intMeta = new HashMap<>();
    intMeta.put("logicalType", "FIXED");
    intMeta.put("precision", "10");
    intMeta.put("scale", "0");
    Map<String, String> textMeta = new HashMap<>();
    textMeta.put("logicalType", "TEXT");

    List<ColumnBatch> roots = new ArrayList<>();
    for (int b = 0; b < ids.length; b++) {
      int rows = ids[b].length;
      Column idVec = ColumnBuilder.of("id", Column.PhysicalKind.INT32, intMeta);
      Column nameVec = ColumnBuilder.of("name", Column.PhysicalKind.UTF8, textMeta);
      for (int r = 0; r < rows; r++) {
        ColumnBuilder.set(idVec, r, ids[b][r]);
        ColumnBuilder.set(nameVec, r, names[b][r].getBytes(StandardCharsets.UTF_8));
      }
      ColumnBuilder.setValueCount(idVec, rows);
      ColumnBuilder.setValueCount(nameVec, rows);
      ColumnBatch root = ColumnBatch.of(idVec, nameVec);
      root.setRowCount(rows);
      roots.add(root);
    }
    return new ArrowResources(new ColumnBatchListReader(roots));
  }

  // --- cursor navigation ---

  @Test
  void shouldIterateSingleBatch() throws Exception {
    ArrowResources res = buildResources(new int[][] {{1, 2, 3}}, new String[][] {{"a", "b", "c"}});
    try (ArrowRowReader reader =
        new ArrowRowReader(res, new DataContextStub(), UNKNOWN_ROW_COUNT)) {
      assertTrue(reader.isBeforeFirst());
      assertFalse(reader.isAfterLast());

      assertTrue(reader.next());
      assertTrue(reader.isFirst());
      assertEquals(0, reader.getCurrentRow());
      assertEquals("a", reader.getString(2));
      assertEquals(1, reader.getInt(1));

      assertTrue(reader.next());
      assertFalse(reader.isFirst());
      assertEquals(1, reader.getCurrentRow());
      assertEquals("b", reader.getString(2));

      assertTrue(reader.next());
      assertEquals(2, reader.getCurrentRow());
      assertEquals("c", reader.getString(2));
      assertEquals(3, reader.getInt(1));

      assertFalse(reader.next());
      assertTrue(reader.isAfterLast());
    }
  }

  @Test
  void shouldCloseArrowResourcesWhenUnclosedReaderIsGarbageCollected() throws Exception {
    Map<String, String> intMeta = new HashMap<>();
    intMeta.put("logicalType", "FIXED");
    intMeta.put("precision", "10");
    intMeta.put("scale", "0");
    Column id = ColumnBuilder.of("id", Column.PhysicalKind.INT32, intMeta);
    ColumnBuilder.set(id, 0, 1);
    ColumnBuilder.setValueCount(id, 1);
    ColumnBatch root = ColumnBatch.of(id);
    root.setRowCount(1);
    List<ColumnBatch> batches = new ArrayList<>();
    batches.add(root);
    new ArrowRowReader(
        new ArrowResources(new ColumnBatchListReader(batches)),
        new DataContextStub(),
        UNKNOWN_ROW_COUNT);

    long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(10);
    boolean closed = false;
    while (!closed && System.nanoTime() < deadline) {
      System.gc();
      try {
        id.isNull(0);
      } catch (IllegalStateException closedColumn) {
        closed = true;
      }
      if (!closed) {
        Thread.sleep(100);
      }
    }
    assertTrue(closed);
  }

  @Test
  void shouldIterateMultipleBatches() throws Exception {
    ArrowResources res =
        buildResources(new int[][] {{10}, {20, 30}}, new String[][] {{"x"}, {"y", "z"}});
    try (ArrowRowReader reader =
        new ArrowRowReader(res, new DataContextStub(), UNKNOWN_ROW_COUNT)) {
      assertTrue(reader.next());
      assertEquals(10, reader.getInt(1));

      assertTrue(reader.next());
      assertEquals(20, reader.getInt(1));

      assertTrue(reader.next());
      assertEquals(30, reader.getInt(1));

      assertFalse(reader.next());
    }
  }

  // --- isLast ---

  @Test
  void shouldReportIsLastOnLastRowOfSingleBatch() throws Exception {
    ArrowResources res = buildResources(new int[][] {{1, 2, 3}}, new String[][] {{"a", "b", "c"}});
    try (ArrowRowReader reader = new ArrowRowReader(res, new DataContextStub(), 3)) {
      assertFalse(reader.isLast());

      assertTrue(reader.next());
      assertFalse(reader.isLast());

      assertTrue(reader.next());
      assertFalse(reader.isLast());

      assertTrue(reader.next());
      assertTrue(reader.isLast());

      assertFalse(reader.next());
      assertFalse(reader.isLast());
    }
  }

  @Test
  void shouldReportIsLastOnLastRowAcrossBatches() throws Exception {
    ArrowResources res =
        buildResources(new int[][] {{10}, {20, 30}}, new String[][] {{"x"}, {"y", "z"}});
    try (ArrowRowReader reader = new ArrowRowReader(res, new DataContextStub(), 3)) {
      assertTrue(reader.next());
      assertFalse(reader.isLast());

      assertTrue(reader.next());
      assertFalse(reader.isLast());

      assertTrue(reader.next());
      assertTrue(reader.isLast());
    }
  }

  @Test
  void shouldReportIsLastBeforeFirstRowOfEmptyResult() throws Exception {
    ArrowResources res = buildResources(new int[][] {{}}, new String[][] {{}});
    try (ArrowRowReader reader = new ArrowRowReader(res, new DataContextStub(), 0)) {
      // Parity with snowflake-jdbc: before-first cursor of an empty result is reported as last.
      assertTrue(reader.isLast());
      assertFalse(reader.next());
      assertFalse(reader.isLast());
    }
  }

  @Test
  void shouldNotReportIsLastWhenRowCountUnknown() throws Exception {
    ArrowResources res = buildResources(new int[][] {{1}}, new String[][] {{"a"}});
    try (ArrowRowReader reader =
        new ArrowRowReader(res, new DataContextStub(), UNKNOWN_ROW_COUNT)) {
      reader.next();
      assertFalse(reader.isLast());
    }
  }

  // --- typed getters ---

  @Test
  void shouldReturnIntAsString() throws Exception {
    ArrowResources res = buildResources(new int[][] {{42}}, new String[][] {{"hi"}});
    try (ArrowRowReader reader =
        new ArrowRowReader(res, new DataContextStub(), UNKNOWN_ROW_COUNT)) {
      reader.next();
      assertEquals("42", reader.getString(1));
      assertEquals("hi", reader.getString(2));
    }
  }

  @Test
  void shouldReturnNumericValuesFromIntColumn() throws Exception {
    ArrowResources res = buildResources(new int[][] {{7}}, new String[][] {{"ignored"}});
    try (ArrowRowReader reader =
        new ArrowRowReader(res, new DataContextStub(), UNKNOWN_ROW_COUNT)) {
      reader.next();
      assertEquals((byte) 7, reader.getByte(1));
      assertEquals((short) 7, reader.getShort(1));
      assertEquals(7, reader.getInt(1));
      assertEquals(7L, reader.getLong(1));
      assertEquals(7.0f, reader.getFloat(1));
      assertEquals(7.0, reader.getDouble(1));
    }
  }

  @Test
  void shouldReturnBooleanFromIntColumn() throws Exception {
    ArrowResources res = buildResources(new int[][] {{0, 1}}, new String[][] {{"a", "b"}});
    try (ArrowRowReader reader =
        new ArrowRowReader(res, new DataContextStub(), UNKNOWN_ROW_COUNT)) {
      reader.next();
      assertFalse(reader.getBoolean(1));

      reader.next();
      assertTrue(reader.getBoolean(1));
    }
  }

  // --- column metadata ---

  @Test
  void shouldReturnColumnCountAndNames() throws Exception {
    ArrowResources res = buildResources(new int[][] {{1}}, new String[][] {{"a"}});
    try (ArrowRowReader reader =
        new ArrowRowReader(res, new DataContextStub(), UNKNOWN_ROW_COUNT)) {
      assertEquals(2, reader.getColumnCount());
      assertEquals("id", reader.getColumnName(1));
      assertEquals("name", reader.getColumnName(2));
    }
  }

  @Test
  void shouldThrowOnColumnIndexOutOfRange() throws Exception {
    ArrowResources res = buildResources(new int[][] {{1}}, new String[][] {{"a"}});
    try (ArrowRowReader reader =
        new ArrowRowReader(res, new DataContextStub(), UNKNOWN_ROW_COUNT)) {
      assertThrows(SFSQLException.class, () -> reader.getColumnName(0));
      assertThrows(SFSQLException.class, () -> reader.getColumnName(3));
    }
  }

  // --- state checks ---

  @Test
  void shouldThrowOnGetterBeforeNext() throws Exception {
    ArrowResources res = buildResources(new int[][] {{1}}, new String[][] {{"a"}});
    try (ArrowRowReader reader =
        new ArrowRowReader(res, new DataContextStub(), UNKNOWN_ROW_COUNT)) {
      assertThrows(IllegalStateException.class, () -> reader.getInt(1));
    }
  }

  @Test
  void shouldThrowOnGetterAfterExhausted() throws Exception {
    ArrowResources res = buildResources(new int[][] {{1}}, new String[][] {{"a"}});
    try (ArrowRowReader reader =
        new ArrowRowReader(res, new DataContextStub(), UNKNOWN_ROW_COUNT)) {
      reader.next();
      reader.next();
      assertTrue(reader.isAfterLast());
      assertThrows(IllegalStateException.class, () -> reader.getInt(1));
    }
  }

  @Test
  void shouldThrowResultSetClosedWhenReadingAfterClose() throws Exception {
    ArrowResources res = buildResources(new int[][] {{1}}, new String[][] {{"a"}});
    try (ArrowRowReader reader =
        new ArrowRowReader(res, new DataContextStub(), UNKNOWN_ROW_COUNT)) {
      assertTrue(reader.next());
      reader.close();
      SFSQLException failure = assertThrows(SFSQLException.class, () -> reader.getInt(1));
      SQLException sql = failure.toSQLException();
      assertEquals(ErrorCode.RESULTSET_ALREADY_CLOSED.getMessageCode(), sql.getErrorCode());
      assertEquals(ErrorCode.RESULTSET_ALREADY_CLOSED.getSqlState(), sql.getSQLState());
      assertEquals("ResultSet is closed", failure.getMessage());
    }
  }

  @Test
  void shouldReturnFalseOnNextAfterClose() throws Exception {
    ArrowResources res = buildResources(new int[][] {{1}}, new String[][] {{"a"}});
    try (ArrowRowReader reader =
        new ArrowRowReader(res, new DataContextStub(), UNKNOWN_ROW_COUNT)) {
      reader.close();
      assertTrue(reader.isClosed());
      assertFalse(reader.next());
    }
  }

  @Test
  void shouldCloseIdempotently() throws Exception {
    ArrowResources res = buildResources(new int[][] {{1}}, new String[][] {{"a"}});
    try (ArrowRowReader reader =
        new ArrowRowReader(res, new DataContextStub(), UNKNOWN_ROW_COUNT)) {
      reader.close();
      reader.close();
      assertTrue(reader.isClosed());
    }
  }

  // --- wasNull ---

  @Test
  void shouldReportNonNullAfterNonNullGet() throws Exception {
    ArrowResources res = buildResources(new int[][] {{5}}, new String[][] {{"val"}});
    try (ArrowRowReader reader =
        new ArrowRowReader(res, new DataContextStub(), UNKNOWN_ROW_COUNT)) {
      reader.next();
      reader.getInt(1);
      assertFalse(reader.wasNull());

      reader.getString(2);
      assertFalse(reader.wasNull());
    }
  }

  @Test
  void shouldDelegateRemainingTypedGettersAndTrackWasNull() throws Exception {
    Map<String, String> fixedMeta = new HashMap<>();
    fixedMeta.put("logicalType", "FIXED");
    fixedMeta.put("precision", "10");
    fixedMeta.put("scale", "0");

    Map<String, String> dateMeta = Collections.singletonMap("logicalType", "DATE");
    Map<String, String> timeMeta = new HashMap<>();
    timeMeta.put("logicalType", "TIME");
    timeMeta.put("scale", "0");
    Map<String, String> tsMeta = new HashMap<>();
    tsMeta.put("logicalType", "TIMESTAMP_NTZ");
    tsMeta.put("scale", "0");
    Map<String, String> ymMeta = Collections.singletonMap("logicalType", "INTERVAL_YEAR_MONTH");
    Map<String, String> dtMeta = Collections.singletonMap("logicalType", "INTERVAL_DAY_TIME");

    Column intCol = ColumnBuilder.of("num", Column.PhysicalKind.INT32, fixedMeta);
    Column dateCol = ColumnBuilder.of("d", Column.PhysicalKind.INT32, dateMeta);
    Column timeCol = ColumnBuilder.of("t", Column.PhysicalKind.INT64, timeMeta);
    Column tsCol = ColumnBuilder.of("ts", Column.PhysicalKind.INT64, tsMeta);
    Column ymCol = ColumnBuilder.of("ym", Column.PhysicalKind.INT32, ymMeta);
    Column dtCol = ColumnBuilder.of("dt", Column.PhysicalKind.INT64, dtMeta);

    ColumnBuilder.set(intCol, 0, 42);
    ColumnBuilder.set(dateCol, 0, 19_737);
    ColumnBuilder.set(timeCol, 0, 3_600L);
    ColumnBuilder.set(tsCol, 0, 1_705_322_096L);
    ColumnBuilder.set(ymCol, 0, 14);
    ColumnBuilder.set(dtCol, 0, 5_000_000_000L);

    ColumnBuilder.setNull(intCol, 1);
    ColumnBuilder.setNull(dateCol, 1);
    ColumnBuilder.setNull(timeCol, 1);
    ColumnBuilder.setNull(tsCol, 1);
    ColumnBuilder.setNull(ymCol, 1);
    ColumnBuilder.setNull(dtCol, 1);

    for (Column col : new Column[] {intCol, dateCol, timeCol, tsCol, ymCol, dtCol}) {
      ColumnBuilder.setValueCount(col, 2);
    }

    ColumnBatch root = ColumnBatch.of(intCol, dateCol, timeCol, tsCol, ymCol, dtCol);
    root.setRowCount(2);

    DataContextStub context = new DataContextStub();
    ArrowResources res =
        new ArrowResources(new ColumnBatchListReader(Collections.singletonList(root)));
    try (ArrowRowReader reader = new ArrowRowReader(res, context, 2)) {
      assertSame(context, reader.getConversionContext());

      assertTrue(reader.next());
      assertEquals(BigDecimal.valueOf(42L), reader.getBigDecimal(1));
      assertFalse(reader.wasNull());
      assertArrayEquals(new byte[] {0, 0, 0, 42}, reader.getBytes(1));
      assertFalse(reader.wasNull());
      assertEquals(42L, reader.getObject(1));
      assertFalse(reader.wasNull());

      TimeZone utc = TimeZone.getTimeZone("UTC");
      assertEquals(Date.valueOf("2024-01-15"), reader.getDate(2, utc));
      assertFalse(reader.wasNull());
      assertEquals(new Time(3_600_000L), reader.getTime(3));
      assertFalse(reader.wasNull());
      assertEquals(new Timestamp(1_705_322_096_000L), reader.getTimestamp(4, utc));
      assertFalse(reader.wasNull());
      assertEquals(Period.of(1, 2, 0), reader.getPeriod(5));
      assertFalse(reader.wasNull());
      assertEquals(Duration.ofSeconds(5), reader.getDuration(6));
      assertFalse(reader.wasNull());

      assertTrue(reader.next());
      assertNull(reader.getBigDecimal(1));
      assertTrue(reader.wasNull());
      assertNull(reader.getBytes(1));
      assertTrue(reader.wasNull());
      assertNull(reader.getObject(1));
      assertTrue(reader.wasNull());
      assertNull(reader.getDate(2));
      assertTrue(reader.wasNull());
      assertNull(reader.getTime(3));
      assertTrue(reader.wasNull());
      assertNull(reader.getTimestamp(4));
      assertTrue(reader.wasNull());
      assertNull(reader.getPeriod(5));
      assertTrue(reader.wasNull());
      assertNull(reader.getDuration(6));
      assertTrue(reader.wasNull());
      assertNull(reader.getList(1));
      assertTrue(reader.wasNull());
    }
  }

  private static class DataContextStub implements DataConversionContext {}
}
