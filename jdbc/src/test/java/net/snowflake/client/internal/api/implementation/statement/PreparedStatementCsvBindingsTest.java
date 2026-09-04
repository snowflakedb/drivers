package net.snowflake.client.internal.api.implementation.statement;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.time.ZoneId;
import java.util.Arrays;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.statement.PreparedStatementBindingSerializer.NativeBindings;
import net.snowflake.client.internal.api.implementation.statement.PreparedStatementBindingSerializer.ParameterValue;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.BinaryDataPtr;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.QueryBindings;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.Test;

public class PreparedStatementCsvBindingsTest {

  // The CSV path allocates on the same shared allocator as the JSON serializer; a leak here means a
  // buffer was opened but not closed inside serialize().
  @AfterAll
  public static void assertSharedAllocatorEmpty() {
    assertEquals(
        0L,
        PreparedStatementBindingSerializer.SHARED_ALLOCATOR.getAllocatedMemory(),
        "ArrowBuf leak: shared allocator still has bytes after CSV binding tests");
  }

  private static ParameterValue column(String... rows) {
    return new ParameterValue(SnowflakeType.TEXT, Arrays.asList(rows));
  }

  private static ParameterValue column(SnowflakeType type, String... rows) {
    return new ParameterValue(type, Arrays.asList(rows));
  }

  private static long csvLength(NativeBindings nativeBindings) {
    QueryBindings bindings = nativeBindings.bindings();
    assertNotNull(bindings, "Expected non-null bindings");
    assertTrue(bindings.hasCsv(), "Expected CSV query bindings");
    assertFalse(bindings.hasJson(), "Stage bindings must not carry a JSON payload");
    return bindings.getCsv().getLength();
  }

  @Test
  public void shouldReturnNullBindingsForEmptyColumns() throws Exception {
    try (NativeBindings nativeBindings =
        PreparedStatementCsvBindings.serialize(new HashMap<>(), 0)) {
      assertNull(nativeBindings.bindings(), "Expected null bindings for an empty batch");
    }
  }

  @Test
  public void shouldReturnNullBindingsWhenRowCountIsZero() throws Exception {
    Map<Integer, ParameterValue> columns = new HashMap<>();
    columns.put(1, column());

    try (NativeBindings nativeBindings = PreparedStatementCsvBindings.serialize(columns, 0)) {
      assertNull(nativeBindings.bindings(), "Expected null bindings when no rows were accumulated");
    }
  }

  @Test
  public void shouldSerializeColumnsOrderedByParameterIndexAndTransposedToRows() throws Exception {
    Map<Integer, ParameterValue> columns = new HashMap<>();
    // Insert out of order to prove the serializer sorts by parameter index, not insertion order.
    columns.put(2, column("two", "x"));
    columns.put(1, column("1", "2"));

    // Every non-null cell is quoted (ODBC-style), so dropping the TreeMap ordering or transposing
    // the wrong way changes the bytes, not just their count.
    byte[] expected = "\"1\",\"two\"\n\"2\",\"x\"\n".getBytes(StandardCharsets.UTF_8);
    assertArrayEquals(expected, PreparedStatementCsvBindings.buildCsv(columns, 2));

    // Native path: the same bytes reach a non-null off-heap pointer of the expected length.
    try (NativeBindings nativeBindings = PreparedStatementCsvBindings.serialize(columns, 2)) {
      assertEquals(
          expected.length,
          csvLength(nativeBindings),
          "CSV byte length should match the column-ordered, row-major payload");

      BinaryDataPtr csvPtr = nativeBindings.bindings().getCsv();
      assertEquals(Long.BYTES, csvPtr.getValue().size(), "Pointer payload should be 8 bytes");
      long pointerValue =
          ByteBuffer.wrap(csvPtr.getValue().toByteArray()).order(ByteOrder.LITTLE_ENDIAN).getLong();
      assertNotEquals(0L, pointerValue, "Native pointer value should not be zero");
    }
  }

