package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.nio.ByteBuffer;
import java.sql.Date;
import java.sql.Time;
import java.sql.Timestamp;
import java.util.HashMap;
import java.util.Map;
import java.util.TimeZone;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.common.core.SnowflakeDateTimeFormat;
import net.snowflake.client.internal.core.arrow.TestHelper;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import net.snowflake.client.internal.jdbc.SnowflakeTimestampWithTimezone;
import org.junit.jupiter.api.Test;

public class BigIntToTimestampLTZConverterTest extends BaseConverterTest {
  // 2020-01-15T00:00:00Z — post-1582, so Julian→Gregorian adjustTimestamp is a no-op.
  private static final long EPOCH_SECONDS = 1_579_046_400L;

  private static Map<String, String> timestampLtzMeta(int scale) {
    Map<String, String> meta = new HashMap<>();
    meta.put("logicalType", "TIMESTAMP_LTZ");
    meta.put("scale", String.valueOf(scale));
    return meta;
  }

  private Column createVector(int scale, long... values) {
    Column vector =
        ColumnBuilder.of("col_ts_ltz", Column.PhysicalKind.INT64, timestampLtzMeta(scale));
    for (int i = 0; i < values.length; i++) {
      ColumnBuilder.set(vector, i, values[i]);
    }
    ColumnBuilder.setValueCount(vector, values.length);
    return vector;
  }

  @Test
  public void shouldConvertScaledEpochToTimestamp() throws Exception {
    int scale = 3;
    // Compact Int64 at scale 3: whole seconds * 10^3 + millisecond fraction.
    long compact = EPOCH_SECONDS * 1_000L + 456L;
    Column vector = createVector(scale, compact);
    try {
      BigIntToTimestampLTZConverter converter =
          new BigIntToTimestampLTZConverter(vector, 0, this, scale);
      Timestamp expected = new Timestamp(EPOCH_SECONDS * 1_000L);
      expected.setNanos(456_000_000); // 456 * 10^(9-3)
      assertEquals(expected, converter.toTimestamp(0, TimeZone.getTimeZone("Europe/Warsaw")));
      assertEquals(expected, converter.toObject(0));
      assertEquals("Tue, 14 Jan 2020 16:00:00 -0800", converter.toString(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldReturnNullForNullRow() throws Exception {
    Column vector = ColumnBuilder.of("col_ts_ltz", Column.PhysicalKind.INT64, timestampLtzMeta(0));
    ColumnBuilder.set(vector, 0, EPOCH_SECONDS);
    ColumnBuilder.setNull(vector, 1);
    ColumnBuilder.setValueCount(vector, 2);
    try {
      BigIntToTimestampLTZConverter converter =
          new BigIntToTimestampLTZConverter(vector, 0, this, 0);
      assertNotNull(converter.toTimestamp(0, null));
      assertNull(converter.toTimestamp(1, null));
      assertNull(converter.toObject(1));
      assertNull(converter.toString(1));
      assertNull(converter.toDate(1, null, false));
      assertNull(converter.toTime(1));
      assertNull(converter.toBytes(1));
      assertFalse(converter.toBoolean(1));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldConvertToDateAndTimeAndBytesAndReadReloadedValues() throws Exception {
    int scale = 3;
    long compact = EPOCH_SECONDS * 1_000L + 456L;
    try (Column vector = createVector(scale, compact)) {
      BigIntToTimestampLTZConverter converter =
          new BigIntToTimestampLTZConverter(vector, 0, this, scale);

      long expectedMillis = EPOCH_SECONDS * 1_000L + 456L;
      byte[] expectedBytes = ByteBuffer.allocate(Long.BYTES).putLong(compact).array();
      assertEquals(new Date(expectedMillis), converter.toDate(0, null, false));
      assertEquals(new Time(expectedMillis), converter.toTime(0));
      assertArrayEquals(expectedBytes, converter.toBytes(0));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(0));

      long reloaded = (EPOCH_SECONDS + 3_600L) * 1_000L;
      ColumnBuilder.set(vector, 0, reloaded);
      assertEquals(new Timestamp(reloaded), converter.toTimestamp(0, null));
      assertEquals("Tue, 14 Jan 2020 17:00:00 -0800", converter.toString(0));
    }
  }

  @Test
  public void shouldReturnSessionTimezoneWrapperAndRejectMissingFormatter() throws Exception {
    DataConversionContext sessionTzCtx =
        new DataConversionContext() {
          @Override
          public boolean isUseSessionTimezone() {
            return true;
          }
        };
    DataConversionContext missingFormatterCtx =
        new DataConversionContext() {
          @Override
          public SnowflakeDateTimeFormat getTimestampLTZFormatter() {
            return null;
          }
        };

    try (Column vector = createVector(0, EPOCH_SECONDS)) {
      BigIntToTimestampLTZConverter sessionTzConverter =
          new BigIntToTimestampLTZConverter(vector, 0, sessionTzCtx, 0);
      Timestamp ts = sessionTzConverter.toTimestamp(0, null);
      assertInstanceOf(SnowflakeTimestampWithTimezone.class, ts);
      assertEquals(EPOCH_SECONDS * 1_000L, ts.getTime());

      BigIntToTimestampLTZConverter missingFormatterConverter =
          new BigIntToTimestampLTZConverter(vector, 0, missingFormatterCtx, 0);
      SFSQLException error =
          assertThrows(SFSQLException.class, () -> missingFormatterConverter.toString(0));
      assertEquals(ErrorCode.INTERNAL_ERROR, error.getErrorCode());
      assertEquals("INTERNAL_ERROR: [missing timestamp LTZ formatter]", error.getMessage());
    }
  }
}
