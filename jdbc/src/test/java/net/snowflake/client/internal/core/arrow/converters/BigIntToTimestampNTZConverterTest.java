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
import net.snowflake.client.internal.jdbc.SnowflakeTimeWithTimezone;
import org.junit.jupiter.api.Test;

public class BigIntToTimestampNTZConverterTest extends BaseConverterTest {
  // 2020-01-15T00:00:00Z — post-1582, so Julian→Gregorian adjustTimestamp is a no-op.
  private static final long EPOCH_SECONDS = 1_579_046_400L;

  private static Map<String, String> timestampNtzMeta(int scale) {
    Map<String, String> meta = new HashMap<>();
    meta.put("logicalType", "TIMESTAMP_NTZ");
    meta.put("scale", String.valueOf(scale));
    return meta;
  }

  private Column createVector(int scale, long... values) {
    Column vector =
        ColumnBuilder.of("col_ts_ntz", Column.PhysicalKind.INT64, timestampNtzMeta(scale));
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
      BigIntToTimestampNTZConverter converter =
          new BigIntToTimestampNTZConverter(vector, 0, noClientTzShift(), scale);
      Timestamp expected = new Timestamp(EPOCH_SECONDS * 1_000L);
      expected.setNanos(456_000_000); // 456 * 10^(9-3)
      assertEquals(expected, converter.toTimestamp(0, TimeZone.getTimeZone("UTC")));
      assertEquals(expected, converter.toObject(0));
      assertEquals("Wed, 15 Jan 2020 00:00:00 Z", converter.toString(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldReturnNullForNullRow() throws Exception {
    Column vector = ColumnBuilder.of("col_ts_ntz", Column.PhysicalKind.INT64, timestampNtzMeta(0));
    ColumnBuilder.set(vector, 0, EPOCH_SECONDS);
    ColumnBuilder.setNull(vector, 1);
    ColumnBuilder.setValueCount(vector, 2);
    try {
      BigIntToTimestampNTZConverter converter =
          new BigIntToTimestampNTZConverter(vector, 0, noClientTzShift(), 0);
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
  public void shouldShiftToCallerTimezoneWhenHonorClientTzEnabledWhileToStringRemainsUtc()
      throws Exception {
    int scale = 3;
    long compact = EPOCH_SECONDS * 1_000L + 456L;
    try (Column vector = createVector(scale, compact)) {
      BigIntToTimestampNTZConverter converter =
          new BigIntToTimestampNTZConverter(vector, 0, this, scale);

      // 2020-01-15 in America/Los_Angeles is PST (UTC-8), so UTC->LA shift adds +8 hours.
      Timestamp shifted = converter.toTimestamp(0, TimeZone.getTimeZone("America/Los_Angeles"));
      Timestamp expectedShifted = new Timestamp(EPOCH_SECONDS * 1_000L + 8L * 3_600_000L);
      expectedShifted.setNanos(456_000_000);
      assertEquals(expectedShifted, shifted);

      // toString ignores honorClientTZForTimestampNTZ and always renders in UTC.
      assertEquals("Wed, 15 Jan 2020 00:00:00 Z", converter.toString(0));
    }
  }

  @Test
  public void shouldConvertToDateAndTimeAndBytesAndReadReloadedValues() throws Exception {
    int scale = 3;
    long compact = EPOCH_SECONDS * 1_000L + 456L;
    try (Column vector = createVector(scale, compact)) {
      BigIntToTimestampNTZConverter converter =
          new BigIntToTimestampNTZConverter(vector, 0, noClientTzShift(), scale);

      long expectedMillis = EPOCH_SECONDS * 1_000L + 456L;
      byte[] expectedBytes = ByteBuffer.allocate(Long.BYTES).putLong(compact).array();
      assertEquals(new Date(expectedMillis), converter.toDate(0, null, false));
      Time time = converter.toTime(0);
      assertInstanceOf(SnowflakeTimeWithTimezone.class, time);
      assertEquals(expectedMillis, time.getTime());
      assertArrayEquals(expectedBytes, converter.toBytes(0));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(0));

      long reloaded = (EPOCH_SECONDS + 86_400L) * 1_000L;
      ColumnBuilder.set(vector, 0, reloaded);
      assertEquals(new Timestamp(reloaded), converter.toTimestamp(0, null));
      assertEquals("Thu, 16 Jan 2020 00:00:00 Z", converter.toString(0));
    }
  }

  @Test
  public void shouldThrowInternalErrorWhenTimestampNtzFormatterIsMissing() throws Exception {
    DataConversionContext missingFormatterCtx =
        new DataConversionContext() {
          @Override
          public SnowflakeDateTimeFormat getTimestampNTZFormatter() {
            return null;
          }
        };
    try (Column vector = createVector(0, EPOCH_SECONDS)) {
      BigIntToTimestampNTZConverter converter =
          new BigIntToTimestampNTZConverter(vector, 0, missingFormatterCtx, 0);
      SFSQLException error = assertThrows(SFSQLException.class, () -> converter.toString(0));
      assertEquals(ErrorCode.INTERNAL_ERROR, error.getErrorCode());
      assertEquals("INTERNAL_ERROR: [missing timestamp NTZ formatter]", error.getMessage());
    }
  }

  private static DataConversionContext noClientTzShift() {
    return new DataConversionContext() {
      @Override
      public boolean isHonorClientTZForTimestampNTZ() {
        return false;
      }
    };
  }
}