  @Test
  public void shouldQuoteEveryNonNullCellAndTerminateEveryRow() throws Exception {
    Map<Integer, ParameterValue> columns = new HashMap<>();
    // One column, seven single-value rows exercising each branch, in this order:
    // comma, embedded quote, newline, backslash, empty string, SQL NULL, multibyte UTF-8.
    columns.put(1, column("val,0", "say\"1\"", "a\nb", "C:\\dir\\3", "", null, "日本語"));

    byte[] expected =
        ("\"val,0\"\n" // comma → quoted
                + "\"say\"\"1\"\"\"\n" // embedded quote → quoted with the quote doubled
                + "\"a\nb\"\n" // newline → quoted
                + "\"C:\\dir\\3\"\n" // backslash → quoted, not doubled (CSV only doubles quotes)
                + "\"\"\n" // empty string → quoted empty
                + "\n" // SQL NULL → unquoted empty cell
                + "\"日本語\"\n") // plain text → still quoted (always-quote rule)
            .getBytes(StandardCharsets.UTF_8);

    assertArrayEquals(expected, PreparedStatementCsvBindings.buildCsv(columns, 7));
  }

  @Test
  public void shouldQuoteBareCarriageReturnUnlikeLegacyEscapeForCsv() throws Exception {
    Map<Integer, ParameterValue> columns = new HashMap<>();
    // A lone \r hits none of legacy escapeForCSV's triggers (" \n , \\) so legacy would emit it
    // bare; with escape_unenclosed_field=NONE that reaches the server as a raw record delimiter.
    // The always-quote rule wraps it, matching ODBC's append_escaped_csv_cell.
    columns.put(1, column("\r"));

    byte[] expected = "\"\r\"\n".getBytes(StandardCharsets.UTF_8);
    assertArrayEquals(expected, PreparedStatementCsvBindings.buildCsv(columns, 1));
  }

  @Test
  public void shouldQuoteEmptyStringButLeaveNullAsAnEmptyCell() throws Exception {
    Map<Integer, ParameterValue> columns = new HashMap<>();
    // "" serializes to a quoted empty field; SQL NULL serializes to nothing, so the two are
    // distinguishable byte-for-byte ("\"\"\n" vs "\n").
    columns.put(1, column("", null));

    byte[] expected = "\"\"\n\n".getBytes(StandardCharsets.UTF_8);
    assertArrayEquals(expected, PreparedStatementCsvBindings.buildCsv(columns, 2));
  }

  @Test
  public void shouldFormatDateEpochMillisForStageCsv() throws Exception {
    Map<Integer, ParameterValue> columns = new HashMap<>();
    columns.put(
        1, column(SnowflakeType.DATE, "0", "86400000", "-86400000", "-62135596800000", null));

    byte[] expected =
        ("\"1970-01-01\"\n" + "\"1970-01-02\"\n" + "\"1969-12-31\"\n" + "\"0001-01-01\"\n" + "\n")
            .getBytes(StandardCharsets.UTF_8);
    assertArrayEquals(expected, PreparedStatementCsvBindings.buildCsv(columns, 5));
  }

  @Test
  public void shouldFormatTimestampEpochNanosForStageCsv() throws Exception {
    assertEquals(
        "1969-12-31 23:59:59.000000000 Z",
        PreparedStatementCsvBindings.formatStageValue(
            SnowflakeType.TIMESTAMP_NTZ, "-1000000000", ZoneId.of("America/Los_Angeles")));
    assertEquals(
        "1899-12-31 23:59:59.900000000 Z",
        PreparedStatementCsvBindings.formatStageValue(
            SnowflakeType.TIMESTAMP_NTZ, "-2208988800100000000", ZoneId.of("America/Los_Angeles")));
    assertEquals(
        "0001-01-01 00:00:00.000000000 Z",
        PreparedStatementCsvBindings.formatStageValue(
            SnowflakeType.TIMESTAMP_NTZ,
            "-62135596800000000000",
            ZoneId.of("America/Los_Angeles")));
    assertEquals(
        "1969-12-31 15:59:59.000000000 -08:00",
        PreparedStatementCsvBindings.formatStageValue(
            SnowflakeType.TIMESTAMP_LTZ, "-1000000000", ZoneId.of("America/Los_Angeles")));
  }

  @Test
  public void shouldFormatTimeNanosOfDayForStageCsv() throws Exception {
    Map<Integer, ParameterValue> columns = new HashMap<>();
    columns.put(
        1,
        column(
            SnowflakeType.TIME,
            "0",
            "1000000000",
            "46800000000000",
            "50400000000000",
            "86399999000000",
            null));

    byte[] expected =
        ("\"00:00:00.000000000\"\n"
                + "\"00:00:01.000000000\"\n"
                + "\"13:00:00.000000000\"\n"
                + "\"14:00:00.000000000\"\n"
                + "\"23:59:59.999000000\"\n"
                + "\n")
            .getBytes(StandardCharsets.UTF_8);
    assertArrayEquals(expected, PreparedStatementCsvBindings.buildCsv(columns, 6));
  }
}
