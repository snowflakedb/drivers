package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.sql.Timestamp;
import java.util.HashMap;
import java.util.Map;
import java.util.TimeZone;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
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
    } finally {
      vector.close();
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
